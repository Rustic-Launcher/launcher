//! Where secrets live: the OS keychain (`keyring`), never a file, never a command line.
//!
//! Windows Credential Manager holds at most 2560 bytes per item, which a sign-in's secrets exceed
//! (Horizon hit the same limit, horizon/docs/microsoft-account.md "Token storage"). A long secret
//! is therefore split: the item `<key>` holds `rustic-chunks:<n>` and the pieces go to
//! `<key>#0`..`<key>#n-1`. Pieces are written before the header and old extra pieces removed
//! after, so a reader never sees a header naming pieces that aren't there yet.

use crate::error::{AuthError, Result};
use std::collections::HashMap;
use std::sync::Mutex;

/// A secret store. Implementations: [`KeyringStore`] (the OS keychain), [`MemoryStore`] (tests,
/// and the fallback where no keychain answers: secrets then last until the launcher closes).
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    /// Deleting a missing item is not an error.
    fn delete(&self, key: &str) -> Result<()>;
    /// Whether secrets survive a restart.
    fn persistent(&self) -> bool {
        true
    }
}

/// Characters per keychain item: 1000 UTF-16 code units = 2000 bytes, under Credential Manager's
/// 2560-byte limit whether the backend stores UTF-8 or UTF-16.
pub const CHUNK: usize = 1000;
const HEADER: &str = "rustic-chunks:";

fn piece_key(key: &str, n: usize) -> String {
    format!("{key}#{n}")
}

/// Reads a possibly split secret from `raw` item access.
fn get_chunked(raw: &dyn RawItems, key: &str) -> Result<Option<String>> {
    let Some(head) = raw.read(key)? else {
        return Ok(None);
    };
    let Some(count) = head.strip_prefix(HEADER) else {
        return Ok(Some(head));
    };
    let count: usize = count
        .parse()
        .map_err(|_| AuthError::Keychain(format!("{key}: damaged item")))?;
    let mut out = String::new();
    for n in 0..count {
        match raw.read(&piece_key(key, n))? {
            Some(piece) => out.push_str(&piece),
            None => return Err(AuthError::Keychain(format!("{key}: a piece is missing"))),
        }
    }
    Ok(Some(out))
}

fn old_piece_count(raw: &dyn RawItems, key: &str) -> usize {
    raw.read(key)
        .ok()
        .flatten()
        .and_then(|h| h.strip_prefix(HEADER).and_then(|c| c.parse().ok()))
        .unwrap_or(0)
}

fn set_chunked(raw: &dyn RawItems, key: &str, value: &str) -> Result<()> {
    let old = old_piece_count(raw, key);
    if value.chars().count() <= CHUNK && !value.starts_with(HEADER) {
        raw.write(key, value)?;
        for n in 0..old {
            let _ = raw.remove(&piece_key(key, n));
        }
        return Ok(());
    }
    let chars: Vec<char> = value.chars().collect();
    let pieces: Vec<String> = chars.chunks(CHUNK).map(|c| c.iter().collect()).collect();
    for (n, piece) in pieces.iter().enumerate() {
        raw.write(&piece_key(key, n), piece)?;
    }
    raw.write(key, &format!("{HEADER}{}", pieces.len()))?;
    for n in pieces.len()..old {
        let _ = raw.remove(&piece_key(key, n));
    }
    Ok(())
}

fn delete_chunked(raw: &dyn RawItems, key: &str) -> Result<()> {
    let old = old_piece_count(raw, key);
    raw.remove(key)?;
    for n in 0..old {
        raw.remove(&piece_key(key, n))?;
    }
    Ok(())
}

/// Single-item access, under the chunking.
trait RawItems {
    fn read(&self, key: &str) -> Result<Option<String>>;
    fn write(&self, key: &str, value: &str) -> Result<()>;
    fn remove(&self, key: &str) -> Result<()>;
}

/// The OS keychain: Credential Manager, Keychain, Secret Service.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, key).map_err(map_keyring)
    }

    /// Whether the keychain answers at all (a write, read and delete of a probe item).
    pub fn probe(&self) -> bool {
        let key = "rustic-probe";
        self.set(key, "1").is_ok() && self.get(key).ok().flatten().as_deref() == Some("1") && {
            let _ = self.delete(key);
            true
        }
    }
}

fn map_keyring(e: keyring::Error) -> AuthError {
    let text = e.to_string();
    // Credential Manager's undocumented total-size limit answers ERROR_NOT_ENOUGH_MEMORY (8).
    if text.contains("os error 8") || text.to_ascii_lowercase().contains("not enough memory") {
        return AuthError::KeychainFull;
    }
    AuthError::Keychain(text)
}

impl RawItems for KeyringStore {
    fn read(&self, key: &str) -> Result<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(map_keyring(e)),
        }
    }
    fn write(&self, key: &str, value: &str) -> Result<()> {
        self.entry(key)?.set_password(value).map_err(map_keyring)
    }
    fn remove(&self, key: &str) -> Result<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(map_keyring(e)),
        }
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        get_chunked(self, key)
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        set_chunked(self, key, value)
    }
    fn delete(&self, key: &str) -> Result<()> {
        delete_chunked(self, key)
    }
}

/// Secrets in memory: tests, and the fallback where no keychain answers.
#[derive(Default)]
pub struct MemoryStore {
    items: Mutex<HashMap<String, String>>,
    /// Refuse writes (to test a full keychain).
    pub fail_writes: std::sync::atomic::AtomicBool,
    /// Largest item accepted, to check chunking (0: no limit).
    pub max_item_chars: usize,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// A store refusing items longer than `max` characters, as Credential Manager does.
    pub fn with_item_limit(max: usize) -> Self {
        Self {
            max_item_chars: max,
            ..Self::default()
        }
    }

    /// Every raw item (tests).
    pub fn raw_items(&self) -> HashMap<String, String> {
        self.items.lock().expect("store lock").clone()
    }
}

impl RawItems for MemoryStore {
    fn read(&self, key: &str) -> Result<Option<String>> {
        Ok(self.items.lock().expect("store lock").get(key).cloned())
    }
    fn write(&self, key: &str, value: &str) -> Result<()> {
        if self.fail_writes.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(AuthError::KeychainFull);
        }
        if self.max_item_chars > 0 && value.chars().count() > self.max_item_chars {
            return Err(AuthError::Keychain("item too long".into()));
        }
        self.items
            .lock()
            .expect("store lock")
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }
    fn remove(&self, key: &str) -> Result<()> {
        self.items.lock().expect("store lock").remove(key);
        Ok(())
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        get_chunked(self, key)
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        set_chunked(self, key, value)
    }
    fn delete(&self, key: &str) -> Result<()> {
        delete_chunked(self, key)
    }
    fn persistent(&self) -> bool {
        false
    }
}

/// The keychain when it answers, else memory (with a warning: secrets last until the launcher
/// closes, as Horizon's launcher does on macOS/Windows without a keychain).
pub fn default_store(service: &str) -> std::sync::Arc<dyn SecretStore> {
    let keyring = KeyringStore::new(service);
    if keyring.probe() {
        std::sync::Arc::new(keyring)
    } else {
        tracing::warn!("no OS keychain available: sign-ins last until the launcher closes");
        std::sync::Arc::new(MemoryStore::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_secrets_are_split_under_the_item_limit() {
        let store = MemoryStore::with_item_limit(CHUNK);
        let long: String = "x".repeat(CHUNK * 3 + 17);
        store.set("acct", &long).unwrap();
        assert_eq!(store.get("acct").unwrap().as_deref(), Some(long.as_str()));
        let raw = store.raw_items();
        assert_eq!(raw["acct"], "rustic-chunks:4");
        assert_eq!(raw.len(), 5);
        // Shrinking removes the extra pieces.
        store.set("acct", "short").unwrap();
        assert_eq!(store.raw_items().len(), 1);
        assert_eq!(store.get("acct").unwrap().as_deref(), Some("short"));
        store.delete("acct").unwrap();
        assert!(store.get("acct").unwrap().is_none());
        assert!(store.raw_items().is_empty());
    }

    #[test]
    fn a_missing_piece_is_an_error_not_a_short_secret() {
        let store = MemoryStore::new();
        store.set("k", &"y".repeat(CHUNK * 2)).unwrap();
        RawItems::remove(&store, "k#1").unwrap();
        assert!(store.get("k").is_err());
    }

    #[test]
    fn a_full_keychain_is_reported() {
        let store = MemoryStore::new();
        store
            .fail_writes
            .store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(matches!(store.set("k", "v"), Err(AuthError::KeychainFull)));
    }
}
