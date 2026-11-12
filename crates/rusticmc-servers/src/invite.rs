//! Invite codes for a local server, in the spirit of Horizon's Play with Friends
//! (horizon/docs/play-with-friends.md): the code itself says where the host is, in Crockford
//! base32 groups of five with a 16-bit checksum so a typo is caught before anything is dialled.
//!
//! Unlike Horizon's codes (an encrypted tunnel to an integrated server, with an 80-bit secret),
//! a Rustic invite points at a dedicated server speaking the plain game protocol, which checks
//! accounts itself in online mode: the code carries addresses only, no secret, and is not
//! compatible with Horizon's `friend:` codes (ADR 0052). The joining launcher tries every endpoint
//! at once and starts the game with `--quickPlayMultiplayer` on the first that answers.
//!
//! Layout: version (1), default port (u16), endpoints (tag; IPv4 4 bytes, IPv6 16 bytes or a
//! hostname length + bytes; tag bit 0x80: its own port follows), then the first two bytes of
//! SHA-256 of all that.

use crate::error::{Result, ServerError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

const VERSION: u8 = 1;
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Where a guest may reach the server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Endpoint {
    Ip(SocketAddr),
    /// A relay or custom hostname, with its port.
    Host(String, u16),
}

impl Endpoint {
    /// `HOST[:PORT]` for `--quickPlayMultiplayer`.
    pub fn address(&self) -> String {
        match self {
            Self::Ip(SocketAddr::V4(a)) => a.to_string(),
            Self::Ip(SocketAddr::V6(a)) => format!("[{}]:{}", a.ip(), a.port()),
            Self::Host(h, 25565) => h.clone(),
            Self::Host(h, p) => format!("{h}:{p}"),
        }
    }
}

/// A decoded invite.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invite {
    pub port: u16,
    pub endpoints: Vec<Endpoint>,
}

impl Invite {
    /// The code, `XXXXX-XXXXX-...`.
    pub fn encode(&self) -> Result<String> {
        let mut bytes = vec![VERSION];
        bytes.extend_from_slice(&self.port.to_be_bytes());
        for e in &self.endpoints {
            let (tag, body, port) = match e {
                Endpoint::Ip(SocketAddr::V4(a)) => (1u8, a.ip().octets().to_vec(), a.port()),
                Endpoint::Ip(SocketAddr::V6(a)) => (2u8, a.ip().octets().to_vec(), a.port()),
                Endpoint::Host(h, p) => {
                    if h.is_empty() || h.len() > 253 || !h.is_ascii() {
                        return Err(ServerError::Invalid(format!("bad host name {h}")));
                    }
                    let mut b = vec![h.len() as u8];
                    b.extend_from_slice(h.as_bytes());
                    (3u8, b, *p)
                }
            };
            let own_port = port != self.port;
            bytes.push(tag | if own_port { 0x80 } else { 0 });
            bytes.extend_from_slice(&body);
            if own_port {
                bytes.extend_from_slice(&port.to_be_bytes());
            }
        }
        let sum = Sha256::digest(&bytes);
        bytes.extend_from_slice(&sum[..2]);
        let text = base32(&bytes);
        Ok(text
            .as_bytes()
            .chunks(5)
            .map(|c| std::str::from_utf8(c).expect("ascii"))
            .collect::<Vec<_>>()
            .join("-"))
    }

    /// Reads a code (case, spaces and dashes ignored; `O` read as 0, `I`/`L` as 1).
    pub fn decode(code: &str) -> Result<Self> {
        let bad =
            |why: &str| ServerError::Invalid(format!("This invite code is not valid ({why})."));
        let bytes = unbase32(code).ok_or_else(|| bad("unexpected character"))?;
        if bytes.len() < 5 {
            return Err(bad("too short"));
        }
        let (body, sum) = bytes.split_at(bytes.len() - 2);
        if Sha256::digest(body)[..2] != *sum {
            return Err(bad("check the code for a typo"));
        }
        if body[0] != VERSION {
            return Err(bad("made by a newer launcher"));
        }
        let port = u16::from_be_bytes([body[1], body[2]]);
        let mut endpoints = Vec::new();
        let mut i = 3;
        let take = |i: &mut usize, n: usize| -> Result<&[u8]> {
            let s = body.get(*i..*i + n).ok_or_else(|| bad("truncated"))?;
            *i += n;
            Ok(s)
        };
        while i < body.len() {
            let tag = take(&mut i, 1)?[0];
            let own_port = tag & 0x80 != 0;
            let ip_or_host = match tag & 0x7f {
                1 => {
                    let b: [u8; 4] = take(&mut i, 4)?.try_into().expect("4");
                    Ok(IpAddr::from(b))
                }
                2 => {
                    let b: [u8; 16] = take(&mut i, 16)?.try_into().expect("16");
                    Ok(IpAddr::from(b))
                }
                3 => {
                    let n = take(&mut i, 1)?[0] as usize;
                    let h = std::str::from_utf8(take(&mut i, n)?).map_err(|_| bad("bad host"))?;
                    Err(h.to_owned())
                }
                _ => return Err(bad("unknown endpoint")),
            };
            let p = if own_port {
                let b = take(&mut i, 2)?;
                u16::from_be_bytes([b[0], b[1]])
            } else {
                port
            };
            endpoints.push(match ip_or_host {
                Ok(ip) => Endpoint::Ip(SocketAddr::new(ip, p)),
                Err(host) => Endpoint::Host(host, p),
            });
        }
        Ok(Self { port, endpoints })
    }

    /// The first endpoint that accepts a TCP connection (all tried at once), within `timeout`.
    pub async fn first_reachable(&self, timeout: Duration) -> Option<Endpoint> {
        let mut set = tokio::task::JoinSet::new();
        for e in self.endpoints.clone() {
            set.spawn(async move {
                let addr = e.address();
                let target = match &e {
                    Endpoint::Ip(a) => a.to_string(),
                    Endpoint::Host(h, p) => format!("{h}:{p}"),
                };
                let _ = addr;
                tokio::time::timeout(timeout, tokio::net::TcpStream::connect(target))
                    .await
                    .ok()?
                    .ok()
                    .map(|_| e)
            });
        }
        while let Some(r) = set.join_next().await {
            if let Ok(Some(e)) = r {
                set.abort_all();
                return Some(e);
            }
        }
        None
    }
}

fn base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let (mut buf, mut bits) = (0u32, 0u32);
    for &b in bytes {
        buf = (buf << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buf >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn unbase32(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in text.chars() {
        let c = match c.to_ascii_uppercase() {
            ' ' | '-' => continue,
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        };
        let v = ALPHABET.iter().position(|&a| a as char == c)? as u32;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invite() -> Invite {
        Invite {
            port: 25565,
            endpoints: vec![
                Endpoint::Ip("192.168.1.20:25565".parse().unwrap()),
                Endpoint::Ip("[2001:db8::5]:25565".parse().unwrap()),
                Endpoint::Ip("203.0.113.5:40000".parse().unwrap()),
                Endpoint::Host("k7m2q9xd.play.rustic.gg".into(), 25565),
            ],
        }
    }

    #[test]
    fn round_trip() {
        let code = invite().encode().unwrap();
        assert!(code.split('-').all(|g| g.len() <= 5));
        assert_eq!(Invite::decode(&code).unwrap(), invite());
        // Case, spaces and look-alike letters are forgiven.
        let sloppy = code
            .to_lowercase()
            .replace('-', " ")
            .replace('0', "o")
            .replace('1', "l");
        assert_eq!(Invite::decode(&sloppy).unwrap(), invite());
    }

    #[test]
    fn typos_are_caught() {
        let code = invite().encode().unwrap();
        let mut chars: Vec<char> = code.chars().collect();
        let i = chars.iter().position(|c| *c != '-').unwrap() + 3;
        chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
        let typo: String = chars.into_iter().collect();
        assert!(Invite::decode(&typo).is_err());
        assert!(Invite::decode("U").is_err());
    }

    #[test]
    fn addresses_for_quick_play() {
        assert_eq!(
            Endpoint::Host("play.example.com".into(), 25565).address(),
            "play.example.com"
        );
        assert_eq!(Endpoint::Host("h".into(), 1).address(), "h:1");
        assert_eq!(
            Endpoint::Ip("[::1]:5".parse().unwrap()).address(),
            "[::1]:5"
        );
    }

    #[tokio::test]
    async fn the_reachable_endpoint_wins() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let closed = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let invite = Invite {
            port,
            endpoints: vec![
                Endpoint::Ip(closed),
                Endpoint::Ip(SocketAddr::new("127.0.0.1".parse().unwrap(), port)),
            ],
        };
        let e = invite
            .first_reachable(Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(
            e,
            Endpoint::Ip(SocketAddr::new("127.0.0.1".parse().unwrap(), port))
        );
    }
}
