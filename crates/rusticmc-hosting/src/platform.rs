//! What the VPS is: OS and architecture, distribution, privileges, tools, resources and the ports
//! already listening. One read-only probe script runs over SSH and prints `key=value` lines plus the
//! raw `/etc/os-release` and `ss -ltn` output; [`parse_probe`] turns that into a [`SystemInfo`].

use crate::error::{HostingError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The probe. Read-only, POSIX `sh`, every command optional (a missing tool just leaves its key out).
pub const PROBE_SCRIPT: &str = r#"printf 'uname=%s\n' "$(uname -sm 2>/dev/null)"
printf 'uid=%s\n' "$(id -u 2>/dev/null)"
printf 'user=%s\n' "$(id -un 2>/dev/null)"
printf 'home=%s\n' "$HOME"
if [ "$(id -u 2>/dev/null)" = 0 ]; then echo elevation=root
elif command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then echo elevation=sudo
else echo elevation=none; fi
for t in systemctl loginctl ufw firewall-cmd curl tar sha256sum openssl useradd adduser userdel deluser runuser su ss netstat install; do
  command -v "$t" >/dev/null 2>&1 && printf 'has=%s\n' "$t"
done
[ -d /run/systemd/system ] && echo systemd=running
awk '/^MemTotal:/ { print "mem_total_kb=" $2 } /^MemAvailable:/ { print "mem_available_kb=" $2 }' /proc/meminfo 2>/dev/null
printf 'cpus=%s\n' "$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null)"
df -Pk "$HOME" 2>/dev/null | awk 'NR == 2 { print "disk_home_available_kb=" $4 }'
df -Pk /var/lib 2>/dev/null | awk 'NR == 2 { print "disk_varlib_available_kb=" $4 }'
echo '--- os-release'
cat /etc/os-release 2>/dev/null
echo '--- listening'
(ss -Hltn 2>/dev/null || netstat -ltn 2>/dev/null) | awk '{ for (i = 1; i <= NF; i++) if ($i ~ /:[0-9]+$/) { print $i; break } }'
echo '--- end'
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86_64,
    Aarch64,
}

impl Arch {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }
}

/// How the login user can act as root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Elevation {
    /// Logged in as root.
    Root,
    /// Passwordless `sudo` (`sudo -n true` succeeded).
    Sudo,
    /// Neither: everything is installed under the login user's home, as a user service.
    None,
}

/// `/etc/os-release`'s useful keys.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsRelease {
    pub id: String,
    pub id_like: Vec<String>,
    pub version_id: String,
    pub pretty_name: String,
}

impl OsRelease {
    /// Debian, Ubuntu and their derivatives.
    pub fn is_debian_like(&self) -> bool {
        self.id == "debian" || self.id == "ubuntu" || self.id_like.iter().any(|l| l == "debian")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub kernel: String,
    pub arch: Arch,
    pub os: OsRelease,
    pub uid: u32,
    pub user: String,
    pub home: String,
    pub elevation: Elevation,
    pub tools: BTreeSet<String>,
    /// systemd is PID 1 (`/run/systemd/system` exists).
    pub systemd: bool,
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub cpus: u32,
    pub disk_home_available_kb: u64,
    pub disk_varlib_available_kb: u64,
    pub listening_ports: BTreeSet<u16>,
}

impl SystemInfo {
    pub fn has(&self, tool: &str) -> bool {
        self.tools.contains(tool)
    }

    /// The firewall front-end in charge, if any (ufw first: on Ubuntu it is the one users know).
    pub fn firewall(&self) -> Option<Firewall> {
        if self.has("ufw") {
            Some(Firewall::Ufw)
        } else if self.has("firewall-cmd") {
            Some(Firewall::Firewalld)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Firewall {
    Ufw,
    Firewalld,
}

/// Parse `/etc/os-release` (shell-style `KEY=value`, values optionally quoted).
pub fn parse_os_release(text: &str) -> OsRelease {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_owned(), unquote(v.trim()));
        }
    }
    OsRelease {
        id: map
            .get("ID")
            .cloned()
            .unwrap_or_default()
            .to_ascii_lowercase(),
        id_like: map
            .get("ID_LIKE")
            .map(|s| s.split_whitespace().map(str::to_ascii_lowercase).collect())
            .unwrap_or_default(),
        version_id: map.get("VERSION_ID").cloned().unwrap_or_default(),
        pretty_name: map.get("PRETTY_NAME").cloned().unwrap_or_default(),
    }
}

fn unquote(v: &str) -> String {
    let inner = if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        &v[1..v.len() - 1]
    } else {
        v
    };
    // os-release allows backslash escapes for `"`, `\`, `$` and backtick.
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse `uname -sm`: only Linux on x86_64 or aarch64 is accepted; everything else is a clear error.
pub fn parse_uname(uname: &str) -> Result<(String, Arch)> {
    let mut parts = uname.split_whitespace();
    let os = parts.next().unwrap_or_default().to_owned();
    let machine = parts.next().unwrap_or_default().to_owned();
    if os != "Linux" {
        return Err(HostingError::UnsupportedPlatform {
            os: if os.is_empty() { "unknown".into() } else { os },
            arch: machine,
            detail: "Rustic installs servers on Linux VPSes only (x86_64 or aarch64)".into(),
        });
    }
    let arch = match machine.as_str() {
        "x86_64" | "amd64" => Arch::X86_64,
        "aarch64" | "arm64" => Arch::Aarch64,
        _ => {
            return Err(HostingError::UnsupportedPlatform {
                os,
                arch: machine,
                detail: "supported architectures are x86_64 and aarch64".into(),
            });
        }
    };
    Ok((os, arch))
}

/// The port of `addr:port`, `[v6]:port` or `*:port`, as `ss`/`netstat` print local addresses.
fn port_of(addr: &str) -> Option<u16> {
    addr.rsplit_once(':').and_then(|(_, p)| p.parse().ok())
}

/// Parse the probe's output.
pub fn parse_probe(output: &str) -> Result<SystemInfo> {
    let mut kv: BTreeMap<String, String> = BTreeMap::new();
    let mut tools = BTreeSet::new();
    let mut section = "";
    let mut os_release = String::new();
    let mut ports = BTreeSet::new();
    for line in output.lines() {
        if let Some(name) = line.strip_prefix("--- ") {
            section = match name.trim() {
                "os-release" => "os",
                "listening" => "ports",
                _ => "end",
            };
            continue;
        }
        match section {
            "" => {
                if let Some((k, v)) = line.split_once('=') {
                    if k == "has" {
                        tools.insert(v.trim().to_owned());
                    } else {
                        kv.insert(k.to_owned(), v.trim().to_owned());
                    }
                }
            }
            "os" => {
                os_release.push_str(line);
                os_release.push('\n');
            }
            "ports" => {
                if let Some(p) = port_of(line.trim()) {
                    ports.insert(p);
                }
            }
            _ => {}
        }
    }
    let uname = kv
        .get("uname")
        .ok_or_else(|| HostingError::Protocol("the system probe printed no `uname`".into()))?;
    let (kernel, arch) = parse_uname(uname)?;
    let num = |k: &str| kv.get(k).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let elevation = match kv.get("elevation").map(String::as_str) {
        Some("root") => Elevation::Root,
        Some("sudo") => Elevation::Sudo,
        _ => Elevation::None,
    };
    Ok(SystemInfo {
        kernel,
        arch,
        os: parse_os_release(&os_release),
        uid: num("uid") as u32,
        user: kv.get("user").cloned().unwrap_or_default(),
        home: kv.get("home").cloned().unwrap_or_default(),
        elevation,
        tools,
        systemd: kv.get("systemd").is_some_and(|v| v == "running"),
        mem_total_kb: num("mem_total_kb"),
        mem_available_kb: num("mem_available_kb"),
        cpus: num("cpus") as u32,
        disk_home_available_kb: num("disk_home_available_kb"),
        disk_varlib_available_kb: num("disk_varlib_available_kb"),
        listening_ports: ports,
    })
}

/// Minimum and recommended resources (docs/HOSTING.md, "VPS requirements"). Horizon's own numbers:
/// 0.7 GB RSS with one player, 1.1 GB with 50 (docs/hosting.md, "How fast is it").
pub const MIN_RAM_KB: u64 = 1024 * 1024;
pub const RECOMMENDED_RAM_KB: u64 = 2 * 1024 * 1024;
/// The release (~100 MB unpacked), the prepared game files and a world with a few backups.
pub const MIN_DISK_KB: u64 = 2 * 1024 * 1024;

/// What the requirement check found: blocking problems, and warnings to show.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementReport {
    pub blocking: Vec<String>,
    pub warnings: Vec<String>,
}

/// Check RAM, disk and the game port. `port_is_ours` skips the port check on a re-run where our own
/// server is the one listening.
pub fn check_requirements(
    info: &SystemInfo,
    disk_available_kb: u64,
    port: u16,
    port_is_ours: bool,
) -> RequirementReport {
    let mut r = RequirementReport::default();
    if info.mem_total_kb > 0 && info.mem_total_kb < MIN_RAM_KB {
        r.blocking.push(format!(
            "{} MB of RAM; a server needs at least 1024 MB",
            info.mem_total_kb / 1024
        ));
    } else if info.mem_total_kb > 0 && info.mem_total_kb < RECOMMENDED_RAM_KB {
        r.warnings.push(format!(
            "{} MB of RAM: fine for a few players; 2 GB or more is recommended (and `engine.low_memory = true`)",
            info.mem_total_kb / 1024
        ));
    }
    if disk_available_kb > 0 && disk_available_kb < MIN_DISK_KB {
        r.blocking.push(format!(
            "{} MB of free disk; at least 2048 MB are needed",
            disk_available_kb / 1024
        ));
    }
    if !port_is_ours && info.listening_ports.contains(&port) {
        r.blocking.push(format!(
            "TCP port {port} is already in use on the VPS: pick another port"
        ));
    }
    for tool in ["curl", "tar", "sha256sum"] {
        if !info.has(tool) {
            r.blocking.push(format!(
                "`{tool}` is missing on the VPS (Horizon's installer needs it): install it with your package manager"
            ));
        }
    }
    if !info.has("openssl") {
        r.warnings.push(
            "OpenSSL 3 is not installed: Horizon's installer then checks GitHub's SHA-256 only, not the release signature".into(),
        );
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    const UBUNTU: &str = "uname=Linux x86_64
uid=1000
user=alex
home=/home/alex
elevation=sudo
has=systemctl
has=loginctl
has=ufw
has=curl
has=tar
has=sha256sum
has=openssl
has=useradd
has=ss
systemd=running
mem_total_kb=4026532
mem_available_kb=3500000
cpus=2
disk_home_available_kb=40000000
disk_varlib_available_kb=40000000
--- os-release
PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"
NAME=\"Ubuntu\"
VERSION_ID=\"24.04\"
ID=ubuntu
ID_LIKE=debian
--- listening
0.0.0.0:22
[::]:22
127.0.0.53%lo:53
*:8080
--- end
";

    #[test]
    fn parses_a_full_probe() {
        let info = parse_probe(UBUNTU).unwrap();
        assert_eq!(info.arch, Arch::X86_64);
        assert_eq!(info.kernel, "Linux");
        assert_eq!(info.elevation, Elevation::Sudo);
        assert_eq!(info.user, "alex");
        assert_eq!(info.home, "/home/alex");
        assert!(info.systemd);
        assert!(info.has("ufw"));
        assert_eq!(info.firewall(), Some(Firewall::Ufw));
        assert_eq!(info.os.id, "ubuntu");
        assert_eq!(info.os.version_id, "24.04");
        assert_eq!(info.os.pretty_name, "Ubuntu 24.04.1 LTS");
        assert!(info.os.is_debian_like());
        assert_eq!(info.cpus, 2);
        assert_eq!(
            info.listening_ports.iter().copied().collect::<Vec<_>>(),
            vec![22, 53, 8080]
        );
    }

    #[test]
    fn uname_variants() {
        assert_eq!(parse_uname("Linux x86_64").unwrap().1, Arch::X86_64);
        assert_eq!(parse_uname("Linux aarch64").unwrap().1, Arch::Aarch64);
        let e = parse_uname("Linux armv7l").unwrap_err();
        assert!(matches!(e, HostingError::UnsupportedPlatform { .. }));
        assert!(e.to_string().contains("armv7l"));
        let e = parse_uname("Darwin arm64").unwrap_err();
        assert!(e.to_string().contains("Linux VPSes only"));
        let e = parse_uname("FreeBSD amd64").unwrap_err();
        assert!(e.to_string().contains("FreeBSD"));
        assert!(parse_uname("").is_err());
    }

    #[test]
    fn os_release_quotes_and_escapes() {
        let r = parse_os_release(
            "# comment\nID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\nVERSION_ID='9.4'\nPRETTY_NAME=\"Rocky \\\"Blue\\\" Onyx\"\n",
        );
        assert_eq!(r.id, "rocky");
        assert_eq!(r.id_like, vec!["rhel", "centos", "fedora"]);
        assert_eq!(r.version_id, "9.4");
        assert_eq!(r.pretty_name, "Rocky \"Blue\" Onyx");
        assert!(!r.is_debian_like());
    }

    #[test]
    fn root_without_systemd() {
        let probe = "uname=Linux aarch64\nuid=0\nuser=root\nhome=/root\nelevation=root\nhas=curl\n--- os-release\nID=alpine\n--- listening\n--- end\n";
        let info = parse_probe(probe).unwrap();
        assert_eq!(info.elevation, Elevation::Root);
        assert_eq!(info.arch, Arch::Aarch64);
        assert!(!info.systemd);
        assert_eq!(info.firewall(), None);
        assert!(info.listening_ports.is_empty());
    }

    #[test]
    fn missing_uname_is_a_protocol_error() {
        assert!(matches!(
            parse_probe("user=x\n"),
            Err(HostingError::Protocol(_))
        ));
    }

    #[test]
    fn requirements() {
        let mut info = parse_probe(UBUNTU).unwrap();
        let ok = check_requirements(&info, 40_000_000, 25565, false);
        assert!(ok.blocking.is_empty(), "{ok:?}");
        let taken = check_requirements(&info, 40_000_000, 8080, false);
        assert_eq!(taken.blocking.len(), 1);
        assert!(taken.blocking[0].contains("8080"));
        assert!(
            check_requirements(&info, 40_000_000, 8080, true)
                .blocking
                .is_empty()
        );
        info.mem_total_kb = 512 * 1024;
        let small = check_requirements(&info, 1024, 25565, false);
        assert_eq!(small.blocking.len(), 2, "{small:?}");
        info.mem_total_kb = 1536 * 1024;
        let tight = check_requirements(&info, 40_000_000, 25565, false);
        assert!(tight.blocking.is_empty());
        assert_eq!(tight.warnings.len(), 1);
        info.tools.remove("curl");
        info.tools.remove("openssl");
        let tools = check_requirements(&info, 40_000_000, 25565, false);
        assert!(tools.blocking.iter().any(|b| b.contains("curl")));
        assert!(tools.warnings.iter().any(|w| w.contains("OpenSSL")));
    }
}
