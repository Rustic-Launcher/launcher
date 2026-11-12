//! Domains: the user's own Porkbun account (connect, list, check, buy, configure) and any other
//! registrar by hand (required records + live verification), ADR 0014, ARCHITECTURE.md §7.

use crate::cmd::servers::{Place, place};
use crate::dto::*;
use crate::state::{AppState, Res, State};
use crate::util::iso;
use rusticmc_launcher_core::{IpcError, IpcErrorCode, Secret, fsutil, time};
use rusticmc_servers::dns::{
    self, DnsProvider, DnsRecord, OwnershipTokens, PublicResolver, RecordKind, RecordStatus,
    RoutingMode, Verification,
};
use rusticmc_servers::porkbun::PurchaseConfirmation;
use std::collections::BTreeMap;
use std::net::IpAddr;
use tauri::Emitter;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct PorkbunMeta {
    account: Option<String>,
    connected_at: Option<u64>,
}

fn meta_path(state: &AppState) -> std::path::PathBuf {
    state.paths.config_dir().join("porkbun.json")
}

fn connection(state: &AppState) -> DnsProviderConnection {
    let meta: PorkbunMeta = fsutil::read_json(&meta_path(state))
        .ok()
        .flatten()
        .unwrap_or_default();
    let connected = state.porkbun.connected();
    DnsProviderConnection {
        provider: "porkbun",
        connected,
        account_label: connected.then(|| state.porkbun.masked_key()).flatten(),
        connected_at: connected.then_some(meta.connected_at.map(iso)).flatten(),
    }
}

#[tauri::command]
pub fn dns_porkbun_status(state: State<'_>) -> DnsProviderConnection {
    connection(&state)
}

#[tauri::command]
pub async fn dns_connect_porkbun(
    state: State<'_>,
    args: ConnectPorkbunArgs,
) -> Res<DnsProviderConnection> {
    let account = state
        .porkbun
        .connect(
            Secret::new(args.api_key.trim()),
            Secret::new(args.secret_api_key.trim()),
        )
        .await?;
    fsutil::write_json(
        &meta_path(&state),
        &PorkbunMeta {
            account: Some(account),
            connected_at: Some(time::now()),
        },
    )?;
    Ok(connection(&state))
}

#[tauri::command]
pub fn dns_disconnect(state: State<'_>, args: ProviderArgs) -> Res<DnsProviderConnection> {
    if args.provider != "porkbun" {
        return Err(IpcError::invalid("unknown provider"));
    }
    state.porkbun.disconnect()?;
    let _ = std::fs::remove_file(meta_path(&state));
    Ok(connection(&state))
}

/// `2027-01-15 10:00:00` → ISO.
fn porkbun_date(d: &str) -> String {
    let n = |a: usize, b: usize| d.get(a..b).and_then(|s| s.parse::<u32>().ok());
    (|| {
        rusticmc_launcher_core::time::from_civil(
            i64::from(n(0, 4)?),
            n(5, 7)?,
            n(8, 10)?,
            n(11, 13).unwrap_or(0),
            n(14, 16).unwrap_or(0),
            n(17, 19).unwrap_or(0),
        )
    })()
    .map(iso)
    .unwrap_or_else(|| d.to_owned())
}

#[tauri::command]
pub async fn dns_list_domains(state: State<'_>, args: ProviderArgs) -> Res<Vec<OwnedDomain>> {
    if args.provider != "porkbun" {
        return Err(IpcError::invalid("unknown provider"));
    }
    Ok(state
        .porkbun
        .list_domains()
        .await?
        .into_iter()
        .map(|d| OwnedDomain {
            expires_at: d
                .expire_date
                .as_deref()
                .map(porkbun_date)
                .unwrap_or_default(),
            // listAll's autoRenew isn't kept by the provider's list: shown as on (Porkbun's default).
            auto_renew: true,
            api_access: d.api_access,
            domain: d.domain,
        })
        .collect())
}

/// `"9.73"` USD → 973 cents.
fn usd(price: &Option<String>) -> Option<Money> {
    let p = price.as_ref()?;
    let (whole, frac) = p.split_once('.').unwrap_or((p, "0"));
    let frac = format!("{frac:0<2}");
    let cents = whole.parse::<u64>().ok()? * 100 + frac.get(..2)?.parse::<u64>().ok()?;
    Some(Money {
        amount_minor: cents,
        currency: "USD".into(),
    })
}

#[tauri::command]
pub async fn dns_check_domain(state: State<'_>, args: QueryArgs) -> Res<Vec<DomainAvailability>> {
    let q = args.query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Err(IpcError::invalid("type a name"));
    }
    let names: Vec<String> = if q.contains('.') {
        vec![q]
    } else {
        let label = rusticmc_launcher_core::fsutil::slug(&q);
        ["com", "net", "gg", "io", "fr"]
            .iter()
            .map(|t| format!("{label}.{t}"))
            .collect()
    };
    let mut out = Vec::new();
    for n in names {
        match state.porkbun.check_domain(&n).await {
            Ok(a) => out.push(DomainAvailability {
                price: usd(&a.price),
                renewal_price: usd(&a.renewal_price),
                domain: a.domain,
                available: a.available && !a.buy_on_website,
                premium: a.premium,
            }),
            Err(e) if out.is_empty() => return Err(e.into()),
            Err(_) => {}
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn dns_buy_domain(state: State<'_>, args: BuyDomainRequest) -> Res<BuyDomainResult> {
    if !args.agree_registrar_terms || !args.authorize_order {
        return Err(IpcError::invalid("Tick both boxes to buy the domain."));
    }
    if args.expected_price.currency != "USD" {
        return Err(IpcError::invalid("Porkbun prices are in USD."));
    }
    let quote = state.porkbun.quote(&args.domain).await?;
    if args.years != 1 {
        return Err(IpcError::invalid(
            "Porkbun registers for the registry minimum (one year) through its API.",
        ));
    }
    if quote.cost_cents != args.expected_price.amount_minor {
        return Err(IpcError {
            code: IpcErrorCode::Conflict,
            message: format!(
                "The price changed: it is now {} (was {}).",
                quote.cost_display,
                args.expected_price.amount_minor as f64 / 100.0
            ),
            details: Some(
                serde_json::json!({ "price": { "amount_minor": quote.cost_cents, "currency": "USD" } }),
            ),
        });
    }
    if !quote.sufficient_funds {
        return Err(IpcError::with_reason(
            IpcErrorCode::Unavailable,
            format!(
                "Your Porkbun credit doesn't cover {}: add credit at {} and try again.",
                quote.cost_display,
                rusticmc_servers::porkbun::CREDIT_URL
            ),
            "dns.insufficient_funds",
        ));
    }
    let bought = state
        .porkbun
        .buy(&PurchaseConfirmation {
            quote_id: quote.quote_id.clone(),
            domain: quote.domain.clone(),
            cost_cents: args.expected_price.amount_minor,
            currency: args.expected_price.currency.clone(),
            agree_to_terms: true,
        })
        .await?;
    Ok(BuyDomainResult {
        order_id: bought.order_id.map(|n| n.to_string()).unwrap_or_default(),
        domain: bought.domain,
        charged: Money {
            amount_minor: bought.cost_cents,
            currency: "USD".into(),
        },
    })
}

fn record_dto(r: &DnsRecord) -> DnsRecordDto {
    let mut dto = DnsRecordDto {
        id: format!("{}:{}", r.kind.as_str(), r.fqdn),
        kind: r.kind.as_str().into(),
        host: r.name.clone(),
        fqdn: r.fqdn.clone(),
        value: r.value.clone(),
        ttl: r.ttl,
        priority: None,
        weight: None,
        port: None,
        purpose: match r.purpose.as_str() {
            "verification" => "ownership",
            "minecraft_srv" => "srv",
            _ => "routing",
        }
        .into(),
        optional: (!r.required).then_some(true),
    };
    if r.kind == RecordKind::Srv {
        let parts: Vec<&str> = r.value.split_whitespace().collect();
        if let [p, w, port, target] = parts.as_slice() {
            dto.priority = p.parse().ok();
            dto.weight = w.parse().ok();
            dto.port = port.parse().ok();
            dto.value = (*target).to_owned();
        }
    }
    dto
}

/// Where a server's hostname must point: the relay route, a router mapping, or a VPS address.
async fn routing(state: &AppState, server_id: &str) -> Res<(RoutingMode, &'static str)> {
    match place(server_id)? {
        Place::Local(id) => {
            let profile = state.servers.profile(&id)?;
            if let Some(host) = state.servers.relay_hostnames(&id).into_iter().next() {
                return Ok((
                    RoutingMode::Relay {
                        target: host,
                        port: 25565,
                    },
                    "local",
                ));
            }
            if let Some(m) = state.servers.port_mapping(&id)
                && let Some(ip) = m.external_ip
            {
                return Ok((
                    RoutingMode::Direct {
                        addresses: vec![ip],
                        port: m.external_port,
                    },
                    "local",
                ));
            }
            let _ = profile;
            Err(IpcError::with_reason(
                IpcErrorCode::Unavailable,
                "Turn on the Rustic Relay or the router port mapping (UPnP) for this server first: the domain needs somewhere to point.",
                "dns.no_target",
            ))
        }
        Place::Remote { profile, name } => {
            let host = state
                .registry
                .profile(&profile)
                .map(|p| p.target.host)
                .ok_or_else(|| IpcError::not_found("no such VPS"))?;
            let port = state.server_meta(&profile, &name).map_or(25565, |m| m.port);
            let addresses: Vec<IpAddr> = match host.parse::<IpAddr>() {
                Ok(ip) => vec![ip],
                Err(_) => tokio::net::lookup_host((host.as_str(), 22))
                    .await
                    .map_err(|e| IpcError::new(IpcErrorCode::Network, e.to_string()))?
                    .map(|a| a.ip())
                    .collect(),
            };
            Ok((RoutingMode::Direct { addresses, port }, "vps"))
        }
    }
}

/// The ownership record Rustic Cloud issues; `None` until the cloud client exists.
async fn verification(state: &AppState, zone: &str, host: &str) -> Option<Verification> {
    let tokens = rusticmc_servers::dns::NoOwnershipTokens;
    let _ = state;
    tokens.verification(zone, host).await.ok()
}

fn fqdn_of(domain: &str, sub: &Option<String>) -> String {
    match sub
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "@")
    {
        Some(s) => format!("{}.{}", s.to_ascii_lowercase(), domain.to_ascii_lowercase()),
        None => domain.to_ascii_lowercase(),
    }
}

async fn records_for(state: &AppState, t: &DomainTarget) -> Res<(Vec<DnsRecord>, &'static str)> {
    let (mode, hosting) = routing(state, &t.server_id).await?;
    let fqdn = fqdn_of(&t.domain, &t.subdomain);
    let v = verification(state, &t.domain, &fqdn).await;
    let records = dns::required_records(
        &t.domain,
        t.subdomain.as_deref().unwrap_or(""),
        &mode,
        v.as_ref(),
    )?;
    Ok((records, hosting))
}

#[tauri::command]
pub async fn dns_required_records(state: State<'_>, args: DomainTarget) -> Res<RequiredRecords> {
    let (records, hosting) = records_for(&state, &args).await?;
    Ok(RequiredRecords {
        fqdn: fqdn_of(&args.domain, &args.subdomain),
        hosting_mode: hosting,
        records: records.iter().map(record_dto).collect(),
    })
}

/// Checks each record against each public resolver apart, for propagation counts.
async fn verify(records: &[DnsRecord], fqdn: &str) -> Res<DnsVerification> {
    let mut per_resolver = Vec::new();
    for ip in dns::PUBLIC_RESOLVERS {
        let resolver = PublicResolver::with_servers(&[ip.parse().expect("ip")])?;
        per_resolver.push(dns::verify(records, &resolver).await);
    }
    let total = per_resolver.len();
    let mut out = Vec::new();
    let mut all = true;
    for (i, r) in records.iter().enumerate() {
        let checks: Vec<&RecordStatus> = per_resolver.iter().map(|c| &c[i].status).collect();
        let ok = checks.iter().filter(|s| ***s == RecordStatus::Ok).count();
        let mut found: Vec<String> = checks
            .iter()
            .flat_map(|s| match s {
                RecordStatus::WrongValue { found, .. } => found.clone(),
                _ => Vec::new(),
            })
            .collect();
        found.sort();
        found.dedup();
        if ok > 0 {
            found.insert(0, r.value.clone());
        }
        let status = if ok == total {
            "ok"
        } else if ok > 0 {
            "propagating"
        } else if checks
            .iter()
            .any(|s| matches!(s, RecordStatus::WrongValue { .. }))
        {
            "wrong_value"
        } else {
            "missing"
        };
        if r.required && status != "ok" {
            all = false;
        }
        out.push(RecordVerification {
            record_id: format!("{}:{}", r.kind.as_str(), r.fqdn),
            status,
            found,
            resolvers_ok: ok,
            resolvers_total: total,
        });
    }
    let routing_ok = out
        .iter()
        .zip(records)
        .any(|(v, r)| r.purpose != "verification" && v.status == "ok");
    Ok(DnsVerification {
        fqdn: fqdn.to_owned(),
        checked_at: iso(time::now()),
        all_ok: all && routing_ok,
        records: out,
    })
}

#[tauri::command]
pub async fn dns_verify(state: State<'_>, args: DomainTarget) -> Res<DnsVerification> {
    let (records, _) = records_for(&state, &args).await?;
    let fqdn = fqdn_of(&args.domain, &args.subdomain);
    let v = verify(&records, &fqdn).await?;
    // Keep the binding's status in step.
    let mut bindings = load_bindings(&state);
    if let Some(b) = bindings.get_mut(&args.server_id)
        && b.fqdn == fqdn
    {
        b.status = if v.all_ok {
            "verified".into()
        } else if v
            .records
            .iter()
            .any(|r| r.status == "propagating" || r.status == "ok")
        {
            "propagating".into()
        } else {
            "pending".into()
        };
        if v.all_ok && b.verified_at.is_none() {
            b.verified_at = Some(iso(time::now()));
        }
        let _ = state.app.emit("dns-status", b.clone());
        save_bindings(&state, &bindings)?;
    }
    Ok(v)
}

fn bindings_path(state: &AppState) -> std::path::PathBuf {
    state.paths.data().join("dns-bindings.json")
}

fn load_bindings(state: &AppState) -> BTreeMap<String, DomainBinding> {
    fsutil::read_json(&bindings_path(state))
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn save_bindings(state: &AppState, b: &BTreeMap<String, DomainBinding>) -> Res<()> {
    Ok(fsutil::write_json(&bindings_path(state), b)?)
}

#[tauri::command]
pub async fn dns_configure_server(state: State<'_>, args: DomainTarget) -> Res<DomainBinding> {
    let (records, _) = records_for(&state, &args).await?;
    let fqdn = fqdn_of(&args.domain, &args.subdomain);
    let porkbun = state.porkbun.connected()
        && state
            .porkbun
            .list_domains()
            .await
            .map(|d| {
                d.iter()
                    .any(|x| x.domain.eq_ignore_ascii_case(&args.domain))
            })
            .unwrap_or(false);
    let mut binding = DomainBinding {
        server_id: args.server_id.clone(),
        domain: args.domain.to_ascii_lowercase(),
        subdomain: args.subdomain.clone().filter(|s| !s.is_empty()),
        fqdn: fqdn.clone(),
        provider: if porkbun { "porkbun" } else { "manual" }.into(),
        status: "pending".into(),
        records: records.iter().map(record_dto).collect(),
        error: None,
        verified_at: None,
    };
    if porkbun {
        if let Err(e) = state.porkbun.apply(&args.domain, &records).await {
            binding.status = "error".into();
            binding.error = Some(e.to_string());
        } else {
            binding.status = "propagating".into();
        }
    }
    if !records.iter().any(|r| r.purpose == "verification") {
        binding.error.get_or_insert_with(|| {
            "The ownership record comes from Rustic Cloud, which isn't connected yet: routing records only.".into()
        });
    }
    if let Place::Local(id) = place(&args.server_id)? {
        let fqdn2 = fqdn.clone();
        let _ = state
            .servers
            .update_profile(&id, |p| p.hostname = Some(fqdn2));
    }
    let mut all = load_bindings(&state);
    all.insert(args.server_id.clone(), binding.clone());
    save_bindings(&state, &all)?;
    let _ = state.app.emit("dns-status", binding.clone());
    Ok(binding)
}

#[tauri::command]
pub fn dns_status(state: State<'_>, args: ServerIdArgs) -> Option<DomainBinding> {
    load_bindings(&state).remove(&args.server_id)
}

#[tauri::command]
pub async fn dns_remove_binding(state: State<'_>, args: ServerIdArgs) -> Res<()> {
    let mut all = load_bindings(&state);
    let Some(b) = all.remove(&args.server_id) else {
        return Ok(());
    };
    if b.provider == "porkbun" {
        let records: Vec<DnsRecord> = b
            .records
            .iter()
            .filter_map(|r| {
                let kind = match r.kind.as_str() {
                    "A" => RecordKind::A,
                    "AAAA" => RecordKind::Aaaa,
                    "CNAME" => RecordKind::Cname,
                    "SRV" => RecordKind::Srv,
                    "TXT" => RecordKind::Txt,
                    "ALIAS" => RecordKind::Alias,
                    _ => return None,
                };
                Some(DnsRecord {
                    kind,
                    fqdn: r.fqdn.clone(),
                    name: r.host.clone(),
                    value: r.value.clone(),
                    ttl: r.ttl,
                    purpose: r.purpose.clone(),
                    required: true,
                })
            })
            .collect();
        state.porkbun.remove(&b.domain, &records).await?;
    }
    if let Place::Local(id) = place(&args.server_id)? {
        let _ = state.servers.update_profile(&id, |p| p.hostname = None);
    }
    save_bindings(&state, &all)
}
