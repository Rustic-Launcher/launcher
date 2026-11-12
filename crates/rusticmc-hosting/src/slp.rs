//! The health check from the player's side: the game protocol's status request (the server list
//! ping), sent from this computer to the server's public address. If this answers, players can
//! reach the server: port open, firewall rule in place, game up.
//!
//! Wire format (vanilla, also what Horizon's server answers): a Handshake packet
//! (`0x00`, protocol VarInt, address String, port u16, next state VarInt 1), then a Status Request
//! (`0x00`, empty); the answer is a Status Response (`0x00`, String of JSON). Each packet is
//! prefixed by its length as a VarInt.

use crate::error::{HostingError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// What the server list would show.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingStatus {
    pub version: Option<String>,
    pub protocol: Option<i64>,
    pub online: Option<u64>,
    pub max: Option<u64>,
    pub motd: Option<String>,
    pub latency_ms: u64,
}

pub fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if v & !0x7f == 0 {
            out.push(v as u8);
            return;
        }
        out.push(((v & 0x7f) | 0x80) as u8);
        v >>= 7;
    }
}

pub async fn read_varint<R: AsyncRead + Unpin>(r: &mut R) -> Result<i32> {
    let mut value: u32 = 0;
    for i in 0..5 {
        let b = r.read_u8().await?;
        value |= u32::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(value as i32);
        }
    }
    Err(HostingError::Protocol("VarInt too long".into()))
}

fn packet(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 5);
    write_varint(&mut out, body.len() as i32);
    out.extend_from_slice(body);
    out
}

/// The two request packets.
pub fn request(host: &str, port: u16) -> Vec<u8> {
    let mut hs = Vec::new();
    write_varint(&mut hs, 0x00);
    // Any protocol number: servers answer status requests from every version.
    write_varint(&mut hs, -1);
    write_varint(&mut hs, host.len() as i32);
    hs.extend_from_slice(host.as_bytes());
    hs.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs, 1);
    let mut out = packet(&hs);
    out.extend_from_slice(&packet(&[0x00]));
    out
}

/// Read a Status Response and return its JSON.
pub async fn read_response<R: AsyncRead + Unpin>(r: &mut R) -> Result<serde_json::Value> {
    let len = read_varint(r).await?;
    if !(1..=1 << 20).contains(&len) {
        return Err(HostingError::Protocol(format!(
            "bad status packet length {len}"
        )));
    }
    let mut body = vec![0u8; len as usize];
    r.read_exact(&mut body).await?;
    let mut cur = std::io::Cursor::new(body);
    let id = read_varint(&mut cur).await?;
    if id != 0 {
        return Err(HostingError::Protocol(format!("unexpected packet {id:#x}")));
    }
    let slen = read_varint(&mut cur).await?;
    let start = cur.position() as usize;
    let body = cur.into_inner();
    let end = start + slen.max(0) as usize;
    if end > body.len() {
        return Err(HostingError::Protocol("truncated status JSON".into()));
    }
    Ok(serde_json::from_slice(&body[start..end])?)
}

fn motd_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(_) => {
            let mut s = v["text"].as_str().unwrap_or_default().to_owned();
            for extra in v["extra"].as_array().into_iter().flatten() {
                if let Some(t) = motd_text(extra) {
                    s.push_str(&t);
                }
            }
            Some(s)
        }
        _ => None,
    }
}

/// Ping `host:port` with a timeout.
pub async fn ping(host: &str, port: u16, timeout: Duration) -> Result<PingStatus> {
    let work = async {
        let mut s = TcpStream::connect((host, port)).await?;
        // The latency is the status round trip, not name resolution and connection set-up
        // (on Windows "localhost" tries ::1 first, refused only after about 2 s).
        let started = std::time::Instant::now();
        s.write_all(&request(host, port)).await?;
        let v = read_response(&mut s).await?;
        Ok::<_, HostingError>((v, started.elapsed()))
    };
    let (v, latency) = tokio::time::timeout(timeout, work)
        .await
        .map_err(|_| HostingError::Timeout(format!("no status answer from {host}:{port}")))??;
    Ok(PingStatus {
        version: v["version"]["name"].as_str().map(str::to_owned),
        protocol: v["version"]["protocol"].as_i64(),
        online: v["players"]["online"].as_u64(),
        max: v["players"]["max"].as_u64(),
        motd: motd_text(&v["description"]),
        latency_ms: latency.as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn varints() {
        let cases: [(i32, &[u8]); 5] = [
            (0, &[0]),
            (1, &[1]),
            (300, &[0xac, 0x02]),
            (25565, &[0xdd, 0xc7, 0x01]),
            (-1, &[0xff, 0xff, 0xff, 0xff, 0x0f]),
        ];
        for (v, bytes) in cases {
            let mut out = Vec::new();
            write_varint(&mut out, v);
            assert_eq!(out, bytes, "{v}");
        }
    }

    #[tokio::test]
    async fn varint_round_trip() {
        for v in [0, 127, 128, 2_097_151, i32::MAX, -1] {
            let mut out = Vec::new();
            write_varint(&mut out, v);
            let mut c = std::io::Cursor::new(out);
            assert_eq!(read_varint(&mut c).await.unwrap(), v);
        }
    }

    #[tokio::test]
    async fn latency_is_the_status_round_trip() {
        // Regression (QA on Windows): pinging "localhost" reported ~2000 ms because the time
        // spent on the refused ::1 attempt was counted. A server bound to IPv4 only:
        let port = fake_server().await;
        let Ok(st) = ping("localhost", port, Duration::from_secs(10)).await else {
            return; // no IPv4 "localhost" on this machine
        };
        assert!(st.latency_ms < 1000, "{} ms", st.latency_ms);
    }

    /// A one-shot status server on 127.0.0.1; returns its port.
    async fn fake_server() -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = l.accept().await.unwrap();
            // Handshake then status request.
            for _ in 0..2 {
                let len = read_varint(&mut s).await.unwrap();
                let mut b = vec![0; len as usize];
                s.read_exact(&mut b).await.unwrap();
            }
            let json = br#"{"version":{"name":"26.3","protocol":775},"players":{"max":20,"online":1},"description":{"text":"Rustic ","extra":[{"text":"server"}]}}"#;
            let mut body = vec![0x00];
            write_varint(&mut body, json.len() as i32);
            body.extend_from_slice(json);
            s.write_all(&packet(&body)).await.unwrap();
        });
        port
    }

    #[tokio::test]
    async fn pings_a_fake_server() {
        let port = fake_server().await;
        let st = ping("127.0.0.1", port, Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(st.version.as_deref(), Some("26.3"));
        assert_eq!(st.online, Some(1));
        assert_eq!(st.max, Some(20));
        assert_eq!(st.motd.as_deref(), Some("Rustic server"));
    }

    #[tokio::test]
    async fn closed_port_is_an_error() {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(
            ping("127.0.0.1", port, Duration::from_secs(2))
                .await
                .is_err()
        );
    }
}
