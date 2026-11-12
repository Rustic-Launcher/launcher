//! Messages of the realtime WebSocket (`GET /v1/ws?access_token=...` on the realtime service).
//!
//! Every frame is a JSON text frame tagged by `type`. The API publishes [`ServerMessage`] values
//! on the Redis channel `rt:u:<user-id>`; the realtime service forwards them verbatim to every
//! socket of that user.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::{
    Activity, EquippedSlot, FriendRequest, Invitation, Message, Notification, Party, PartyInvite,
    Presence, PresenceStatus, PublicUser,
};

/// Client → server frames.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Send every `heartbeat_interval_secs` (see `hello`). Presence expires after 3 missed beats.
    Heartbeat {
        status: PresenceStatus,
        #[serde(default)]
        activity: Option<Activity>,
        #[serde(default)]
        share_server: bool,
    },
    /// Application-level ping; answered by `pong` with the same nonce.
    Ping { nonce: u64 },
}

/// Server → client frames.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello {
        user_id: Uuid,
        connection_id: Uuid,
        heartbeat_interval_secs: u64,
    },
    Pong {
        nonce: u64,
    },
    PresenceChanged {
        user_id: Uuid,
        presence: Presence,
    },
    FriendRequestReceived {
        request: FriendRequest,
    },
    FriendAdded {
        friend: PublicUser,
    },
    FriendRemoved {
        user_id: Uuid,
    },
    MessageReceived {
        message: Message,
    },
    /// Read receipt: `reader_id` read the conversation with you up to `read_at`.
    MessagesRead {
        reader_id: Uuid,
        read_at: DateTime<Utc>,
    },
    /// The user accepted a server invitation (e.g. from the in-game companion): the launcher
    /// should (re)start the game into `address`.
    JoinServer {
        address: String,
        invitation_id: Uuid,
    },
    PartyInviteReceived {
        invite: PartyInvite,
    },
    /// `party = null` when the party was disbanded or the user left/was kicked.
    PartyUpdated {
        party_id: Uuid,
        party: Option<Party>,
    },
    InvitationReceived {
        invitation: Invitation,
    },
    NotificationCreated {
        notification: Notification,
    },
    EquippedChanged {
        user_id: Uuid,
        items: Vec<EquippedSlot>,
        updated_at: DateTime<Utc>,
    },
    /// The session behind this socket was revoked: the server closes the socket after this.
    SessionRevoked {
        session_id: Uuid,
    },
    RateLimited {
        retry_after_ms: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

/// Redis channel carrying [`ServerMessage`]s for one user.
pub fn user_channel(user_id: Uuid) -> String {
    format!("{USER_CHANNEL_PREFIX}{user_id}")
}

/// Prefix of per-user channels (the realtime service pattern-subscribes to `rt:u:*`).
pub const USER_CHANNEL_PREFIX: &str = "rt:u:";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_snake_case() {
        let m = ServerMessage::Pong { nonce: 7 };
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            r#"{"type":"pong","nonce":7}"#
        );
        let c: ClientMessage =
            serde_json::from_str(r#"{"type":"heartbeat","status":"online"}"#).unwrap();
        assert_eq!(
            c,
            ClientMessage::Heartbeat {
                status: PresenceStatus::Online,
                activity: None,
                share_server: false
            }
        );
    }

    #[test]
    fn channel_name() {
        let id = Uuid::nil();
        assert_eq!(user_channel(id), format!("rt:u:{id}"));
    }
}
