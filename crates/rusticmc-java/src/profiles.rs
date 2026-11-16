//! The player's Java Edition profiles (`<data>/java/profiles.json`): a name, a game version, a
//! mod loader, memory, resolution, Java and JVM settings, and a game folder of its own (its
//! `mods/`, worlds and options). Kept apart from Rustic instances (ADR 0172).

use crate::loaders::LoaderKind;
use crate::{JavaError, Result};
use rusticmc_launcher_core::fsutil;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaProfile {
    pub id: String,
    pub name: String,
    /// Mojang's version id (`1.21.1`, `26.3`, `26.4-snapshot-8`).
    pub game_version: String,
    pub loader: LoaderKind,
    /// The loader's version (none for vanilla; the newest stable one when left empty).
    #[serde(default)]
    pub loader_version: Option<String>,
    /// The version file the profile starts (set by its install: `fabric-loader-...`).
    #[serde(default)]
    pub version_id: Option<String>,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    #[serde(default)]
    pub resolution: Option<Resolution>,
    #[serde(default)]
    pub fullscreen: bool,
    /// A Java executable or home of the player's choice (else Mojang's runtime).
    #[serde(default)]
    pub java_path: Option<String>,
    /// The player's JVM arguments (replace the version's suggested ones).
    #[serde(default)]
    pub jvm_args: Vec<String>,
    /// A game folder of the player's choice (else `<data>/java/profiles/<id>`).
    #[serde(default)]
    pub game_dir: Option<String>,
    pub created_at: u64,
    #[serde(default)]
    pub last_played_at: Option<u64>,
    /// Seconds played.
    #[serde(default)]
    pub play_time: u64,
}

/// What can change on a profile.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProfilePatch {
    pub name: Option<String>,
    pub game_version: Option<String>,
    pub loader: Option<LoaderKind>,
    /// `Some(None)` clears it.
    #[serde(default, with = "double_option")]
    pub loader_version: Option<Option<String>>,
    pub memory_min_mb: Option<u32>,
    pub memory_max_mb: Option<u32>,
    #[serde(default, with = "double_option")]
    pub resolution: Option<Option<Resolution>>,
    pub fullscreen: Option<bool>,
    #[serde(default, with = "double_option")]
    pub java_path: Option<Option<String>>,
    pub jvm_args: Option<Vec<String>>,
    #[serde(default, with = "double_option")]
    pub game_dir: Option<Option<String>>,
}

mod double_option {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Ok(Some(Option::deserialize(d)?))
    }
}

/// A new profile.
#[derive(Clone, Debug, Deserialize)]
pub struct NewProfile {
    pub name: String,
    pub game_version: String,
    pub loader: LoaderKind,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub memory_max_mb: Option<u32>,
}

pub struct ProfileStore {
    file: PathBuf,
    dir: PathBuf,
    lock: Mutex<()>,
}

/// Memory bounds a profile accepts (MB).
pub const MIN_MEMORY_MB: u32 = 512;
pub const MAX_MEMORY_MB: u32 = 65_536;

impl ProfileStore {
    /// Profiles in `<java>/profiles.json`, their default game folders under `<java>/profiles/`.
    pub fn new(java_dir: &Path) -> Self {
        Self {
            file: java_dir.join("profiles.json"),
            dir: java_dir.join("profiles"),
            lock: Mutex::new(()),
        }
    }

    pub fn list(&self) -> Result<Vec<JavaProfile>> {
        Ok(fsutil::read_json::<Vec<JavaProfile>>(&self.file)?.unwrap_or_default())
    }

    fn save(&self, list: &[JavaProfile]) -> Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| JavaError::io(parent, e))?;
        }
        fsutil::write_json(&self.file, &list)?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<JavaProfile> {
        self.list()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| JavaError::UnknownProfile(id.to_owned()))
    }

    /// The profile's game folder.
    pub fn game_dir(&self, p: &JavaProfile) -> PathBuf {
        match p.game_dir.as_deref().filter(|d| !d.trim().is_empty()) {
            Some(d) => PathBuf::from(d),
            None => self.dir.join(&p.id),
        }
    }

    pub fn create(&self, new: NewProfile, default_memory_mb: u32, now: u64) -> Result<JavaProfile> {
        let name = clean_name(&new.name)?;
        if !crate::version::safe_id(&new.game_version) {
            return Err(JavaError::Invalid("pick a game version".into()));
        }
        let max = new
            .memory_max_mb
            .unwrap_or(default_memory_mb)
            .clamp(MIN_MEMORY_MB, MAX_MEMORY_MB);
        let profile = JavaProfile {
            id: format!("java-{}", uuid::Uuid::new_v4().simple()),
            name,
            game_version: new.game_version,
            loader: new.loader,
            loader_version: new
                .loader_version
                .filter(|v| !v.is_empty() && new.loader != LoaderKind::Vanilla),
            version_id: None,
            memory_min_mb: MIN_MEMORY_MB.min(max),
            memory_max_mb: max,
            resolution: None,
            fullscreen: false,
            java_path: None,
            jvm_args: Vec::new(),
            game_dir: None,
            created_at: now,
            last_played_at: None,
            play_time: 0,
        };
        let _g = self.lock.lock().expect("profiles");
        let mut list = self.list()?;
        list.push(profile.clone());
        self.save(&list)?;
        Ok(profile)
    }

    /// Applies `patch`; a new version or loader forgets the installed version file.
    pub fn update(&self, id: &str, patch: ProfilePatch) -> Result<JavaProfile> {
        self.modify(id, |p| {
            if let Some(n) = &patch.name {
                p.name = clean_name(n)?;
            }
            let mut reinstall = false;
            if let Some(v) = &patch.game_version {
                if !crate::version::safe_id(v) {
                    return Err(JavaError::Invalid("bad game version".into()));
                }
                reinstall |= *v != p.game_version;
                p.game_version = v.clone();
            }
            if let Some(l) = patch.loader {
                reinstall |= l != p.loader;
                p.loader = l;
            }
            if let Some(lv) = &patch.loader_version {
                let lv = lv.clone().filter(|v| !v.is_empty());
                reinstall |= lv != p.loader_version;
                p.loader_version = lv;
            }
            if p.loader == LoaderKind::Vanilla {
                p.loader_version = None;
            }
            if reinstall {
                p.version_id = None;
            }
            if let Some(m) = patch.memory_max_mb {
                p.memory_max_mb = m.clamp(MIN_MEMORY_MB, MAX_MEMORY_MB);
            }
            if let Some(m) = patch.memory_min_mb {
                p.memory_min_mb = m.clamp(MIN_MEMORY_MB, MAX_MEMORY_MB);
            }
            p.memory_min_mb = p.memory_min_mb.min(p.memory_max_mb);
            if let Some(r) = &patch.resolution {
                if let Some(r) = r
                    && (r.width < 320 || r.height < 240 || r.width > 16_384 || r.height > 16_384)
                {
                    return Err(JavaError::Invalid("resolution out of range".into()));
                }
                p.resolution = r.clone();
            }
            if let Some(f) = patch.fullscreen {
                p.fullscreen = f;
            }
            if let Some(j) = &patch.java_path {
                p.java_path = j.clone().filter(|s| !s.trim().is_empty());
            }
            if let Some(a) = &patch.jvm_args {
                p.jvm_args = a
                    .iter()
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            if let Some(d) = &patch.game_dir {
                p.game_dir = d.clone().filter(|s| !s.trim().is_empty());
            }
            Ok(())
        })
    }

    /// Changes a profile under the store's lock.
    pub fn modify(
        &self,
        id: &str,
        f: impl FnOnce(&mut JavaProfile) -> Result<()>,
    ) -> Result<JavaProfile> {
        let _g = self.lock.lock().expect("profiles");
        let mut list = self.list()?;
        let p = list
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| JavaError::UnknownProfile(id.to_owned()))?;
        f(p)?;
        let out = p.clone();
        self.save(&list)?;
        Ok(out)
    }

    /// Removes a profile; its default game folder too when `delete_files` (never a folder the
    /// player chose).
    pub fn delete(&self, id: &str, delete_files: bool) -> Result<()> {
        let _g = self.lock.lock().expect("profiles");
        let mut list = self.list()?;
        let Some(pos) = list.iter().position(|p| p.id == id) else {
            return Err(JavaError::UnknownProfile(id.to_owned()));
        };
        let p = list.remove(pos);
        self.save(&list)?;
        if delete_files && p.game_dir.is_none() {
            let dir = self.dir.join(&p.id);
            if dir.starts_with(&self.dir) {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
        Ok(())
    }
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 64 {
        return Err(JavaError::Invalid("a name of 1 to 64 characters".into()));
    }
    Ok(n.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(name: &str) -> NewProfile {
        NewProfile {
            name: name.into(),
            game_version: "1.21.1".into(),
            loader: LoaderKind::Fabric,
            loader_version: None,
            memory_max_mb: None,
        }
    }

    #[test]
    fn create_update_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(tmp.path());
        let p = store.create(new(" Fabric 1.21 "), 4096, 1).unwrap();
        assert_eq!(p.name, "Fabric 1.21");
        assert_eq!(p.memory_max_mb, 4096);
        assert_eq!(store.game_dir(&p), tmp.path().join("profiles").join(&p.id));
        store
            .modify(&p.id, |p| {
                p.version_id = Some("fabric-loader-0.17.2-1.21.1".into());
                Ok(())
            })
            .unwrap();
        // Memory is clamped; a new version forgets the installed version file.
        let patch: ProfilePatch = serde_json::from_value(serde_json::json!({
            "memory_max_mb": 100, "game_version": "1.20.1", "resolution": {"width": 1280, "height": 720}
        }))
        .unwrap();
        let u = store.update(&p.id, patch).unwrap();
        assert_eq!(u.memory_max_mb, MIN_MEMORY_MB);
        assert!(u.version_id.is_none());
        assert_eq!(u.resolution.as_ref().unwrap().width, 1280);
        let clear: ProfilePatch =
            serde_json::from_value(serde_json::json!({"resolution": null, "loader": "vanilla"}))
                .unwrap();
        let c = store.update(&p.id, clear).unwrap();
        assert!(c.resolution.is_none());
        assert!(c.loader_version.is_none());
        std::fs::create_dir_all(store.game_dir(&c).join("mods")).unwrap();
        store.delete(&p.id, true).unwrap();
        assert!(!store.game_dir(&c).exists());
        assert!(store.list().unwrap().is_empty());
        assert!(store.create(new(""), 4096, 1).is_err());
    }
}
