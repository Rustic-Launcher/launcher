//! The Rustic Cloud calls the companion makes, as plain request descriptions (the game crate
//! sends them with `ureq` on a worker thread) and their answers. Proposed to the cloud owner in
//! launcher/docs/HANDOFF.md; the companion follows `services/api/openapi.json` once it has them.
//!
//! | Call | Auth | Body | Answer |
//! |---|---|---|---|
//! | `POST <local>/v1/companion/session` | the launcher's one-time companion token | | [`Session`] |
//! | `POST /v1/cosmetics/worn` | none | `{"players": [uuid hex, at most 64]}` | [`WornAnswer`] |
//! | `GET <cdn>/cosmetics/<id>/<version>/<sha256>.json` | none | | a cosmetic bundle |
//! | `GET /v1/social` | bearer | | [`SocialSnapshot`] (friends, invites, messages, owned emotes) |
//! | `POST /v1/social/invites` | bearer | `{"to": uuid hex, "address"}` | 204 |
//! | `POST /v1/social/invites/<id>/accept` | bearer | | 204 (the launcher joins) |
//! | `POST /v1/emotes/playing` | bearer | `{"emote": id \| null}` | 204 |
//! | `POST /v1/public/minecraft/lookup` | none | `{"uuids": [uuid hex, at most 100]}` | [`LookupAnswer`]: who uses Rustic (the badge, ADR 0072) |
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The largest batch `POST /v1/cosmetics/worn` takes.
pub const MAX_BATCH: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// One HTTP call to make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub body: Option<String>,
    /// Send the Rustic session's bearer token.
    pub auth: bool,
    /// Send this bearer token instead (the launcher's one-time companion token).
    pub bearer: Option<String>,
}

impl Request {
    fn post(url: String, body: serde_json::Value, auth: bool) -> Self {
        Self {
            method: Method::Post,
            url,
            body: Some(body.to_string()),
            auth,
            bearer: None,
        }
    }

    fn get(url: String, auth: bool) -> Self {
        Self {
            method: Method::Get,
            url,
            body: None,
            auth,
            bearer: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    /// Seconds.
    pub expires_in: u64,
    /// The signed-in player's UUID.
    pub uuid: String,
}

/// One worn cosmetic: where its bundle is on the CDN.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WornItem {
    pub id: String,
    pub version: String,
    /// SHA-256 of the bundle: its address and its check.
    pub hash: String,
}

/// An emote someone plays, `ms` milliseconds in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Playing {
    pub item: WornItem,
    pub ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WornAnswer {
    /// Seconds this answer holds (kept within 60 to 3600).
    #[serde(default)]
    pub ttl: u64,
    /// By UUID hex; a player wearing nothing is left out.
    #[serde(default)]
    pub players: BTreeMap<String, Vec<WornItem>>,
    /// Emotes playing now, by UUID hex.
    #[serde(default)]
    pub playing: BTreeMap<String, Playing>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Friend {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub online: bool,
    /// What they play, when they share it ("Rustic SMP", "Single player").
    #[serde(default)]
    pub activity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Invite {
    pub id: String,
    pub from: String,
    /// A server's name as the inviter shows it.
    pub server: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SocialSnapshot {
    #[serde(default)]
    pub friends: Vec<Friend>,
    #[serde(default)]
    pub invites: Vec<Invite>,
    #[serde(default)]
    pub messages: Vec<Message>,
    /// The emotes the account owns, for the wheel (at most 8 shown).
    #[serde(default)]
    pub emotes: Vec<WornItem>,
}

/// One entry of `POST /v1/public/minecraft/lookup`'s answer. The cloud also sends the linked
/// account's public profile (`player.user`: id, username, display name) and what it has equipped;
/// the badge reads only whether `player` is there, and nothing else is kept.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct LookupEntry {
    pub uuid: String,
    /// `null` when the Minecraft account is not linked to an active Rustic account.
    #[serde(default)]
    pub player: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct LookupAnswer {
    #[serde(default)]
    pub players: Vec<LookupEntry>,
}

impl LookupAnswer {
    /// The UUIDs the cloud says use Rustic (linked to an active Rustic account).
    pub fn rustic(&self) -> Vec<u128> {
        self.players
            .iter()
            .filter(|e| e.player.as_ref().is_some_and(|p| !p.is_null()))
            .filter_map(|e| crate::parse_uuid(&e.uuid))
            .collect()
    }
}

/// The cloud's endpoints, and the launcher's loopback channel (ADR 0008: the launcher hands the
/// game a one-time `RUSTIC_COMPANION_TOKEN` and `RUSTIC_COMPANION_ENDPOINT`, never a Rustic
/// refresh token; the companion trades the one for a short-lived Rustic access token).
#[derive(Clone, Debug)]
pub struct Endpoints {
    pub api: String,
    pub cdn: String,
    /// `http://127.0.0.1:<port>`, loopback only.
    pub local: Option<String>,
}

/// Whether `url` is a loopback address (the only plain-http address the companion calls).
pub fn loopback(url: &str) -> bool {
    url.strip_prefix("http://")
        .and_then(|r| r.split(['/', ':']).next())
        .is_some_and(|h| h == "127.0.0.1" || h == "localhost")
}

fn plain(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
        && !s.contains("..")
}

impl Endpoints {
    /// Trades the launcher's one-time companion token for a Rustic session (None without a
    /// loopback endpoint).
    pub fn session(&self, companion_token: &str) -> Option<Request> {
        let local = self.local.as_deref().filter(|l| loopback(l))?;
        Some(Request {
            method: Method::Post,
            url: format!("{local}/v1/companion/session"),
            body: Some("{}".into()),
            auth: false,
            bearer: Some(companion_token.to_owned()),
        })
    }

    pub fn worn(&self, players: &[u128]) -> Request {
        let ids: Vec<String> = players
            .iter()
            .take(MAX_BATCH)
            .map(|u| crate::uuid_hex(*u))
            .collect();
        Request::post(
            format!("{}/v1/cosmetics/worn", self.api),
            serde_json::json!({ "players": ids }),
            false,
        )
    }

    /// The bundle's address; None for an item whose names could leave the CDN's paths.
    pub fn bundle(&self, item: &WornItem) -> Option<Request> {
        let hex = item.hash.len() == 64 && item.hash.bytes().all(|b| b.is_ascii_hexdigit());
        (plain(&item.id) && plain(&item.version) && hex).then(|| {
            Request::get(
                format!(
                    "{}/cosmetics/{}/{}/{}.json",
                    self.cdn,
                    item.id,
                    item.version,
                    item.hash.to_ascii_lowercase()
                ),
                false,
            )
        })
    }

    /// Who among `players` uses Rustic (the badge): at most [`crate::badge::MAX_BATCH`], no
    /// session needed (the lookup is public).
    pub fn rustic_lookup(&self, players: &[u128]) -> Request {
        let ids: Vec<String> = players
            .iter()
            .take(crate::badge::MAX_BATCH)
            .map(|u| crate::uuid_hex(*u))
            .collect();
        Request::post(
            format!("{}/v1/public/minecraft/lookup", self.api),
            serde_json::json!({ "uuids": ids }),
            false,
        )
    }

    pub fn social(&self) -> Request {
        Request::get(format!("{}/v1/social", self.api), true)
    }

    pub fn invite(&self, to: u128, address: &str) -> Request {
        Request::post(
            format!("{}/v1/social/invites", self.api),
            serde_json::json!({ "to": crate::uuid_hex(to), "address": address }),
            true,
        )
    }

    pub fn accept(&self, invite: &str) -> Option<Request> {
        plain(invite).then(|| Request {
            method: Method::Post,
            url: format!("{}/v1/social/invites/{invite}/accept", self.api),
            body: None,
            auth: true,
            bearer: None,
        })
    }

    pub fn playing(&self, emote: Option<&str>) -> Request {
        Request::post(
            format!("{}/v1/emotes/playing", self.api),
            serde_json::json!({ "emote": emote }),
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e() -> Endpoints {
        Endpoints {
            api: "https://api.x".into(),
            cdn: "https://cdn.x".into(),
            local: Some("http://127.0.0.1:4711".into()),
        }
    }

    #[test]
    fn the_session_comes_from_the_launcher_over_loopback_only() {
        let r = e().session("t0k").unwrap();
        assert_eq!(r.url, "http://127.0.0.1:4711/v1/companion/session");
        assert_eq!(r.bearer.as_deref(), Some("t0k"));
        let remote = Endpoints {
            local: Some("http://evil.example:80".into()),
            ..e()
        };
        assert!(remote.session("t0k").is_none());
    }

    #[test]
    fn worn_batches_at_most_64() {
        let r = e().worn(&(0..100u128).collect::<Vec<_>>());
        let v: serde_json::Value = serde_json::from_str(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(v["players"].as_array().unwrap().len(), 64);
        assert_eq!(v["players"][1], "00000000000000000000000000000001");
        assert!(!r.auth);
    }

    #[test]
    fn bundle_paths_are_checked() {
        let ok = WornItem {
            id: "top_hat".into(),
            version: "1.2.0".into(),
            hash: "a".repeat(64),
        };
        assert_eq!(
            e().bundle(&ok).unwrap().url,
            format!(
                "https://cdn.x/cosmetics/top_hat/1.2.0/{}.json",
                "a".repeat(64)
            )
        );
        let bad = WornItem {
            id: "../x".into(),
            ..ok.clone()
        };
        assert!(e().bundle(&bad).is_none());
        let short = WornItem {
            hash: "abc".into(),
            ..ok
        };
        assert!(e().bundle(&short).is_none());
        assert!(e().accept("../../admin").is_none());
    }

    #[test]
    fn the_lookup_asks_at_most_100_and_keeps_only_who_is_linked() {
        let r = e().rustic_lookup(&(1..200u128).collect::<Vec<_>>());
        assert_eq!(r.url, "https://api.x/v1/public/minecraft/lookup");
        assert!(!r.auth && r.bearer.is_none());
        let v: serde_json::Value = serde_json::from_str(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(v["uuids"].as_array().unwrap().len(), 100);
        assert_eq!(v["uuids"][0], "00000000000000000000000000000001");
        // The cloud's own shape (hyphenated UUIDs, the public profile), parsed down to a list.
        let a: LookupAnswer = serde_json::from_str(
            r#"{"players": [
                {"uuid": "00000000-0000-0000-0000-000000000001", "player": {"user": {"id": "x", "username": "a", "display_name": "A"}, "equipped": []}},
                {"uuid": "00000000-0000-0000-0000-000000000002", "player": null},
                {"uuid": "00000000000000000000000000000003"},
                {"uuid": "nope", "player": {}}
            ]}"#,
        )
        .unwrap();
        assert_eq!(a.rustic(), vec![1]);
        let empty: LookupAnswer = serde_json::from_str("{}").unwrap();
        assert!(empty.rustic().is_empty());
    }

    #[test]
    fn answers_parse_with_defaults() {
        let a: WornAnswer = serde_json::from_str(
            r#"{"ttl": 120, "players": {"00000000000000000000000000000001": [{"id": "h", "version": "1.0.0", "hash": "x"}]}}"#,
        )
        .unwrap();
        assert_eq!(a.players.len(), 1);
        assert!(a.playing.is_empty());
        let s: SocialSnapshot = serde_json::from_str("{}").unwrap();
        assert!(s.friends.is_empty());
    }
}
