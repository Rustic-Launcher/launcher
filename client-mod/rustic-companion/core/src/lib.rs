//! The Rustic companion's logic, with no game type in it so it is tested without building
//! Horizon. The `rusticmc-companion` crate wires it to Horizon's hooks.
//!
//! - [`badge`]: who uses Rustic (the cloud's lookup, batched, cached, backed off) and the Rustic
//!   badge's picture at the game's text size, in the place of Horizon's mark.
//! - [`config`]: keys, endpoints, switches (`config/rustic-companion.json`).
//! - [`protocol`]: the Rustic Cloud calls the companion makes (proposed in
//!   launcher/docs/HANDOFF.md until the cloud's OpenAPI has them).
//! - [`worn`]: what the players in view wear, asked in batches of 64, cached for the service's
//!   TTL, with backoff; never asked from the render thread.
//! - [`store`]: cosmetic bundles by content hash, verified, baked once, with their sprite ids.
//! - [`social`], [`overlay`]: friends, invites and messages, and the overlay's layout and clicks.
//! - [`wheel`]: the emote wheel's slices and pick.
//! - [`title`]: the Rustic main menu over the title screen's panorama (logo, buttons, footer).
//! - [`notify`]: discreet notifications (at most a few, deduplicated, rate-limited).
//! - [`trail`]: trail particles placed from distance walked and age only (deterministic).
#![forbid(unsafe_code)]

pub mod backoff;
pub mod badge;
pub mod config;
pub mod notify;
pub mod overlay;
pub mod presence;
pub mod protocol;
pub mod social;
pub mod store;
pub mod title;
pub mod trail;
pub mod wheel;
pub mod worn;

/// A player's UUID as the game gives it, as 32 lowercase hex digits (the cloud's spelling).
pub fn uuid_hex(uuid: u128) -> String {
    format!("{uuid:032x}")
}

/// The reverse of [`uuid_hex`]; dashes are accepted.
pub fn parse_uuid(text: &str) -> Option<u128> {
    let hex: String = text.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn uuids_round_trip() {
        let u = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdefu128;
        assert_eq!(super::parse_uuid(&super::uuid_hex(u)), Some(u));
        assert_eq!(
            super::parse_uuid("01234567-89ab-cdef-0123-456789abcdef"),
            Some(u)
        );
        assert_eq!(super::parse_uuid("nope"), None);
    }
}
