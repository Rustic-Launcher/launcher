//! The stage Cargo compiles: one folder for every build, where only files whose content changed
//! are written. Cargo decides what to recompile by file times, so switching a mod on or off
//! recompiles only the crates it touches. Every file written gets the current time (never an
//! archive's or a copy's), else a file older than the last build would read as already compiled
//! (Horizon's launcher, #10421).
use crate::error::{IoContext, NativeError, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Every file under `dir` as `(path relative to dir with '/', path)`, sorted, skipping folders
/// named in `skip` at any depth. Links are refused (horizon-mixins refuses them too).
pub fn walk(dir: &Path, skip: &[&str]) -> Result<Vec<(String, PathBuf)>> {
    let mut out = Vec::new();
    let mut todo = vec![(dir.to_path_buf(), String::new())];
    while let Some((path, rel)) = todo.pop() {
        let entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && path == dir => {
                return Ok(out);
            }
            Err(e) => return Err(NativeError::io(&path, e)),
        };
        for entry in entries {
            let entry = entry.at(&path)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = entry.file_type().at(&entry.path())?;
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if kind.is_symlink() {
                return Err(NativeError::Source(format!(
                    "{}: links are not allowed in a build's source",
                    entry.path().display()
                )));
            }
            if kind.is_dir() {
                if skip.contains(&name.as_str()) {
                    continue;
                }
                todo.push((entry.path(), child_rel));
            } else if kind.is_file() {
                if skip.contains(&name.as_str()) {
                    // A `.git` file (submodule pointer) is skipped like the folder.
                    continue;
                }
                out.push((child_rel, entry.path()));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Writes `bytes` to `path` unless it already holds exactly them. Returns whether it wrote.
pub fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if let Ok(meta) = std::fs::metadata(path)
        && meta.len() == bytes.len() as u64
        && std::fs::read(path).ok().as_deref() == Some(bytes)
    {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).at(parent)?;
    }
    // A fresh file (not a truncated one): its time is now on every platform.
    let _ = std::fs::remove_file(path);
    std::fs::write(path, bytes).at(path)?;
    Ok(true)
}

/// What a sync did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Synced {
    pub written: usize,
    pub removed: usize,
    pub unchanged: usize,
}

/// Makes `to` hold exactly the files of `from` (but folders named in `keep`, left alone at the
/// top of `to`), writing only changed files.
pub fn sync(from: &Path, to: &Path, keep: &[&str]) -> Result<Synced> {
    let mut done = Synced::default();
    let files = walk(from, &[])?;
    let mut wanted = BTreeSet::new();
    for (rel, path) in &files {
        let bytes = std::fs::read(path).at(path)?;
        if write_if_changed(&to.join(rel), &bytes)? {
            done.written += 1;
        } else {
            done.unchanged += 1;
        }
        wanted.insert(rel.as_str());
    }
    for (rel, path) in walk(to, keep)? {
        if !wanted.contains(rel.as_str()) {
            std::fs::remove_file(&path).at(&path)?;
            done.removed += 1;
        }
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changed_files_are_written() {
        let t = tempfile::tempdir().unwrap();
        let (a, b) = (t.path().join("a"), t.path().join("b"));
        std::fs::create_dir_all(a.join("x/y")).unwrap();
        std::fs::write(a.join("x/y/one.rs"), "1").unwrap();
        std::fs::write(a.join("two.rs"), "2").unwrap();
        let s = sync(&a, &b, &[]).unwrap();
        assert_eq!(s.written, 2);
        let before = std::fs::metadata(b.join("x/y/one.rs"))
            .unwrap()
            .modified()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::write(a.join("two.rs"), "22").unwrap();
        std::fs::remove_file(a.join("x/y/one.rs")).unwrap();
        std::fs::write(a.join("three.rs"), "3").unwrap();
        std::fs::create_dir_all(b.join("target")).unwrap();
        std::fs::write(b.join("target/keep"), "k").unwrap();
        let s = sync(&a, &b, &["target"]).unwrap();
        assert_eq!(
            s,
            Synced {
                written: 2,
                removed: 1,
                unchanged: 0
            }
        );
        assert!(!b.join("x/y/one.rs").exists());
        assert_eq!(std::fs::read_to_string(b.join("two.rs")).unwrap(), "22");
        assert!(b.join("target/keep").exists());
        // Unchanged files keep their time.
        std::fs::write(a.join("x.rs"), "x").unwrap();
        sync(&a, &b, &[]).unwrap();
        let t3 = std::fs::metadata(b.join("three.rs"))
            .unwrap()
            .modified()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        let s = sync(&a, &b, &[]).unwrap();
        assert_eq!(s.written, 0);
        assert_eq!(
            std::fs::metadata(b.join("three.rs"))
                .unwrap()
                .modified()
                .unwrap(),
            t3
        );
        let _ = before;
    }

    #[test]
    fn walking_skips_build_folders() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("crates/target/x")).unwrap();
        std::fs::write(t.path().join("crates/target/x/f"), "").unwrap();
        std::fs::write(t.path().join("crates/lib.rs"), "").unwrap();
        let files: Vec<String> = walk(t.path(), &["target"])
            .unwrap()
            .into_iter()
            .map(|f| f.0)
            .collect();
        assert_eq!(files, ["crates/lib.rs"]);
        assert!(walk(&t.path().join("missing"), &[]).unwrap().is_empty());
    }
}
