//! How friends reach a server running on this PC (ADR 0125):
//!
//! - **LAN**: the PC's local addresses ([`local_addresses`]), shown as `192.168.1.20:25565`.
//! - **Temporary relay hostname**: Rustic Relay exposes the local port at `<name>.<relay domain>`
//!   through an outbound tunnel. The relay engineer's `rusticmc-tunnel` implements [`RelayTunnel`].
//! - **Invite code**, in the spirit of Horizon's Play with Friends: Horizon's `horizon-friends`
//!   host side forwards *any* loopback TCP port behind an end-to-end encrypted invite code
//!   (`HostConfig.lan`), so it can front a dedicated server too; but only Horizon's official client
//!   (`horizon-official`, which registers the `friend:` resolver) can join with a code, vanilla
//!   clients cannot, at most 8 guests at once (`host::MAX_SESSIONS`), and every guest arrives from
//!   127.0.0.1 (IP bans can't work). An implementation of [`InviteSharer`] over `horizon-friends` is
//!   proposed in HANDOFF.md; until it lands, invite codes are reported as not available.

use crate::error::{HostingError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressKind {
    /// RFC 1918 private network: friends on the same Wi-Fi/LAN.
    Lan,
    /// A VPN/tailnet address (Tailscale's CGNAT range 100.64.0.0/10; ZeroTier often uses 10.x
    /// but its interface name tells it).
    Vpn,
    /// A public address on the interface itself (rare at home).
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAddress {
    pub ip: Ipv4Addr,
    pub interface: String,
    pub kind: AddressKind,
}

/// Interfaces that are not a network friends are on: container and VM bridges.
fn is_virtual_bridge(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "docker", "br-", "veth", "virbr", "vboxnet", "vmnet", "lxc", "cni", "flannel",
    ]
    .iter()
    .any(|p| n.starts_with(p))
        || n.contains("wsl")
        || n.contains("hyper-v")
        || n.contains("default switch")
}

fn is_vpn_interface(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["zt", "wg", "utun", "tun"].iter().any(|p| n.starts_with(p))
        || ["tailscale", "zerotier", "nordlynx", "wireguard"]
            .iter()
            .any(|p| n.contains(p))
}

/// Classify one IPv4 address on an interface; `None` for addresses friends can't use.
pub fn classify(ip: Ipv4Addr, interface: &str) -> Option<AddressKind> {
    if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() || ip.is_multicast() {
        return None;
    }
    if is_virtual_bridge(interface) {
        return None;
    }
    let o = ip.octets();
    let cgnat = o[0] == 100 && (64..128).contains(&o[1]);
    if cgnat || is_vpn_interface(interface) {
        return Some(AddressKind::Vpn);
    }
    if ip.is_private() {
        return Some(AddressKind::Lan);
    }
    Some(AddressKind::Public)
}

/// This PC's usable IPv4 addresses, LAN first.
pub fn local_addresses() -> Result<Vec<LocalAddress>> {
    let mut out: Vec<LocalAddress> = if_addrs::get_if_addrs()?
        .into_iter()
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(ip) => classify(ip, &i.name).map(|kind| LocalAddress {
                ip,
                interface: i.name.clone(),
                kind,
            }),
            std::net::IpAddr::V6(_) => None,
        })
        .collect();
    out.sort_by_key(|a| (a.kind != AddressKind::Lan, a.kind != AddressKind::Vpn, a.ip));
    out.dedup_by_key(|a| a.ip);
    Ok(out)
}

/// A temporary public hostname on Rustic Relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayLease {
    /// `<name>.<relay domain>`: what players type.
    pub hostname: String,
    pub port: u16,
    /// Seconds since 1970; the launcher renews before it.
    pub expires_at: u64,
}

/// Implemented by `rusticmc-tunnel` (relay engineer).
#[async_trait]
pub trait RelayTunnel: Send + Sync {
    /// Expose `127.0.0.1:local_port` until `close` (or the launcher exits).
    async fn open(&self, local_port: u16, preferred_name: Option<&str>) -> Result<RelayLease>;
    async fn close(&self, hostname: &str) -> Result<()>;
}

/// An invite code for a server (Play with Friends style).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    /// The full code (Crockford base32 in groups of five).
    pub code: String,
    /// A relay's short code (`K7QM-3XPD`), when one is configured.
    pub short_code: Option<String>,
    /// Who can join and what to do if friends on the internet can't (Horizon's verdict text).
    pub summary: String,
}

/// Implemented over `horizon-friends`' `Host` (see the module docs and HANDOFF.md).
#[async_trait]
pub trait InviteSharer: Send + Sync {
    async fn share(&self, local_port: u16) -> Result<Invite>;
    async fn stop(&self, local_port: u16) -> Result<()>;
}

/// How a local server is shared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum AccessMode {
    Lan,
    Relay { preferred_name: Option<String> },
    Invite,
}

/// What the UI shows for a shared server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum AccessInfo {
    Lan {
        addresses: Vec<String>,
        note: String,
    },
    Relay(RelayLease),
    Invite(Invite),
}

/// The glue the IPC layer calls (`hosting_local_access`).
#[derive(Default, Clone)]
pub struct AccessManager {
    relay: Option<Arc<dyn RelayTunnel>>,
    invites: Option<Arc<dyn InviteSharer>>,
}

impl AccessManager {
    pub fn new(
        relay: Option<Arc<dyn RelayTunnel>>,
        invites: Option<Arc<dyn InviteSharer>>,
    ) -> Self {
        Self { relay, invites }
    }

    pub async fn open(&self, port: u16, mode: &AccessMode) -> Result<AccessInfo> {
        match mode {
            AccessMode::Lan => {
                let addrs = local_addresses()?;
                Ok(AccessInfo::Lan {
                    addresses: addrs.iter().map(|a| format!("{}:{port}", a.ip)).collect(),
                    note: format!(
                        "Friends on the same network join at one of these addresses. Windows may ask to allow the server through its firewall (private networks) for TCP {port}."
                    ),
                })
            }
            AccessMode::Relay { preferred_name } => {
                let relay = self.relay.as_ref().ok_or_else(|| {
                    HostingError::Unsupported(
                        "Rustic Relay is not wired yet (rusticmc-tunnel)".into(),
                    )
                })?;
                Ok(AccessInfo::Relay(
                    relay.open(port, preferred_name.as_deref()).await?,
                ))
            }
            AccessMode::Invite => {
                let sharer = self.invites.as_ref().ok_or_else(|| HostingError::NotAvailableYet {
                    provider: "Invite codes",
                    detail: "invite codes for servers need Horizon's horizon-friends host (see HOSTING.md); use the LAN address or a relay hostname meanwhile".into(),
                })?;
                Ok(AccessInfo::Invite(sharer.share(port).await?))
            }
        }
    }

    pub async fn close(&self, port: u16, info: &AccessInfo) -> Result<()> {
        match info {
            AccessInfo::Lan { .. } => Ok(()),
            AccessInfo::Relay(lease) => match &self.relay {
                Some(r) => r.close(&lease.hostname).await,
                None => Ok(()),
            },
            AccessInfo::Invite(_) => match &self.invites {
                Some(s) => s.stop(port).await,
                None => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_addresses() {
        let ip = |s: &str| s.parse::<Ipv4Addr>().unwrap();
        assert_eq!(
            classify(ip("192.168.1.20"), "Ethernet"),
            Some(AddressKind::Lan)
        );
        assert_eq!(classify(ip("10.0.0.5"), "wlan0"), Some(AddressKind::Lan));
        assert_eq!(
            classify(ip("100.101.102.103"), "Tailscale"),
            Some(AddressKind::Vpn)
        );
        assert_eq!(
            classify(ip("10.147.17.4"), "ztabcdef"),
            Some(AddressKind::Vpn)
        );
        assert_eq!(
            classify(ip("203.0.113.5"), "eth0"),
            Some(AddressKind::Public)
        );
        assert_eq!(classify(ip("127.0.0.1"), "lo"), None);
        assert_eq!(classify(ip("169.254.10.1"), "Ethernet 2"), None);
        assert_eq!(classify(ip("172.17.0.1"), "docker0"), None);
        assert_eq!(
            classify(ip("172.28.0.1"), "vEthernet (WSL (Hyper-V firewall))"),
            None
        );
    }

    #[test]
    fn lists_this_machine() {
        // Whatever the machine has: no loopback, no duplicates.
        let a = local_addresses().unwrap();
        assert!(a.iter().all(|x| !x.ip.is_loopback()));
        let mut ips: Vec<_> = a.iter().map(|x| x.ip).collect();
        ips.dedup();
        assert_eq!(ips.len(), a.len());
    }

    struct FakeRelay;
    #[async_trait]
    impl RelayTunnel for FakeRelay {
        async fn open(&self, port: u16, name: Option<&str>) -> Result<RelayLease> {
            Ok(RelayLease {
                hostname: format!("{}.play.example", name.unwrap_or("x")),
                port,
                expires_at: 1,
            })
        }
        async fn close(&self, _: &str) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn modes() {
        let m = AccessManager::default();
        match m.open(25565, &AccessMode::Lan).await.unwrap() {
            AccessInfo::Lan { addresses, .. } => {
                assert!(addresses.iter().all(|a| a.ends_with(":25565")))
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            m.open(25565, &AccessMode::Invite).await.unwrap_err().code(),
            "not_available_yet"
        );
        assert_eq!(
            m.open(
                25565,
                &AccessMode::Relay {
                    preferred_name: None
                }
            )
            .await
            .unwrap_err()
            .code(),
            "unsupported"
        );
        let m = AccessManager::new(Some(Arc::new(FakeRelay)), None);
        let info = m
            .open(
                25565,
                &AccessMode::Relay {
                    preferred_name: Some("alex".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            info,
            AccessInfo::Relay(RelayLease {
                hostname: "alex.play.example".into(),
                port: 25565,
                expires_at: 1
            })
        );
        m.close(25565, &info).await.unwrap();
    }
}
