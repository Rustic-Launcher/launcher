//! Java for Horizon's preparer: it runs the player's Minecraft jars to export game data, with the
//! Java major `horizon-prepare --launch-api` names (`java_major`, 25 for 26.3), passed as
//! `--java-home`. Runtimes are found without starting them: a Java home's `release` file names
//! its version (`JAVA_VERSION="25.0.1"`), as Horizon's launcher reads it (launcher/src/java.rs).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaSource {
    Configured,
    JavaHome,
    Path,
    System,
    /// The official launcher's own runtimes.
    MinecraftLauncher,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaRuntime {
    pub home: PathBuf,
    /// Full version, `25.0.1`.
    pub version: String,
    pub major: u32,
    pub source: JavaSource,
}

/// The version a Java home's `release` file names, with its major (`1.8.0_x` → 8).
pub fn release_version(home: &Path) -> Option<(String, u32)> {
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    let version = text
        .lines()
        .find_map(|l| l.strip_prefix("JAVA_VERSION="))?
        .trim()
        .trim_matches('"')
        .to_owned();
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    let major = if first == 1 {
        parts.next()?.parse().ok()?
    } else {
        first
    };
    Some((version, major))
}

fn java_exe(home: &Path) -> PathBuf {
    home.join("bin")
        .join(if cfg!(windows) { "java.exe" } else { "java" })
}

/// Folders whose children are Java homes, per OS.
fn system_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        for base in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(pf) = std::env::var_os(base) {
                let pf = PathBuf::from(pf);
                for vendor in [
                    "Java",
                    "Eclipse Adoptium",
                    "Eclipse Foundation",
                    "Microsoft",
                    "Zulu",
                    "BellSoft",
                    "Amazon Corretto",
                    "Semeru",
                ] {
                    roots.push(pf.join(vendor));
                }
            }
        }
    } else if cfg!(target_os = "macos") {
        roots.push("/Library/Java/JavaVirtualMachines".into());
    } else {
        roots.extend([
            "/usr/lib/jvm".into(),
            "/usr/java".into(),
            "/opt/java".into(),
        ]);
    }
    if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
        roots.push(home.join(".jdks"));
        roots.push(home.join(".sdkman/candidates/java"));
        if cfg!(target_os = "macos") {
            roots.push(home.join("Library/Java/JavaVirtualMachines"));
        }
    }
    roots
}

/// A Java home inside a candidate folder (macOS bundles keep it in `Contents/Home`).
fn as_home(dir: &Path) -> Option<PathBuf> {
    [dir.to_path_buf(), dir.join("Contents/Home")]
        .into_iter()
        .find(|h| h.join("release").is_file() && java_exe(h).is_file())
}

/// The official launcher's runtimes: `<game dir>/runtime/<component>/<platform>/<component>`,
/// and on Windows the Store launcher's copy.
fn minecraft_runtime_roots(minecraft_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut runtime_dirs = Vec::new();
    if let Some(dir) = minecraft_dir {
        runtime_dirs.push(dir.join("runtime"));
    }
    if cfg!(windows)
        && let Some(local) = std::env::var_os("LOCALAPPDATA")
    {
        runtime_dirs.push(
            PathBuf::from(local)
                .join("Packages/Microsoft.4297127D64EC6_8wekyb3d8bbwe/LocalCache/Local/runtime"),
        );
    }
    for runtime in runtime_dirs {
        for component in read_dirs(&runtime) {
            for platform in read_dirs(&component) {
                roots.push(platform);
            }
        }
    }
    roots
}

fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|e| {
            e.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

/// Every Java found: the configured ones, `JAVA_HOME`, the `PATH`, the usual folders and the
/// official launcher's runtimes. Each home once.
pub fn discover(configured: &[PathBuf], minecraft_dir: Option<&Path>) -> Vec<JavaRuntime> {
    let mut candidates: Vec<(PathBuf, JavaSource)> = Vec::new();
    for c in configured {
        candidates.push((c.clone(), JavaSource::Configured));
    }
    if let Some(h) = std::env::var_os("JAVA_HOME") {
        candidates.push((PathBuf::from(h), JavaSource::JavaHome));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let exe = dir.join(if cfg!(windows) { "java.exe" } else { "java" });
            if exe.is_file() {
                let real = std::fs::canonicalize(&exe).unwrap_or(exe);
                // Windows' canonical form is a verbatim (\\?\) path: show the plain one.
                let real = match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
                    Some(plain) => PathBuf::from(plain),
                    None => real,
                };
                if let Some(home) = real.parent().and_then(Path::parent) {
                    candidates.push((home.to_path_buf(), JavaSource::Path));
                }
            }
        }
    }
    for root in system_roots() {
        for dir in read_dirs(&root) {
            candidates.push((dir, JavaSource::System));
        }
    }
    for root in minecraft_runtime_roots(minecraft_dir) {
        for dir in std::iter::once(root.clone()).chain(read_dirs(&root)) {
            candidates.push((dir, JavaSource::MinecraftLauncher));
        }
    }
    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter_map(|(dir, source)| {
            let home = as_home(&dir)?;
            let key = std::fs::canonicalize(&home).unwrap_or_else(|_| home.clone());
            if !seen.insert(key) {
                return None;
            }
            let (version, major) = release_version(&home)?;
            Some(JavaRuntime {
                home,
                version,
                major,
                source,
            })
        })
        .collect()
}

/// A Java of exactly `major` (the preparer checks its pins against it), the configured one first.
pub fn find(
    major: u32,
    configured: &[PathBuf],
    minecraft_dir: Option<&Path>,
) -> Option<JavaRuntime> {
    discover(configured, minecraft_dir)
        .into_iter()
        .find(|r| r.major == major)
}

/// The official launcher's game folder (`.minecraft`), where the player's own files are.
pub fn default_minecraft_dir() -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    let dir = if cfg!(windows) {
        base.data_dir().join(".minecraft")
    } else if cfg!(target_os = "macos") {
        base.data_dir().join("minecraft")
    } else {
        base.home_dir().join(".minecraft")
    };
    dir.is_dir().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_home(dir: &Path, version: &str) -> PathBuf {
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(java_exe(dir), b"").unwrap();
        std::fs::write(
            dir.join("release"),
            format!("IMPLEMENTOR=\"x\"\nJAVA_VERSION=\"{version}\"\n"),
        )
        .unwrap();
        dir.to_path_buf()
    }

    #[test]
    fn versions_from_release_files() {
        let t = tempfile::tempdir().unwrap();
        let h = fake_home(&t.path().join("j8"), "1.8.0_402");
        assert_eq!(release_version(&h).unwrap().1, 8);
        let h = fake_home(&t.path().join("j25"), "25.0.1");
        assert_eq!(release_version(&h), Some(("25.0.1".into(), 25)));
    }

    #[test]
    fn finds_configured_and_launcher_runtimes() {
        let t = tempfile::tempdir().unwrap();
        let configured = fake_home(&t.path().join("my-jdk"), "21.0.5");
        let mc = t.path().join(".minecraft");
        fake_home(
            &mc.join("runtime/java-runtime-epsilon/windows-x64/java-runtime-epsilon"),
            "25.0.1",
        );
        let found = discover(std::slice::from_ref(&configured), Some(&mc));
        assert!(
            found
                .iter()
                .any(|r| r.major == 21 && r.source == JavaSource::Configured)
        );
        let j25 = find(25, &[configured], Some(&mc)).expect("the launcher's Java 25");
        assert_eq!(j25.source, JavaSource::MinecraftLauncher);
        assert!(find(99, &[], Some(&mc)).is_none());
    }
}
