//! Making a local server reachable: this computer's addresses (LAN, VPNs, IPv6), a router port
//! mapping (UPnP IGD through `igd-next`, then NAT-PMP, RFC 6886), and a hook to test from outside
//! whether the public address answers. Horizon's Play with Friends does the same discovery for
//! its integrated server (horizon/docs/play-with-friends.md: LAN, VPN `100.x`, IPv6, PCP/NAT-PMP/UPnP).

use crate::error::{Result, ServerError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

/// A local address players may use.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalAddress {
    pub ip: IpAddr,
    /// `lan`, `vpn` (Tailscale/ZeroTier style ranges), `ipv6`.
    pub kind: String,
    pub interface: String,
}

fn is_cgnat(ip: Ipv4Addr) -> bool {
    // 100.64.0.0/10: Tailscale and carrier-grade NAT.
    ip.octets()[0] == 100 && (ip.octets()[1] & 0xC0) == 64
}

/// Classifies an address; `None` for loopback, link-local and unspecified ones.
pub fn classify(ip: IpAddr) -> Option<&'static str> {
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_loopback() || v4.is_link_local() || v4.is_unspecified() {
                None
            } else if is_cgnat(v4) {
                Some("vpn")
            } else {
                Some("lan")
            }
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments()[0];
            if v6.is_loopback() || v6.is_unspecified() || (seg & 0xffc0) == 0xfe80 {
                None
            } else {
                Some("ipv6")
            }
        }
    }
}

/// This computer's usable addresses. Container and VM bridges are left out by name.
pub fn local_addresses() -> Vec<LocalAddress> {
    let mut out = Vec::new();
    for iface in netdev::get_interfaces() {
        let label = iface
            .friendly_name
            .clone()
            .unwrap_or_else(|| iface.name.clone());
        let lower = label.to_lowercase();
        if [
            "docker",
            "veth",
            "vethernet",
            "virtualbox",
            "vmware",
            "br-",
            "hyper-v",
            "wsl",
        ]
        .iter()
        .any(|b| lower.contains(b))
        {
            continue;
        }
        for v4 in &iface.ipv4 {
            let ip = IpAddr::V4(v4.addr());
            if let Some(kind) = classify(ip) {
                out.push(LocalAddress {
                    ip,
                    kind: kind.into(),
                    interface: label.clone(),
                });
            }
        }
        for v6 in &iface.ipv6 {
            let ip = IpAddr::V6(v6.addr());
            if let Some(kind) = classify(ip) {
                out.push(LocalAddress {
                    ip,
                    kind: kind.into(),
                    interface: label.clone(),
                });
            }
        }
    }
    out.sort_by_key(|a| match a.kind.as_str() {
        "lan" => 0,
        "vpn" => 1,
        _ => 2,
    });
    out
}

/// The LAN IPv4 address used for the default route, when known.
pub fn primary_lan_ip() -> Option<Ipv4Addr> {
    let iface = netdev::get_default_interface().ok()?;
    iface
        .ipv4
        .iter()
        .map(|n| n.addr())
        .find(|ip| classify(IpAddr::V4(*ip)) == Some("lan"))
}

/// A router port mapping that was made.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortMapping {
    /// `upnp` or `nat-pmp`.
    pub method: String,
    pub external_ip: Option<IpAddr>,
    pub external_port: u16,
    pub internal: SocketAddr,
    /// Seconds; 0 permanent.
    pub lease_seconds: u32,
}

/// Lease asked for (one hour, renewed every half hour while the server runs, like Horizon).
pub const LEASE_SECONDS: u32 = 3600;

/// Asks the router for a TCP mapping of `port` to this computer: UPnP IGD first, then NAT-PMP.
pub async fn map_port(port: u16, description: &str) -> Result<PortMapping> {
    let local = primary_lan_ip()
        .ok_or_else(|| ServerError::Network("no LAN address on this computer".into()))?;
    let internal = SocketAddr::V4(SocketAddrV4::new(local, port));
    match map_upnp(internal, port, description).await {
        Ok(m) => return Ok(m),
        Err(e) => tracing::info!(error = %e, "UPnP mapping failed; trying NAT-PMP"),
    }
    let gateway = netdev::get_default_gateway()
        .ok()
        .and_then(|g| g.ipv4.first().copied())
        .ok_or_else(|| ServerError::Network("no default gateway".into()))?;
    nat_pmp_map(
        SocketAddrV4::new(gateway, NAT_PMP_PORT),
        port,
        port,
        LEASE_SECONDS,
    )
    .await
}

async fn map_upnp(internal: SocketAddr, port: u16, description: &str) -> Result<PortMapping> {
    let gateway = igd_next::aio::tokio::search_gateway(search_options())
        .await
        .map_err(|e| ServerError::Network(format!("UPnP: {e}")))?;
    let mut lease = LEASE_SECONDS;
    if let Err(e) = gateway
        .add_port(
            igd_next::PortMappingProtocol::TCP,
            port,
            internal,
            lease,
            description,
        )
        .await
    {
        // Some routers take only permanent mappings.
        tracing::debug!(error = %e, "timed UPnP mapping refused; asking for a permanent one");
        lease = 0;
        gateway
            .add_port(
                igd_next::PortMappingProtocol::TCP,
                port,
                internal,
                0,
                description,
            )
            .await
            .map_err(|e| ServerError::Network(format!("UPnP: {e}")))?;
    }
    let external_ip = gateway.get_external_ip().await.ok();
    Ok(PortMapping {
        method: "upnp".into(),
        external_ip,
        external_port: port,
        internal,
        lease_seconds: lease,
    })
}

/// Removes a mapping made by [`map_port`].
pub async fn unmap_port(mapping: &PortMapping) -> Result<()> {
    match mapping.method.as_str() {
        "upnp" => {
            let gateway = igd_next::aio::tokio::search_gateway(search_options())
                .await
                .map_err(|e| ServerError::Network(format!("UPnP: {e}")))?;
            gateway
                .remove_port(igd_next::PortMappingProtocol::TCP, mapping.external_port)
                .await
                .map_err(|e| ServerError::Network(format!("UPnP: {e}")))
        }
        _ => {
            let gateway = netdev::get_default_gateway()
                .ok()
                .and_then(|g| g.ipv4.first().copied())
                .ok_or_else(|| ServerError::Network("no default gateway".into()))?;
            // Lifetime 0 deletes the mapping (RFC 6886 section 3.4).
            nat_pmp_map(
                SocketAddrV4::new(gateway, NAT_PMP_PORT),
                mapping.internal.port(),
                0,
                0,
            )
            .await
            .map(|_| ())
        }
    }
}

pub const NAT_PMP_PORT: u16 = 5351;

fn search_options() -> igd_next::SearchOptions {
    let mut options = igd_next::SearchOptions::default();
    options.timeout = Some(Duration::from_secs(3));
    options
}

/// One NAT-PMP TCP mapping request (RFC 6886 section 3.3) to `gateway`, retried on the RFC's
/// 250 ms doubling schedule (shortened to 4 tries). Also asks the external address (3.2).
pub async fn nat_pmp_map(
    gateway: SocketAddrV4,
    internal_port: u16,
    external_port: u16,
    lifetime: u32,
) -> Result<PortMapping> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| ServerError::Network(format!("NAT-PMP: {e}")))?;
    socket
        .connect(gateway)
        .await
        .map_err(|e| ServerError::Network(format!("NAT-PMP: {e}")))?;
    let ask = |request: Vec<u8>, expect_op: u8, size: usize| {
        let socket = &socket;
        async move {
            let mut wait = Duration::from_millis(250);
            for _ in 0..4 {
                socket
                    .send(&request)
                    .await
                    .map_err(|e| ServerError::Network(format!("NAT-PMP: {e}")))?;
                let mut buf = [0u8; 16];
                match tokio::time::timeout(wait, socket.recv(&mut buf)).await {
                    Ok(Ok(n)) if n >= size && buf[0] == 0 && buf[1] == expect_op => {
                        let code = u16::from_be_bytes([buf[2], buf[3]]);
                        if code != 0 {
                            return Err(ServerError::Network(format!(
                                "NAT-PMP: the router refused (result {code})"
                            )));
                        }
                        return Ok(buf);
                    }
                    _ => wait *= 2,
                }
            }
            Err(ServerError::Network(
                "NAT-PMP: no answer from the router".into(),
            ))
        }
    };
    // Mapping request: version 0, opcode 2 (TCP), reserved, internal port, external port, lifetime.
    let mut req = vec![0u8, 2, 0, 0];
    req.extend_from_slice(&internal_port.to_be_bytes());
    req.extend_from_slice(&external_port.to_be_bytes());
    req.extend_from_slice(&lifetime.to_be_bytes());
    let buf = ask(req, 128 + 2, 16).await?;
    let mapped_port = u16::from_be_bytes([buf[10], buf[11]]);
    let lease = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);
    let external_ip = ask(vec![0u8, 0], 128, 12)
        .await
        .ok()
        .map(|b| IpAddr::V4(Ipv4Addr::new(b[8], b[9], b[10], b[11])));
    let local = primary_lan_ip().unwrap_or(Ipv4Addr::LOCALHOST);
    Ok(PortMapping {
        method: "nat-pmp".into(),
        external_ip,
        external_port: mapped_port,
        internal: SocketAddr::V4(SocketAddrV4::new(local, internal_port)),
        lease_seconds: lease,
    })
}

/// Whether a public address answers from outside (the computer can't test its own public address
/// reliably: many routers don't hairpin). Implemented by Rustic Cloud's checker when it exists.
#[async_trait]
pub trait ReachabilityProbe: Send + Sync {
    async fn check(&self, address: SocketAddr) -> Result<Reachability>;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Reachability {
    Reachable,
    Unreachable {
        reason: String,
    },
    /// No external checker configured.
    Unknown,
}

/// A checker at a URL: `GET <url>?host=<ip>&port=<port>` answering `{"reachable": bool, "reason"?}`
/// (the contract Rustic Cloud is asked to implement, HANDOFF.md).
pub struct HttpReachability {
    pub http: rusticmc_launcher_core::reqwest::Client,
    pub url: String,
}

#[async_trait]
impl ReachabilityProbe for HttpReachability {
    async fn check(&self, address: SocketAddr) -> Result<Reachability> {
        let v: serde_json::Value = self
            .http
            .get(&self.url)
            .query(&[
                ("host", address.ip().to_string()),
                ("port", address.port().to_string()),
            ])
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        Ok(if v["reachable"].as_bool() == Some(true) {
            Reachability::Reachable
        } else {
            Reachability::Unreachable {
                reason: v["reason"].as_str().unwrap_or("no answer").to_owned(),
            }
        })
    }
}

/// No external checker: always [`Reachability::Unknown`].
pub struct NoReachability;

#[async_trait]
impl ReachabilityProbe for NoReachability {
    async fn check(&self, _: SocketAddr) -> Result<Reachability> {
        Ok(Reachability::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_kinds() {
        assert_eq!(classify("192.168.1.20".parse().unwrap()), Some("lan"));
        assert_eq!(classify("100.101.102.103".parse().unwrap()), Some("vpn"));
        assert_eq!(classify("127.0.0.1".parse().unwrap()), None);
        assert_eq!(classify("169.254.3.4".parse().unwrap()), None);
        assert_eq!(classify("fe80::1".parse().unwrap()), None);
        assert_eq!(classify("2001:db8::5".parse().unwrap()), Some("ipv6"));
    }

    #[test]
    fn lists_addresses_without_panicking() {
        for a in local_addresses() {
            assert!(classify(a.ip).is_some());
        }
    }

    #[tokio::test]
    async fn nat_pmp_against_a_fake_router() {
        let router = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = match router.local_addr().unwrap() {
            SocketAddr::V4(a) => a,
            _ => unreachable!(),
        };
        tokio::spawn(async move {
            let mut buf = [0u8; 16];
            for _ in 0..2 {
                let (n, from) = router.recv_from(&mut buf).await.unwrap();
                if n == 12 && buf[1] == 2 {
                    let mut reply = vec![0u8, 130, 0, 0, 0, 0, 0, 1];
                    reply.extend_from_slice(&buf[4..6]); // internal port
                    reply.extend_from_slice(&40000u16.to_be_bytes()); // mapped external port
                    reply.extend_from_slice(&3600u32.to_be_bytes());
                    router.send_to(&reply, from).await.unwrap();
                } else if n == 2 {
                    let reply = [0u8, 128, 0, 0, 0, 0, 0, 1, 203, 0, 113, 5];
                    router.send_to(&reply, from).await.unwrap();
                }
            }
        });
        let m = nat_pmp_map(addr, 25565, 25565, 3600).await.unwrap();
        assert_eq!(m.method, "nat-pmp");
        assert_eq!(m.external_port, 40000);
        assert_eq!(m.lease_seconds, 3600);
        assert_eq!(m.external_ip, Some("203.0.113.5".parse().unwrap()));
    }

    #[tokio::test]
    async fn a_silent_router_is_reported() {
        let silent = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let SocketAddr::V4(addr) = silent.local_addr().unwrap() else {
            unreachable!()
        };
        let err = nat_pmp_map(addr, 1, 1, 60).await.unwrap_err();
        assert!(err.to_string().contains("no answer"));
    }
}
