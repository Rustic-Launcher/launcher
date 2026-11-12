//! Secrets: SSH private keys and key passphrases (passwords are used once, never stored). They live **only** in the OS keychain
//! (`keyring`, ADR 0123): never in a config file, a log line, an IPC reply or a command line.
//!
//! Windows' Credential Manager caps a secret at 2560 bytes (UTF-16, so 1280 characters), less than
//! an RSA-4096 private key in PEM. [`KeyringStore`] therefore splits long values into numbered
//! chunks (`<name>#0`, `<name>#1`... with a `<name>` header holding the count).

use crate::error::{HostingError, Result};
use std::collections::HashMap;
use std::sync::Mutex;

/// A string that never prints. Same contract as `rusticmc_launcher_core::Secret` (kept local so this
/// crate builds on its own; see HANDOFF.md).
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    /// The value, only where it leaves for its one destination (the SSH library, the keychain).
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl From<&str> for Secret {
    fn from(v: &str) -> Self {
        Self(v.to_owned())
    }
}

impl From<String> for Secret {
    fn from(v: String) -> Self {
        Self(v)
    }
}

/// Where secrets are kept. The real one is the OS keychain; tests use [`MemoryStore`].
pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<Secret>>;
    fn set(&self, name: &str, value: &Secret) -> Result<()>;
    fn delete(&self, name: &str) -> Result<()>;
}

/// The keychain service name (one per app; entries are `hosting/<profile>/<what>`).
pub const KEYCHAIN_SERVICE: &str = "gg.rustic.launcher.hosting";
/// Characters per keychain entry (Windows: 2560 bytes of UTF-16 at most).
const CHUNK_CHARS: usize = 1000;
const CHUNKED_PREFIX: &str = "rustic-chunked:";

/// The OS keychain.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new() -> Self {
        Self {
            service: KEYCHAIN_SERVICE.into(),
        }
    }

    fn entry(&self, name: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, name).map_err(|e| HostingError::Keychain(e.to_string()))
    }

    fn raw_get(&self, name: &str) -> Result<Option<String>> {
        match self.entry(name)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(HostingError::Keychain(e.to_string())),
        }
    }

    fn raw_set(&self, name: &str, value: &str) -> Result<()> {
        self.entry(name)?
            .set_password(value)
            .map_err(|e| HostingError::Keychain(e.to_string()))
    }

    fn raw_delete(&self, name: &str) -> Result<()> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(HostingError::Keychain(e.to_string())),
        }
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, name: &str) -> Result<Option<Secret>> {
        let Some(head) = self.raw_get(name)? else {
            return Ok(None);
        };
        let Some(count) = head.strip_prefix(CHUNKED_PREFIX) else {
            return Ok(Some(Secret(head)));
        };
        let count: usize = count
            .parse()
            .map_err(|_| HostingError::Keychain(format!("{name}: bad chunk header")))?;
        let mut out = String::new();
        for i in 0..count {
            let part = self
                .raw_get(&format!("{name}#{i}"))?
                .ok_or_else(|| HostingError::Keychain(format!("{name}: chunk {i} is missing")))?;
            out.push_str(&part);
        }
        Ok(Some(Secret(out)))
    }

    fn set(&self, name: &str, value: &Secret) -> Result<()> {
        self.delete(name)?;
        let chunks = split_chunks(value.expose(), CHUNK_CHARS);
        if chunks.len() <= 1 && !value.expose().starts_with(CHUNKED_PREFIX) {
            return self.raw_set(name, value.expose());
        }
        for (i, c) in chunks.iter().enumerate() {
            self.raw_set(&format!("{name}#{i}"), c)?;
        }
        self.raw_set(name, &format!("{CHUNKED_PREFIX}{}", chunks.len()))
    }

    fn delete(&self, name: &str) -> Result<()> {
        if let Some(head) = self.raw_get(name)?
            && let Some(count) = head.strip_prefix(CHUNKED_PREFIX)
            && let Ok(count) = count.parse::<usize>()
        {
            for i in 0..count {
                self.raw_delete(&format!("{name}#{i}"))?;
            }
        }
        self.raw_delete(name)
    }
}

/// Split on character boundaries.
pub(crate) fn split_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    chars
        .chunks(max_chars)
        .map(|c| c.iter().collect())
        .collect()
}

/// In memory, for tests and for the "don't remember" option.
#[derive(Default)]
pub struct MemoryStore {
    map: Mutex<HashMap<String, Secret>>,
}

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<Secret>> {
        Ok(self.map.lock().unwrap().get(name).cloned())
    }
    fn set(&self, name: &str, value: &Secret) -> Result<()> {
        self.map
            .lock()
            .unwrap()
            .insert(name.to_owned(), value.clone());
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<()> {
        self.map.lock().unwrap().remove(name);
        Ok(())
    }
}

/// Keychain entry names for a connection profile.
pub mod names {
    pub fn private_key(profile: &str) -> String {
        format!("hosting/{profile}/private-key")
    }
    pub fn passphrase(profile: &str) -> String {
        format!("hosting/{profile}/passphrase")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_never_prints() {
        let s = Secret::new("hunter2");
        assert_eq!(format!("{s:?}"), "[redacted]");
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn chunks_split_on_chars() {
        assert_eq!(split_chunks("", 3), vec![""]);
        assert_eq!(split_chunks("abcdefg", 3), vec!["abc", "def", "g"]);
        assert_eq!(split_chunks("ééé", 2), vec!["éé", "é"]);
        let pem = "x".repeat(3300);
        let parts = split_chunks(&pem, CHUNK_CHARS);
        assert_eq!(parts.len(), 4);
        assert_eq!(parts.concat(), pem);
    }

    #[test]
    fn memory_store_round_trip() {
        let m = MemoryStore::default();
        assert!(m.get("a").unwrap().is_none());
        m.set("a", &"v".into()).unwrap();
        assert_eq!(m.get("a").unwrap().unwrap().expose(), "v");
        m.delete("a").unwrap();
        assert!(m.get("a").unwrap().is_none());
    }

    /// Touches the real keychain: opt in with RUSTIC_KEYCHAIN_TEST=1 (a desktop session is needed).
    #[test]
    fn keyring_round_trip_with_chunks() {
        if std::env::var("RUSTIC_KEYCHAIN_TEST").as_deref() != Ok("1") {
            return;
        }
        let store = KeyringStore {
            service: format!("{KEYCHAIN_SERVICE}.test"),
        };
        let long = Secret::new("k".repeat(3300));
        store.set("test/long", &long).unwrap();
        assert_eq!(store.get("test/long").unwrap().unwrap(), long);
        store.set("test/long", &"short".into()).unwrap();
        assert_eq!(store.get("test/long").unwrap().unwrap().expose(), "short");
        store.delete("test/long").unwrap();
        assert!(store.get("test/long").unwrap().is_none());
    }
}
