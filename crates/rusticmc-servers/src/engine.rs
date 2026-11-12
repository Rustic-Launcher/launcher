//! The `rustic-server` program a Rustic Server runs (ADR 0152).
//!
//! `horizon-server` runs whatever `engine.path` names; for a Rustic Server that is `rustic-server`,
//! Horizon's game linked with Rustic's trait mod. It reads the build's prepared resources, so it
//! must come from the same Horizon commit as the build. Looked up in this order:
//!
//! 1. the server's own engine (`profile.engine`: an uploaded or downloaded one);
//! 2. the launcher's manual override (`[rustic_server] executable` in `launcher.toml`, set in
//!    Settings > Game and instances > Rustic Server);
//! 3. the server's build folder (`<builds>/<build>/rustic-server[.exe]`: a Rustic edition bundle
//!    ships it);
//! 4. beside the launcher's own program;
//! 5. the PATH.
//!
//! The first two are choices someone made: when that file is gone the lookup fails (and says so)
//! rather than silently running another engine. The last three are automatic.

use crate::error::{Result, ServerError};
use rusticmc_instances::builds::{RUSTIC_SERVER, exe};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where the `rustic-server` in use was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineOrigin {
    /// The server's own engine (`profile.engine`).
    Server,
    /// The launcher's manual override (`rustic_server.executable`).
    Configured,
    /// The server's Horizon build folder.
    Build,
    /// Beside the launcher's own program.
    BesideLauncher,
    /// A folder of the PATH.
    Path,
}

/// A `rustic-server` program and where it was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RusticServerProgram {
    pub path: PathBuf,
    pub origin: EngineOrigin,
}

/// The places to look in, besides the automatic ones (beside the launcher, the PATH).
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineSearch<'a> {
    /// The server's own engine.
    pub server: Option<&'a Path>,
    /// The launcher's manual override.
    pub configured: Option<&'a Path>,
    /// The server's build folder.
    pub build_dir: Option<&'a Path>,
}

/// The `rustic-server` program to run, in the order of the module's documentation.
pub fn find_rustic_server(search: EngineSearch<'_>) -> Option<RusticServerProgram> {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf));
    find_in(
        search,
        beside.as_deref(),
        std::env::var_os("PATH").as_deref(),
    )
}

fn find_in(
    search: EngineSearch<'_>,
    beside_dir: Option<&Path>,
    path_var: Option<&OsStr>,
) -> Option<RusticServerProgram> {
    let found = |path: PathBuf, origin| {
        path.is_file()
            .then_some(RusticServerProgram { path, origin })
    };
    // A choice someone made is used as is, never replaced by another engine behind their back.
    let chosen = [
        (search.server, EngineOrigin::Server),
        (search.configured, EngineOrigin::Configured),
    ];
    for (path, origin) in chosen {
        if let Some(p) = path.filter(|p| !p.as_os_str().is_empty()) {
            return found(p.to_path_buf(), origin);
        }
    }
    let name = exe(RUSTIC_SERVER);
    search
        .build_dir
        .and_then(|d| found(d.join(&name), EngineOrigin::Build))
        .or_else(|| beside_dir.and_then(|d| found(d.join(&name), EngineOrigin::BesideLauncher)))
        .or_else(|| {
            std::env::split_paths(path_var?).find_map(|d| found(d.join(&name), EngineOrigin::Path))
        })
}

/// How long `rustic-server --rustic-version` may take (a first run of a 120 MB program can be
/// slowed down by an antivirus scan).
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `rustic-server --rustic-version` (the program's own flag: it prints one line such as
/// `Rustic 0.1.0 (rustic:api 0.1) on Horizon (Minecraft 26.3)` and exits) and returns that line.
/// Anything else (not a program, another program, an error, no answer) is
/// [`ServerError::NotRusticServer`].
pub async fn rustic_server_version(path: &Path) -> Result<String> {
    let not = |why: String| ServerError::NotRusticServer {
        path: path.to_path_buf(),
        why,
    };
    if !path.is_file() {
        return Err(not("no such file".into()));
    }
    let mut cmd = tokio::process::Command::new(path);
    cmd.arg("--rustic-version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    crate::supervisor::no_window(&mut cmd);
    let out = tokio::time::timeout(PROBE_TIMEOUT, cmd.output())
        .await
        .map_err(|_| not("it did not answer --rustic-version".into()))?
        .map_err(|e| not(format!("it does not run: {e}")))?;
    let line = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    if out.status.success() && line.starts_with("Rustic ") {
        Ok(line)
    } else {
        Err(not(format!(
            "--rustic-version answered {:?} ({})",
            line, out.status
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn touch(p: &Path) -> PathBuf {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"").unwrap();
        p.to_path_buf()
    }

    struct Dirs {
        _t: tempfile::TempDir,
        server: PathBuf,
        configured: PathBuf,
        build: PathBuf,
        beside: PathBuf,
        on_path: PathBuf,
    }

    fn dirs() -> Dirs {
        let t = tempfile::tempdir().unwrap();
        let r = t.path().to_path_buf();
        Dirs {
            server: r.join("engines/mine"),
            configured: r.join("chosen/rustic-server-custom"),
            build: r.join("builds/b1"),
            beside: r.join("launcher"),
            on_path: r.join("bin"),
            _t: t,
        }
    }

    fn path_var(d: &Dirs) -> OsString {
        std::env::join_paths([d.beside.join("nothing-here"), d.on_path.clone()]).unwrap()
    }

    fn search<'a>(d: &'a Dirs, server: bool, configured: bool) -> EngineSearch<'a> {
        EngineSearch {
            server: server.then_some(d.server.as_path()),
            configured: configured.then_some(d.configured.as_path()),
            build_dir: Some(&d.build),
        }
    }

    #[test]
    fn follows_the_documented_order() {
        let d = dirs();
        let name = exe(RUSTIC_SERVER);
        let pv = path_var(&d);
        let look = |s| find_in(s, Some(&d.beside), Some(&pv)).map(|p| (p.origin, p.path));

        // Nothing anywhere.
        assert_eq!(look(search(&d, false, false)), None);
        // The PATH, then beside the launcher, then the build, each above the previous one.
        let on_path = touch(&d.on_path.join(&name));
        assert_eq!(
            look(search(&d, false, false)),
            Some((EngineOrigin::Path, on_path))
        );
        let beside = touch(&d.beside.join(&name));
        assert_eq!(
            look(search(&d, false, false)),
            Some((EngineOrigin::BesideLauncher, beside))
        );
        let build = touch(&d.build.join(&name));
        assert_eq!(
            look(search(&d, false, false)),
            Some((EngineOrigin::Build, build))
        );
        // The manual override above the build, the server's own engine above everything.
        let configured = touch(&d.configured);
        assert_eq!(
            look(search(&d, false, true)),
            Some((EngineOrigin::Configured, configured))
        );
        let server = touch(&d.server);
        assert_eq!(
            look(search(&d, true, true)),
            Some((EngineOrigin::Server, server))
        );
    }

    #[test]
    fn a_chosen_program_that_is_gone_is_not_replaced() {
        let d = dirs();
        let pv = path_var(&d);
        touch(&d.build.join(exe(RUSTIC_SERVER)));
        // The override names a file that no longer exists: no silent fallback to the build's.
        assert_eq!(
            find_in(search(&d, false, true), Some(&d.beside), Some(&pv)),
            None
        );
        // Same for a server's own engine, even with a valid override.
        touch(&d.configured);
        assert_eq!(
            find_in(search(&d, true, true), Some(&d.beside), Some(&pv)),
            None
        );
    }

    #[test]
    fn an_empty_override_means_automatic() {
        let d = dirs();
        let build = touch(&d.build.join(exe(RUSTIC_SERVER)));
        let s = EngineSearch {
            server: Some(Path::new("")),
            configured: Some(Path::new("")),
            build_dir: Some(&d.build),
        };
        assert_eq!(
            find_in(s, None, None),
            Some(RusticServerProgram {
                path: build,
                origin: EngineOrigin::Build
            })
        );
    }

    #[test]
    fn a_folder_is_not_a_program() {
        let d = dirs();
        std::fs::create_dir_all(d.build.join(exe(RUSTIC_SERVER))).unwrap();
        assert_eq!(find_in(search(&d, false, false), None, None), None);
    }

    #[tokio::test]
    async fn the_probe_refuses_what_is_not_rustic_server() {
        let d = dirs();
        let missing = rustic_server_version(&d.configured).await.unwrap_err();
        assert_eq!(missing.code(), "server.not_rustic_server");
        // An empty file is no program at all.
        let empty = touch(&d.configured);
        let err = rustic_server_version(&empty).await.unwrap_err();
        assert!(matches!(err, ServerError::NotRusticServer { .. }), "{err}");
    }
}
