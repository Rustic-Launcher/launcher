//! The Rustic Relay tunnel protocol, shared by the relay and the tunnel client (docs/RELAY.md).
//!
//! 1. The client opens TCP to the relay's tunnel port and TLS 1.2/1.3 on it, ALPN [`ALPN`].
//! 2. It sends [`ClientMsg::Hello`] as a control frame (u32 big-endian length, then JSON, at most
//!    [`MAX_FRAME`] bytes); the relay answers [`RelayMsg::Welcome`] or [`RelayMsg::Refused`].
//! 3. After `Welcome` both sides run yamux on the same TLS stream: the client is yamux's client
//!    (odd stream ids), the relay its server. The client opens the **control stream** first and
//!    keeps it for the session (pings, token refreshes, notices), writing a `Ping` at once (yamux
//!    announces a stream with its first frame). The relay sends [`RelayMsg::Ready`] on it once the
//!    hostnames route to this tunnel.
//! 4. For each player connection the relay opens a **data stream**: a [`StreamHeader`] control
//!    frame, then the client answers one byte, [`ACK_OK`] once it reached the local server or
//!    [`ACK_UNREACHABLE`]; after `ACK_OK` the relay writes the bytes it already read from the
//!    player (the handshake) and both sides copy raw bytes until either closes.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The protocol version in `Hello`, `Welcome` and stream headers.
pub const VERSION: u16 = 1;
/// The TLS application protocol of the tunnel port.
pub const ALPN: &[u8] = b"rustic-relay/1";
/// The largest control frame either side accepts.
pub const MAX_FRAME: usize = 16 * 1024;
/// The client's answer to a stream header: connected to the local server.
pub const ACK_OK: u8 = 1;
/// The client's answer to a stream header: the local server refused or did not answer.
pub const ACK_UNREACHABLE: u8 = 0;

/// Messages from the tunnel client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ClientMsg {
    /// First frame on the TLS stream.
    Hello {
        v: u16,
        /// The signed tunnel token issued by Rustic Cloud (docs/RELAY.md, "Tunnel token").
        token: String,
        /// `rusticmc-tunnel/<version>`, for logs and compatibility decisions.
        agent: String,
    },
    /// Keep-alive on the control stream; the relay answers [`RelayMsg::Pong`].
    Ping { seq: u64 },
    /// A fresh token for the same server before the current one expires.
    Refresh { token: String },
}

/// Messages from the relay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RelayMsg {
    Welcome(Welcome),
    /// The `Hello` was refused; the TLS stream closes after it.
    Refused {
        code: ErrorCode,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_s: Option<u32>,
    },
    /// First message on the control stream: the tunnel is registered and these hostnames route
    /// to it (the token's, minus names the relay refused or another server holds).
    Ready(Routes),
    Pong {
        seq: u64,
    },
    /// The refreshed token was accepted: its expiry and the hostnames now routed.
    Refreshed {
        expires_at: u64,
        routes: Routes,
    },
    /// Something the server owner may want to see (quota reached, player limit hit).
    Notice {
        code: NoticeCode,
        message: String,
    },
    /// The relay is shutting down: open streams get `deadline_s` seconds, then the tunnel closes.
    Drain {
        deadline_s: u32,
    },
    /// The relay closes the tunnel after this.
    Goodbye {
        code: ErrorCode,
        message: String,
    },
}

/// The relay's answer to an accepted `Hello`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    pub v: u16,
    /// This tunnel session's id (for support; random, not secret).
    pub session: String,
    /// The server id from the token (`sub`).
    pub server_id: String,
    /// The hostnames the token asks for (the routed ones come in [`RelayMsg::Ready`]).
    pub hostnames: Vec<String>,
    pub limits: Limits,
    /// When the token expires (Unix seconds): refresh before then.
    pub expires_at: u64,
    /// How often the relay expects a ping.
    pub ping_every_s: u32,
}

/// The hostnames routed to a tunnel.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routes {
    /// Every routed hostname, permanent and temporary.
    pub hostnames: Vec<String>,
    /// The temporary ones, with when each stops routing.
    #[serde(default)]
    pub temporary: Vec<TempRoute>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempRoute {
    pub host: String,
    /// Unix seconds.
    pub expires_at: u64,
}

/// The limits the relay enforces on this tunnel (plan defaults or the token's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Players (login and transfer connections) at once.
    pub max_players: u32,
    /// Bytes a second, both directions together; 0: no limit.
    pub rate_bytes_per_s: u64,
    /// Bytes a day (UTC), both directions together; 0: no limit.
    pub daily_bytes: u64,
    /// Server list pings in flight at once.
    pub max_status: u32,
}

/// Why the relay refused or closed a tunnel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    Version,
    TokenInvalid,
    TokenExpired,
    RateLimited,
    Full,
    Draining,
    /// Another tunnel for the same server connected.
    Replaced,
    Timeout,
    Internal,
    #[serde(other)]
    Unknown,
}

impl ErrorCode {
    /// Whether the same token can never succeed (the client should fetch a new one).
    pub fn is_auth(self) -> bool {
        matches!(self, Self::TokenInvalid | Self::TokenExpired)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeCode {
    /// The daily transfer quota is used up: players are refused until the next UTC day.
    QuotaExceeded,
    /// A player was refused because the server has as many players as its plan allows.
    PlayerLimit,
    Info,
    #[serde(other)]
    Unknown,
}

/// What the handshake asked for (its `intent` field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Status,
    Login,
    Transfer,
}

impl Intent {
    /// The handshake's `intent` value (1 status, 2 login, 3 transfer).
    pub fn from_wire(v: i32) -> Option<Self> {
        match v {
            1 => Some(Self::Status),
            2 => Some(Self::Login),
            3 => Some(Self::Transfer),
            _ => None,
        }
    }
    pub fn to_wire(self) -> i32 {
        match self {
            Self::Status => 1,
            Self::Login => 2,
            Self::Transfer => 3,
        }
    }
    /// Login and transfer connections are players; status pings are not.
    pub fn is_player(self) -> bool {
        !matches!(self, Self::Status)
    }
}

/// First frame of every data stream (relay to client).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamHeader {
    pub v: u16,
    /// The player's address as the relay saw it.
    pub peer: SocketAddr,
    /// The relay address the player connected to.
    pub relay: SocketAddr,
    /// The normalised hostname that routed the connection.
    pub host: String,
    pub intent: Intent,
    /// The protocol version from the handshake.
    pub protocol: i32,
}

/// Write one control frame.
pub async fn write_frame<W, T>(w: &mut W, msg: &T) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let body = serde_json::to_vec(msg).map_err(io::Error::other)?;
    if body.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "control frame too large",
        ));
    }
    let mut buf = Vec::with_capacity(4 + body.len());
    buf.extend_from_slice(&(body.len() as u32).to_be_bytes());
    buf.extend_from_slice(&body);
    w.write_all(&buf).await?;
    w.flush().await
}

/// Read one control frame. Not cancel-safe: run it in its own task or to completion.
pub async fn read_frame<R, T>(r: &mut R) -> io::Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len = [0u8; 4];
    r.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control frame too large",
        ));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).await?;
    serde_json::from_slice(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_round_trip() {
        let msgs = vec![
            RelayMsg::Welcome(Welcome {
                v: VERSION,
                session: "s".into(),
                server_id: "srv".into(),
                hostnames: vec!["a.example".into()],
                limits: Limits {
                    max_players: 10,
                    rate_bytes_per_s: 1,
                    daily_bytes: 2,
                    max_status: 3,
                },
                expires_at: 99,
                ping_every_s: 15,
            }),
            RelayMsg::Refused {
                code: ErrorCode::TokenExpired,
                message: "x".into(),
                retry_after_s: None,
            },
            RelayMsg::Drain { deadline_s: 5 },
        ];
        let mut buf = Vec::new();
        for m in &msgs {
            write_frame(&mut buf, m).await.unwrap();
        }
        let mut r = &buf[..];
        for m in &msgs {
            let got: RelayMsg = read_frame(&mut r).await.unwrap();
            assert_eq!(&got, m);
        }
    }

    #[test]
    fn json_shape_is_stable() {
        let hello = ClientMsg::Hello {
            v: 1,
            token: "t".into(),
            agent: "a".into(),
        };
        assert_eq!(
            serde_json::to_string(&hello).unwrap(),
            r#"{"op":"hello","v":1,"token":"t","agent":"a"}"#
        );
        let unknown: ErrorCode = serde_json::from_str(r#""something_new""#).unwrap();
        assert_eq!(unknown, ErrorCode::Unknown);
    }

    #[tokio::test]
    async fn oversized_frames_are_refused() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&((MAX_FRAME + 1) as u32).to_be_bytes());
        let mut r = &buf[..];
        let err = read_frame::<_, ClientMsg>(&mut r).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
