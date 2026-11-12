//! Starting the game (horizon/docs/api/launch.md, launch-guide.md step 6-7):
//!
//! ```text
//! horizon-client --resources PREPARED --game-dir INSTANCE --home INSTANCE --width W --height H --session-stdin
//! ```
//!
//! Only options the build's `--launch-api` lists are passed. The session JSON goes to the game's
//! standard input, which is then closed: the token never touches the disk or the command line.
//! stdout and stderr stream as log lines; at exit, the code says how it ended (0 quit, 1 error,
//! 2 refused command line, 101 panic) and the last stderr line names the crash report.

use crate::error::{InstanceError, Result};
use crate::launch_api::ClientLaunchApi;
use rusticmc_launcher_core::{Secret, time};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;

/// Opening a world or a server at once.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "target", rename_all = "snake_case")]
pub enum QuickPlay {
    /// A folder name in `saves/`.
    Singleplayer(String),
    /// `HOST[:PORT]`.
    Multiplayer(String),
}

/// Window and start options.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
}

/// One game start.
#[derive(Clone, Debug)]
pub struct LaunchRequest {
    pub client: PathBuf,
    pub resources: PathBuf,
    pub game_dir: PathBuf,
    pub home: PathBuf,
    pub window: WindowOptions,
    pub quick_play: Option<QuickPlay>,
    /// The session JSON (online), `None`: the game starts without a session (offline).
    pub session: Option<Secret>,
    /// The player's extra options (instance settings), checked against `--launch-api`.
    pub extra_args: Vec<String>,
}

/// Options the launcher sets itself: never taken from the player's extra arguments.
const MANAGED: &[&str] = &[
    "--resources",
    "--game-dir",
    "--gameDir",
    "--home",
    "--session-file",
    "--session-stdin",
    "--launch-api",
];

/// The command line for `req`, refusing an option the build doesn't list.
pub fn launch_args(api: &ClientLaunchApi, req: &LaunchRequest) -> Result<Vec<OsString>> {
    let need = |name: &str| -> Result<()> {
        if api.accepts(name) {
            Ok(())
        } else {
            Err(InstanceError::OptionNotSupported(name.into()))
        }
    };
    let mut args: Vec<OsString> = Vec::new();
    let mut push = |name: &str, value: Option<OsString>| -> Result<()> {
        need(name)?;
        args.push(name.into());
        if let Some(v) = value {
            args.push(v);
        }
        Ok(())
    };
    push("--resources", Some(req.resources.clone().into()))?;
    push("--game-dir", Some(req.game_dir.clone().into()))?;
    push("--home", Some(req.home.clone().into()))?;
    if let Some(w) = req.window.width {
        push("--width", Some(w.to_string().into()))?;
    }
    if let Some(h) = req.window.height {
        push("--height", Some(h.to_string().into()))?;
    }
    if req.window.fullscreen {
        push("--fullscreen", None)?;
    }
    match &req.quick_play {
        Some(QuickPlay::Singleplayer(world)) => {
            if world.contains(['/', '\\']) || world == ".." {
                return Err(InstanceError::Process("a world is one folder name".into()));
            }
            push("--quickPlaySingleplayer", Some(world.into()))?;
        }
        Some(QuickPlay::Multiplayer(server)) => {
            push("--quickPlayMultiplayer", Some(server.into()))?;
        }
        None => {}
    }
    if req.session.is_some() {
        push("--session-stdin", None)?;
    }
    // Extra options: only listed ones, with their value when they take one.
    let mut extra = req.extra_args.iter();
    while let Some(word) = extra.next() {
        if MANAGED.contains(&word.as_str()) {
            return Err(InstanceError::OptionNotSupported(format!(
                "{word} (set by the launcher)"
            )));
        }
        let spec = api
            .options
            .iter()
            .find(|o| &o.name == word)
            .ok_or_else(|| InstanceError::OptionNotSupported(word.clone()))?;
        args.push(word.into());
        if spec.value.is_some() {
            let value = extra.next().ok_or_else(|| {
                InstanceError::OptionNotSupported(format!("{word} needs a value"))
            })?;
            args.push(value.into());
        }
    }
    Ok(args)
}

/// How the game ended.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExitOutcome {
    /// Exit 0.
    Quit,
    /// Exit 1 (an error) or 101 (a panic): a crash report was saved.
    Crashed {
        code: i32,
    },
    /// Exit 2: stderr names the refused option (a launcher bug).
    RefusedCommandLine {
        message: String,
    },
    /// Stopped by the launcher.
    Killed,
    Other {
        code: Option<i32>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// What a running game reports.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    Started {
        pid: Option<u32>,
    },
    Log {
        stream: Stream,
        line: String,
    },
    /// "Horizon: not signed in: ..." on stderr: online-mode servers will refuse the player.
    SessionRefused {
        message: String,
    },
    Exited {
        outcome: ExitOutcome,
        crash_report: Option<PathBuf>,
        played_seconds: u64,
    },
}

/// A started game.
pub struct RunningGame {
    pub pid: Option<u32>,
    pub started_at: u64,
    events: broadcast::Sender<GameEvent>,
    kill: tokio::sync::mpsc::Sender<()>,
    done: tokio::sync::watch::Receiver<Option<GameEvent>>,
}

impl RunningGame {
    pub fn subscribe(&self) -> broadcast::Receiver<GameEvent> {
        self.events.subscribe()
    }

    /// Kills the game.
    pub async fn kill(&self) {
        let _ = self.kill.send(()).await;
    }

    /// Waits for the end; returns the `Exited` event.
    pub async fn wait(&self) -> GameEvent {
        let mut done = self.done.clone();
        loop {
            if let Some(e) = done.borrow().clone() {
                return e;
            }
            if done.changed().await.is_err() {
                return done.borrow().clone().unwrap_or(GameEvent::Exited {
                    outcome: ExitOutcome::Other { code: None },
                    crash_report: None,
                    played_seconds: 0,
                });
            }
        }
    }
}

/// The newest file in `dir` modified at or after `since` (unix seconds).
fn newest_since(dir: &Path, since: u64) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            let secs = modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            (secs + 2 >= since).then_some((secs, e.path()))
        })
        .max_by_key(|(s, _)| *s)
        .map(|(_, p)| p)
}

/// Starts the game. `on_event` sees every event (also available through
/// [`RunningGame::subscribe`]).
pub async fn launch(
    api: &ClientLaunchApi,
    req: LaunchRequest,
    on_event: impl Fn(&GameEvent) + Send + Sync + 'static,
) -> Result<RunningGame> {
    let args = launch_args(api, &req)?;
    for dir in [&req.game_dir, &req.home] {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| InstanceError::io(dir, e))?;
    }
    let mut cmd = tokio::process::Command::new(&req.client);
    cmd.args(&args)
        .current_dir(&req.game_dir)
        .stdin(if req.session.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // The game must not inherit the launcher's webview renderer workarounds
        // (Horizon launcher, LAUNCHER_RENDER_ENV).
        .env_remove("WEBKIT_DISABLE_DMABUF_RENDERER")
        .env_remove("WEBKIT_DISABLE_COMPOSITING_MODE")
        .env_remove("LIBGL_ALWAYS_SOFTWARE")
        .env_remove("GALLIUM_DRIVER");
    // Its output is piped to the launcher's log: no empty console window next to the game.
    crate::process::no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| InstanceError::io(&req.client, e))?;
    let pid = child.id();
    if let Some(session) = &req.session {
        let mut stdin = child.stdin.take().expect("piped");
        let write = async {
            stdin.write_all(session.expose().as_bytes()).await?;
            stdin.flush().await?;
            stdin.shutdown().await
        };
        if let Err(e) = write.await {
            tracing::warn!(error = %e, "could not hand the session to the game");
        }
        drop(stdin); // closed: the game reads to the end
    }
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let started_at = time::now();
    let (events, _) = broadcast::channel(1024);
    let (done_tx, done_rx) = tokio::sync::watch::channel(None);
    let (kill_tx, mut kill_rx) = tokio::sync::mpsc::channel::<()>(1);
    let on_event = Arc::new(on_event);
    let emit = {
        let events = events.clone();
        let on_event = on_event.clone();
        move |e: GameEvent| {
            on_event(&e);
            let _ = events.send(e);
        }
    };
    emit(GameEvent::Started { pid });
    let crash_prefix = api
        .crash_report_line
        .clone()
        .unwrap_or_else(|| "Horizon: crash report saved to ".into());
    let home = req.home.clone();
    let mut child = child;
    tokio::spawn(async move {
        let mut out = BufReader::new(stdout).lines();
        let mut err = BufReader::new(stderr).lines();
        let mut last_err = String::new();
        let (mut out_done, mut err_done) = (false, false);
        let mut killed = false;
        let mut kill_open = true;
        while !(out_done && err_done) {
            tokio::select! {
                k = kill_rx.recv(), if kill_open => match k {
                    Some(()) => {
                        killed = true;
                        let _ = child.start_kill();
                    }
                    None => kill_open = false,
                },
                l = out.next_line(), if !out_done => match l {
                    Ok(Some(line)) => emit(GameEvent::Log { stream: Stream::Stdout, line }),
                    _ => out_done = true,
                },
                l = err.next_line(), if !err_done => match l {
                    Ok(Some(line)) => {
                        if let Some(msg) = line.strip_prefix("Horizon: not signed in: ") {
                            emit(GameEvent::SessionRefused { message: msg.into() });
                        }
                        if !line.trim().is_empty() {
                            last_err = line.clone();
                        }
                        emit(GameEvent::Log { stream: Stream::Stderr, line });
                    }
                    _ => err_done = true,
                },
            }
        }
        let status = loop {
            tokio::select! {
                s = child.wait() => break s.ok(),
                k = kill_rx.recv(), if kill_open => match k {
                    Some(()) => {
                        killed = true;
                        let _ = child.start_kill();
                    }
                    None => kill_open = false,
                },
            }
        };
        let code = status.and_then(|s| s.code());
        let outcome = match code {
            _ if killed => ExitOutcome::Killed,
            Some(0) => ExitOutcome::Quit,
            Some(c @ (1 | 101)) => ExitOutcome::Crashed { code: c },
            Some(2) => ExitOutcome::RefusedCommandLine {
                message: last_err.clone(),
            },
            other => ExitOutcome::Other { code: other },
        };
        let crash_report = match outcome {
            ExitOutcome::Crashed { .. } | ExitOutcome::Other { .. } => last_err
                .strip_prefix(crash_prefix.as_str())
                .map(|p| PathBuf::from(p.trim()))
                .or_else(|| newest_since(&home.join("crash-reports"), started_at)),
            _ => None,
        };
        let exited = GameEvent::Exited {
            outcome,
            crash_report,
            played_seconds: time::now().saturating_sub(started_at),
        };
        emit(exited.clone());
        let _ = done_tx.send(Some(exited));
    });
    Ok(RunningGame {
        pid,
        started_at,
        events,
        kill: kill_tx,
        done: done_rx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch_api::OptionSpec;

    fn api(names: &[&str]) -> ClientLaunchApi {
        ClientLaunchApi {
            launch_api: "0.1.0".into(),
            horizon: None,
            minecraft: None,
            options: names
                .iter()
                .map(|n| OptionSpec {
                    name: (*n).into(),
                    value: None,
                })
                .collect(),
            content: serde_json::Value::Null,
            crash_report_line: None,
            exit_codes: vec![],
        }
    }

    fn req(session: bool) -> LaunchRequest {
        LaunchRequest {
            client: "c".into(),
            resources: "r".into(),
            game_dir: "g".into(),
            home: "h".into(),
            window: WindowOptions {
                width: Some(1280),
                height: Some(720),
                fullscreen: false,
            },
            quick_play: Some(QuickPlay::Multiplayer("play.example.com".into())),
            session: session.then(|| Secret::new("{\"access_token\":\"t\"}")),
            extra_args: Vec::new(),
        }
    }

    #[test]
    fn the_token_is_never_an_argument() {
        let all = api(&[
            "--resources",
            "--game-dir",
            "--home",
            "--width",
            "--height",
            "--quickPlayMultiplayer",
            "--session-stdin",
        ]);
        let args: Vec<String> = launch_args(&all, &req(true))
            .unwrap()
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "--resources",
                "r",
                "--game-dir",
                "g",
                "--home",
                "h",
                "--width",
                "1280",
                "--height",
                "720",
                "--quickPlayMultiplayer",
                "play.example.com",
                "--session-stdin"
            ]
        );
        assert!(!args.iter().any(|a| a.contains("access_token")));
        // Offline: no session option at all.
        let offline: Vec<String> = launch_args(&all, &req(false))
            .unwrap()
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(!offline.contains(&"--session-stdin".to_string()));
    }

    #[test]
    fn extra_arguments_are_checked() {
        let mut all = api(&[
            "--resources",
            "--game-dir",
            "--home",
            "--width",
            "--height",
            "--quickPlayMultiplayer",
            "--session-stdin",
            "--player-dir",
        ]);
        all.options
            .iter_mut()
            .find(|o| o.name == "--player-dir")
            .unwrap()
            .value = Some("DIR".into());
        let mut r = req(false);
        r.extra_args = vec!["--player-dir".into(), "p2".into()];
        let args = launch_args(&all, &r).unwrap();
        assert!(args.ends_with(&["--player-dir".into(), "p2".into()]));
        r.extra_args = vec!["--accessToken".into(), "x".into()];
        assert!(launch_args(&all, &r).is_err());
        r.extra_args = vec!["--session-stdin".into()];
        assert!(launch_args(&all, &r).is_err());
        r.extra_args = vec!["--player-dir".into()];
        assert!(launch_args(&all, &r).is_err());
    }

    #[test]
    fn unlisted_options_are_refused_before_starting() {
        let old = api(&[
            "--resources",
            "--game-dir",
            "--home",
            "--width",
            "--height",
            "--session-stdin",
        ]);
        assert!(matches!(
            launch_args(&old, &req(true)),
            Err(InstanceError::OptionNotSupported(o)) if o == "--quickPlayMultiplayer"
        ));
    }
}
