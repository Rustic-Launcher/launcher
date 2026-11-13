//! Where the player is, for the launcher's Discord Rich Presence: one line on the game's standard
//! output when it changes, `rustic-presence {"place":"menu"|"singleplayer"|"server","server":…}`,
//! which the launcher reads with the game's other output (`apps/launcher/src-tauri/src/discord.rs`).

/// The place the game reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Menu,
    Singleplayer,
    Server(String),
}

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
        let json = match self {
            Place::Menu => serde_json::json!({ "place": "menu" }),
            Place::Singleplayer => serde_json::json!({ "place": "singleplayer" }),
            Place::Server(host) => serde_json::json!({ "place": "server", "server": host }),
        };
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

/// Reports the place once per change.
#[derive(Default)]
pub struct Reporter {
    last: Option<Place>,
}

impl Reporter {
    /// The line to print when `place` differs from the last one reported.
    pub fn report(&mut self, place: Place) -> Option<String> {
        if self.last.as_ref() == Some(&place) {
            return None;
        }
        let line = place.line();
        self.last = Some(place);
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            r.report(Place::Menu).as_deref(),
            Some(r#"rustic-presence {"place":"menu"}"#)
        );
        assert_eq!(r.report(Place::Menu), None);
        assert_eq!(
            r.report(Place::Server("mc.example.org".into())).as_deref(),
            Some(r#"rustic-presence {"place":"server","server":"mc.example.org"}"#)
        );
        assert_eq!(r.report(Place::Server("mc.example.org".into())), None);
        assert!(r.report(Place::Singleplayer).is_some());
    }
}
