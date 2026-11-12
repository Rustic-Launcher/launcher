//! The user's own Porkbun account (Porkbun API v3, https://porkbun.com/api/json/v3/documentation
//! and its OpenAPI spec, read 2026-11-10):
//!
//! - Base `https://api.porkbun.com/api/json/v3`; JSON bodies carry `apikey` and `secretapikey`.
//! - `POST /ping` → `credentialsValid`, `account`, `scopes`.
//! - `POST /domain/listAll` (`start`, 1000 per page) → `domains[]` with `domain`, `status`,
//!   `expireDate`, `apiAccess` (1/0: per-domain API opt-in).
//! - `POST /dns/retrieve/{domain}` → `records[]` with `id`, `name` (fully qualified), `type`,
//!   `content`, `ttl`, `prio` (strings), `notes`.
//! - `POST /dns/create/{domain}` (`name` relative, `type`, `content`, `ttl`, `prio`, `notes`) →
//!   `id` (a number); `POST /dns/edit/{domain}/{id}`; `POST /dns/delete/{domain}/{id}`.
//! - `POST /domain/checkDomain/{domain}` → `response.avail`, `price` (USD), `regularPrice`,
//!   `premium`, `firstYearPromo`, `minDuration`, `additional.renewal.price`.
//! - `POST /domain/create/{domain}` with `cost` (cents, the current price), `agreeToTerms`
//!   (`"yes"`), `dryRun` (a quote with `cost: 0`): paid from account credit (`INSUFFICIENT_FUNDS`
//!   otherwise). Premium domains can't be bought through the API; an order is at most $100.
//! - Errors: `{"status": "ERROR", "code", "message"}`.
//!
//! The keys live only in the OS keychain (item `porkbun`), never in a file or a log.
//! SRV records: the spec documents `prio` as the SRV priority but not the content's layout; the
//! launcher sends `weight port target` as content with `prio` apart (Porkbun's DNS editor
//! layout), see RISKS.md.

use crate::dns::{ApplyReport, DnsProvider, DnsRecord, DomainInfo, RecordKind};
use crate::error::{Result, ServerError};
use async_trait::async_trait;
use rusticmc_auth::SecretStore;
use rusticmc_launcher_core::{Secret, reqwest, time};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const KEY: &str = "porkbun";
/// A quote stays valid this long.
const QUOTE_SECONDS: u64 = 600;
/// Where Porkbun account credit is added.
pub const CREDIT_URL: &str = "https://porkbun.com/account/credit";

#[derive(Clone, Serialize, Deserialize)]
struct Keys {
    api_key: Secret,
    secret_key: Secret,
}

/// A record as Porkbun keeps it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PorkbunRecord {
    pub id: String,
    /// Fully qualified.
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub content: String,
    pub ttl: Option<String>,
    pub prio: Option<String>,
    pub notes: Option<String>,
}

/// Availability and price of a domain.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainAvailability {
    pub domain: String,
    pub available: bool,
    pub premium: bool,
    /// USD, as Porkbun writes it (`"9.73"`).
    pub price: Option<String>,
    pub regular_price: Option<String>,
    pub first_year_promo: bool,
    pub renewal_price: Option<String>,
    pub min_duration_years: Option<u64>,
    /// Premium names can't be bought through the API: buy them on porkbun.com.
    pub buy_on_website: bool,
}

/// A dry-run purchase quote the UI shows before asking for confirmation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PurchaseQuote {
    pub quote_id: String,
    pub domain: String,
    pub cost_cents: u64,
    pub currency: String,
    pub cost_display: String,
    pub balance_cents: Option<u64>,
    pub sufficient_funds: bool,
    pub would_succeed: bool,
    pub message: Option<String>,
    pub expires_at: u64,
}

/// What the player confirmed in the UI: the exact price and currency shown, and the terms.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PurchaseConfirmation {
    pub quote_id: String,
    pub domain: String,
    pub cost_cents: u64,
    pub currency: String,
    /// Porkbun's registration agreement and auto-renewal terms, accepted explicitly.
    pub agree_to_terms: bool,
}

/// A completed purchase.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Purchase {
    pub domain: String,
    pub order_id: Option<u64>,
    pub cost_cents: u64,
    pub balance_cents: Option<u64>,
}

/// The Porkbun provider.
pub struct PorkbunDns {
    http: reqwest::Client,
    base: String,
    store: Arc<dyn SecretStore>,
    quotes: Mutex<HashMap<String, PurchaseQuote>>,
}

fn as_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn opt_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

impl PorkbunDns {
    pub fn new(
        http: reqwest::Client,
        base: impl Into<String>,
        store: Arc<dyn SecretStore>,
    ) -> Self {
        Self {
            http,
            base: base.into().trim_end_matches('/').to_owned(),
            store,
            quotes: Mutex::new(HashMap::new()),
        }
    }

    fn keys(&self) -> Result<Keys> {
        let text = self
            .store
            .get(KEY)?
            .ok_or_else(|| ServerError::Porkbun("Connect your Porkbun account first.".into()))?;
        serde_json::from_str(&text)
            .map_err(|_| ServerError::Porkbun("Reconnect your Porkbun account.".into()))
    }

    pub fn connected(&self) -> bool {
        self.keys().is_ok()
    }

    /// The API key with all but its prefix and last four characters hidden (`pk1_…3f9a`).
    pub fn masked_key(&self) -> Option<String> {
        let keys = self.keys().ok()?;
        let k = keys.api_key.expose();
        let tail: String = k
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let head: String = k.chars().take(4).collect();
        Some(format!("{head}…{tail}"))
    }

    async fn post_with(&self, keys: &Keys, path: &str, body: Value) -> Result<Value> {
        self.post_full(keys, path, body, None).await
    }

    async fn post_full(
        &self,
        keys: &Keys,
        path: &str,
        mut body: Value,
        idempotency_key: Option<&str>,
    ) -> Result<Value> {
        body["apikey"] = json!(keys.api_key.expose());
        body["secretapikey"] = json!(keys.secret_key.expose());
        // Logged by path only: the body holds the keys.
        tracing::debug!(endpoint = %path.split('/').take(3).collect::<Vec<_>>().join("/"), "porkbun");
        let mut request = self.http.post(format!("{}{path}", self.base)).json(&body);
        if let Some(key) = idempotency_key {
            request = request.header("Idempotency-Key", key);
        }
        let resp = request.send().await.map_err(|e| {
            ServerError::Porkbun(format!("could not reach Porkbun: {}", e.without_url()))
        })?;
        let status = resp.status().as_u16();
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        if v["status"] == "SUCCESS" && (200..300).contains(&status) {
            return Ok(v);
        }
        let code = v["code"].as_str().unwrap_or("");
        let message = v["message"]
            .as_str()
            .unwrap_or("Porkbun refused the request");
        Err(ServerError::Porkbun(match code {
            "API_ACCESS_DISABLED" => format!(
                "{message} Turn on API access for this domain at porkbun.com (Domain Management > API Access)."
            ),
            "INSUFFICIENT_FUNDS" => {
                format!("{message} Add credit at {CREDIT_URL}, then try again.")
            }
            "" => format!("{message} (HTTP {status})"),
            code => format!("{message} ({code})"),
        }))
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let keys = self.keys()?;
        self.post_with(&keys, path, body).await
    }

    /// Checks the keys with `/ping` and keeps them in the keychain. Returns the account name.
    pub async fn connect(&self, api_key: Secret, secret_key: Secret) -> Result<String> {
        let keys = Keys {
            api_key,
            secret_key,
        };
        let v = self.post_with(&keys, "/ping", json!({})).await?;
        if v["credentialsValid"] != true {
            return Err(ServerError::Porkbun(
                "Porkbun did not accept these API keys.".into(),
            ));
        }
        let text = serde_json::to_string(&keys).map_err(|e| ServerError::Porkbun(e.to_string()))?;
        self.store.set(KEY, &text)?;
        Ok(v["account"].as_str().unwrap_or_default().to_owned())
    }

    pub fn disconnect(&self) -> Result<()> {
        Ok(self.store.delete(KEY)?)
    }

    /// The records of a domain.
    pub async fn records(&self, domain: &str) -> Result<Vec<PorkbunRecord>> {
        let v = self
            .post(&format!("/dns/retrieve/{domain}"), json!({}))
            .await?;
        Ok(v["records"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|r| PorkbunRecord {
                        id: opt_string(&r["id"]).unwrap_or_default(),
                        name: r["name"].as_str().unwrap_or_default().to_ascii_lowercase(),
                        kind: r["type"].as_str().unwrap_or_default().to_owned(),
                        content: r["content"].as_str().unwrap_or_default().to_owned(),
                        ttl: opt_string(&r["ttl"]),
                        prio: opt_string(&r["prio"]),
                        notes: opt_string(&r["notes"]),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Porkbun's `(content, prio)` for one of our records.
    fn content_of(record: &DnsRecord) -> (String, Option<u64>) {
        if record.kind == RecordKind::Srv {
            let mut parts = record.value.splitn(2, ' ');
            let prio = parts.next().and_then(|p| p.parse().ok());
            (parts.next().unwrap_or_default().to_owned(), prio)
        } else {
            (record.value.clone(), None)
        }
    }

    fn body_of(record: &DnsRecord) -> Value {
        let (content, prio) = Self::content_of(record);
        let mut body = json!({
            "name": if record.name == "@" { "" } else { record.name.as_str() },
            "type": record.kind.as_str(),
            "content": content,
            "ttl": record.ttl,
            "notes": "Rustic Launcher",
        });
        if let Some(p) = prio {
            body["prio"] = json!(p);
        }
        body
    }

    /// Availability and price (`/domain/checkDomain/{domain}`).
    pub async fn check_domain(&self, domain: &str) -> Result<DomainAvailability> {
        let domain = domain.trim().to_ascii_lowercase();
        let v = self
            .post(&format!("/domain/checkDomain/{domain}"), json!({}))
            .await?;
        let r = &v["response"];
        let premium = r["premium"] == "yes";
        Ok(DomainAvailability {
            available: r["avail"] == "yes",
            premium,
            price: opt_string(&r["price"]),
            regular_price: opt_string(&r["regularPrice"]),
            first_year_promo: r["firstYearPromo"] == "yes",
            renewal_price: opt_string(&r["additional"]["renewal"]["price"]),
            min_duration_years: as_u64(&r["minDuration"]),
            buy_on_website: premium,
            domain,
        })
    }

    /// A dry-run registration: the exact cost in cents and whether the account's credit covers
    /// it. Nothing is bought.
    pub async fn quote(&self, domain: &str) -> Result<PurchaseQuote> {
        let domain = domain.trim().to_ascii_lowercase();
        let v = self
            .post(
                &format!("/domain/create/{domain}"),
                json!({ "cost": 0, "agreeToTerms": "yes", "dryRun": true }),
            )
            .await?;
        let cost = as_u64(&v["cost"])
            .ok_or_else(|| ServerError::Porkbun("the quote has no cost".into()))?;
        let quote = PurchaseQuote {
            quote_id: uuid::Uuid::new_v4().to_string(),
            domain,
            cost_cents: cost,
            currency: "USD".into(),
            cost_display: v["costDisplay"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("${}.{:02}", cost / 100, cost % 100)),
            balance_cents: as_u64(&v["balance"]),
            sufficient_funds: v["sufficientFunds"].as_bool().unwrap_or(false),
            would_succeed: v["wouldSucceed"].as_bool().unwrap_or(false),
            message: v["message"].as_str().map(str::to_owned),
            expires_at: time::now() + QUOTE_SECONDS,
        };
        self.quotes
            .lock()
            .expect("quotes")
            .insert(quote.quote_id.clone(), quote.clone());
        Ok(quote)
    }

    /// Buys a domain, only with a confirmation matching a fresh quote exactly (domain, price in
    /// cents, currency) and the terms accepted. Paid from the Porkbun account's credit.
    pub async fn buy(&self, confirmation: &PurchaseConfirmation) -> Result<Purchase> {
        let quote = self
            .quotes
            .lock()
            .expect("quotes")
            .get(&confirmation.quote_id)
            .cloned()
            .ok_or_else(|| {
                ServerError::PurchaseNotConfirmed(
                    "unknown or used quote: check the price again".into(),
                )
            })?;
        if !confirmation.agree_to_terms {
            return Err(ServerError::PurchaseNotConfirmed(
                "the registration terms were not accepted".into(),
            ));
        }
        if time::now() > quote.expires_at {
            return Err(ServerError::PurchaseNotConfirmed(
                "the price shown has expired: check it again".into(),
            ));
        }
        if confirmation.domain.to_ascii_lowercase() != quote.domain
            || confirmation.cost_cents != quote.cost_cents
            || confirmation.currency != quote.currency
        {
            return Err(ServerError::PurchaseNotConfirmed(
                "the confirmed domain or price differs from the quote".into(),
            ));
        }
        // One use per quote, whatever happens next.
        self.quotes
            .lock()
            .expect("quotes")
            .remove(&confirmation.quote_id);
        // The quote id doubles as the attempt's idempotency key: a retried request after a
        // network error can't register (and charge) twice.
        let keys = self.keys()?;
        let v = self
            .post_full(
                &keys,
                &format!("/domain/create/{}", quote.domain),
                json!({ "cost": quote.cost_cents, "agreeToTerms": "yes" }),
                Some(&quote.quote_id),
            )
            .await?;
        Ok(Purchase {
            domain: v["domain"].as_str().unwrap_or(&quote.domain).to_owned(),
            order_id: as_u64(&v["orderId"]),
            cost_cents: as_u64(&v["cost"]).unwrap_or(quote.cost_cents),
            balance_cents: as_u64(&v["balance"]),
        })
    }
}

#[async_trait]
impl DnsProvider for PorkbunDns {
    fn id(&self) -> &'static str {
        "porkbun"
    }

    async fn list_domains(&self) -> Result<Vec<DomainInfo>> {
        let mut out = Vec::new();
        let mut start = 0u64;
        loop {
            let v = self
                .post("/domain/listAll", json!({ "start": start }))
                .await?;
            let page = v["domains"].as_array().cloned().unwrap_or_default();
            let n = page.len() as u64;
            out.extend(page.iter().map(|d| DomainInfo {
                domain: d["domain"].as_str().unwrap_or_default().to_owned(),
                status: d["status"].as_str().map(str::to_owned),
                expire_date: d["expireDate"].as_str().map(str::to_owned),
                api_access: as_u64(&d["apiAccess"]) == Some(1),
            }));
            if n < 1000 {
                return Ok(out);
            }
            start += n;
        }
    }

    async fn apply(&self, zone: &str, records: &[DnsRecord]) -> Result<ApplyReport> {
        let existing = self.records(zone).await?;
        let mut report = ApplyReport::default();
        for r in records {
            let label = format!("{} {}", r.kind.as_str(), r.fqdn);
            let (content, prio) = Self::content_of(r);
            let same_slot: Vec<&PorkbunRecord> = existing
                .iter()
                .filter(|e| {
                    e.name.trim_end_matches('.') == r.fqdn
                        && e.kind.eq_ignore_ascii_case(r.kind.as_str())
                })
                .collect();
            let matches = |e: &&PorkbunRecord| {
                let content_ok = if matches!(
                    r.kind,
                    RecordKind::Cname | RecordKind::Srv | RecordKind::Alias
                ) {
                    e.content
                        .trim_end_matches('.')
                        .eq_ignore_ascii_case(content.trim_end_matches('.'))
                } else {
                    e.content == content
                };
                content_ok
                    && prio
                        .is_none_or(|p| e.prio.as_deref().and_then(|x| x.parse().ok()) == Some(p))
            };
            if same_slot.iter().any(matches) {
                report.unchanged.push(label);
                continue;
            }
            // A/AAAA may hold several values; others are replaced in place.
            let replace = match r.kind {
                RecordKind::A | RecordKind::Aaaa => None,
                // TXT: replace only our own earlier value (keep other TXT records at that name).
                RecordKind::Txt => same_slot
                    .iter()
                    .find(|e| e.notes.as_deref() == Some("Rustic Launcher"))
                    .copied(),
                _ => same_slot.first().copied(),
            };
            match replace {
                Some(e) => {
                    self.post(&format!("/dns/edit/{zone}/{}", e.id), Self::body_of(r))
                        .await?;
                    report.updated.push(label);
                }
                None => {
                    self.post(&format!("/dns/create/{zone}"), Self::body_of(r))
                        .await?;
                    report.created.push(label);
                }
            }
        }
        Ok(report)
    }

    async fn remove(&self, zone: &str, records: &[DnsRecord]) -> Result<usize> {
        let existing = self.records(zone).await?;
        let mut removed = 0;
        for r in records {
            for e in existing.iter().filter(|e| {
                e.name.trim_end_matches('.') == r.fqdn
                    && e.kind.eq_ignore_ascii_case(r.kind.as_str())
            }) {
                self.post(&format!("/dns/delete/{zone}/{}", e.id), json!({}))
                    .await?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

/// Registrars without an API: the records are shown for the player to enter by hand.
pub struct ManualDns;

#[async_trait]
impl DnsProvider for ManualDns {
    fn id(&self) -> &'static str {
        "manual"
    }
    async fn list_domains(&self) -> Result<Vec<DomainInfo>> {
        Ok(Vec::new())
    }
    async fn apply(&self, _: &str, records: &[DnsRecord]) -> Result<ApplyReport> {
        Err(ServerError::Dns(format!(
            "Enter these {} records at your registrar, then check them.",
            records.len()
        )))
    }
    async fn remove(&self, _: &str, _: &[DnsRecord]) -> Result<usize> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::{RoutingMode, Verification, required_records};
    use rusticmc_auth::MemoryStore;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn porkbun() -> (MockServer, PorkbunDns, Arc<MemoryStore>) {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::new());
        let p = PorkbunDns::new(
            rusticmc_launcher_core::http::client().unwrap(),
            server.uri(),
            store.clone(),
        );
        (server, p, store)
    }

    async fn connected() -> (MockServer, PorkbunDns, Arc<MemoryStore>) {
        let (server, p, store) = porkbun().await;
        Mock::given(method("POST"))
            .and(path("/ping"))
            .and(body_partial_json(json!({"apikey": "pk1_x", "secretapikey": "sk1_y"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS", "yourIp": "1.2.3.4", "credentialsValid": true, "account": "sam", "scopes": ["full"]
            })))
            .mount(&server)
            .await;
        assert_eq!(
            p.connect(Secret::new("pk1_x"), Secret::new("sk1_y"))
                .await
                .unwrap(),
            "sam"
        );
        (server, p, store)
    }

    #[tokio::test]
    async fn keys_are_checked_and_kept_in_the_keychain() {
        let (server, p, store) = porkbun().await;
        Mock::given(method("POST"))
            .and(path("/ping"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"status": "SUCCESS", "yourIp": "1.2.3.4"})),
            )
            .mount(&server)
            .await;
        assert!(
            p.connect(Secret::new("bad"), Secret::new("bad"))
                .await
                .is_err()
        );
        assert!(store.raw_items().is_empty());
        let (_s, p, store) = connected().await;
        assert!(p.connected());
        assert!(store.raw_items()["porkbun"].contains("pk1_x"));
        p.disconnect().unwrap();
        assert!(!p.connected());
    }

    #[tokio::test]
    async fn lists_domains_with_api_access() {
        let (server, p, _) = connected().await;
        Mock::given(method("POST"))
            .and(path("/domain/listAll"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS", "count": 2,
                "domains": [
                    {"domain": "example.com", "status": "ACTIVE", "expireDate": "2027-01-15 10:00:00", "apiAccess": 1},
                    {"domain": "other.net", "status": "ACTIVE", "apiAccess": 0}
                ]
            })))
            .mount(&server)
            .await;
        let d = p.list_domains().await.unwrap();
        assert_eq!(d.len(), 2);
        assert!(d[0].api_access && !d[1].api_access);
    }

    #[tokio::test]
    async fn configure_is_idempotent() {
        let (server, p, _) = connected().await;
        let records = required_records(
            "example.com",
            "play",
            &RoutingMode::Relay {
                target: "k7m2.relay.rustic.gg".into(),
                port: 30000,
            },
            Some(&Verification {
                name: "_rustic-verify.play".into(),
                value: "rustic-verify=abc".into(),
            }),
        )
        .unwrap();
        // Existing: the CNAME already right, an old SRV to update, no TXT yet.
        Mock::given(method("POST"))
            .and(path("/dns/retrieve/example.com"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS", "cloudflare": "disabled",
                "records": [
                    {"id": "11", "name": "play.example.com", "type": "CNAME", "content": "k7m2.relay.rustic.gg", "ttl": "600", "prio": null, "notes": null},
                    {"id": "12", "name": "_minecraft._tcp.play.example.com", "type": "SRV", "content": "5 25565 old.example.net", "ttl": "600", "prio": "0", "notes": null},
                    {"id": "13", "name": "example.com", "type": "A", "content": "1.2.3.4", "ttl": "600", "prio": null, "notes": null}
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/dns/edit/example.com/12"))
            .and(body_partial_json(json!({"type": "SRV", "content": "5 30000 k7m2.relay.rustic.gg", "prio": 0, "name": "_minecraft._tcp.play"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "SUCCESS"})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/dns/create/example.com"))
            .and(body_partial_json(json!({"type": "TXT", "name": "_rustic-verify.play", "content": "rustic-verify=abc", "ttl": 600})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "SUCCESS", "id": 123456789})))
            .expect(1)
            .mount(&server)
            .await;
        let report = p.apply("example.com", &records).await.unwrap();
        assert_eq!(report.unchanged, ["CNAME play.example.com"]);
        assert_eq!(report.updated, ["SRV _minecraft._tcp.play.example.com"]);
        assert_eq!(report.created, ["TXT _rustic-verify.play.example.com"]);
    }

    #[tokio::test]
    async fn errors_carry_porkbuns_message() {
        let (server, p, _) = connected().await;
        Mock::given(method("POST"))
            .and(path("/dns/retrieve/example.com"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "status": "ERROR", "code": "API_ACCESS_DISABLED", "message": "The domain is not opted in to API access."
            })))
            .mount(&server)
            .await;
        let err = p.records("example.com").await.unwrap_err().to_string();
        assert!(
            err.contains("opted in") && err.contains("API Access"),
            "{err}"
        );
        assert!(!err.contains("sk1_y"));
    }

    #[tokio::test]
    async fn buying_needs_a_matching_confirmation() {
        let (server, p, _) = connected().await;
        Mock::given(method("POST"))
            .and(path("/domain/checkDomain/rusticfun.com"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS",
                "response": {"avail": "yes", "type": "registration", "price": "9.73", "firstYearPromo": "no", "regularPrice": "9.73", "premium": "no", "minDuration": 1,
                             "additional": {"renewal": {"type": "renewal", "price": "10.37", "regularPrice": "10.37"}}}
            })))
            .mount(&server)
            .await;
        let a = p.check_domain("RusticFun.com").await.unwrap();
        assert!(a.available && !a.premium);
        assert_eq!(a.price.as_deref(), Some("9.73"));
        assert_eq!(a.renewal_price.as_deref(), Some("10.37"));
        Mock::given(method("POST"))
            .and(path("/domain/create/rusticfun.com"))
            .and(body_partial_json(json!({"dryRun": true, "cost": 0})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS", "dryRun": true, "wouldSucceed": true, "domain": "rusticfun.com", "cost": 973,
                "costDisplay": "$9.73", "balance": 5000, "sufficientFunds": true
            })))
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/domain/create/rusticfun.com"))
            .and(body_partial_json(json!({"cost": 973, "agreeToTerms": "yes"})))
            .and(wiremock::matchers::header_exists("idempotency-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS", "domain": "rusticfun.com", "cost": 973, "orderId": 12345678, "balance": 4027
            })))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        let quote = p.quote("rusticfun.com").await.unwrap();
        assert_eq!((quote.cost_cents, quote.currency.as_str()), (973, "USD"));
        let confirm = PurchaseConfirmation {
            quote_id: quote.quote_id.clone(),
            domain: "rusticfun.com".into(),
            cost_cents: 973,
            currency: "USD".into(),
            agree_to_terms: true,
        };
        // A different price, or no consent: refused before any call.
        let cheaper = PurchaseConfirmation {
            cost_cents: 100,
            ..confirm.clone()
        };
        assert!(matches!(
            p.buy(&cheaper).await,
            Err(ServerError::PurchaseNotConfirmed(_))
        ));
        let no_terms = PurchaseConfirmation {
            agree_to_terms: false,
            ..confirm.clone()
        };
        assert!(matches!(
            p.buy(&no_terms).await,
            Err(ServerError::PurchaseNotConfirmed(_))
        ));
        let bought = p.buy(&confirm).await.unwrap();
        assert_eq!(bought.order_id, Some(12345678));
        assert_eq!(bought.balance_cents, Some(4027));
        // A quote is used once.
        assert!(matches!(
            p.buy(&confirm).await,
            Err(ServerError::PurchaseNotConfirmed(_))
        ));
    }
}
