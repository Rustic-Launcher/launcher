//! The system a version is installed for, as Mojang's files name it.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Osx,
    Linux,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86,
    X86_64,
    Arm64,
    Arm32,
}

/// An OS, an architecture and the OS version (rules can test `os.version`, a pattern, and
/// `os.versionRange`, `min`/`max`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
    /// Dotted, as the official launcher reads it (`10.0.26200` on Windows 11, `15.1` on macOS).
    pub os_version: Option<String>,
}

impl Platform {
    /// This computer (the OS version comes from the caller, see [`Platform::with_os_version`]).
    pub fn current() -> Self {
        let os = if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Osx
        } else {
            Os::Linux
        };
        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Arm64
        } else if cfg!(target_arch = "x86") {
            Arch::X86
        } else {
            Arch::Arm32
        };
        Self {
            os,
            arch,
            os_version: None,
        }
    }

    pub fn with_os_version(mut self, version: Option<String>) -> Self {
        self.os_version = version.filter(|v| !v.trim().is_empty());
        self
    }

    /// `windows`, `osx` or `linux` (rules, `natives`).
    pub fn os_name(&self) -> &'static str {
        match self.os {
            Os::Windows => "windows",
            Os::Osx => "osx",
            Os::Linux => "linux",
        }
    }

    /// `x86` for 32-bit Intel, else what the JVM's `os.arch` says (rules compare it).
    pub fn arch_name(&self) -> &'static str {
        match self.arch {
            Arch::X86 => "x86",
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "arm64",
            Arch::Arm32 => "arm32",
        }
    }

    /// `${arch}` in old `natives` classifiers: `32` or `64`.
    pub fn bits(&self) -> &'static str {
        match self.arch {
            Arch::X86 | Arch::Arm32 => "32",
            Arch::X86_64 | Arch::Arm64 => "64",
        }
    }

    /// The key of Mojang's Java runtime manifest for this system, when it has one.
    pub fn runtime_key(&self) -> Option<&'static str> {
        Some(match (self.os, self.arch) {
            (Os::Windows, Arch::X86_64) => "windows-x64",
            (Os::Windows, Arch::X86) => "windows-x86",
            (Os::Windows, Arch::Arm64) => "windows-arm64",
            (Os::Linux, Arch::X86_64) => "linux",
            (Os::Linux, Arch::X86) => "linux-i386",
            (Os::Osx, Arch::X86_64) => "mac-os",
            (Os::Osx, Arch::Arm64) => "mac-os-arm64",
            _ => return None,
        })
    }

    /// The classpath separator.
    pub fn classpath_separator(&self) -> &'static str {
        if self.os == Os::Windows { ";" } else { ":" }
    }

    /// The Java executable inside a Mojang runtime folder.
    pub fn runtime_java(&self, runtime_dir: &std::path::Path) -> std::path::PathBuf {
        match self.os {
            Os::Windows => runtime_dir.join("bin").join("java.exe"),
            Os::Osx => runtime_dir.join("jre.bundle/Contents/Home/bin/java"),
            Os::Linux => runtime_dir.join("bin").join("java"),
        }
    }
}

/// The dotted OS version the official launcher compares, from what the system reports: on Windows
/// the build number (`26200`) becomes `10.0.26200` (Windows 10 and 11 both say 10.0).
pub fn os_version_from(os: Os, kernel_or_build: Option<&str>) -> Option<String> {
    let raw = kernel_or_build?.trim();
    if raw.is_empty() {
        return None;
    }
    match os {
        Os::Windows if raw.chars().all(|c| c.is_ascii_digit()) => Some(format!("10.0.{raw}")),
        _ => Some(raw.to_owned()),
    }
}

/// Compares dotted versions numerically (`10.0.9` < `10.0.17134`); missing parts are 0.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |s: &str| -> Vec<u64> {
        s.split(['.', '-', '_', ' '])
            .map(|p| {
                p.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn runtime_keys() {
        let p = |os, arch| Platform {
            os,
            arch,
            os_version: None,
        };
        assert_eq!(
            p(Os::Windows, Arch::X86_64).runtime_key(),
            Some("windows-x64")
        );
        assert_eq!(p(Os::Osx, Arch::Arm64).runtime_key(), Some("mac-os-arm64"));
        assert_eq!(p(Os::Linux, Arch::X86).runtime_key(), Some("linux-i386"));
        assert_eq!(p(Os::Linux, Arch::Arm64).runtime_key(), None);
    }

    #[test]
    fn os_versions() {
        assert_eq!(
            os_version_from(Os::Windows, Some("26200")).as_deref(),
            Some("10.0.26200")
        );
        assert_eq!(
            os_version_from(Os::Osx, Some("15.1")).as_deref(),
            Some("15.1")
        );
        assert_eq!(os_version_from(Os::Linux, Some(" ")), None);
        assert_eq!(compare_versions("10.0.9", "10.0.17134"), Ordering::Less);
        assert_eq!(
            compare_versions("10.0.17134", "10.0.17134"),
            Ordering::Equal
        );
        assert_eq!(
            compare_versions("10.0.26200", "10.0.17134"),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions("6.8.0-45-generic", "6.8"),
            Ordering::Greater
        );
    }
}
