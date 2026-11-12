//! Domains for servers (launcher/AGENTS.md "Hosting modes and domains"): the records a hostname
//! needs ([`required_records`]), checking them in public DNS ([`verify`], through a
//! [`DnsResolver`]: Hickory against 1.1.1.1 and 8.8.8.8, ADR 0054), and creating them through a
//! registrar's API ([`DnsProvider`]; [`crate::porkbun::PorkbunDns`] for the user's own Porkbun
//! account). Any other registrar: the same records, entered by hand (manual mode).
//!
//! Record shapes follow Rustic Cloud's wire type `DnsRecord` (rusticmc-api-client `types.rs`):
//! `fqdn`, `name` relative to the zone (`@` = apex), `value` (TXT content, CNAME target, SRV
//! `priority weight port target`), `ttl`, `purpose` (`verification`, `routing`,
//! `minecraft_srv`), `required`.

use crate::error::{Result, ServerError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// The game's default port: no SRV record is needed for it.
pub const DEFAULT_PORT: u16 = 25565;
/// TTL of records the launcher creates.
pub const TTL: u32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecordKind {
    A,
    Aaaa,
    /// Porkbun's ALIAS (a CNAME-like record allowed at the apex).
    Alias,
    Cname,
    Srv,
    Txt,
}

impl RecordKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Aaaa => "AAAA",
            Self::Alias => "ALIAS",
            Self::Cname => "CNAME",
            Self::Srv => "SRV",
            Self::Txt => "TXT",
        }
    }
}

/// One record to create.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsRecord {
    pub kind: RecordKind,
    pub fqdn: String,
    /// Relative to the zone; `@` for the apex.
    pub name: String,
    pub value: String,
    pub ttl: u32,
    pub purpose: String,
    pub required: bool,
}

/// How players reach the hostname.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum RoutingMode {
    /// Through Rustic Relay: CNAME to the relay hostname (SRV when apex or not port 25565).
    Relay { target: String, port: u16 },
    /// An SRV record only (`_minecraft._tcp.<name>`), e.g. at the apex.
    Srv { target: String, port: u16 },
    /// Straight to a machine (a VPS): A/AAAA records, SRV when the port isn't 25565.
    Direct { addresses: Vec<IpAddr>, port: u16 },
}

/// The TXT record proving ownership, as Rustic Cloud hands it out for a domain
/// (ARCHITECTURE.md section 7: `_rustic-challenge.<host> TXT "rustic-verify=<token>"`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verification {
    /// Relative to the zone (`_rustic-challenge.play`, ...).
    pub name: String,
    pub value: String,
}

impl Verification {
    /// The challenge record for `sub` (empty: the apex) with the cloud's `token`.
    pub fn challenge(sub: &str, token: &str) -> Self {
        let sub = normalize(sub);
        Self {
            name: if sub.is_empty() || sub == "@" {
                "_rustic-challenge".into()
            } else {
                format!("_rustic-challenge.{sub}")
            },
            value: format!("rustic-verify={token}"),
        }
    }
}

/// Where the ownership token comes from (Rustic Cloud's `AddDomain`; a trait until the client
/// exists).
#[async_trait]
pub trait OwnershipTokens: Send + Sync {
    async fn verification(&self, zone: &str, hostname: &str) -> Result<Verification>;
}

/// No cloud yet: the token must be passed in.
pub struct NoOwnershipTokens;

#[async_trait]
impl OwnershipTokens for NoOwnershipTokens {
    async fn verification(&self, _: &str, _: &str) -> Result<Verification> {
        Err(ServerError::Dns(
            "Rustic Cloud is not available yet to issue the ownership record".into(),
        ))
    }
}

fn normalize(name: &str) -> String {
    name.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn fqdn(zone: &str, relative: &str) -> String {
    if relative.is_empty() || relative == "@" {
        zone.to_owned()
    } else {
        format!("{relative}.{zone}")
    }
}

fn valid_label(l: &str) -> bool {
    !l.is_empty()
        && l.len() <= 63
        && !l.starts_with('-')
        && !l.ends_with('-')
        && l.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// The records `sub.zone` (or the apex when `sub` is empty) needs for `mode`, plus the ownership
/// TXT record when given.
pub fn required_records(
    zone: &str,
    sub: &str,
    mode: &RoutingMode,
    verification: Option<&Verification>,
) -> Result<Vec<DnsRecord>> {
    let zone = normalize(zone);
    let sub = normalize(sub);
    let sub = if sub == "@" { String::new() } else { sub };
    if !zone.contains('.') || !zone.split('.').all(valid_label) {
        return Err(ServerError::Dns(format!("{zone} is not a domain name")));
    }
    if !sub.is_empty() && !sub.split('.').all(valid_label) {
        return Err(ServerError::Dns(format!("{sub} is not a valid sub-domain")));
    }
    let name = if sub.is_empty() {
        "@".to_owned()
    } else {
        sub.clone()
    };
    let host = fqdn(&zone, &sub);
    let srv_name = if sub.is_empty() {
        "_minecraft._tcp".to_owned()
    } else {
        format!("_minecraft._tcp.{sub}")
    };
    let srv = |target: &str, port: u16, required: bool| DnsRecord {
        kind: RecordKind::Srv,
        fqdn: fqdn(&zone, &srv_name),
        name: srv_name.clone(),
        value: format!("0 5 {port} {}", normalize(target)),
        ttl: TTL,
        purpose: "minecraft_srv".into(),
        required,
    };
    let mut out = Vec::new();
    match mode {
        RoutingMode::Relay { target, port } => {
            let target = normalize(target);
            if sub.is_empty() {
                // An apex can't hold a CNAME: ALIAS where the registrar has it (Porkbun does),
                // the SRV form otherwise (ARCHITECTURE.md section 7).
                out.push(DnsRecord {
                    kind: RecordKind::Alias,
                    fqdn: host.clone(),
                    name: name.clone(),
                    value: target.clone(),
                    ttl: TTL,
                    purpose: "routing".into(),
                    required: false,
                });
                out.push(srv(&target, *port, false));
            } else {
                out.push(DnsRecord {
                    kind: RecordKind::Cname,
                    fqdn: host.clone(),
                    name: name.clone(),
                    value: target.clone(),
                    ttl: TTL,
                    purpose: "routing".into(),
                    required: *port == DEFAULT_PORT,
                });
                if *port != DEFAULT_PORT {
                    out.push(srv(&target, *port, true));
                }
            }
        }
        RoutingMode::Srv { target, port } => out.push(srv(target, *port, true)),
        RoutingMode::Direct { addresses, port } => {
            if addresses.is_empty() {
                return Err(ServerError::Dns("no address to point to".into()));
            }
            for ip in addresses {
                out.push(DnsRecord {
                    kind: if ip.is_ipv4() {
                        RecordKind::A
                    } else {
                        RecordKind::Aaaa
                    },
                    fqdn: host.clone(),
                    name: name.clone(),
                    value: ip.to_string(),
                    ttl: TTL,
                    purpose: "routing".into(),
                    required: true,
                });
            }
            if *port != DEFAULT_PORT {
                out.push(srv(&host, *port, true));
            }
        }
    }
    if let Some(v) = verification {
        let vname = normalize(&v.name);
        out.push(DnsRecord {
            kind: RecordKind::Txt,
            fqdn: fqdn(&zone, &vname),
            name: vname,
            value: v.value.clone(),
            ttl: TTL,
            purpose: "verification".into(),
            required: true,
        });
    }
    Ok(out)
}

/// One record's state in public DNS.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RecordStatus {
    Ok,
    Missing,
    WrongValue {
        found: Vec<String>,
        expected: String,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordCheck {
    pub record: DnsRecord,
    #[serde(flatten)]
    pub status: RecordStatus,
}

/// Looks records up. Values: A/AAAA the address, CNAME the target, TXT the joined strings, SRV
/// `priority weight port target`; names and targets lowercase without the final dot. A name
/// that doesn't exist is `Ok(vec![])`.
#[async_trait]
pub trait DnsResolver: Send + Sync {
    async fn lookup(
        &self,
        fqdn: &str,
        kind: RecordKind,
    ) -> std::result::Result<Vec<String>, String>;
}

fn same_value(kind: RecordKind, found: &str, expected: &str) -> bool {
    match kind {
        RecordKind::Txt => found == expected,
        RecordKind::A | RecordKind::Aaaa => found
            .parse::<IpAddr>()
            .ok()
            .zip(expected.parse::<IpAddr>().ok())
            .is_some_and(|(a, b)| a == b),
        _ => normalize(found) == normalize(expected),
    }
}

/// An ALIAS is invisible in DNS: the name answers the target's addresses. OK when the two sets
/// of A records meet.
async fn check_alias(r: &DnsRecord, resolver: &dyn DnsResolver) -> RecordStatus {
    let (mine, target) = (
        resolver.lookup(&r.fqdn, RecordKind::A).await,
        resolver.lookup(&r.value, RecordKind::A).await,
    );
    match (mine, target) {
        (Err(message), _) | (_, Err(message)) => RecordStatus::Error { message },
        (Ok(m), _) if m.is_empty() => RecordStatus::Missing,
        (Ok(m), Ok(t)) if m.iter().any(|a| t.contains(a)) => RecordStatus::Ok,
        (Ok(m), _) => RecordStatus::WrongValue {
            found: m,
            expected: format!("the addresses of {}", r.value),
        },
    }
}

/// Checks every record in public DNS.
pub async fn verify(records: &[DnsRecord], resolver: &dyn DnsResolver) -> Vec<RecordCheck> {
    let mut out = Vec::new();
    for r in records {
        if r.kind == RecordKind::Alias {
            out.push(RecordCheck {
                record: r.clone(),
                status: check_alias(r, resolver).await,
            });
            continue;
        }
        let status = match resolver.lookup(&r.fqdn, r.kind).await {
            Err(message) => RecordStatus::Error { message },
            Ok(found) if found.is_empty() => RecordStatus::Missing,
            Ok(found) if found.iter().any(|f| same_value(r.kind, f, &r.value)) => RecordStatus::Ok,
            Ok(found) => RecordStatus::WrongValue {
                found: found.into_iter().take(8).collect(),
                expected: r.value.clone(),
            },
        };
        out.push(RecordCheck {
            record: r.clone(),
            status,
        });
    }
    out
}

/// Whether the checks route players: every required record OK, and at least one routing record
/// (CNAME, ALIAS, SRV or A/AAAA) OK.
pub fn all_ok(checks: &[RecordCheck]) -> bool {
    let routing_ok = checks
        .iter()
        .filter(|c| c.record.purpose != "verification")
        .any(|c| c.status == RecordStatus::Ok);
    routing_ok
        && checks
            .iter()
            .filter(|c| c.record.required)
            .all(|c| c.status == RecordStatus::Ok)
}

/// Hickory against public resolvers (1.1.1.1, 8.8.8.8), not the system's, to bypass local caches.
pub struct PublicResolver {
    inner: hickory_resolver::TokioResolver,
}

/// The public resolvers checks ask, each on its own for propagation counts.
pub const PUBLIC_RESOLVERS: [&str; 3] = ["1.1.1.1", "8.8.8.8", "9.9.9.9"];

impl PublicResolver {
    pub fn new() -> Result<Self> {
        let ips: Vec<IpAddr> = PUBLIC_RESOLVERS
            .iter()
            .map(|ip| ip.parse().expect("ip"))
            .collect();
        Self::with_servers(&ips)
    }

    /// A resolver asking only `servers`.
    pub fn with_servers(servers: &[IpAddr]) -> Result<Self> {
        use hickory_resolver::config::{NameServerConfig, ResolverConfig};
        let servers = servers
            .iter()
            .map(|ip| NameServerConfig::udp_and_tcp(*ip))
            .collect();
        let config = ResolverConfig::from_name_servers(servers);
        let inner = hickory_resolver::Resolver::builder_with_config(
            config,
            hickory_resolver::net::runtime::TokioRuntimeProvider::default(),
        )
        .build()
        .map_err(|e| ServerError::Dns(e.to_string()))?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl DnsResolver for PublicResolver {
    async fn lookup(
        &self,
        fqdn: &str,
        kind: RecordKind,
    ) -> std::result::Result<Vec<String>, String> {
        use hickory_resolver::proto::rr::{RData, RecordType};
        let rtype = match kind {
            RecordKind::A => RecordType::A,
            RecordKind::Aaaa => RecordType::AAAA,
            RecordKind::Alias => RecordType::A,
            RecordKind::Cname => RecordType::CNAME,
            RecordKind::Srv => RecordType::SRV,
            RecordKind::Txt => RecordType::TXT,
        };
        let name = format!("{}.", normalize(fqdn));
        let lookup = match self.inner.lookup(name.as_str(), rtype).await {
            Ok(l) => l,
            Err(e) if e.is_no_records_found() || e.is_nx_domain() => return Ok(Vec::new()),
            Err(e) => return Err(e.to_string()),
        };
        let mut out = Vec::new();
        for record in lookup.answers() {
            let v = match &record.data {
                RData::A(a) => a.0.to_string(),
                RData::AAAA(a) => a.0.to_string(),
                RData::CNAME(c) if kind == RecordKind::Cname => normalize(&c.0.to_string()),
                RData::SRV(s) => format!(
                    "{} {} {} {}",
                    s.priority,
                    s.weight,
                    s.port,
                    normalize(&s.target.to_string())
                ),
                RData::TXT(t) => t
                    .txt_data
                    .iter()
                    .map(|d| String::from_utf8_lossy(d).into_owned())
                    .collect::<String>(),
                _ => continue,
            };
            out.push(v);
        }
        Ok(out)
    }
}

/// A registrar the launcher can write records to.
#[async_trait]
pub trait DnsProvider: Send + Sync {
    /// `porkbun`, `manual`...
    fn id(&self) -> &'static str;
    /// The zones the account manages.
    async fn list_domains(&self) -> Result<Vec<DomainInfo>>;
    /// Creates or updates `records` in `zone` (idempotent).
    async fn apply(&self, zone: &str, records: &[DnsRecord]) -> Result<ApplyReport>;
    /// Removes `records` (those that match by name and type).
    async fn remove(&self, zone: &str, records: &[DnsRecord]) -> Result<usize>;
}

/// A domain in a registrar account.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainInfo {
    pub domain: String,
    pub status: Option<String>,
    pub expire_date: Option<String>,
    /// Whether the registrar's API may change it (Porkbun: per-domain opt-in).
    pub api_access: bool,
}

/// What [`DnsProvider::apply`] did.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyReport {
    pub created: Vec<String>,
    pub updated: Vec<String>,
    pub unchanged: Vec<String>,
}

/// Configures a server's hostname end to end: the records, applied through `provider`.
pub async fn configure_server(
    provider: &dyn DnsProvider,
    zone: &str,
    sub: &str,
    mode: &RoutingMode,
    verification: Option<&Verification>,
) -> Result<(Vec<DnsRecord>, ApplyReport)> {
    let records = required_records(zone, sub, mode, verification)?;
    let report = provider.apply(zone, &records).await?;
    Ok((records, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn token() -> Verification {
        Verification::challenge("play", "abc123")
    }

    #[test]
    fn relay_on_a_subdomain() {
        let r = required_records(
            "Example.com.",
            "play",
            &RoutingMode::Relay {
                target: "k7m2.relay.rustic.gg.".into(),
                port: 25565,
            },
            Some(&token()),
        )
        .unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            (r[0].kind, r[0].fqdn.as_str(), r[0].value.as_str()),
            (
                RecordKind::Cname,
                "play.example.com",
                "k7m2.relay.rustic.gg"
            )
        );
        assert_eq!(
            (r[1].kind, r[1].fqdn.as_str()),
            (RecordKind::Txt, "_rustic-challenge.play.example.com")
        );
    }

    #[test]
    fn apex_and_other_ports_use_srv() {
        let r = required_records(
            "example.com",
            "",
            &RoutingMode::Relay {
                target: "r.rustic.gg".into(),
                port: 30000,
            },
            None,
        )
        .unwrap();
        assert_eq!(
            r.iter().map(|r| r.kind).collect::<Vec<_>>(),
            [RecordKind::Alias, RecordKind::Srv]
        );
        assert_eq!(r[1].name, "_minecraft._tcp");
        assert_eq!(r[1].value, "0 5 30000 r.rustic.gg");
        let r = required_records(
            "example.com",
            "mc",
            &RoutingMode::Relay {
                target: "r.rustic.gg".into(),
                port: 30000,
            },
            None,
        )
        .unwrap();
        assert_eq!(
            r.iter().map(|r| r.kind).collect::<Vec<_>>(),
            [RecordKind::Cname, RecordKind::Srv]
        );
        assert_eq!(r[1].fqdn, "_minecraft._tcp.mc.example.com");
    }

    #[test]
    fn a_vps_gets_address_records() {
        let r = required_records(
            "example.com",
            "survival",
            &RoutingMode::Direct {
                addresses: vec![
                    "203.0.113.7".parse().unwrap(),
                    "2001:db8::7".parse().unwrap(),
                ],
                port: 25565,
            },
            None,
        )
        .unwrap();
        assert_eq!(
            r.iter().map(|r| r.kind).collect::<Vec<_>>(),
            [RecordKind::A, RecordKind::Aaaa]
        );
        assert!(
            required_records(
                "not a domain",
                "",
                &RoutingMode::Srv {
                    target: "x.y".into(),
                    port: 1
                },
                None
            )
            .is_err()
        );
        assert!(
            required_records(
                "example.com",
                "bad name",
                &RoutingMode::Srv {
                    target: "x.y".into(),
                    port: 1
                },
                None
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn an_apex_alias_is_checked_by_its_addresses() {
        let records = required_records(
            "example.com",
            "",
            &RoutingMode::Relay {
                target: "r1.relay.rustic.gg".into(),
                port: 25565,
            },
            None,
        )
        .unwrap();
        let mut answers = HashMap::new();
        answers.insert(
            ("r1.relay.rustic.gg".to_owned(), RecordKind::A),
            Ok(vec!["198.51.100.7".to_owned()]),
        );
        answers.insert(
            ("example.com".to_owned(), RecordKind::A),
            Ok(vec!["198.51.100.7".to_owned()]),
        );
        let checks = verify(&records, &Fake(answers)).await;
        assert_eq!(checks[0].status, RecordStatus::Ok);
        assert!(all_ok(&checks), "the optional SRV may be missing");
    }

    struct Fake(HashMap<(String, RecordKind), std::result::Result<Vec<String>, String>>);

    #[async_trait]
    impl DnsResolver for Fake {
        async fn lookup(
            &self,
            fqdn: &str,
            kind: RecordKind,
        ) -> std::result::Result<Vec<String>, String> {
            self.0
                .get(&(fqdn.to_owned(), kind))
                .cloned()
                .unwrap_or(Ok(vec![]))
        }
    }

    #[tokio::test]
    async fn verification_per_record() {
        let records = required_records(
            "example.com",
            "play",
            &RoutingMode::Relay {
                target: "k7m2.relay.rustic.gg".into(),
                port: 25565,
            },
            Some(&token()),
        )
        .unwrap();
        let mut answers = HashMap::new();
        answers.insert(
            ("play.example.com".to_owned(), RecordKind::Cname),
            Ok(vec!["K7M2.relay.rustic.gg".to_owned()]),
        );
        let fake = Fake(answers);
        let checks = verify(&records, &fake).await;
        assert_eq!(
            checks[0].status,
            RecordStatus::Ok,
            "case and final dot don't matter"
        );
        assert_eq!(checks[1].status, RecordStatus::Missing);
        assert!(!all_ok(&checks));
        let mut answers = fake.0;
        answers.insert(
            (
                "_rustic-challenge.play.example.com".to_owned(),
                RecordKind::Txt,
            ),
            Ok(vec!["rustic-verify=old".to_owned()]),
        );
        let checks = verify(&records, &Fake(answers.clone())).await;
        assert!(
            matches!(&checks[1].status, RecordStatus::WrongValue { found, .. } if found == &["rustic-verify=old".to_owned()])
        );
        answers.insert(
            (
                "_rustic-challenge.play.example.com".to_owned(),
                RecordKind::Txt,
            ),
            Ok(vec!["rustic-verify=abc123".to_owned()]),
        );
        assert!(all_ok(&verify(&records, &Fake(answers.clone())).await));
        answers.insert(
            ("play.example.com".to_owned(), RecordKind::Cname),
            Err("SERVFAIL".to_owned()),
        );
        let checks = verify(&records, &Fake(answers)).await;
        assert!(matches!(checks[0].status, RecordStatus::Error { .. }));
    }
}
