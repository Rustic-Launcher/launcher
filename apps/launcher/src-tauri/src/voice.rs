//! Voice (ADR 0176): the launcher is the voice client (LiveKit in the main window's webview).
//! This module is its native half:
//!
//! - the microphone permission of the webview (WebView2's `PermissionRequested`, WebKitGTK's
//!   `permission-request`): granted to the main window only, never the camera or the screen;
//!   on Linux WebKitGTK's media stream and WebRTC switches, which are off by default;
//! - the companion's `rustic-voice {...}` lines on the game's output (push-to-talk key, the
//!   player's position while proximity voice is on), forwarded to the webview as `voice-game`
//!   events and kept out of the game log;
//! - `voice_overlay_set`: what the in-game overlay shows (who is in the room, who speaks), written
//!   to `<instance>/game/rustic/voice.json` of the running instances, which the companion reads.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::webview::{PermissionKind, PermissionResponse};

use crate::state::{Res, State};

/// The window that runs the voice engine.
pub const ENGINE_WINDOW: &str = "main";

/// The webview's permission requests: the microphone for the engine's window only.
pub fn permission(window_label: &str, kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Microphone if window_label == ENGINE_WINDOW => PermissionResponse::Allow,
        PermissionKind::Microphone | PermissionKind::Camera | PermissionKind::DisplayCapture => {
            PermissionResponse::Deny
        }
        _ => PermissionResponse::Default,
    }
}

/// WebKitGTK ships with media capture and WebRTC off: turn them on for the engine's window.
#[cfg(target_os = "linux")]
pub fn enable_media(window: &tauri::WebviewWindow) {
    let done = window.with_webview(|w| {
        use webkit2gtk::{SettingsExt, WebViewExt};
        if let Some(s) = w.inner().settings() {
            s.set_enable_media_stream(true);
            s.set_enable_webrtc(true);
        }
    });
    if let Err(e) = done {
        tracing::warn!(error = %e, "voice: could not enable WebRTC in the webview");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn enable_media(_window: &tauri::WebviewWindow) {}

/// A player's position as the companion reports it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GamePosition {
    pub server: String,
    pub dim: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub yaw: f64,
}

impl GamePosition {
    fn valid(&self) -> bool {
        let label = |s: &str, max: usize| {
            !s.is_empty()
                && s.len() <= max
                && s.bytes().all(|b| {
                    b.is_ascii_alphanumeric()
                        || matches!(b, b'.' | b'_' | b':' | b'-' | b'[' | b']' | b'/')
                })
        };
        label(&self.server, 255)
            && label(&self.dim, 128)
            && [self.x, self.y, self.z, self.yaw]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 30_000_000.0)
    }
}

/// What a `rustic-voice` line says.
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceLine {
    /// The push-to-talk key went down or up in game.
    Ptt(bool),
    /// Where the player is (None: out of a world).
    Position(Option<GamePosition>),
}

/// The voice report on a line of the game's output, if it is one.
pub fn parse_game_line(line: &str) -> Option<VoiceLine> {
    let json = line.trim().split_once("rustic-voice ").map(|(_, j)| j)?;
    if json.len() > 2048 {
        return None;
    }
    let serde_json::Value::Object(map) = serde_json::from_str(json).ok()? else {
        return None;
    };
    if let Some(down) = map.get("ptt").and_then(serde_json::Value::as_bool) {
        return Some(VoiceLine::Ptt(down));
    }
    match map.get("pos")? {
        serde_json::Value::Null => Some(VoiceLine::Position(None)),
        v => {
            let p: GamePosition = serde_json::from_value(v.clone()).ok()?;
            p.valid().then_some(VoiceLine::Position(Some(p)))
        }
    }
}

/// The `voice-game` event's payload.
#[derive(Clone, Debug, Default, Serialize)]
pub struct VoiceGameEvent {
    pub instance_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ptt: Option<bool>,
    /// Present (maybe null) when the line carried a position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<GamePosition>>,
    /// Where the game is (from the presence line): `menu`, `singleplayer`, `server`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub exited: bool,
}

impl VoiceGameEvent {
    pub fn of_line(instance_id: &str, line: VoiceLine) -> Self {
        let mut e = Self {
            instance_id: instance_id.to_owned(),
            ..Self::default()
        };
        match line {
            VoiceLine::Ptt(d) => e.ptt = Some(d),
            VoiceLine::Position(p) => e.position = Some(p),
        }
        e
    }

    pub fn of_place(instance_id: &str, place: &crate::discord::Place) -> Self {
        use crate::discord::Place;
        let (kind, server) = match place {
            Place::Launcher | Place::Menu => ("menu", None),
            Place::Singleplayer => ("singleplayer", None),
            Place::Server(s) => ("server", Some(s.to_ascii_lowercase())),
        };
        Self {
            instance_id: instance_id.to_owned(),
            place: Some(kind.to_owned()),
            server,
            ..Self::default()
        }
    }

    pub fn exited(instance_id: &str) -> Self {
        Self {
            instance_id: instance_id.to_owned(),
            exited: true,
            ..Self::default()
        }
    }
}

/// One person in the overlay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayMember {
    pub name: String,
    pub speaking: bool,
    #[serde(default)]
    pub muted: bool,
}

/// What the companion's overlay shows, and what it should report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceOverlay {
    /// In a voice room now.
    pub active: bool,
    /// Report the player's position (`rustic-voice {"pos":…}` lines): proximity is on.
    #[serde(default)]
    pub positions: bool,
    /// The push-to-talk key's game name (`key.keyboard.v`), when push-to-talk is the mode.
    #[serde(default)]
    pub ptt_key: Option<String>,
    /// The room's label (`Party`, a server's name).
    #[serde(default)]
    pub room: String,
    /// "You" in the launcher's language.
    #[serde(default)]
    pub you: String,
    #[serde(default)]
    pub self_speaking: bool,
    #[serde(default)]
    pub self_muted: bool,
    #[serde(default)]
    pub deafened: bool,
    #[serde(default)]
    pub members: Vec<OverlayMember>,
    /// Draw the overlay (the player may keep voice on and the overlay off).
    #[serde(default = "yes")]
    pub show: bool,
}

fn yes() -> bool {
    true
}

impl VoiceOverlay {
    /// Bounds what reaches the game: at most 32 members, short names, a known key form.
    pub fn bounded(mut self) -> Self {
        let clip = |s: &str, n: usize| s.chars().filter(|c| !c.is_control()).take(n).collect();
        self.room = clip(&self.room, 48);
        self.you = clip(&self.you, 24);
        self.members.truncate(32);
        for m in &mut self.members {
            m.name = clip(&m.name, 32);
        }
        self.ptt_key = self.ptt_key.filter(|k| {
            k.len() <= 40
                && (k.starts_with("key.keyboard.") || k.starts_with("key.mouse."))
                && k.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.')
        });
        self
    }
}

/// `<game dir>/rustic/voice.json`.
pub fn overlay_path(game_dir: &Path) -> PathBuf {
    game_dir.join("rustic").join("voice.json")
}

/// Writes the overlay file atomically (a temporary file, then a rename).
pub fn write_overlay(game_dir: &Path, overlay: &VoiceOverlay) -> std::io::Result<()> {
    let path = overlay_path(game_dir);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec(overlay).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(&tmp, &path)
}

#[derive(Deserialize)]
pub struct VoiceOverlayArgs {
    pub overlay: VoiceOverlay,
}

/// `voice_overlay_set`: hands the in-game overlay its state (every running instance).
#[tauri::command]
pub fn voice_overlay_set(state: State<'_>, args: VoiceOverlayArgs) -> Res<()> {
    let overlay = args.overlay.bounded();
    for id in state.instances.running() {
        let dir = state.instances.store.game_dir(&id);
        if let Err(e) = write_overlay(&dir, &overlay) {
            tracing::debug!(instance = %id, error = %e, "voice: overlay file not written");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_parsed_and_checked() {
        assert_eq!(
            parse_game_line(r#"[12:00] rustic-voice {"ptt":true}"#),
            Some(VoiceLine::Ptt(true))
        );
        assert_eq!(
            parse_game_line(r#"rustic-voice {"pos":null}"#),
            Some(VoiceLine::Position(None))
        );
        let p = parse_game_line(
            r#"rustic-voice {"pos":{"server":"play.example.org","dim":"minecraft:overworld","x":1.5,"y":64,"z":-3,"yaw":90}}"#,
        );
        assert_eq!(
            p,
            Some(VoiceLine::Position(Some(GamePosition {
                server: "play.example.org".into(),
                dim: "minecraft:overworld".into(),
                x: 1.5,
                y: 64.0,
                z: -3.0,
                yaw: 90.0
            })))
        );
        for bad in [
            r#"rustic-voice {"pos":{"server":"a b","dim":"d","x":0,"y":0,"z":0}}"#,
            r#"rustic-voice {"pos":{"server":"a","dim":"d","x":1e300,"y":0,"z":0}}"#,
            "rustic-voice nonsense",
            r#"rustic-presence {"place":"menu"}"#,
            "hello",
        ] {
            assert_eq!(parse_game_line(bad), None, "{bad}");
        }
        let long = format!(r#"rustic-voice {{"ptt":true,"x":"{}"}}"#, "a".repeat(3000));
        assert_eq!(parse_game_line(&long), None);
    }

    #[test]
    fn events_serialise_as_the_ipc_doc_says() {
        let e = VoiceGameEvent::of_line("i1", VoiceLine::Position(None));
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({ "instance_id": "i1", "position": null })
        );
        let e = VoiceGameEvent::of_place("i1", &crate::discord::Place::Server("Play.X.org".into()));
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({ "instance_id": "i1", "place": "server", "server": "play.x.org" })
        );
        assert_eq!(
            serde_json::to_value(VoiceGameEvent::exited("i1")).unwrap(),
            serde_json::json!({ "instance_id": "i1", "exited": true })
        );
    }

    #[test]
    fn the_overlay_is_bounded_and_written_atomically() {
        let o: VoiceOverlay = serde_json::from_value(serde_json::json!({
            "active": true,
            "ptt_key": "key.keyboard.v",
            "room": "Party",
            "members": (0..40).map(|i| serde_json::json!({ "name": format!("player\u{7}{i}{}", "x".repeat(40)), "speaking": i == 0 })).collect::<Vec<_>>(),
        }))
        .unwrap();
        let o = o.bounded();
        assert!(o.show);
        assert_eq!(o.members.len(), 32);
        assert!(
            o.members
                .iter()
                .all(|m| m.name.chars().count() <= 32 && !m.name.contains('\u{7}'))
        );
        assert_eq!(o.ptt_key.as_deref(), Some("key.keyboard.v"));
        let bad = VoiceOverlay {
            ptt_key: Some("../../etc".into()),
            ..o.clone()
        }
        .bounded();
        assert_eq!(bad.ptt_key, None);

        let dir = std::env::temp_dir().join(format!("rustic-voice-{}", std::process::id()));
        write_overlay(&dir, &o).unwrap();
        let back: VoiceOverlay =
            serde_json::from_slice(&std::fs::read(overlay_path(&dir)).unwrap()).unwrap();
        assert_eq!(back, o);
        assert!(!overlay_path(&dir).with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_the_engine_window_gets_the_microphone() {
        assert!(matches!(
            permission("main", PermissionKind::Microphone),
            PermissionResponse::Allow
        ));
        assert!(matches!(
            permission("social", PermissionKind::Microphone),
            PermissionResponse::Deny
        ));
        assert!(matches!(
            permission("main", PermissionKind::Camera),
            PermissionResponse::Deny
        ));
        assert!(matches!(
            permission("main", PermissionKind::Notifications),
            PermissionResponse::Default
        ));
    }
}
