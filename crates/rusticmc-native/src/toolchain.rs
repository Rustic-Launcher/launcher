//! The Rust a native build compiles with.
//!
//! Exactly the release Horizon's launcher pins for this Horizon ([`RUST_VERSION`],
//! `horizon/launcher/src/rust_toolchain.rs` at the pinned commit; Horizon's workspace has no
//! `rust-toolchain.toml`), so the build compiles as the edition's CI does and Cargo's cache stays
//! valid. Found in this order ([`detect`]):
//!
//! 1. the launcher's own (`<data>/native/rustup`), installed earlier;
//! 2. the player's rustup, with that release installed or as the default;
//! 3. a plain `rustc` of that release on `PATH`.
//!
//! A system Rust also needs its linker: on Windows an MSVC Rust needs Visual Studio's C++ build
//! tools (found with `vswhere`), a GNU one `gcc`; elsewhere `cc`. On Linux the game also builds
//! against ALSA and libudev (`pkg-config`), which the player installs ([`prerequisites`]).
//!
//! When none fits, the launcher installs its own after the player agreed ([`install`]):
//! rustup-init from static.rust-lang.org, checked against the SHA-256 published beside it, run
//! silently with `RUSTUP_HOME`/`CARGO_HOME` in the launcher's folder (`--no-modify-path`: nothing
//! changes outside it), the minimal profile of [`RUST_VERSION`]. On Windows x64 that is the GNU
//! host (`x86_64-pc-windows-gnu`, as Horizon's launcher) with WinLibs' GCC ([`MINGW`], the archive
//! and SHA-256 Horizon's launcher pins), so no Visual Studio is needed.
use crate::error::{IoContext, NativeError, Result};
use crate::paths::NativeRoot;
use crate::process;
use rusticmc_launcher_core::{CancellationToken, Checksum, DownloadManager, DownloadRequest};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// The Rust release native builds use (Horizon's launcher `rust_toolchain::VERSION` at a84f637d).
pub const RUST_VERSION: &str = "1.99.1";

/// WinLibs' GCC and binutils on MinGW-w64 (MSVCRT, POSIX threads, SEH): the C compiler and linker
/// of the launcher's own Rust on Windows x64. Url and SHA-256 as Horizon's launcher pins them
/// (`rust_toolchain::MINGW`).
pub const MINGW: Archive = Archive {
    name: "GCC 16.2.0, MinGW-w64 14.0.0 (WinLibs r3)",
    url: "https://github.com/brechtsanders/winlibs_mingw/releases/download/16.2.0posix-23.1.3-14.0.0-msvcrt-r3/winlibs-x86_64-posix-seh-gcc-16.2.0-mingw-w64msvcrt-14.0.0-r3.zip",
    sha256: "62af32ea58acdea1318ea5dd8eb7017eaee694dd40b269d52a59c769dcfefb82",
};

/// A pinned download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Archive {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

/// Where a usable Rust comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Installed by the launcher in its own folder.
    Managed,
    /// The player's rustup.
    Rustup,
    /// A `rustc` on `PATH`.
    System,
}

/// A Rust native builds can use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Toolchain {
    pub kind: Kind,
    pub version: String,
    pub host: String,
    /// The folder with `cargo` and `rustc` (a toolchain's own `bin/`, never rustup's proxies).
    pub bin: PathBuf,
    /// Folders put first on `PATH` for the build (the launcher's MinGW).
    pub path_prepend: Vec<PathBuf>,
}

impl Toolchain {
    pub fn cargo(&self) -> PathBuf {
        self.bin.join(crate::exe("cargo"))
    }
    pub fn rustc(&self) -> PathBuf {
        self.bin.join(crate::exe("rustc"))
    }

    /// `program` in the build's environment: the player's Rust settings dropped
    /// ([`process::scrub`]), this toolchain's `rustc`, the launcher's Cargo home, its temp folder,
    /// no console window, below-normal priority. It starts from the drive's root: Cargo reads a
    /// `.cargo/config.toml` in the folder it starts in and every one above it, and the launcher's
    /// data folder is under the player's home, whose `.cargo/config.toml` would apply.
    pub fn command(&self, program: &Path, root: &NativeRoot) -> Command {
        let mut cmd = Command::new(program);
        process::scrub(&mut cmd);
        process::background(&mut cmd);
        cmd.env("RUSTC", self.rustc())
            .env("CARGO_HOME", root.cargo_home())
            .env("CARGO_TERM_COLOR", "never");
        if self.kind == Kind::Managed {
            cmd.env("RUSTUP_HOME", root.rustup_home());
        }
        let tmp = root.tmp();
        let _ = std::fs::create_dir_all(&tmp);
        for var in ["TMP", "TEMP", "TMPDIR"] {
            cmd.env(var, &tmp);
        }
        let mut path: Vec<PathBuf> = self.path_prepend.clone();
        path.push(self.bin.clone());
        if let Some(old) = std::env::var_os("PATH") {
            path.extend(std::env::split_paths(&old));
        }
        if let Ok(joined) = std::env::join_paths(path) {
            cmd.env("PATH", joined);
        }
        cmd.current_dir(drive_root(root.root()));
        cmd
    }
}

/// The root of the drive (Windows) or `/`.
pub fn drive_root(path: &Path) -> PathBuf {
    path.ancestors()
        .last()
        .map(Path::to_path_buf)
        .filter(|p| p.is_absolute() || cfg!(windows))
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Whether a usable Rust is there, and if not why and whether the launcher can install one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ToolchainStatus {
    Ready {
        toolchain: Toolchain,
    },
    Missing {
        /// Why no Rust found fits.
        reason: String,
        /// The launcher can install its own (after the player agreed).
        installable: bool,
        /// What the player has to install themselves first (Linux and macOS system packages).
        prerequisites: Vec<String>,
    },
}

impl ToolchainStatus {
    pub fn ready(&self) -> Option<&Toolchain> {
        match self {
            Self::Ready { toolchain } => Some(toolchain),
            Self::Missing { .. } => None,
        }
    }
}

/// What detection asks of the computer, so it is testable without one.
pub trait Probe {
    /// Standard output of `program args`, when it ran and succeeded.
    fn output(&self, program: &Path, args: &[&str]) -> Option<String>;
    /// A program found on `PATH`.
    fn which(&self, name: &str) -> Option<PathBuf>;
    fn is_file(&self, path: &Path) -> bool;
    /// Windows: Visual Studio's C++ build tools (MSVC's linker) are installed.
    fn msvc_linker(&self) -> bool;
    /// The player's home (for `~/.cargo/bin`).
    fn home(&self) -> Option<PathBuf>;
    fn os(&self) -> &str {
        std::env::consts::OS
    }
    fn arch(&self) -> &str {
        std::env::consts::ARCH
    }
}

/// The real computer.
pub struct System;

impl Probe for System {
    fn output(&self, program: &Path, args: &[&str]) -> Option<String> {
        let mut cmd = std::process::Command::new(program);
        // Asking rustup about a toolchain it lacks must never download it.
        cmd.env("RUSTUP_AUTO_INSTALL", "0")
            .env_remove("RUSTUP_TOOLCHAIN")
            .args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let out = cmd.output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn which(&self, name: &str) -> Option<PathBuf> {
        let file = crate::exe(name);
        std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|dir| dir.join(&file))
            .find(|p| p.is_file())
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn msvc_linker(&self) -> bool {
        if !cfg!(windows) {
            return false;
        }
        let base = std::env::var_os("ProgramFiles(x86)")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
        let vswhere = base.join(r"Microsoft Visual Studio\Installer\vswhere.exe");
        if !vswhere.is_file() {
            return false;
        }
        self.output(
            &vswhere,
            &[
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ],
        )
        .is_some_and(|out| !out.trim().is_empty())
    }

    fn home(&self) -> Option<PathBuf> {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
    }
}

/// `rustc -vV`'s release and host.
fn version_of(probe: &dyn Probe, rustc: &Path) -> Option<(String, String)> {
    let out = probe.output(rustc, &["-vV"])?;
    let field = |name: &str| {
        out.lines()
            .find_map(|l| l.strip_prefix(name))
            .map(|v| v.trim().to_owned())
    };
    Some((field("release:")?, field("host:")?))
}

/// The host the launcher installs its own Rust for on this computer, if it can.
pub fn managed_host(os: &str, arch: &str) -> Option<&'static str> {
    Some(match (os, arch) {
        ("windows", "x86_64") => "x86_64-pc-windows-gnu",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        _ => return None,
    })
}

/// The folder of the launcher's own toolchain.
fn managed_bin(root: &NativeRoot, host: &str) -> PathBuf {
    root.rustup_home()
        .join("toolchains")
        .join(format!("{RUST_VERSION}-{host}"))
        .join("bin")
}

/// Whether `host`'s linker is there.
fn linker_ok(probe: &dyn Probe, host: &str) -> std::result::Result<(), String> {
    if host.ends_with("windows-msvc") {
        if probe.msvc_linker() {
            Ok(())
        } else {
            Err(
                "this Rust targets MSVC, and Visual Studio's C++ build tools are not installed"
                    .into(),
            )
        }
    } else if host.contains("windows-gnu") {
        if probe.which("gcc").is_some() {
            Ok(())
        } else {
            Err("this Rust targets MinGW, and no gcc is on PATH".into())
        }
    } else if probe.which("cc").is_some() {
        Ok(())
    } else {
        Err("no C compiler (cc) is installed".into())
    }
}

/// What the player must install on Linux and macOS before any build (the launcher can't install
/// system packages): a C compiler and, on Linux, `pkg-config` with ALSA's and libudev's development
/// files, which the game links (as Horizon's launcher checks).
pub fn prerequisites(probe: &dyn Probe) -> Vec<String> {
    let mut missing = Vec::new();
    match probe.os() {
        "linux" => {
            if probe.which("cc").is_none() {
                missing.push("a C compiler (cc), e.g. `sudo apt install build-essential`".into());
            }
            match probe.which("pkg-config") {
                None => missing.push("pkg-config (`sudo apt install pkg-config`)".into()),
                Some(pc) => {
                    for (lib, pkg) in [("alsa", "libasound2-dev"), ("libudev", "libudev-dev")] {
                        if probe.output(&pc, &["--exists", lib]).is_none() {
                            missing.push(format!(
                                "{lib} development files (`sudo apt install {pkg}`)"
                            ));
                        }
                    }
                }
            }
        }
        "macos"
            if probe
                .output(Path::new("xcode-select"), &["-p"])
                .is_none_or(|p| p.trim().is_empty()) =>
        {
            missing.push("Apple's Command Line Tools (`xcode-select --install`)".into());
        }
        _ => {}
    }
    missing
}

/// Looks for a usable Rust (see the module's order).
pub fn detect_with(probe: &dyn Probe, root: &NativeRoot) -> ToolchainStatus {
    let mut why: Vec<String> = Vec::new();
    let prerequisites = prerequisites(probe);
    // 1. The launcher's own.
    if let Some(host) = managed_host(probe.os(), probe.arch()) {
        let bin = managed_bin(root, host);
        let rustc = bin.join(crate::exe("rustc"));
        if probe.is_file(&rustc) {
            let mingw = root.mingw().join("bin");
            let mingw_ok = !host.contains("windows-gnu") || probe.is_file(&mingw.join("gcc.exe"));
            match version_of(probe, &rustc) {
                Some((v, h)) if v == RUST_VERSION && mingw_ok && prerequisites.is_empty() => {
                    return ToolchainStatus::Ready {
                        toolchain: Toolchain {
                            kind: Kind::Managed,
                            version: v,
                            host: h,
                            bin,
                            path_prepend: if host.contains("windows-gnu") {
                                vec![mingw]
                            } else {
                                vec![]
                            },
                        },
                    };
                }
                Some(_) if !mingw_ok => why.push("the launcher's Rust is missing its MinGW".into()),
                Some((v, _)) => why.push(format!("the launcher's Rust is {v}")),
                None => why.push("the launcher's Rust does not run".into()),
            }
        }
    }
    // 2. rustup's: the pinned release, else its default.
    let home_bin = probe.home().map(|h| h.join(".cargo").join("bin"));
    let rustup = probe.which("rustup").or_else(|| {
        home_bin
            .as_ref()
            .map(|b| b.join(crate::exe("rustup")))
            .filter(|p| probe.is_file(p))
    });
    let mut candidates: Vec<(Kind, PathBuf)> = Vec::new();
    if let Some(rustup) = &rustup {
        for args in [
            &["which", "--toolchain", RUST_VERSION, "rustc"][..],
            &["which", "rustc"][..],
        ] {
            if let Some(path) = probe.output(rustup, args) {
                let path = PathBuf::from(path.trim());
                if !path.as_os_str().is_empty() {
                    candidates.push((Kind::Rustup, path));
                }
            }
        }
    }
    // 3. A plain rustc on PATH (not a rustup proxy, which the above covers).
    if rustup.is_none()
        && let Some(rustc) = probe.which("rustc")
    {
        candidates.push((Kind::System, rustc));
    }
    for (kind, rustc) in candidates {
        let Some((version, host)) = version_of(probe, &rustc) else {
            continue;
        };
        if version != RUST_VERSION {
            why.push(format!("Rust {version} is installed"));
            continue;
        }
        let bin = rustc.parent().map(Path::to_path_buf).unwrap_or_default();
        if !probe.is_file(&bin.join(crate::exe("cargo"))) {
            why.push(format!("Rust {version} has no cargo"));
            continue;
        }
        if let Err(e) = linker_ok(probe, &host) {
            why.push(e);
            continue;
        }
        if !prerequisites.is_empty() {
            break;
        }
        return ToolchainStatus::Ready {
            toolchain: Toolchain {
                kind,
                version,
                host,
                bin,
                path_prepend: vec![],
            },
        };
    }
    let installable = managed_host(probe.os(), probe.arch()).is_some();
    if why.is_empty() {
        why.push("Rust is not installed".into());
    }
    if !installable {
        why.push(format!(
            "the launcher can't install Rust on {} {}",
            probe.os(),
            probe.arch()
        ));
    }
    why.dedup();
    ToolchainStatus::Missing {
        reason: why.join("; "),
        installable: installable && prerequisites.is_empty(),
        prerequisites,
    }
}

/// [`detect_with`] on this computer.
pub async fn detect(root: &NativeRoot) -> ToolchainStatus {
    let root = root.clone();
    tokio::task::spawn_blocking(move || detect_with(&System, &root))
        .await
        .unwrap_or_else(|e| ToolchainStatus::Missing {
            reason: e.to_string(),
            installable: false,
            prerequisites: vec![],
        })
}

/// rustup-init for this computer: its URL and the URL of its published SHA-256.
pub fn rustup_init_url(os: &str, arch: &str) -> Option<(String, String)> {
    // The installer itself; the toolchain's host is chosen with --default-host.
    let triple = match (os, arch) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        _ => return None,
    };
    let exe = if os == "windows" { ".exe" } else { "" };
    let url = format!("https://static.rust-lang.org/rustup/dist/{triple}/rustup-init{exe}");
    Some((url.clone(), format!("{url}.sha256")))
}

/// The hash in a `.sha256` file (`<hex>  <name>` or `<hex> *<name>`).
pub fn parse_sha256(text: &str) -> Option<Checksum> {
    Checksum::sha256(text.split_whitespace().next()?)
}

/// Installs the launcher's own Rust (see the module), reporting `(fraction, message)`.
pub async fn install(
    root: &NativeRoot,
    downloads: &DownloadManager,
    cancel: &CancellationToken,
    on_progress: &(dyn Fn(f64, &str) + Send + Sync),
) -> Result<Toolchain> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let host = managed_host(os, arch)
        .ok_or_else(|| NativeError::ToolchainInstall(format!("not available on {os} {arch}")))?;
    let missing = prerequisites(&System);
    if !missing.is_empty() {
        return Err(NativeError::ToolchainInstall(format!(
            "install these first: {}",
            missing.join(", ")
        )));
    }
    let (url, sha_url) = rustup_init_url(os, arch).expect("a managed host has an installer");
    std::fs::create_dir_all(root.downloads()).at(&root.downloads())?;
    on_progress(0.02, "Checking rustup-init's published checksum");
    let sha_text = downloads
        .client()
        .get(&sha_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| NativeError::Download(format!("{sha_url}: {e}")))?
        .text()
        .await
        .map_err(|e| NativeError::Download(format!("{sha_url}: {e}")))?;
    let checksum = parse_sha256(&sha_text)
        .ok_or_else(|| NativeError::ToolchainInstall(format!("{sha_url}: no SHA-256 in it")))?;
    on_progress(0.05, "Downloading rustup-init");
    let init = root.downloads().join(crate::exe("rustup-init"));
    let req = DownloadRequest::new(url, &init)
        .checksum(checksum)
        .label("rustup-init")
        .no_cache();
    download(downloads, req, cancel).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&init, std::fs::Permissions::from_mode(0o755));
    }
    on_progress(0.1, &format!("Installing Rust {RUST_VERSION} (rustup)"));
    let mut cmd = Command::new(&init);
    process::scrub(&mut cmd);
    process::background(&mut cmd);
    cmd.env("RUSTUP_HOME", root.rustup_home())
        .env("CARGO_HOME", root.cargo_home())
        .env("RUSTUP_INIT_SKIP_PATH_CHECK", "yes")
        .args([
            "-y",
            "--no-modify-path",
            "--profile",
            "minimal",
            "--default-toolchain",
            RUST_VERSION,
            "--default-host",
            host,
        ]);
    let finished = process::run(
        cmd,
        cancel,
        |l| tracing::debug!(target: "native", "rustup-init: {l}"),
        |l| {
            tracing::debug!(target: "native", "rustup-init: {l}");
            if l.contains("downloading component") || l.contains("installing component") {
                on_progress(0.4, l.trim());
            }
        },
    )
    .await?;
    if !finished.success {
        return Err(NativeError::ToolchainInstall(format!(
            "rustup-init failed: {}",
            finished.stderr_tail.join("\n")
        )));
    }
    let _ = std::fs::remove_file(&init);
    if host.contains("windows-gnu") {
        on_progress(0.6, &format!("Downloading {}", MINGW.name));
        install_mingw(root, downloads, cancel).await?;
    }
    on_progress(0.98, "Checking the installed Rust");
    match detect(root).await {
        ToolchainStatus::Ready { toolchain } if toolchain.kind == Kind::Managed => Ok(toolchain),
        ToolchainStatus::Ready { toolchain } => Ok(toolchain),
        ToolchainStatus::Missing { reason, .. } => Err(NativeError::ToolchainInstall(reason)),
    }
}

async fn download(
    downloads: &DownloadManager,
    req: DownloadRequest,
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    let handle = downloads.enqueue(req);
    let token = handle.token();
    tokio::select! {
        r = handle.wait() => r.map_err(|e| match e {
            rusticmc_launcher_core::DownloadError::Cancelled => NativeError::Cancelled,
            e => NativeError::Download(e.to_string()),
        }),
        () = cancel.cancelled() => {
            token.cancel();
            Err(NativeError::Cancelled)
        }
    }
}

/// WinLibs' `mingw64/` into `<native>/mingw64`, its documentation left out.
async fn install_mingw(
    root: &NativeRoot,
    downloads: &DownloadManager,
    cancel: &CancellationToken,
) -> Result<()> {
    let zip_path = root.downloads().join("winlibs.zip");
    let req = DownloadRequest::new(MINGW.url, &zip_path)
        .checksum(Checksum::sha256(MINGW.sha256).expect("a pinned SHA-256"))
        .label(MINGW.name)
        .no_cache();
    download(downloads, req, cancel).await?;
    let dest = root.mingw();
    let partial = root.root().join(".mingw64.partial");
    let zip_for_task = zip_path.clone();
    let partial_for_task = partial.clone();
    tokio::task::spawn_blocking(move || unpack_mingw(&zip_for_task, &partial_for_task))
        .await
        .map_err(|e| NativeError::Other(e.to_string()))??;
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(partial.join("mingw64"), &dest).at(&dest)?;
    let _ = std::fs::remove_dir_all(&partial);
    let _ = std::fs::remove_file(&zip_path);
    Ok(())
}

fn unpack_mingw(zip_path: &Path, partial: &Path) -> Result<()> {
    let _ = std::fs::remove_dir_all(partial);
    std::fs::create_dir_all(partial).at(partial)?;
    let file = std::fs::File::open(zip_path).at(zip_path)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| NativeError::ToolchainInstall(format!("{}: {e}", MINGW.name)))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| NativeError::ToolchainInstall(e.to_string()))?;
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let text = rel.to_string_lossy().replace('\\', "/");
        if !text.starts_with("mingw64/") || text.starts_with("mingw64/share/doc/") {
            continue;
        }
        let out = partial.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).at(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).at(parent)?;
        }
        let mut w = std::fs::File::create(&out).at(&out)?;
        std::io::copy(&mut entry, &mut w).at(&out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    /// A computer described by hand.
    #[derive(Default)]
    struct Fake {
        os: &'static str,
        arch: &'static str,
        outputs: HashMap<(PathBuf, String), String>,
        path: HashMap<&'static str, PathBuf>,
        files: HashSet<PathBuf>,
        msvc: bool,
        home: Option<PathBuf>,
    }

    impl Fake {
        fn windows() -> Self {
            Self {
                os: "windows",
                arch: "x86_64",
                home: Some(PathBuf::from("/home/p")),
                ..Default::default()
            }
        }
        fn out(mut self, program: &str, args: &[&str], out: &str) -> Self {
            self.outputs
                .insert((PathBuf::from(program), args.join(" ")), out.into());
            self
        }
        fn on_path(mut self, name: &'static str, at: &str) -> Self {
            self.path.insert(name, PathBuf::from(at));
            self
        }
        fn file(mut self, at: impl Into<PathBuf>) -> Self {
            self.files.insert(at.into());
            self
        }
    }

    impl Probe for Fake {
        fn output(&self, program: &Path, args: &[&str]) -> Option<String> {
            self.outputs
                .get(&(program.to_path_buf(), args.join(" ")))
                .cloned()
        }
        fn which(&self, name: &str) -> Option<PathBuf> {
            self.path.get(name).cloned()
        }
        fn is_file(&self, path: &Path) -> bool {
            self.files.contains(path)
        }
        fn msvc_linker(&self) -> bool {
            self.msvc
        }
        fn home(&self) -> Option<PathBuf> {
            self.home.clone()
        }
        fn os(&self) -> &str {
            self.os
        }
        fn arch(&self) -> &str {
            self.arch
        }
    }

    fn vv(release: &str, host: &str) -> String {
        format!("rustc {release} (x 2026-10-13)\nbinary: rustc\nhost: {host}\nrelease: {release}\n")
    }

    fn root() -> NativeRoot {
        NativeRoot::at("/data/native")
    }

    #[test]
    fn nothing_installed_is_missing_and_installable() {
        let s = detect_with(&Fake::windows(), &root());
        let ToolchainStatus::Missing {
            reason,
            installable,
            prerequisites,
        } = s
        else {
            panic!("{s:?}")
        };
        assert!(installable);
        assert!(prerequisites.is_empty());
        assert!(reason.contains("not installed"), "{reason}");
    }

    #[test]
    fn rustup_with_the_pinned_release_and_msvc_is_used_directly() {
        let rustc = "/home/p/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/rustc.exe";
        let mut fake = Fake::windows()
            .on_path("rustup", "/home/p/.cargo/bin/rustup.exe")
            .out(
                "/home/p/.cargo/bin/rustup.exe",
                &["which", "rustc"],
                &format!("{rustc}\n"),
            )
            .out(rustc, &["-vV"], &vv(RUST_VERSION, "x86_64-pc-windows-msvc"))
            .file("/home/p/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe");
        fake.msvc = true;
        let s = detect_with(&fake, &root());
        let t = s.ready().expect("ready");
        assert_eq!(t.kind, Kind::Rustup);
        assert_eq!(
            t.bin,
            PathBuf::from("/home/p/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin")
        );
        assert_eq!(t.host, "x86_64-pc-windows-msvc");
    }

    #[test]
    fn msvc_rust_without_build_tools_is_not_used() {
        let rustc = "/r/bin/rustc.exe";
        let fake = Fake::windows()
            .on_path("rustup", "/r/rustup.exe")
            .out("/r/rustup.exe", &["which", "rustc"], rustc)
            .out(rustc, &["-vV"], &vv(RUST_VERSION, "x86_64-pc-windows-msvc"))
            .file("/r/bin/cargo.exe");
        let s = detect_with(&fake, &root());
        let ToolchainStatus::Missing {
            reason,
            installable,
            ..
        } = s
        else {
            panic!("{s:?}")
        };
        assert!(reason.contains("Visual Studio"), "{reason}");
        assert!(installable);
    }

    #[test]
    fn another_release_is_not_used() {
        let fake = Fake::windows()
            .on_path("rustc", "/usr/bin/rustc.exe")
            .out(
                "/usr/bin/rustc.exe",
                &["-vV"],
                &vv("1.80.0", "x86_64-pc-windows-gnu"),
            )
            .file("/usr/bin/cargo.exe")
            .on_path("gcc", "/usr/bin/gcc.exe");
        let s = detect_with(&fake, &root());
        let ToolchainStatus::Missing { reason, .. } = s else {
            panic!("{s:?}")
        };
        assert!(reason.contains("1.80.0"), "{reason}");
    }

    #[test]
    fn the_launchers_own_rust_wins_when_complete() {
        let bin = managed_bin(&root(), "x86_64-pc-windows-gnu");
        let rustc = bin.join("rustc.exe");
        let fake = Fake::windows()
            .file(rustc.clone())
            .out(
                rustc.to_str().unwrap(),
                &["-vV"],
                &vv(RUST_VERSION, "x86_64-pc-windows-gnu"),
            )
            .file(root().mingw().join("bin").join("gcc.exe"));
        let t = detect_with(&fake, &root()).ready().cloned().expect("ready");
        assert_eq!(t.kind, Kind::Managed);
        assert_eq!(t.path_prepend, vec![root().mingw().join("bin")]);
        // Without MinGW it is not complete.
        let fake = Fake::windows().file(rustc.clone()).out(
            rustc.to_str().unwrap(),
            &["-vV"],
            &vv(RUST_VERSION, "x86_64-pc-windows-gnu"),
        );
        assert!(detect_with(&fake, &root()).ready().is_none());
    }

    #[test]
    fn linux_names_the_system_packages_to_install() {
        let fake = Fake {
            os: "linux",
            arch: "x86_64",
            ..Default::default()
        }
        .on_path("pkg-config", "/usr/bin/pkg-config")
        .out("/usr/bin/pkg-config", &["--exists", "alsa"], "");
        let ToolchainStatus::Missing {
            prerequisites,
            installable,
            ..
        } = detect_with(&fake, &root())
        else {
            panic!()
        };
        assert!(!installable);
        assert!(prerequisites.iter().any(|p| p.contains("build-essential")));
        assert!(prerequisites.iter().any(|p| p.contains("libudev-dev")));
        assert!(!prerequisites.iter().any(|p| p.contains("libasound2-dev")));
    }

    #[test]
    fn unsupported_platforms_are_not_installable() {
        let fake = Fake {
            os: "freebsd",
            arch: "x86_64",
            ..Default::default()
        };
        let ToolchainStatus::Missing { installable, .. } = detect_with(&fake, &root()) else {
            panic!()
        };
        assert!(!installable);
    }

    #[test]
    fn installer_urls_and_checksums() {
        let (url, sha) = rustup_init_url("windows", "x86_64").unwrap();
        assert_eq!(
            url,
            "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
        );
        assert_eq!(sha, format!("{url}.sha256"));
        assert!(rustup_init_url("windows", "aarch64").is_none());
        let hex = "a".repeat(64);
        assert_eq!(
            parse_sha256(&format!("{hex}  rustup-init.exe\n"))
                .unwrap()
                .hex,
            hex
        );
        assert_eq!(parse_sha256(&format!("{hex} *x")).unwrap().hex, hex);
        assert!(parse_sha256("nope").is_none());
        assert!(Checksum::sha256(MINGW.sha256).is_some());
    }

    #[test]
    fn the_drive_root_is_where_cargo_starts() {
        let r = drive_root(Path::new("/data/native/x"));
        assert!(r.parent().is_none());
    }

    /// The real computer: whatever it has, detection answers without panicking.
    #[test]
    fn this_computer_answers() {
        let s = detect_with(&System, &root());
        match s {
            ToolchainStatus::Ready { toolchain } => {
                assert_eq!(toolchain.version, RUST_VERSION);
                assert!(toolchain.cargo().is_file());
            }
            ToolchainStatus::Missing { reason, .. } => assert!(!reason.is_empty()),
        }
    }
}
