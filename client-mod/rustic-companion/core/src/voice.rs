//! Voice in game (launcher ADR 0176). The launcher is the voice client; the companion is its
//! eyes and ears in the game:
//!
//! - it reads what to show from `rustic/voice.json` in the game folder, which the launcher
//!   rewrites (atomically) while it is in a voice room, at least every 2 s: who is in the room,
//!   who speaks, push-to-talk's key, whether to report positions. A file older than [`STALE`] is
//!   a launcher that is gone: nothing is shown;
//! - it tells the launcher, on its standard output like the presence line, when the
//!   push-to-talk key goes down or up (`rustic-voice {"ptt":true}`) and, only while the room uses
//!   proximity, where the player is (`rustic-voice {"pos":{...}}`), at most 5 times a second;
//! - it draws a small list of the room's members in a corner, the speaking ones lit.
//!
//! Nothing here sends audio or reaches the network.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::overlay::Draw;

/// The overlay file, relative to the game folder (the game's working directory).
pub const FILE: &str = "rustic/voice.json";
/// A file not rewritten for this long is ignored (the launcher writes every 2 s).
pub const STALE: Duration = Duration::from_secs(6);
/// The file is looked at this often.
pub const POLL: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Member {
    pub name: String,
    #[serde(default)]
    pub speaking: bool,
    #[serde(default)]
    pub muted: bool,
}

fn yes() -> bool {
    true
}

/// What the launcher asks the overlay to show (the launcher's `VoiceOverlay`).
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Overlay {
    pub active: bool,
    #[serde(default)]
    pub positions: bool,
    #[serde(default)]
    pub ptt_key: Option<String>,
    #[serde(default)]
    pub room: String,
    #[serde(default)]
    pub you: String,
    #[serde(default)]
    pub self_speaking: bool,
    #[serde(default)]
    pub self_muted: bool,
    #[serde(default)]
    pub deafened: bool,
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default = "yes")]
    pub show: bool,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            active: false,
            positions: false,
            ptt_key: None,
            room: String::new(),
            you: String::new(),
            self_speaking: false,
            self_muted: false,
            deafened: false,
            members: Vec::new(),
            show: true,
        }
    }
}

impl Overlay {
    /// From the file's text: at most 32 members, short labels, or nothing for a bad file.
    pub fn parse(text: &str) -> Option<Overlay> {
        if text.len() > 64 * 1024 {
            return None;
        }
        let mut o: Overlay = serde_json::from_str(text).ok()?;
        let clip = |s: &str, n: usize| -> String {
            s.chars().filter(|c| !c.is_control()).take(n).collect()
        };
        o.room = clip(&o.room, 48);
        o.you = clip(&o.you, 24);
        o.members.truncate(32);
        for m in &mut o.members {
            m.name = clip(&m.name, 32);
        }
        Some(o)
    }

    /// Whether the push-to-talk key is `key` (a vanilla key name).
    pub fn is_ptt(&self, key: &str) -> bool {
        self.active && self.ptt_key.as_deref() == Some(key)
    }
}

/// The overlay file, read again when it changes, dropped when stale.
#[derive(Default)]
pub struct OverlayFile {
    next: Option<Instant>,
    modified: Option<SystemTime>,
    pub current: Overlay,
}

impl OverlayFile {
    pub fn path(game_dir: &Path) -> PathBuf {
        game_dir.join(FILE)
    }

    /// Looks at the file (at most every [`POLL`]); true when what to show changed.
    pub fn poll(&mut self, game_dir: &Path, now: Instant, wall: SystemTime) -> bool {
        if self.next.is_some_and(|n| now < n) {
            return false;
        }
        self.next = Some(now + POLL);
        let path = Self::path(game_dir);
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let fresh = modified.is_some_and(|m| wall.duration_since(m).unwrap_or_default() < STALE);
        let next = if !fresh {
            Overlay::default()
        } else if modified == self.modified {
            return false;
        } else {
            match std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| Overlay::parse(&t))
            {
                Some(o) => o,
                // Mid-rename or garbage: keep what was shown, look again next time.
                None => return false,
            }
        };
        self.modified = if fresh { modified } else { None };
        let changed = next != self.current;
        self.current = next;
        changed
    }
}

/// `rustic-voice {"ptt":true|false}`.
pub fn ptt_line(down: bool) -> String {
    format!("rustic-voice {}", serde_json::json!({ "ptt": down }))
}

/// Where the player is, as the line reports it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Position {
    pub server: String,
    pub dim: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
}

impl Position {
    fn moved_from(&self, other: &Position) -> bool {
        let d =
            ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
                .sqrt();
        let turn = ((self.yaw - other.yaw + 540.0).rem_euclid(360.0) - 180.0).abs();
        self.server != other.server || self.dim != other.dim || d > 0.25 || turn > 5.0
    }
}

/// `rustic-voice {"pos":{...}}`, or `{"pos":null}` out of a world.
pub fn position_line(p: Option<&Position>) -> String {
    format!("rustic-voice {}", serde_json::json!({ "pos": p }))
}

/// Rate-limits the position lines: at most every [`PositionReporter::MIN`], only after a real
/// move or a turn, and once a second at least (so a new listener hears where we are).
#[derive(Default)]
pub struct PositionReporter {
    last: Option<Option<Position>>,
    at: Option<Instant>,
}

impl PositionReporter {
    pub const MIN: Duration = Duration::from_millis(200);
    pub const HEARTBEAT: Duration = Duration::from_secs(1);

    /// The line to print now, if any.
    pub fn next(&mut self, pos: Option<Position>, now: Instant) -> Option<String> {
        let since = self.at.map(|a| now.saturating_duration_since(a));
        if since.is_some_and(|s| s < Self::MIN) {
            return None;
        }
        let due = since.is_none_or(|s| s >= Self::HEARTBEAT);
        let changed = match (&self.last, &pos) {
            (None, _) => true,
            (Some(None), None) => false,
            (Some(Some(a)), Some(b)) => b.moved_from(a),
            _ => true,
        };
        if !changed && !due {
            return None;
        }
        // Out of a world: said once, not repeated every second.
        if pos.is_none() && matches!(self.last, Some(None)) {
            return None;
        }
        self.at = Some(now);
        let line = position_line(pos.as_ref());
        self.last = Some(pos);
        Some(line)
    }

    /// Proximity went off: the next report starts afresh.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The members' list in the top-left corner: the room, then you and each member, a lit dot for
/// whoever speaks. `width` measures text in GUI pixels.
pub fn layout(o: &Overlay, origin: [f32; 2], width: impl Fn(&str) -> f32) -> Vec<Draw> {
    const ROW: f32 = 11.0;
    const SPEAK: u32 = 0x55_FF77;
    const IDLE: u32 = 0xDD_DDDD;
    const DIM: u32 = 0x88_8888;
    if !o.active || !o.show {
        return Vec::new();
    }
    let you = if o.you.is_empty() {
        "You"
    } else {
        o.you.as_str()
    };
    let mut rows: Vec<(String, bool, bool)> =
        vec![(you.to_owned(), o.self_speaking, o.self_muted || o.deafened)];
    rows.extend(
        o.members
            .iter()
            .map(|m| (m.name.clone(), m.speaking, m.muted)),
    );
    let header = if o.room.is_empty() {
        "Voice".to_owned()
    } else {
        o.room.clone()
    };
    let w = rows
        .iter()
        .map(|(n, _, _)| width(n) + 12.0)
        .chain([width(&header)])
        .fold(0.0_f32, f32::max)
        + 8.0;
    let h = ROW * (rows.len() as f32 + 1.0) + 4.0;
    let [x, y] = origin;
    let mut d = vec![
        Draw::Fill {
            rect: [x, y, w, h],
            argb: 0x7000_0000,
        },
        Draw::Text {
            at: [x + 4.0, y + 3.0],
            text: header,
            rgb: 0xFF_C080,
        },
    ];
    for (i, (name, speaking, muted)) in rows.into_iter().enumerate() {
        let ry = y + 3.0 + ROW * (i as f32 + 1.0);
        d.push(Draw::Fill {
            rect: [x + 4.0, ry + 2.0, 5.0, 5.0],
            argb: if speaking { 0xFF55_FF77 } else { 0xFF44_4444 },
        });
        d.push(Draw::Text {
            at: [x + 13.0, ry],
            text: name,
            rgb: if speaking {
                SPEAK
            } else if muted {
                DIM
            } else {
                IDLE
            },
        });
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f64, yaw: f64) -> Position {
        Position {
            server: "play.example.org".into(),
            dim: "minecraft:overworld".into(),
            x,
            y: 64.0,
            z: 0.0,
            yaw,
        }
    }

    #[test]
    fn the_file_is_parsed_and_bounded() {
        let text = serde_json::json!({
            "active": true,
            "positions": true,
            "ptt_key": "key.keyboard.v",
            "room": "Party",
            "members": (0..40).map(|i| serde_json::json!({"name": format!("p{i}\u{7}{}", "x".repeat(50)), "speaking": i == 1})).collect::<Vec<_>>(),
        })
        .to_string();
        let o = Overlay::parse(&text).unwrap();
        assert!(o.active && o.positions && o.show);
        assert_eq!(o.members.len(), 32);
        assert!(
            o.members
                .iter()
                .all(|m| m.name.chars().count() <= 32 && !m.name.contains('\u{7}'))
        );
        assert!(o.is_ptt("key.keyboard.v") && !o.is_ptt("key.keyboard.b"));
        assert_eq!(Overlay::parse("nope"), None);
        assert!(!Overlay::default().is_ptt("key.keyboard.v"));
    }

    #[test]
    fn the_file_is_followed_and_dropped_when_stale() {
        let dir = std::env::temp_dir().join(format!("rustic-voice-core-{}", std::process::id()));
        let path = OverlayFile::path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"active":true,"room":"Party","members":[{"name":"Lea","speaking":true}]}"#,
        )
        .unwrap();
        let mut f = OverlayFile::default();
        let t0 = Instant::now();
        let now = SystemTime::now();
        assert!(f.poll(&dir, t0, now));
        assert_eq!(f.current.members[0].name, "Lea");
        // Not looked at again before the poll interval, nor re-read when unchanged.
        assert!(!f.poll(&dir, t0 + Duration::from_millis(10), now));
        assert!(!f.poll(&dir, t0 + POLL, now));
        // A launcher gone quiet: hidden.
        assert!(f.poll(&dir, t0 + POLL * 2, now + STALE + Duration::from_secs(1)));
        assert!(!f.current.active);
        // No file: nothing.
        let _ = std::fs::remove_dir_all(&dir);
        let mut g = OverlayFile::default();
        assert!(!g.poll(&dir, t0, now));
        assert!(!g.current.active);
    }

    #[test]
    fn lines_are_what_the_launcher_parses() {
        assert_eq!(ptt_line(true), r#"rustic-voice {"ptt":true}"#);
        assert_eq!(position_line(None), r#"rustic-voice {"pos":null}"#);
        let line = position_line(Some(&pos(1.5, 90.0)));
        assert!(line.starts_with("rustic-voice {\"pos\":{"));
        let v: serde_json::Value =
            serde_json::from_str(line.trim_start_matches("rustic-voice ")).unwrap();
        assert_eq!(v["pos"]["server"], "play.example.org");
        assert_eq!(v["pos"]["x"], 1.5);
    }

    #[test]
    fn positions_are_rate_limited() {
        let mut r = PositionReporter::default();
        let t = Instant::now();
        assert!(r.next(Some(pos(0.0, 0.0)), t).is_some());
        // Too soon.
        assert!(
            r.next(Some(pos(5.0, 0.0)), t + Duration::from_millis(50))
                .is_none()
        );
        // Not moved: nothing until the heartbeat.
        assert!(
            r.next(Some(pos(0.1, 0.0)), t + Duration::from_millis(300))
                .is_none()
        );
        assert!(
            r.next(Some(pos(0.1, 0.0)), t + Duration::from_millis(1000))
                .is_some()
        );
        // Moved or turned.
        let t2 = t + Duration::from_millis(1300);
        assert!(r.next(Some(pos(3.0, 0.0)), t2).is_some());
        assert!(
            r.next(Some(pos(3.0, 20.0)), t2 + Duration::from_millis(250))
                .is_some()
        );
        // Leaving the world: once.
        let t3 = t2 + Duration::from_millis(600);
        assert_eq!(
            r.next(None, t3).as_deref(),
            Some(r#"rustic-voice {"pos":null}"#)
        );
        assert!(r.next(None, t3 + Duration::from_secs(5)).is_none());
        r.reset();
        assert!(r.next(None, t3 + Duration::from_secs(6)).is_some());
    }

    #[test]
    fn the_list_lights_whoever_speaks() {
        let o = Overlay {
            active: true,
            room: "Party".into(),
            you: "Vous".into(),
            self_speaking: false,
            members: vec![
                Member {
                    name: "Lea".into(),
                    speaking: true,
                    muted: false,
                },
                Member {
                    name: "Max".into(),
                    speaking: false,
                    muted: true,
                },
            ],
            ..Overlay::default()
        };
        let d = layout(&o, [4.0, 4.0], |s| s.len() as f32 * 6.0);
        let texts: Vec<(&str, u32)> = d
            .iter()
            .filter_map(|x| match x {
                Draw::Text { text, rgb, .. } => Some((text.as_str(), *rgb)),
                _ => None,
            })
            .collect();
        assert_eq!(texts[0].0, "Party");
        assert_eq!(texts[1].0, "Vous");
        assert_eq!(texts[2], ("Lea", 0x55_FF77));
        assert_eq!(texts[3], ("Max", 0x88_8888));
        assert!(
            layout(
                &Overlay {
                    show: false,
                    ..o.clone()
                },
                [0.0, 0.0],
                |_| 1.0
            )
            .is_empty()
        );
        assert!(layout(&Overlay::default(), [0.0, 0.0], |_| 1.0).is_empty());
    }
}
