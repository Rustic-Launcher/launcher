//! The launcher's list of VPS profiles and the servers it manages on them, in
//! `<launcher data>/hosting/registry.json`. No secrets here (they are in the keychain); the truth
//! about a server (installed? running?) is always read from the VPS itself.

use crate::error::{HostingError, Result};
use crate::model::ServerRef;
use crate::remote::RemoteProfile;
use crate::shell::validate_name;
use crate::ssh::hostkey::write_atomic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryData {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub profiles: Vec<RemoteProfile>,
    #[serde(default)]
    pub servers: Vec<ServerRef>,
}

pub struct Registry {
    path: Option<PathBuf>,
    data: Mutex<RegistryData>,
}

impl Registry {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let data = match std::fs::read_to_string(&path) {
            Ok(t) => serde_json::from_str(&t)
                .map_err(|e| HostingError::Invalid(format!("{}: {e}", path.display())))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => RegistryData {
                version: 1,
                ..Default::default()
            },
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path: Some(path),
            data: Mutex::new(data),
        })
    }

    pub fn in_memory() -> Self {
        Self {
            path: None,
            data: Mutex::new(RegistryData::default()),
        }
    }

    pub fn snapshot(&self) -> RegistryData {
        self.data.lock().unwrap().clone()
    }

    pub fn profile(&self, id: &str) -> Option<RemoteProfile> {
        self.data
            .lock()
            .unwrap()
            .profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    pub fn upsert_profile(&self, profile: RemoteProfile) -> Result<()> {
        validate_name(&profile.id)?;
        {
            let mut d = self.data.lock().unwrap();
            d.profiles.retain(|p| p.id != profile.id);
            d.profiles.push(profile);
            d.profiles.sort_by(|a, b| a.id.cmp(&b.id));
        }
        self.save()
    }

    /// Remove a profile and its servers from the list (nothing is touched on the VPS).
    pub fn remove_profile(&self, id: &str) -> Result<()> {
        {
            let mut d = self.data.lock().unwrap();
            d.profiles.retain(|p| p.id != id);
            d.servers.retain(|s| s.target != id);
        }
        self.save()
    }

    pub fn add_server(&self, server: ServerRef) -> Result<()> {
        {
            let mut d = self.data.lock().unwrap();
            if !d.servers.contains(&server) {
                d.servers.push(server);
            }
        }
        self.save()
    }

    pub fn remove_server(&self, server: &ServerRef) -> Result<()> {
        self.data.lock().unwrap().servers.retain(|s| s != server);
        self.save()
    }

    fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let text = serde_json::to_string_pretty(&*self.data.lock().unwrap())?;
        write_atomic(path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProviderKind;
    use crate::remote::AuthMethod;
    use crate::ssh::SshTarget;

    #[test]
    fn persists_profiles_and_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosting/registry.json");
        let r = Registry::open(&path).unwrap();
        r.upsert_profile(RemoteProfile {
            id: "vps".into(),
            label: "VPS".into(),
            target: SshTarget {
                host: "h".into(),
                port: 22,
                user: "u".into(),
            },
            auth: AuthMethod::KeychainKey,
        })
        .unwrap();
        let s = ServerRef {
            provider: ProviderKind::RemoteSsh,
            target: "vps".into(),
            name: "survival".into(),
        };
        r.add_server(s.clone()).unwrap();
        r.add_server(s.clone()).unwrap();
        let again = Registry::open(&path).unwrap();
        assert_eq!(again.snapshot().servers, vec![s]);
        assert!(again.profile("vps").is_some());
        assert!(
            r.upsert_profile(RemoteProfile {
                id: "Bad Id".into(),
                ..again.profile("vps").unwrap()
            })
            .is_err()
        );
        again.remove_profile("vps").unwrap();
        assert!(Registry::open(&path).unwrap().snapshot().servers.is_empty());
    }
}
