//! Where the launcher keeps things.
//!
//! By default the OS conventions through `directories` (Windows `%APPDATA%\Rustic\Rustic
//! Launcher\{data,config}` and `%LOCALAPPDATA%\...\cache`, macOS `~/Library/Application
//! Support/gg.rustic.Rustic-Launcher`, Linux XDG). `RUSTIC_LAUNCHER_HOME` (or
//! [`LauncherPaths::under`]) puts everything under one folder instead: portable installs and tests.

use crate::error::{CoreError, Result};
use std::path::{Path, PathBuf};

/// The environment variable that puts every launcher folder under one root.
pub const HOME_ENV: &str = "RUSTIC_LAUNCHER_HOME";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherPaths {
    data: PathBuf,
    config: PathBuf,
    cache: PathBuf,
}

impl LauncherPaths {
    /// The OS folders, or `RUSTIC_LAUNCHER_HOME` when set.
    pub fn from_os() -> Result<Self> {
        if let Some(root) = std::env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
            return Ok(Self::under(PathBuf::from(root)));
        }
        let dirs = directories::ProjectDirs::from("gg", "Rustic", "Rustic Launcher")
            .ok_or(CoreError::NoHomeDir)?;
        Ok(Self {
            data: dirs.data_dir().to_path_buf(),
            config: dirs.config_dir().to_path_buf(),
            cache: dirs.cache_dir().to_path_buf(),
        })
    }

    /// Everything under `root` (`root/data`, `root/config`, `root/cache`).
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            data: root.join("data"),
            config: root.join("config"),
            cache: root.join("cache"),
        }
    }

    pub fn data(&self) -> &Path {
        &self.data
    }
    pub fn config_dir(&self) -> &Path {
        &self.config
    }
    pub fn cache(&self) -> &Path {
        &self.cache
    }
    /// `launcher.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.config.join("launcher.toml")
    }
    /// Rotated log files.
    pub fn logs(&self) -> PathBuf {
        self.data.join("logs")
    }
    /// The content-addressed download cache (`<algo>/<xx>/<hex>`).
    pub fn download_cache(&self) -> PathBuf {
        self.cache.join("downloads")
    }
    /// One folder per instance.
    pub fn instances(&self) -> PathBuf {
        self.data.join("instances")
    }
    /// One folder per installed Horizon build.
    pub fn builds(&self) -> PathBuf {
        self.data.join("builds")
    }
    /// Prepared game resources, one folder per build (shared by its instances).
    pub fn prepared(&self) -> PathBuf {
        self.data.join("prepared")
    }
    /// One folder per local server (horizon-server's `--dir`).
    pub fn servers(&self) -> PathBuf {
        self.data.join("servers")
    }
    /// Public account data (never a token).
    pub fn accounts_file(&self) -> PathBuf {
        self.data.join("accounts.json")
    }
    /// Server jars and other owner-gated downloads.
    pub fn owner_downloads(&self) -> PathBuf {
        self.data.join("owner-downloads")
    }
    /// The game files downloaded from Mojang for a verified owner, in the official launcher's
    /// layout (used when no launcher on this computer has them whole).
    pub fn game_files(&self) -> PathBuf {
        self.data.join("minecraft")
    }
    /// Scratch space.
    pub fn tmp(&self) -> PathBuf {
        self.cache.join("tmp")
    }

    /// Creates every folder.
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.data.clone(),
            self.config.clone(),
            self.cache.clone(),
            self.logs(),
            self.download_cache(),
            self.instances(),
            self.builds(),
            self.prepared(),
            self.servers(),
            self.tmp(),
        ] {
            std::fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_one_root() {
        let dir = tempfile::tempdir().unwrap();
        let paths = LauncherPaths::under(dir.path());
        paths.ensure().unwrap();
        assert!(paths.instances().starts_with(dir.path()));
        assert!(paths.config_file().starts_with(dir.path()));
        assert!(paths.instances().is_dir() && paths.download_cache().is_dir());
    }

    #[test]
    fn os_paths_resolve() {
        // On CI and dev boxes a home folder exists.
        let paths = LauncherPaths::from_os().unwrap();
        assert!(paths.data().is_absolute());
    }
}
