//! Mojang's Java runtimes (the `java-runtime-*` and `jre-legacy` components the official launcher
//! installs): the index per platform, each component's file list, installed under
//! `runtimes/<component>/` with every file checked against its sha1.

use crate::download::{FileDownload, Progress, fetch_all};
use crate::maven::safe_relative;
use crate::platform::Platform;
use crate::{JavaError, Result};
use rusticmc_launcher_core::{CancellationToken, DownloadManager};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `all.json`: platform → component → entries (empty when Mojang has none for that platform).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RuntimeIndex(pub BTreeMap<String, BTreeMap<String, Vec<RuntimeEntry>>>);

#[derive(Clone, Debug, Deserialize)]
pub struct RuntimeEntry {
    pub manifest: RuntimeManifestRef,
    #[serde(default)]
    pub version: Option<RuntimeVersion>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuntimeManifestRef {
    pub sha1: String,
    #[serde(default)]
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuntimeVersion {
    pub name: String,
}

/// A component's files.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RuntimeFiles {
    pub files: BTreeMap<String, RuntimeFile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RuntimeFile {
    File {
        downloads: RuntimeDownloads,
        #[serde(default)]
        executable: bool,
    },
    Directory,
    Link {
        target: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuntimeDownloads {
    pub raw: RawFile,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawFile {
    pub sha1: String,
    #[serde(default)]
    pub size: Option<u64>,
    pub url: String,
}

impl RuntimeIndex {
    pub fn parse(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|e| JavaError::bad(format!("runtime index: {e}")))
    }

    /// The entry of `component` for `platform`.
    pub fn entry(&self, platform: &Platform, component: &str) -> Result<&RuntimeEntry> {
        let none = || JavaError::NoRuntime {
            component: component.to_owned(),
            platform: platform.runtime_key().unwrap_or("unknown").to_owned(),
        };
        let key = platform.runtime_key().ok_or_else(none)?;
        self.0
            .get(key)
            .and_then(|c| c.get(component))
            .and_then(|e| e.first())
            .ok_or_else(none)
    }
}

impl RuntimeFiles {
    pub fn parse(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|e| JavaError::bad(format!("runtime files: {e}")))
    }

    /// The files to fetch, relative to the component's folder.
    pub fn downloads(&self) -> Result<Vec<FileDownload>> {
        let mut out = Vec::new();
        for (name, f) in &self.files {
            if let RuntimeFile::File { downloads, .. } = f {
                let path = safe_relative(name)
                    .ok_or_else(|| JavaError::bad(format!("runtime file {name:?}")))?;
                out.push(FileDownload {
                    url: downloads.raw.url.clone(),
                    sha1: Some(downloads.raw.sha1.clone()),
                    size: downloads.raw.size,
                    path,
                });
            }
        }
        Ok(out)
    }
}

/// Where a component lives under the game folder.
pub fn component_dir(root: &Path, component: &str) -> PathBuf {
    root.join("runtimes").join(component)
}

const MARKER: &str = ".rustic-runtime";

/// Whether `component` is installed as `manifest_sha1` describes (a marker written last).
pub fn is_installed(dir: &Path, manifest_sha1: &str, platform: &Platform) -> bool {
    std::fs::read_to_string(dir.join(MARKER)).is_ok_and(|m| m.trim() == manifest_sha1)
        && platform.runtime_java(dir).is_file()
}

/// Installs the component's files into `dir` (directories, files checked by sha1, executable bits
/// and links on Unix), then writes the marker. Returns the Java executable.
#[allow(clippy::too_many_arguments)]
pub async fn install_files(
    downloads: &DownloadManager,
    dir: &Path,
    manifest_sha1: &str,
    files: &RuntimeFiles,
    platform: &Platform,
    extra_hosts: &[String],
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).map_err(|e| JavaError::io(dir, e))?;
    let _ = std::fs::remove_file(dir.join(MARKER));
    for (name, f) in &files.files {
        if matches!(f, RuntimeFile::Directory)
            && let Some(rel) = safe_relative(name)
        {
            let d = dir.join(rel);
            std::fs::create_dir_all(&d).map_err(|e| JavaError::io(&d, e))?;
        }
    }
    fetch_all(
        downloads,
        dir,
        files.downloads()?,
        extra_hosts,
        cancel,
        progress,
    )
    .await?;
    for (name, f) in &files.files {
        let Some(rel) = safe_relative(name) else {
            continue;
        };
        match f {
            RuntimeFile::File {
                executable: true, ..
            } => make_executable(&dir.join(rel)),
            RuntimeFile::Link { target } => link(&dir.join(rel), target),
            _ => {}
        }
    }
    let java = platform.runtime_java(dir);
    if !java.is_file() {
        return Err(JavaError::NoJavaExecutable(java));
    }
    std::fs::write(dir.join(MARKER), manifest_sha1).map_err(|e| JavaError::io(dir, e))?;
    Ok(java)
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(m) = std::fs::metadata(path) {
        let mut p = m.permissions();
        p.set_mode(p.mode() | 0o755);
        let _ = std::fs::set_permissions(path, p);
    }
}

#[cfg(not(unix))]
fn make_executable(_: &Path) {}

#[cfg(unix)]
fn link(path: &Path, target: &str) {
    // Relative links inside the runtime only (macOS bundles use them).
    if target.starts_with('/') || target.split('/').filter(|c| *c == "..").count() > 4 {
        return;
    }
    let _ = std::fs::remove_file(path);
    let _ = std::os::unix::fs::symlink(target, path);
}

#[cfg(not(unix))]
fn link(_: &Path, _: &str) {}

/// The major of a Java home or executable, from its `release` file (`JAVA_VERSION="21.0.7"`,
/// `"1.8.0_51"` → 8). `None` when there is none.
pub fn java_major(java: &Path) -> Option<u32> {
    // `.../bin/java(.exe)` → the home is two levels up.
    let home = if java.is_file() {
        java.parent()?.parent()?.to_path_buf()
    } else {
        java.to_path_buf()
    };
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    let version = text
        .lines()
        .find_map(|l| l.strip_prefix("JAVA_VERSION="))?
        .trim()
        .trim_matches('"')
        .to_owned();
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// A Java executable from a profile's setting: a `java`/`javaw` file or a Java home.
pub fn custom_java(path: &Path) -> Result<PathBuf> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let candidate = path.join("bin").join(exe);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(JavaError::NoJavaExecutable(path.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Os};

    /// Shapes of Mojang's runtime files, written for this test.
    const INDEX: &str = r#"{
      "windows-x64": {"java-runtime-delta": [{"availability": {"group": 1, "progress": 100}, "manifest": {"sha1": "m1", "size": 10, "url": "https://piston-meta.mojang.com/v1/packages/m1/manifest.json"}, "version": {"name": "21.0.7", "released": "2025-01-01T00:00:00+00:00"}}]},
      "mac-os-arm64": {"jre-legacy": []}
    }"#;

    const FILES: &str = r#"{"files": {
      "bin": {"type": "directory"},
      "bin/java.exe": {"type": "file", "executable": true, "downloads": {"raw": {"sha1": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "size": 3, "url": "https://piston-data.mojang.com/v1/objects/aaaa/java.exe"}, "lzma": {"sha1": "x", "size": 1, "url": "https://piston-data.mojang.com/x"}}},
      "legal/link": {"type": "link", "target": "../bin"}
    }}"#;

    #[test]
    fn picks_the_platform_component() {
        let idx = RuntimeIndex::parse(INDEX).unwrap();
        let win = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
            os_version: None,
        };
        assert_eq!(
            idx.entry(&win, "java-runtime-delta").unwrap().manifest.sha1,
            "m1"
        );
        let mac = Platform {
            os: Os::Osx,
            arch: Arch::Arm64,
            os_version: None,
        };
        assert!(matches!(
            idx.entry(&mac, "jre-legacy"),
            Err(JavaError::NoRuntime { .. })
        ));
    }

    #[test]
    fn lists_raw_files_only() {
        let f = RuntimeFiles::parse(FILES).unwrap();
        let d = f.downloads().unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path, PathBuf::from("bin/java.exe"));
        assert!(d[0].url.ends_with("java.exe"));
    }

    #[test]
    fn reads_java_majors() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("release"), "JAVA_VERSION=\"1.8.0_51\"\n").unwrap();
        assert_eq!(java_major(tmp.path()), Some(8));
        std::fs::write(tmp.path().join("release"), "JAVA_VERSION=\"21.0.7\"\n").unwrap();
        assert_eq!(java_major(tmp.path()), Some(21));
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join(if cfg!(windows) { "java.exe" } else { "java" });
        std::fs::write(&exe, "").unwrap();
        assert_eq!(java_major(&exe), Some(21));
        assert_eq!(custom_java(tmp.path()).unwrap(), exe);
        assert!(custom_java(&tmp.path().join("nope")).is_err());
    }
}
