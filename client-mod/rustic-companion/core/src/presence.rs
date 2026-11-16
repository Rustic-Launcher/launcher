//! Where the player is, for the launcher's Discord Rich Presence: one line on the game's standard
//! output when it changes, `rustic-presence {"place":"menu"|"singleplayer"|"server","server":…}`,
//! which the launcher reads with the game's other output (`apps/launcher/src-tauri/src/discord.rs`).
//!
//! The line also carries the place's own picture when it has one (`"icon"`: a PNG in base64): the
//! server's ping favicon as `servers.dat` keeps it, or the singleplayer world's `icon.png`. The
//! launcher shows it as the small image of the presence (launcher ADR 0173); the companion only
//! reads what the game already has on disk, it never asks a server for anything.

use base64::Engine as _;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{Duration, Instant};

/// The place the game reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Menu,
    Singleplayer,
    Server(String),
}

/// The largest icon the line carries: a favicon and a world icon are 64x64 PNGs of a few KiB.
pub const MAX_ICON_BYTES: usize = 128 * 1024;

const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

impl Place {
    /// From the client's state: in a world with a server address it is a server, in a world
    /// without one the integrated server (singleplayer), out of a world a menu.
    pub fn of(in_world: bool, server_address: Option<&str>) -> Place {
        match (
            in_world,
            server_address.map(str::trim).filter(|a| !a.is_empty()),
        ) {
            // Horizon's own integrated server (singleplayer) is a session at the address "local".
            (true, Some(INTEGRATED)) => Place::Singleplayer,
            (true, Some(address)) => Place::Server(host_of(address).to_owned()),
            (true, None) => Place::Singleplayer,
            (false, _) => Place::Menu,
        }
    }

    pub fn line(&self) -> String {
        self.line_with(None)
    }

    /// The line, with the place's own icon when there is one (a menu has none).
    pub fn line_with(&self, icon: Option<&[u8]>) -> String {
        let mut json = match self {
            Place::Menu => serde_json::json!({ "place": "menu" }),
            Place::Singleplayer => serde_json::json!({ "place": "singleplayer" }),
            Place::Server(host) => serde_json::json!({ "place": "server", "server": host }),
        };
        if let Some(png) = icon.filter(|p| *self != Place::Menu && usable_icon(p)) {
            json["icon"] = base64::engine::general_purpose::STANDARD.encode(png).into();
        }
        format!("rustic-presence {json}")
    }
}

/// The session address Horizon gives its integrated (singleplayer) server.
const INTEGRATED: &str = "local";

/// The host of an address as typed (`play.example.com:25566` → `play.example.com`; IPv6 kept).
fn host_of(address: &str) -> &str {
    if address.starts_with('[') {
        return address
            .split_once(']')
            .map_or(address, |(h, _)| h.trim_start_matches('['));
    }
    match address.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') && port.parse::<u16>().is_ok() => host,
        _ => address,
    }
}

/// Two addresses as the server list compares them: trimmed, any case, the default port implied.
fn same_server(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        let s = s.trim().trim_end_matches('.').to_ascii_lowercase();
        s.strip_suffix(":25565").map(str::to_owned).unwrap_or(s)
    };
    norm(a) == norm(b)
}

/// A PNG small enough for the line.
pub fn usable_icon(png: &[u8]) -> bool {
    png.len() <= MAX_ICON_BYTES && png.starts_with(PNG_MAGIC)
}

/// The icon of the server at `address` as the server list keeps it (`servers.dat`: each entry's
/// address as typed and the base64 PNG of its last ping), shown and hidden entries alike.
pub fn server_icon<'a>(
    entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    address: &str,
) -> Option<Vec<u8>> {
    entries
        .into_iter()
        .filter(|(ip, _)| same_server(ip, address))
        .find_map(|(_, icon)| {
            let text: String = icon?.chars().filter(|c| !c.is_whitespace()).collect();
            base64::engine::general_purpose::STANDARD
                .decode(text)
                .ok()
                .filter(|png| usable_icon(png))
        })
}

/// The `icon.png` of the singleplayer world `folder` in `saves`. Only a plain folder name is read,
/// never a path.
pub fn world_icon(saves: &Path, folder: &str) -> Option<Vec<u8>> {
    let plain = !folder.is_empty()
        && folder != "."
        && folder != ".."
        && !folder.contains(['/', '\\', ':'])
        && !folder.starts_with('.');
    if !plain {
        return None;
    }
    let path = saves.join(folder).join("icon.png");
    if std::fs::metadata(&path).ok()?.len() > MAX_ICON_BYTES as u64 {
        return None;
    }
    std::fs::read(path).ok().filter(|png| usable_icon(png))
}

/// When to look for the place's icon: at once on a new place, then every few seconds for a
/// while when there was none (a new world writes its `icon.png` a few seconds after it opens, a
/// server's favicon arrives with the list's next ping).
#[derive(Default)]
pub struct IconLookup {
    place: Option<Place>,
    icon: Option<Vec<u8>>,
    next: Option<Instant>,
    tries: u32,
}

impl IconLookup {
    /// Seconds between two looks while the place has no icon.
    pub const RETRY: Duration = Duration::from_secs(5);
    /// Looks at most this many times for one place (two minutes).
    pub const TRIES: u32 = 24;

    /// Whether to look now for the icon of `place`.
    pub fn due(&mut self, place: &Place, now: Instant) -> bool {
        if self.place.as_ref() != Some(place) {
            *self = Self {
                place: Some(place.clone()),
                ..Self::default()
            };
        }
        if *place == Place::Menu || self.icon.is_some() || self.tries >= Self::TRIES {
            return false;
        }
        self.next.is_none_or(|t| now >= t)
    }

    /// What a look found.
    pub fn found(&mut self, icon: Option<Vec<u8>>, now: Instant) {
        self.tries += 1;
        self.next = Some(now + Self::RETRY);
        self.icon = icon.filter(|p| usable_icon(p));
    }

    /// The icon of the current place, once found.
    pub fn icon(&self) -> Option<&[u8]> {
        self.icon.as_deref()
    }
}

/// Reports the place once per change (a new icon for the same place is a change).
#[derive(Default)]
pub struct Reporter {
    last: Option<(Place, Option<[u8; 32]>)>,
}

impl Reporter {
    /// The line to print when `place` (or its icon) differs from the last one reported.
    pub fn report(&mut self, place: Place, icon: Option<&[u8]>) -> Option<String> {
        let icon = icon.filter(|p| place != Place::Menu && usable_icon(p));
        let key = (place, icon.map(|p| Sha256::digest(p).into()));
        if self.last.as_ref() == Some(&key) {
            return None;
        }
        let line = key.0.line_with(icon);
        self.last = Some(key);
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(extra: &[u8]) -> Vec<u8> {
        let mut p = PNG_MAGIC.to_vec();
        p.extend_from_slice(extra);
        p
    }

    #[test]
    fn the_place_follows_the_client_state() {
        assert_eq!(Place::of(false, None), Place::Menu);
        assert_eq!(Place::of(false, Some("play.example.com")), Place::Menu);
        assert_eq!(Place::of(true, None), Place::Singleplayer);
        assert_eq!(Place::of(true, Some("  ")), Place::Singleplayer);
        // Regression: singleplayer showed "Multiplayer" on Discord.
        assert_eq!(Place::of(true, Some("local")), Place::Singleplayer);
        assert_eq!(
            Place::of(true, Some("play.example.com:25566")),
            Place::Server("play.example.com".into())
        );
        assert_eq!(
            Place::of(true, Some("[2001:db8::1]:25565")),
            Place::Server("2001:db8::1".into())
        );
    }

    #[test]
    fn a_line_is_printed_once_per_change() {
        let mut r = Reporter::default();
        assert_eq!(
            r.report(Place::Menu, None).as_deref(),
            Some(r#"rustic-presence {"place":"menu"}"#)
        );
        assert_eq!(r.report(Place::Menu, None), None);
        assert_eq!(
            r.report(Place::Server("mc.example.org".into()), None)
                .as_deref(),
            Some(r#"rustic-presence {"place":"server","server":"mc.example.org"}"#)
        );
        assert_eq!(r.report(Place::Server("mc.example.org".into()), None), None);
        assert!(r.report(Place::Singleplayer, None).is_some());
    }

    #[test]
    fn the_icon_rides_on_the_line_and_a_new_one_is_a_change() {
        let mut r = Reporter::default();
        let icon = png(b"server");
        let server = Place::Server("mc.example.org".into());
        let line = r.report(server.clone(), Some(&icon)).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(line.strip_prefix("rustic-presence ").unwrap()).unwrap();
        let sent = base64::engine::general_purpose::STANDARD
            .decode(json["icon"].as_str().unwrap())
            .unwrap();
        assert_eq!(sent, icon);
        assert_eq!(json["server"], "mc.example.org");
        assert_eq!(r.report(server.clone(), Some(&icon)), None);
        // The same place gains or changes its icon: said again.
        assert!(r.report(server.clone(), Some(&png(b"other"))).is_some());
        assert!(!r.report(server, None).unwrap().contains("icon"));
        // A menu never carries one; neither does something that isn't a PNG, or a huge one.
        assert!(!Place::Menu.line_with(Some(&icon)).contains("icon"));
        assert!(
            !Place::Singleplayer
                .line_with(Some(b"GIF89a"))
                .contains("icon")
        );
        let huge = png(&vec![0; MAX_ICON_BYTES]);
        assert!(!Place::Singleplayer.line_with(Some(&huge)).contains("icon"));
    }

    #[test]
    fn the_server_list_gives_the_favicon_of_the_address() {
        let icon = png(b"fav");
        let b64 = base64::engine::general_purpose::STANDARD.encode(&icon);
        // servers.dat may wrap the base64; the default port and the case don't matter.
        let wrapped = format!("{}\n{}", &b64[..4], &b64[4..]);
        let entries = vec![
            ("other.example.com", Some("AAAA")),
            ("Play.Example.com:25565", Some(wrapped.as_str())),
        ];
        assert_eq!(
            server_icon(entries.clone(), "play.example.com"),
            Some(icon.clone())
        );
        assert_eq!(server_icon(entries, "nowhere.example.com"), None);
        // Not base64, not a PNG, or no icon at all: none.
        assert_eq!(
            server_icon(vec![("a.example", Some("@@@"))], "a.example"),
            None
        );
        assert_eq!(
            server_icon(vec![("a.example", Some("R0lG"))], "a.example"),
            None
        );
        assert_eq!(server_icon(vec![("a.example", None)], "a.example"), None);
    }

    #[test]
    fn a_world_icon_is_read_from_its_plain_folder_only() {
        let saves = std::env::temp_dir().join(format!("rustic-presence-{}", std::process::id()));
        let world = saves.join("New World");
        std::fs::create_dir_all(&world).unwrap();
        let icon = png(b"world");
        std::fs::write(world.join("icon.png"), &icon).unwrap();
        assert_eq!(world_icon(&saves, "New World"), Some(icon));
        assert_eq!(world_icon(&saves, "Missing"), None);
        for bad in ["", "..", "../New World", "a/b", r"a\b", ".hidden", "C:"] {
            assert_eq!(world_icon(&saves, bad), None, "{bad}");
        }
        std::fs::write(world.join("icon.png"), b"not a png").unwrap();
        assert_eq!(world_icon(&saves, "New World"), None);
        let _ = std::fs::remove_dir_all(&saves);
    }

    #[test]
    fn the_icon_is_looked_for_on_arrival_then_retried_for_a_while() {
        let t0 = Instant::now();
        let mut l = IconLookup::default();
        let world = Place::Singleplayer;
        assert!(!l.due(&Place::Menu, t0), "a menu has no icon");
        assert!(l.due(&world, t0));
        l.found(None, t0);
        assert!(!l.due(&world, t0 + Duration::from_secs(1)));
        assert!(l.due(&world, t0 + IconLookup::RETRY));
        l.found(Some(png(b"w")), t0 + IconLookup::RETRY);
        assert_eq!(l.icon(), Some(png(b"w").as_slice()));
        assert!(
            !l.due(&world, t0 + Duration::from_secs(60)),
            "found: no more looks"
        );
        // Another place starts over; after the last try it gives up.
        let server = Place::Server("a.example".into());
        let mut t = t0;
        for _ in 0..IconLookup::TRIES {
            assert!(l.due(&server, t));
            assert_eq!(l.icon(), None);
            l.found(None, t);
            t += IconLookup::RETRY;
        }
        assert!(!l.due(&server, t));
    }
}
