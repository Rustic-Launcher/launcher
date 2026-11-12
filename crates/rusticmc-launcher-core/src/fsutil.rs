//! File helpers: atomic writes, folder sizes, JSON/TOML files.

use crate::error::{CoreError, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

/// Writes `bytes` to `path` through a temporary file in the same folder and a rename, so a crash
/// never leaves a half-written file. Creates the parent folder.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    let tmp = temp_sibling(path);
    std::fs::write(&tmp, bytes).map_err(|e| CoreError::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        CoreError::io(path, e)
    })
}

/// A unique temporary path beside `path`.
pub fn temp_sibling(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.{n}.tmp", std::process::id()))
}

/// Reads a JSON file; `None` when it doesn't exist.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| CoreError::ConfigInvalid {
                    path: path.into(),
                    message: e.to_string(),
                })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::io(path, e)),
    }
}

/// Writes a JSON file atomically, pretty-printed.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| CoreError::ConfigInvalid {
        path: path.into(),
        message: e.to_string(),
    })?;
    write_atomic(path, &bytes)
}

/// Reads a TOML file; `None` when it doesn't exist.
pub fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text)
            .map(Some)
            .map_err(|e| CoreError::ConfigInvalid {
                path: path.into(),
                message: e.to_string(),
            }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::io(path, e)),
    }
}

/// Writes a TOML file atomically.
pub fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = toml::to_string_pretty(value).map_err(|e| CoreError::ConfigInvalid {
        path: path.into(),
        message: e.to_string(),
    })?;
    write_atomic(path, text.as_bytes())
}

/// The total size of the files under `dir` (0 when it doesn't exist).
pub fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(_) => e.metadata().map_or(0, |m| m.len()),
            Err(_) => 0,
        })
        .sum()
}

/// Copies a folder recursively.
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A file or folder name that is safe on every OS: letters, digits, `-` and `_`, at most 48
/// characters, never empty.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if (c == ' ' || c == '-' || c == '_' || c == '.') && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 48 {
            break;
        }
    }
    let out = out.trim_matches('-').to_owned();
    if out.is_empty() { "item".into() } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c.json");
        write_json(&path, &serde_json::json!({"x": 1})).unwrap();
        let v: serde_json::Value = read_json(&path).unwrap().unwrap();
        assert_eq!(v["x"], 1);
        assert!(
            read_json::<serde_json::Value>(&dir.path().join("none"))
                .unwrap()
                .is_none()
        );
        // No temporary file left behind.
        let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("My Survival World!"), "my-survival-world");
        assert_eq!(slug("  "), "item");
        assert_eq!(slug("../../etc"), "etc");
    }
}
