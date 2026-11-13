//! The compile cache (`native/target/`): Cargo's outputs, which make later builds incremental. As
//! in Horizon's launcher ("Compilation cache"), after a build the cache is cleared when it exceeds
//! the limit (30 GiB by default; 0 turns the automatic cleanup off): a later build recompiles. It
//! is an after-build limit, not a cap on the space a build needs while it runs. Clearing never
//! touches the toolchain, the downloaded crates, the source or anything an instance plays (built
//! clients live in the instances' folders).
use crate::error::{IoContext, Result};
use crate::paths::NativeRoot;
use serde::Serialize;
use std::path::Path;

pub const DEFAULT_LIMIT_GIB: u64 = 30;
pub const GIB: u64 = 1024 * 1024 * 1024;

/// Bytes under `dir`, links not followed.
pub fn size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let Ok(kind) = e.file_type() else { continue };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                todo.push(e.path());
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

/// What the native folder holds, for Settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Usage {
    /// The compile cache (what the limit and "Clean" apply to).
    pub cache_bytes: u64,
    /// The launcher's own Rust and MinGW.
    pub toolchain_bytes: u64,
    /// Crates downloaded by builds.
    pub crates_bytes: u64,
    /// The edition's source, the stage and horizon-mixins.
    pub source_bytes: u64,
    pub limit_bytes: u64,
    pub path: String,
}

pub fn usage(root: &NativeRoot, limit_gib: u64) -> Usage {
    let cargo = root.cargo_home();
    Usage {
        cache_bytes: size(&root.target()),
        toolchain_bytes: size(&root.rustup_home()) + size(&root.mingw()) + size(&cargo.join("bin")),
        crates_bytes: size(&cargo.join("registry")) + size(&cargo.join("git")),
        source_bytes: size(&root.sources())
            + size(&root.stage())
            + size(&root.work())
            + size(&root.root().join("tools")),
        limit_bytes: limit_gib.saturating_mul(GIB),
        path: root.root().to_string_lossy().into_owned(),
    }
}

/// Clears the compile cache. Returns the bytes freed.
pub fn clear(root: &NativeRoot) -> Result<u64> {
    let target = root.target();
    let before = size(&target);
    if target.exists() {
        // Refuse a link: never clear what it points at.
        let meta = std::fs::symlink_metadata(&target).at(&target)?;
        if meta.file_type().is_symlink() {
            return Ok(0);
        }
        std::fs::remove_dir_all(&target).at(&target)?;
    }
    let _ = std::fs::remove_dir_all(root.tmp());
    Ok(before)
}

/// After a build: clears the cache when it exceeds `limit_gib` (0: never). Returns the bytes freed.
pub fn enforce(root: &NativeRoot, limit_gib: u64) -> Result<u64> {
    if limit_gib == 0 {
        return Ok(0);
    }
    if size(&root.target()) > limit_gib.saturating_mul(GIB) {
        clear(root)
    } else {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_is_cleared_only_above_the_limit() {
        let t = tempfile::tempdir().unwrap();
        let root = NativeRoot::at(t.path());
        std::fs::create_dir_all(root.target().join("fast/deps")).unwrap();
        std::fs::write(root.target().join("fast/deps/a.rlib"), vec![0u8; 2048]).unwrap();
        std::fs::create_dir_all(root.rustup_home()).unwrap();
        std::fs::write(root.rustup_home().join("r"), vec![0u8; 10]).unwrap();
        let u = usage(&root, 30);
        assert_eq!(u.cache_bytes, 2048);
        assert_eq!(u.toolchain_bytes, 10);
        assert_eq!(u.limit_bytes, 30 * GIB);
        assert_eq!(enforce(&root, 30).unwrap(), 0);
        assert_eq!(enforce(&root, 0).unwrap(), 0);
        assert!(root.target().exists());
        assert_eq!(clear(&root).unwrap(), 2048);
        assert!(!root.target().exists());
        // The toolchain stays.
        assert!(root.rustup_home().join("r").exists());
        assert_eq!(clear(&root).unwrap(), 0);
    }
}
