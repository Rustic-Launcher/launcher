//! What the game gets: the session JSON of Horizon's Launch API (horizon/docs/api/launch.md,
//! "The session JSON"), handed over on stdin (`--session-stdin`), never on the command line.

use rusticmc_launcher_core::Secret;
use serde::Serialize;

/// The signed-in account a game plays as.
#[derive(Clone, Debug)]
pub struct GameSession {
    pub name: String,
    /// Undashed UUID.
    pub uuid: String,
    pub access_token: Secret,
    pub xuid: Option<String>,
}

impl GameSession {
    /// `{"name", "uuid", "access_token", "user_type": "msa", "xuid"}`, as a secret (it holds the
    /// token).
    pub fn to_json(&self) -> Secret {
        #[derive(Serialize)]
        struct Json<'a> {
            name: &'a str,
            uuid: &'a str,
            access_token: &'a str,
            user_type: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            xuid: Option<&'a str>,
        }
        let json = Json {
            name: &self.name,
            uuid: &self.uuid,
            access_token: self.access_token.expose(),
            user_type: "msa",
            xuid: self.xuid.as_deref(),
        };
        Secret::new(serde_json::to_string(&json).expect("plain strings serialize"))
    }
}

/// How a launch is authenticated.
#[derive(Clone, Debug)]
pub enum LaunchAuth {
    /// Online: hand this session to the game.
    Online(GameSession),
    /// The services couldn't be reached, but this account's last answer (within the window) was
    /// "owns": the game starts without a session, offline (online-mode servers will refuse it).
    /// No network, but the owner's last answer is recent enough: the game starts with `session`
    /// (official name and UUID, last token) and goes offline on its own.
    Offline {
        uuid: String,
        name: String,
        reason: String,
        session: GameSession,
    },
}

/// Proof that an account passed the ownership check just now. Only this crate makes one (after a
/// successful entitlements answer); owner-only actions (downloading the server jar for Horizon's
/// preparer, docs/api/launch.md "Preparing") require it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedOwner {
    uuid: String,
    name: String,
    checked_at: u64,
}

impl VerifiedOwner {
    pub(crate) fn new(uuid: String, name: String, checked_at: u64) -> Self {
        Self {
            uuid,
            name,
            checked_at,
        }
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn checked_at(&self) -> u64 {
        self.checked_at
    }

    /// A proof for other crates' tests.
    #[cfg(feature = "test-support")]
    pub fn for_tests(uuid: &str, name: &str) -> Self {
        Self::new(
            uuid.into(),
            name.into(),
            rusticmc_launcher_core::time::now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_json_has_the_launch_api_fields() {
        let s = GameSession {
            name: "Alex".into(),
            uuid: "069a79f444e94726a5befca90e38aaf5".into(),
            access_token: Secret::new("s3cr3t"),
            xuid: None,
        };
        let v: serde_json::Value = serde_json::from_str(s.to_json().expose()).unwrap();
        assert_eq!(v["user_type"], "msa");
        assert_eq!(v["access_token"], "s3cr3t");
        assert!(v.get("xuid").is_none());
        assert!(!format!("{s:?}").contains("s3cr3t"));
    }
}
