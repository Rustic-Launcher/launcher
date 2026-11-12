//! Discord Rich Presence ("Playing Rustic"): what the player is doing, in their Discord profile.
//!
//! The launcher is the only process that talks to Discord, over Discord's local IPC (a named pipe
//! `\\?\pipe\discord-ipc-N` on Windows, a Unix socket `discord-ipc-N` elsewhere; frames of an
//! opcode, a length and JSON). The game tells the launcher where the player is through a line on
//! its standard output (`rustic-presence {...}`, written by the Rustic edition's companion), with
//! the "Connecting to <host>, <port>" log line as a fallback for builds without the companion.
//! Discord not running is normal: the launcher tries again now and then, silently.
//! Off, or the server address left out, in Settings > Privacy.

use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::watch;

/// The "Rustic" application in Discord's Developer Portal (public, not a secret).
pub const APP_ID: &str = "1570549155250573444";
/// The art asset uploaded to that application (Rich Presence > Art Assets).
const LARGE_IMAGE: &str = "rustic";
const WEBSITE: &str = "https://rustic-launcher.com";

/// Where the player is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Launcher,
    /// The game's title screen (or any menu outside a world).
    Menu,
    Singleplayer,
    /// On a server; its address as the player typed it.
    Server(String),
}

/// What is shown, before the player's settings apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presence {
    pub place: Place,
    /// "Horizon 26.3" while the game runs.
    pub game: Option<String>,
    /// When this activity began (unix seconds): the launcher's start, then the game's.
    pub since: u64,
}

/// The player's choices (Settings > Privacy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub enabled: bool,
    pub show_server: bool,
}

/// The `activity` object of `SET_ACTIVITY`.
pub fn activity(p: &Presence, o: Options) -> Value {
    let game = p.game.clone().unwrap_or_else(|| "Horizon".into());
    let (details, state) = match &p.place {
        Place::Launcher => ("In the launcher".to_owned(), None),
        Place::Menu => (format!("Playing {game}"), Some("Main menu".to_owned())),
        Place::Singleplayer => (format!("Playing {game}"), Some("Singleplayer".to_owned())),
        Place::Server(host) if o.show_server && is_shareable_host(host) => {
            (format!("Playing {game}"), Some(format!("On {host}")))
        }
        Place::Server(_) => (format!("Playing {game}"), Some("Multiplayer".to_owned())),
    };
    let mut a = json!({
        "details": details,
        "timestamps": { "start": p.since },
        "assets": { "large_image": LARGE_IMAGE, "large_text": "Rustic" },
        "buttons": [ { "label": "Get Rustic", "url": WEBSITE } ],
    });
    if let Some(state) = state {
        a["state"] = state.into();
    }
    a
}

/// A server address worth showing: a public host name, never a private or loopback address
/// (a LAN game, a server on this PC) nor something that isn't a host name at all.
fn is_shareable_host(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if h.is_empty() || h.len() > 253 || h == "localhost" || h.ends_with(".local") {
        return false;
    }
    if let Ok(ip) = h.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => {
                !(v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified())
            }
            std::net::IpAddr::V6(v6) => !(v6.is_loopback() || v6.is_unspecified()),
        };
    }
    h.contains('.')
        && h.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// The place a line of the game's output reports, if it reports one.
pub fn place_from_game_line(line: &str) -> Option<Place> {
    #[derive(Deserialize)]
    struct Report {
        place: String,
        #[serde(default)]
        server: Option<String>,
    }
    let line = line.trim();
    if let Some(json) = line.split_once("rustic-presence ").map(|(_, j)| j) {
        let r: Report = serde_json::from_str(json).ok()?;
        return match r.place.as_str() {
            "menu" => Some(Place::Menu),
            "singleplayer" => Some(Place::Singleplayer),
            "server" => Some(Place::Server(r.server.filter(|s| !s.is_empty())?)),
            _ => None,
        };
    }
    // Fallback (a build without the companion): "[Server Connector #1/INFO]: Connecting to <host>, <port>".
    let rest = line.split_once("Connecting to ").map(|(_, r)| r)?;
    let host = rest.split(',').next()?.trim();
    (!host.is_empty() && !host.contains(' ')).then(|| Place::Server(host.to_owned()))
}

/// The presence service: set what to show; a background task keeps Discord in step.
pub struct Discord {
    tx: watch::Sender<(Option<Presence>, Options)>,
}

impl Discord {
    pub fn start(options: Options) -> Self {
        let (tx, rx) = watch::channel((None, options));
        tauri::async_runtime::spawn(run(rx));
        Self { tx }
    }

    pub fn set(&self, presence: Presence) {
        self.tx.send_modify(|(p, _)| *p = Some(presence));
    }

    /// The place changes, keeping the game and the start time (a new place of the same game).
    pub fn set_place(&self, place: Place) {
        self.tx.send_if_modified(|(p, _)| match p {
            Some(p) if p.place != place => {
                p.place = place;
                true
            }
            _ => false,
        });
    }

    pub fn configure(&self, options: Options) {
        self.tx.send_if_modified(|(_, o)| {
            let changed = *o != options;
            *o = options;
            changed
        });
    }
}

trait Pipe: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Pipe for T {}

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;
/// Discord refuses bigger frames; ours are a few hundred bytes.
const MAX_FRAME: u32 = 64 * 1024;

fn frame(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string().into_bytes();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

async fn read_frame(pipe: &mut dyn Pipe) -> std::io::Result<(u32, Value)> {
    let mut head = [0u8; 8];
    pipe.read_exact(&mut head).await?;
    let op = u32::from_le_bytes(head[..4].try_into().expect("4 bytes"));
    let len = u32::from_le_bytes(head[4..].try_into().expect("4 bytes"));
    if len > MAX_FRAME {
        return Err(std::io::Error::other("oversized Discord frame"));
    }
    let mut body = vec![0u8; len as usize];
    pipe.read_exact(&mut body).await?;
    Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

/// Sends `payload` and waits for Discord's answer (answering its pings meanwhile).
async fn exchange(pipe: &mut dyn Pipe, op: u32, payload: &Value) -> std::io::Result<Value> {
    pipe.write_all(&frame(op, payload)).await?;
    loop {
        let (op, body) = tokio::time::timeout(Duration::from_secs(5), read_frame(pipe))
            .await
            .map_err(|_| std::io::Error::other("Discord did not answer"))??;
        match op {
            OP_PING => pipe.write_all(&frame(OP_PONG, &body)).await?,
            OP_CLOSE => return Err(std::io::Error::other("Discord closed the connection")),
            _ => return Ok(body),
        }
    }
}

#[cfg(windows)]
async fn open(n: u8) -> std::io::Result<Box<dyn Pipe>> {
    let pipe = tokio::net::windows::named_pipe::ClientOptions::new()
        .open(format!(r"\\?\pipe\discord-ipc-{n}"))?;
    Ok(Box::new(pipe))
}

#[cfg(unix)]
async fn open(n: u8) -> std::io::Result<Box<dyn Pipe>> {
    let mut dirs: Vec<std::path::PathBuf> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(std::env::var_os)
        .map(Into::into)
        .collect();
    dirs.push("/tmp".into());
    let mut last = std::io::Error::from(std::io::ErrorKind::NotFound);
    for dir in dirs {
        // Discord itself, then its Flatpak and Snap packages.
        for sub in ["", "app/com.discordapp.Discord", "snap.discord"] {
            match tokio::net::UnixStream::connect(dir.join(sub).join(format!("discord-ipc-{n}")))
                .await
            {
                Ok(s) => return Ok(Box::new(s)),
                Err(e) => last = e,
            }
        }
    }
    Err(last)
}

/// A connection to the running Discord client, handshake done.
async fn connect() -> Option<Box<dyn Pipe>> {
    for n in 0..10 {
        let Ok(mut pipe) = open(n).await else {
            continue;
        };
        let hello = json!({ "v": 1, "client_id": APP_ID });
        if exchange(pipe.as_mut(), OP_HANDSHAKE, &hello).await.is_ok() {
            return Some(pipe);
        }
    }
    None
}

async fn send_activity(pipe: &mut dyn Pipe, activity: Option<Value>) -> std::io::Result<()> {
    let cmd = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": activity },
        "nonce": uuid::Uuid::new_v4().to_string(),
    });
    let answer = exchange(pipe, OP_FRAME, &cmd).await?;
    if answer["evt"] == "ERROR" {
        tracing::debug!(answer = %answer, "Discord refused the activity");
    }
    Ok(())
}

/// Keeps Discord showing the latest presence: connects when there is something to show,
/// reconnects after Discord restarts, clears the activity when the player turns it off.
async fn run(mut rx: watch::Receiver<(Option<Presence>, Options)>) {
    let mut pipe: Option<Box<dyn Pipe>> = None;
    loop {
        let (presence, options) = rx.borrow_and_update().clone();
        let wanted = presence
            .filter(|_| options.enabled)
            .map(|p| activity(&p, options));
        let mut done = false;
        if wanted.is_some() && pipe.is_none() {
            pipe = connect().await;
        }
        if let Some(p) = pipe.as_mut() {
            match send_activity(p.as_mut(), wanted.clone()).await {
                Ok(()) => done = true,
                Err(e) => {
                    tracing::debug!(error = %e, "Discord presence lost");
                    pipe = None;
                }
            }
            if wanted.is_none() {
                // Turned off: leave Discord alone until it is turned on again.
                pipe = None;
            }
        } else if wanted.is_none() {
            done = true;
        }
        // Wait for a change; when Discord wasn't reachable, try again every 20 s.
        let wait = if done {
            None
        } else {
            Some(Duration::from_secs(20))
        };
        let changed = match wait {
            None => rx.changed().await.is_ok(),
            Some(d) => match tokio::time::timeout(d, rx.changed()).await {
                Ok(r) => r.is_ok(),
                Err(_) => true,
            },
        };
        if !changed {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: Options = Options {
        enabled: true,
        show_server: true,
    };

    fn at(place: Place) -> Presence {
        Presence {
            place,
            game: Some("Horizon 26.3".into()),
            since: 100,
        }
    }

    #[test]
    fn the_activity_follows_the_place() {
        let a = activity(
            &Presence {
                place: Place::Launcher,
                game: None,
                since: 7,
            },
            ON,
        );
        assert_eq!(a["details"], "In the launcher");
        assert!(a.get("state").is_none());
        assert_eq!(a["timestamps"]["start"], 7);
        assert_eq!(a["assets"]["large_image"], "rustic");
        assert_eq!(activity(&at(Place::Menu), ON)["state"], "Main menu");
        assert_eq!(
            activity(&at(Place::Menu), ON)["details"],
            "Playing Horizon 26.3"
        );
        assert_eq!(
            activity(&at(Place::Singleplayer), ON)["state"],
            "Singleplayer"
        );
        let server = at(Place::Server("play.example.com".into()));
        assert_eq!(activity(&server, ON)["state"], "On play.example.com");
    }

    #[test]
    fn the_server_is_hidden_when_asked_or_private() {
        let off = Options {
            enabled: true,
            show_server: false,
        };
        let server = at(Place::Server("play.example.com".into()));
        assert_eq!(activity(&server, off)["state"], "Multiplayer");
        for private in [
            "192.168.1.20",
            "127.0.0.1",
            "localhost",
            "10.0.0.2",
            "box.local",
            "my pc",
        ] {
            assert_eq!(
                activity(&at(Place::Server(private.into())), ON)["state"],
                "Multiplayer",
                "{private}"
            );
        }
    }

    #[test]
    fn the_game_reports_where_it_is() {
        assert_eq!(
            place_from_game_line(r#"rustic-presence {"place":"menu"}"#),
            Some(Place::Menu)
        );
        assert_eq!(
            place_from_game_line(r#"[12:00] rustic-presence {"place":"singleplayer"}"#),
            Some(Place::Singleplayer)
        );
        assert_eq!(
            place_from_game_line(r#"rustic-presence {"place":"server","server":"mc.example.org"}"#),
            Some(Place::Server("mc.example.org".into()))
        );
        assert_eq!(
            place_from_game_line(r#"rustic-presence {"place":"server"}"#),
            None
        );
        assert_eq!(place_from_game_line("rustic-presence nonsense"), None);
        assert_eq!(
            place_from_game_line(
                "[18:50:11] [Server Connector #1/INFO]: Connecting to play.playhorizon.dev, 25565"
            ),
            Some(Place::Server("play.playhorizon.dev".into()))
        );
        assert_eq!(place_from_game_line("[CHAT] Bob joined the game"), None);
    }

    #[test]
    fn frames_are_opcode_length_json() {
        let f = frame(OP_HANDSHAKE, &json!({ "v": 1 }));
        assert_eq!(&f[..4], &0u32.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes(f[4..8].try_into().unwrap()) as usize,
            f.len() - 8
        );
        assert_eq!(&f[8..], br#"{"v":1}"#);
    }

    #[tokio::test]
    async fn an_exchange_answers_pings_and_reads_the_reply() {
        let (mut ours, mut discord) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            let (op, body) = read_frame(&mut discord).await.unwrap();
            assert_eq!(op, OP_FRAME);
            assert_eq!(body["cmd"], "SET_ACTIVITY");
            discord
                .write_all(&frame(OP_PING, &json!({ "n": 1 })))
                .await
                .unwrap();
            let (pong, _) = read_frame(&mut discord).await.unwrap();
            assert_eq!(pong, OP_PONG);
            discord
                .write_all(&frame(
                    OP_FRAME,
                    &json!({ "evt": null, "cmd": "SET_ACTIVITY" }),
                ))
                .await
                .unwrap();
        });
        send_activity(&mut ours, Some(json!({ "details": "x" })))
            .await
            .unwrap();
        server.await.unwrap();
    }

    /// Against the Discord client running on this computer (manual check):
    /// `cargo test -p rustic-launcher real_discord -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "needs the Discord client running"]
    async fn real_discord_accepts_the_presence() {
        let mut pipe = connect()
            .await
            .expect("Discord is running and answers the handshake");
        let a = activity(&at(Place::Server("play.example.com".into())), ON);
        let cmd = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": a },
            "nonce": uuid::Uuid::new_v4().to_string(),
        });
        let answer = exchange(pipe.as_mut(), OP_FRAME, &cmd).await.unwrap();
        println!("Discord answered: {answer}");
        assert_ne!(answer["evt"], "ERROR", "{answer}");
        tokio::time::sleep(Duration::from_secs(20)).await;
        send_activity(pipe.as_mut(), None).await.unwrap();
    }
}
