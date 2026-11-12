//! Host key verification: trust on first use with an **explicit** confirmation, then pinning.
//!
//! - Unknown host: the connection is dropped before authentication (no password or key signature is
//!   sent) and the UI shows the SHA256 fingerprint; only `trust` (the user confirmed it, ideally
//!   against the provider's console) pins it.
//! - Pinned host, same key: go on.
//! - Pinned host, another key: hard error. The pin is only removed by an explicit `forget`.
//!
//! Pins are kept in `<launcher data>/hosting/known_hosts.json` (public data: fingerprints, no
//! secrets), written atomically.

use crate::error::{HostingError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A presented key, reduced to what we pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKey {
    /// `ssh-ed25519`, `ecdsa-sha2-nistp256`, `rsa-sha2-512`...
    pub algorithm: String,
    /// `SHA256:<base64>` as OpenSSH prints it.
    pub fingerprint: String,
}

impl HostKey {
    pub fn from_public_key(key: &russh::keys::PublicKey) -> Self {
        Self {
            algorithm: key.algorithm().to_string(),
            fingerprint: key.fingerprint(russh::keys::HashAlg::Sha256).to_string(),
        }
    }
}

/// `host:port` as the pin key (host lowercased; IPv6 in brackets).
pub fn host_id(host: &str, port: u16) -> String {
    let h = host.trim().to_ascii_lowercase();
    if h.contains(':') && !h.starts_with('[') {
        format!("[{h}]:{port}")
    } else {
        format!("{h}:{port}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub algorithm: String,
    pub fingerprint: String,
    /// Seconds since 1970.
    pub pinned_at: u64,
}

/// The decision for a presented key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Trusted,
    Unknown(HostKey),
    Mismatch { expected: String, actual: HostKey },
}

impl Verdict {
    pub fn into_result(self, host: &str) -> Result<()> {
        match self {
            Self::Trusted => Ok(()),
            Self::Unknown(k) => Err(HostingError::HostKeyUnknown {
                host: host.to_owned(),
                algorithm: k.algorithm,
                fingerprint: k.fingerprint,
            }),
            Self::Mismatch { expected, actual } => Err(HostingError::HostKeyMismatch {
                host: host.to_owned(),
                expected,
                actual: actual.fingerprint,
            }),
        }
    }
}

/// The pin decision, pure. Fingerprint comparison is exact (they are canonical base64).
pub fn verify(pins: &BTreeMap<String, Pin>, host: &str, presented: &HostKey) -> Verdict {
    match pins.get(host) {
        None => Verdict::Unknown(presented.clone()),
        Some(pin) if pin.fingerprint == presented.fingerprint => Verdict::Trusted,
        Some(pin) => Verdict::Mismatch {
            expected: pin.fingerprint.clone(),
            actual: presented.clone(),
        },
    }
}

/// The pin file.
#[derive(Debug)]
pub struct KnownHosts {
    path: Option<PathBuf>,
    pins: Mutex<BTreeMap<String, Pin>>,
}

impl KnownHosts {
    /// Load (or start) the file at `path`.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let pins = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| HostingError::Invalid(format!("{}: {e}", path.display())))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path: Some(path),
            pins: Mutex::new(pins),
        })
    }

    /// Not persisted (tests).
    pub fn in_memory() -> Self {
        Self {
            path: None,
            pins: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn verify(&self, host: &str, presented: &HostKey) -> Verdict {
        verify(&self.pins.lock().unwrap(), host, presented)
    }

    pub fn get(&self, host: &str) -> Option<Pin> {
        self.pins.lock().unwrap().get(host).cloned()
    }

    /// Pin a key the user confirmed. Refuses to silently replace a different pin: `forget` first.
    pub fn trust(&self, host: &str, key: &HostKey) -> Result<()> {
        if !key.fingerprint.starts_with("SHA256:") {
            return Err(HostingError::Invalid(
                "a fingerprint to trust must be the SHA256 one (SHA256:...)".into(),
            ));
        }
        {
            let mut pins = self.pins.lock().unwrap();
            if let Some(old) = pins.get(host)
                && old.fingerprint != key.fingerprint
            {
                return Err(HostingError::HostKeyMismatch {
                    host: host.to_owned(),
                    expected: old.fingerprint.clone(),
                    actual: key.fingerprint.clone(),
                });
            }
            pins.insert(
                host.to_owned(),
                Pin {
                    algorithm: key.algorithm.clone(),
                    fingerprint: key.fingerprint.clone(),
                    pinned_at: now(),
                },
            );
        }
        self.save()
    }

    /// Remove a pin (the user reinstalled the VPS and checked the new key).
    pub fn forget(&self, host: &str) -> Result<()> {
        self.pins.lock().unwrap().remove(host);
        self.save()
    }

    fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let text = serde_json::to_string_pretty(&*self.pins.lock().unwrap())?;
        write_atomic(path, text.as_bytes())
    }
}

pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Write through a temp file and rename (no half-written pin file after a crash).
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(fp: &str) -> HostKey {
        HostKey {
            algorithm: "ssh-ed25519".into(),
            fingerprint: fp.into(),
        }
    }

    #[test]
    fn host_ids() {
        assert_eq!(host_id("VPS.example.com", 22), "vps.example.com:22");
        assert_eq!(host_id("2001:db8::1", 2222), "[2001:db8::1]:2222");
        assert_eq!(host_id("[2001:db8::1]", 22), "[2001:db8::1]:22");
    }

    #[test]
    fn tofu_then_pin_then_mismatch() {
        let kh = KnownHosts::in_memory();
        let host = host_id("vps", 22);
        let a = key("SHA256:aaaa");
        let b = key("SHA256:bbbb");
        assert_eq!(kh.verify(&host, &a), Verdict::Unknown(a.clone()));
        let err = kh.verify(&host, &a).into_result(&host).unwrap_err();
        assert!(matches!(err, HostingError::HostKeyUnknown { .. }));
        kh.trust(&host, &a).unwrap();
        assert_eq!(kh.verify(&host, &a), Verdict::Trusted);
        let v = kh.verify(&host, &b);
        assert!(matches!(v, Verdict::Mismatch { .. }));
        let err = v.into_result(&host).unwrap_err();
        assert_eq!(err.code(), "host_key_mismatch");
        // Trusting another key over a pin is refused: forget first.
        assert!(kh.trust(&host, &b).is_err());
        kh.forget(&host).unwrap();
        kh.trust(&host, &b).unwrap();
        assert_eq!(kh.verify(&host, &b), Verdict::Trusted);
        // Pins are per host:port.
        assert!(matches!(
            kh.verify(&host_id("vps", 2222), &b),
            Verdict::Unknown(_)
        ));
    }

    #[test]
    fn only_sha256_fingerprints_are_trusted() {
        let kh = KnownHosts::in_memory();
        assert!(kh.trust("h:22", &key("MD5:aa:bb")).is_err());
    }

    #[test]
    fn pins_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosting/known_hosts.json");
        let kh = KnownHosts::open(&path).unwrap();
        kh.trust("h:22", &key("SHA256:zz")).unwrap();
        let again = KnownHosts::open(&path).unwrap();
        assert_eq!(again.verify("h:22", &key("SHA256:zz")), Verdict::Trusted);
        assert_eq!(again.get("h:22").unwrap().algorithm, "ssh-ed25519");
    }

    #[test]
    fn real_key_fingerprint_format() {
        let k = russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)
            .unwrap();
        let hk = HostKey::from_public_key(k.public_key());
        assert_eq!(hk.algorithm, "ssh-ed25519");
        assert!(hk.fingerprint.starts_with("SHA256:"));
        assert_eq!(hk.fingerprint.len(), "SHA256:".len() + 43);
    }
}
