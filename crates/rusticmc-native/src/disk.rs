//! Free space before a build: refused with a clear message below what it may need.
use crate::error::{NativeError, Result};
use std::path::{Path, PathBuf};

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// What a first build may need: crates (~0.7 GB), the source and stage (~0.3 GB), the compile
/// cache (~2-5 GB with the edition), the client, and headroom.
pub const FIRST_BUILD_GIB: f64 = 10.0;
/// A later build, the cache already there.
pub const REBUILD_GIB: f64 = 3.0;
/// The launcher's own Rust and MinGW (Windows), installed before the first build.
pub const TOOLCHAIN_GIB: f64 = 2.5;

/// Free bytes on the drive holding `path` (the longest mount point it is under).
pub fn free_bytes(path: &Path) -> Option<u64> {
    let probe: PathBuf = path
        .ancestors()
        .find(|p| p.exists())
        .map(Path::to_path_buf)?;
    let full = std::fs::canonicalize(&probe).unwrap_or(probe);
    let full = PathBuf::from(
        full.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_owned(),
    );
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| starts_with_ci(&full, d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

fn starts_with_ci(path: &Path, prefix: &Path) -> bool {
    if cfg!(windows) {
        let p = path.to_string_lossy().to_ascii_lowercase();
        let m = prefix.to_string_lossy().to_ascii_lowercase();
        p.starts_with(&m)
    } else {
        path.starts_with(prefix)
    }
}

/// Refuses when the drive of `path` has less than `needed_gib` free (unknown free space passes).
pub fn check(path: &Path, needed_gib: f64) -> Result<()> {
    check_free(path, free_bytes(path), needed_gib)
}

pub fn check_free(path: &Path, free: Option<u64>, needed_gib: f64) -> Result<()> {
    match free {
        Some(free) if (free as f64) < needed_gib * GIB => Err(NativeError::DiskSpace {
            path: path.to_owned(),
            free_gib: free as f64 / GIB,
            needed_gib,
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_space_is_refused_with_numbers() {
        let p = Path::new("/data");
        let e = check_free(p, Some(2 * 1024 * 1024 * 1024), 10.0).unwrap_err();
        assert_eq!(e.code(), "disk_space");
        let text = e.to_string();
        assert!(
            text.contains("2.0 GiB free") && text.contains("10.0 GiB needed"),
            "{text}"
        );
        assert!(check_free(p, Some(u64::MAX), 10.0).is_ok());
        assert!(check_free(p, None, 10.0).is_ok());
    }

    #[test]
    fn this_computers_drive_answers() {
        let dir = std::env::temp_dir();
        assert!(free_bytes(&dir).is_some_and(|b| b > 0));
    }
}
