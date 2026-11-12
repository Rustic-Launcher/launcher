//! PROXY protocol version 2 (HAProxy's `proxy-protocol.txt`, section 2.2): the binary header a
//! tunnel client may put in front of a player's bytes so the local server sees the player's address.
//!
//! Only for servers that read it: Horizon reads no PROXY header on its game port (it supports
//! Velocity modern forwarding only), so the option is off by default (docs/RELAY.md).
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

/// The 12-byte signature every v2 header starts with.
pub const SIGNATURE: [u8; 12] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];

/// Version 2, command PROXY.
const VERSION_PROXY: u8 = 0x21;
const TCP4: u8 = 0x11;
const TCP6: u8 = 0x21;

/// A v2 header for a TCP connection from `src` to `dst`. Mixed families are sent as IPv6, the
/// IPv4 side mapped (`::ffff:a.b.c.d`).
pub fn header(src: SocketAddr, dst: SocketAddr) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + 36);
    out.extend_from_slice(&SIGNATURE);
    out.push(VERSION_PROXY);
    match (src.ip(), dst.ip()) {
        (IpAddr::V4(s), IpAddr::V4(d)) => {
            out.push(TCP4);
            out.extend_from_slice(&12u16.to_be_bytes());
            out.extend_from_slice(&s.octets());
            out.extend_from_slice(&d.octets());
        }
        (s, d) => {
            out.push(TCP6);
            out.extend_from_slice(&36u16.to_be_bytes());
            out.extend_from_slice(&v6(s).octets());
            out.extend_from_slice(&v6(d).octets());
        }
    }
    out.extend_from_slice(&src.port().to_be_bytes());
    out.extend_from_slice(&dst.port().to_be_bytes());
    out
}

fn v6(ip: IpAddr) -> Ipv6Addr {
    match ip {
        IpAddr::V4(v4) => v4.to_ipv6_mapped(),
        IpAddr::V6(v6) => v6,
    }
}

/// Read a v2 PROXY/TCP header at the front of `buf`: source, destination and the header's length.
/// `None` when `buf` does not start with a complete, supported one.
pub fn parse(buf: &[u8]) -> Option<(SocketAddr, SocketAddr, usize)> {
    if buf.len() < 16 || buf[..12] != SIGNATURE || buf[12] != VERSION_PROXY {
        return None;
    }
    let len = usize::from(u16::from_be_bytes([buf[14], buf[15]]));
    let body = buf.get(16..16 + len)?;
    let port = |b: &[u8]| u16::from_be_bytes([b[0], b[1]]);
    let (src, dst) = match buf[13] {
        TCP4 if len >= 12 => {
            let s: [u8; 4] = body[0..4].try_into().ok()?;
            let d: [u8; 4] = body[4..8].try_into().ok()?;
            (
                SocketAddr::new(IpAddr::from(s), port(&body[8..])),
                SocketAddr::new(IpAddr::from(d), port(&body[10..])),
            )
        }
        TCP6 if len >= 36 => {
            let s: [u8; 16] = body[0..16].try_into().ok()?;
            let d: [u8; 16] = body[16..32].try_into().ok()?;
            (
                SocketAddr::new(IpAddr::from(s), port(&body[32..])),
                SocketAddr::new(IpAddr::from(d), port(&body[34..])),
            )
        }
        _ => return None,
    };
    Some((src, dst, 16 + len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_header_bytes() {
        let h = header(
            "203.0.113.7:51000".parse().unwrap(),
            "10.0.0.1:25565".parse().unwrap(),
        );
        assert_eq!(h.len(), 28);
        assert_eq!(&h[12..16], &[0x21, 0x11, 0x00, 0x0C]);
        assert_eq!(&h[16..20], &[203, 0, 113, 7]);
        assert_eq!(&h[24..26], &51000u16.to_be_bytes());
        let (s, d, n) = parse(&h).unwrap();
        assert_eq!(s, "203.0.113.7:51000".parse().unwrap());
        assert_eq!(d, "10.0.0.1:25565".parse().unwrap());
        assert_eq!(n, 28);
    }

    #[test]
    fn mixed_families_go_as_ipv6() {
        let src: SocketAddr = "[2001:db8::1]:4000".parse().unwrap();
        let dst: SocketAddr = "192.0.2.1:25565".parse().unwrap();
        let h = header(src, dst);
        assert_eq!(h.len(), 52);
        let (s, d, _) = parse(&h).unwrap();
        assert_eq!(s, src);
        assert_eq!(d.ip(), IpAddr::V6("::ffff:192.0.2.1".parse().unwrap()));
    }

    #[test]
    fn truncated_or_foreign_bytes_are_not_headers() {
        let h = header("1.2.3.4:1".parse().unwrap(), "5.6.7.8:2".parse().unwrap());
        assert!(parse(&h[..20]).is_none());
        assert!(parse(b"PROXY TCP4 1.2.3.4 5.6.7.8 1 2\r\n").is_none());
    }
}
