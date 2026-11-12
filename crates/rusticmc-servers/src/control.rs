//! `horizon-server`'s control API (horizon/docs/api/server.md, server-reference.md "Control API
//! (`/api/v1`)"): HTTP and JSON on 127.0.0.1. A running server writes its address and a random
//! token to `.horizon/api.json`; every request sends `Authorization: Bearer <token>`. Errors are
//! `{"error": "..."}` with 400, 401, 403, 404, 405 or 409.

use crate::error::{Result, ServerError};
use futures_util::StreamExt;
use rusticmc_launcher_core::{Secret, reqwest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use tokio::sync::mpsc;

/// `.horizon/api.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct Endpoint {
    pub address: String,
    pub token: Secret,
    #[serde(default)]
    pub pid: Option<u32>,
}

impl Endpoint {
    /// The endpoint a running supervisor wrote in `dir`, `None` when there is none.
    pub fn read(dir: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(dir.join(".horizon/api.json")).ok()?;
        serde_json::from_str(&text).ok()
    }
}

/// The supervisor's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Starting,
    Running,
    Stopped,
    Stopping,
    Restarting,
    #[serde(other)]
    Unknown,
}

/// `GET /api/v1/status`, typed where the launcher needs it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub phase: Phase,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub uptime_seconds: Option<u64>,
    #[serde(default)]
    pub engine_pid: Option<u32>,
    #[serde(default)]
    pub engine_uptime_seconds: Option<u64>,
    #[serde(default)]
    pub crashes: Option<u64>,
    #[serde(default)]
    pub last_crash: Value,
    /// `{pid, rss_bytes, cpu_percent, game: {tps, mspt: {average, p95}, players: [...]}}`.
    #[serde(default)]
    pub sample: Value,
    #[serde(default)]
    pub update: Value,
}

/// RAM, CPU, TPS, MSPT and players from a status sample.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub tps: Option<f64>,
    pub mspt: Option<f64>,
    pub mspt_p95: Option<f64>,
    pub rss_bytes: Option<u64>,
    pub cpu_percent: Option<f64>,
    pub players_online: Option<usize>,
}

impl Status {
    pub fn metrics(&self) -> Metrics {
        let s = &self.sample;
        let game = &s["game"];
        Metrics {
            tps: game["tps"].as_f64(),
            mspt: game["mspt"]["average"].as_f64(),
            mspt_p95: game["mspt"]["p95"].as_f64(),
            rss_bytes: s["rss_bytes"].as_u64(),
            cpu_percent: s["cpu_percent"].as_f64(),
            players_online: game["players"].as_array().map(Vec::len),
        }
    }
}

/// One sample of `GET /api/v1/metrics` (one a second, the last 10 minutes).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MetricSample {
    pub t: u64,
    #[serde(default)]
    pub tps: Option<f64>,
    #[serde(default)]
    pub mspt: Option<f64>,
    #[serde(default)]
    pub mspt_p95: Option<f64>,
    #[serde(default)]
    pub rss_bytes: Option<u64>,
    #[serde(default)]
    pub cpu_percent: Option<f64>,
    #[serde(default)]
    pub players: Option<u64>,
}

/// A backup (`GET /api/v1/backups`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Backup {
    pub name: String,
    pub bytes: u64,
}

/// What a moderation action did.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionReply {
    pub changed: bool,
    #[serde(default)]
    pub output: Vec<String>,
}

/// What `PATCH /api/v1/config` (or `config set`) did.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigOutcome {
    #[serde(default)]
    pub saved: Vec<String>,
    #[serde(default)]
    pub applied: Vec<String>,
    #[serde(default)]
    pub restart: Vec<String>,
    #[serde(default)]
    pub running: bool,
}

/// One event of `GET /api/v1/events` (`log`, `phase`, `join`, `leave`, `kick`, `chat`,
/// `chat_filtered`, `death`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ServerEvent {
    pub id: u64,
    #[serde(rename = "type")]
    pub kind: String,
    /// The whole event (`line` for a log, `name`/`uuid`/`message` for chat...).
    pub data: Value,
}

/// A moderation action.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlayerAction {
    Kick {
        name: String,
        reason: Option<String>,
    },
    Ban {
        name: String,
        reason: Option<String>,
    },
    Pardon {
        name: String,
    },
    BanIp {
        target: String,
        reason: Option<String>,
    },
    PardonIp {
        ip: String,
    },
    Op {
        name: String,
    },
    Deop {
        name: String,
    },
    WhitelistAdd {
        name: String,
    },
    WhitelistRemove {
        name: String,
    },
    Whitelist {
        enabled: bool,
    },
}

impl PlayerAction {
    fn route(&self) -> (&'static str, Value) {
        let with_reason = |name: &str, reason: &Option<String>| match reason {
            Some(r) => json!({ "name": name, "reason": r }),
            None => json!({ "name": name }),
        };
        match self {
            Self::Kick { name, reason } => ("/api/v1/players/kick", with_reason(name, reason)),
            Self::Ban { name, reason } => ("/api/v1/players/ban", with_reason(name, reason)),
            Self::Pardon { name } => ("/api/v1/players/pardon", json!({ "name": name })),
            Self::BanIp { target, reason } => {
                // `{"ip", "reason"}`, or `{"name"}` of an online player.
                let is_ip = target.parse::<std::net::IpAddr>().is_ok();
                let mut body = if is_ip {
                    json!({ "ip": target })
                } else {
                    json!({ "name": target })
                };
                if let Some(r) = reason {
                    body["reason"] = json!(r);
                }
                ("/api/v1/players/ban-ip", body)
            }
            Self::PardonIp { ip } => ("/api/v1/players/pardon-ip", json!({ "ip": ip })),
            Self::Op { name } => ("/api/v1/players/op", json!({ "name": name })),
            Self::Deop { name } => ("/api/v1/players/deop", json!({ "name": name })),
            Self::WhitelistAdd { name } => ("/api/v1/whitelist/add", json!({ "name": name })),
            Self::WhitelistRemove { name } => ("/api/v1/whitelist/remove", json!({ "name": name })),
            Self::Whitelist { enabled } => ("/api/v1/whitelist", json!({ "enabled": enabled })),
        }
    }
}

/// A client of one server's control API.
#[derive(Clone)]
pub struct ControlClient {
    http: reqwest::Client,
    base: String,
    token: Secret,
}

impl ControlClient {
    pub fn new(http: reqwest::Client, endpoint: &Endpoint) -> Self {
        Self {
            http,
            base: format!("http://{}", endpoint.address),
            token: endpoint.token.clone(),
        }
    }

    /// The client of the supervisor running in `dir`.
    pub fn open(http: reqwest::Client, dir: &Path) -> Result<Self> {
        let endpoint = Endpoint::read(dir).ok_or(ServerError::NotRunning)?;
        Ok(Self::new(http, &endpoint))
    }

    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value> {
        let mut req = self
            .http
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(self.token.expose())
            .header("Accept", "application/json");
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ServerError::ApiUnreachable(e.to_string()))?;
        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| ServerError::ApiUnreachable(e.to_string()))?;
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
        if (200..300).contains(&status) {
            Ok(value)
        } else {
            Err(ServerError::Api {
                status,
                message: value["error"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string()),
            })
        }
    }

    async fn get(&self, path: &str) -> Result<Value> {
        self.call(reqwest::Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value> {
        self.call(reqwest::Method::POST, path, Some(&body)).await
    }

    fn typed<T: serde::de::DeserializeOwned>(v: Value) -> Result<T> {
        serde_json::from_value(v)
            .map_err(|e| ServerError::Supervisor(format!("unexpected reply: {e}")))
    }

    pub async fn status(&self) -> Result<Status> {
        Self::typed(self.get("/api/v1/status").await?)
    }

    pub async fn metrics(&self, since: Option<u64>) -> Result<Vec<MetricSample>> {
        let path = match since {
            Some(t) => format!("/api/v1/metrics?since={t}"),
            None => "/api/v1/metrics".into(),
        };
        let v = self.get(&path).await?;
        Self::typed(v["samples"].clone())
    }

    /// The log lines after the `since`th; returns `(next, lines)`.
    pub async fn logs(&self, since: u64) -> Result<(u64, Vec<String>)> {
        let v = self.get(&format!("/api/v1/logs?since={since}")).await?;
        Ok((
            v["next"].as_u64().unwrap_or(since),
            Self::typed(v["lines"].clone())?,
        ))
    }

    /// Runs one console command; the lines it logged.
    pub async fn command(&self, line: &str) -> Result<Vec<String>> {
        let line = line.trim().trim_start_matches('/');
        if line.is_empty() || line.contains('\n') {
            return Err(ServerError::Invalid("one command, on one line".into()));
        }
        let v = self
            .post("/api/v1/command", json!({ "line": line }))
            .await?;
        Ok(match v {
            Value::Array(lines) => lines
                .into_iter()
                .filter_map(|l| l.as_str().map(str::to_owned))
                .collect(),
            other => other["lines"]
                .as_array()
                .or_else(|| other["output"].as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|l| l.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    pub async fn start(&self) -> Result<()> {
        self.post("/api/v1/start", json!({})).await.map(|_| ())
    }
    pub async fn stop(&self) -> Result<()> {
        self.post("/api/v1/stop", json!({})).await.map(|_| ())
    }
    pub async fn restart(&self) -> Result<()> {
        self.post("/api/v1/restart", json!({})).await.map(|_| ())
    }

    pub async fn backups(&self) -> Result<Vec<Backup>> {
        let v = self.get("/api/v1/backups").await?;
        Self::typed(v["backups"].clone())
    }

    /// Backs up now (a running game saves first and stays up); the file's name.
    pub async fn backup(&self, name: Option<&str>) -> Result<String> {
        let body = match name {
            Some(n) => json!({ "name": n }),
            None => json!({}),
        };
        let v = self.post("/api/v1/backups", body).await?;
        Ok(v["name"].as_str().unwrap_or_default().to_owned())
    }

    pub async fn delete_backup(&self, name: &str) -> Result<()> {
        self.call(
            reqwest::Method::DELETE,
            &format!("/api/v1/backups?name={}", urlencode(name)),
            None,
        )
        .await
        .map(|_| ())
    }

    /// Restarts the game with a backup's world (or one dimension of it).
    pub async fn restore(&self, name: &str, dimension: Option<&str>) -> Result<()> {
        let mut body = json!({ "name": name });
        if let Some(d) = dimension {
            body["dimension"] = json!(d);
        }
        self.post("/api/v1/restore", body).await.map(|_| ())
    }

    pub async fn crash_reports(&self) -> Result<Vec<Backup>> {
        let v = self.get("/api/v1/crash-reports").await?;
        Self::typed(v["reports"].clone())
    }

    pub async fn crash_report(&self, name: &str) -> Result<String> {
        match self
            .get(&format!(
                "/api/v1/crash-reports/read?name={}",
                urlencode(name)
            ))
            .await?
        {
            Value::String(s) => Ok(s),
            other => Ok(other.to_string()),
        }
    }

    /// Restarts on an installed release, or downloads the newest first.
    pub async fn update(&self, tag: Option<&str>, timeout: Option<u64>) -> Result<()> {
        let mut body = json!({});
        if let Some(t) = tag {
            body["tag"] = json!(t);
        }
        if let Some(t) = timeout {
            body["timeout"] = json!(t);
        }
        self.post("/api/v1/update", body).await.map(|_| ())
    }

    /// `GET /api/v1/players`: online, ops, whitelist, bans, ip_bans.
    pub async fn players(&self) -> Result<Value> {
        self.get("/api/v1/players").await
    }

    pub async fn player_action(&self, action: &PlayerAction) -> Result<ActionReply> {
        let (path, body) = action.route();
        Self::typed(self.post(path, body).await?)
    }

    pub async fn config(&self) -> Result<Value> {
        Ok(self.get("/api/v1/config").await?["config"].clone())
    }

    /// `PATCH /api/v1/config` (not `engine.*`, `panel.*`, `api.*`, RCON, query or management).
    pub async fn patch_config(
        &self,
        changes: &serde_json::Map<String, Value>,
    ) -> Result<ConfigOutcome> {
        let v = self
            .call(
                reqwest::Method::PATCH,
                "/api/v1/config",
                Some(&json!({ "changes": changes })),
            )
            .await?;
        Self::typed(v)
    }

    pub async fn mods(&self) -> Result<Value> {
        self.get("/api/v1/mods").await
    }

    pub async fn prometheus(&self) -> Result<String> {
        Ok(match self.get("/api/v1/metrics/prometheus").await? {
            Value::String(s) => s,
            other => other.to_string(),
        })
    }

    /// A one-time login link for the web panel.
    pub async fn panel_login(&self) -> Result<String> {
        Ok(self.post("/api/v1/panel/login", json!({})).await?["url"]
            .as_str()
            .unwrap_or_default()
            .to_owned())
    }

    /// The events after `since` (JSON form).
    pub async fn events_after(&self, since: u64) -> Result<(u64, Vec<ServerEvent>)> {
        let v = self.get(&format!("/api/v1/events?since={since}")).await?;
        let events = v["events"]
            .as_array()
            .map(|a| a.iter().filter_map(event_from).collect())
            .unwrap_or_default();
        Ok((v["next"].as_u64().unwrap_or(since), events))
    }

    /// Streams events as server-sent events into `tx`, resuming after the last id when the
    /// connection drops, until `tx` closes or `cancel` fires.
    pub async fn stream_events(
        &self,
        since: Option<u64>,
        tx: mpsc::Sender<ServerEvent>,
        cancel: rusticmc_launcher_core::CancellationToken,
    ) -> Result<()> {
        let mut last = since;
        loop {
            if cancel.is_cancelled() || tx.is_closed() {
                return Ok(());
            }
            let url = match last {
                Some(n) => format!("{}/api/v1/events?since={n}", self.base),
                None => format!("{}/api/v1/events", self.base),
            };
            let resp = self
                .http
                .get(url)
                .bearer_auth(self.token.expose())
                .header("Accept", "text/event-stream")
                .send()
                .await;
            let resp = match resp {
                Ok(r) if r.status().is_success() => r,
                Ok(r) => {
                    return Err(ServerError::Api {
                        status: r.status().as_u16(),
                        message: "events".into(),
                    });
                }
                Err(e) => return Err(ServerError::ApiUnreachable(e.to_string())),
            };
            let mut stream = resp.bytes_stream();
            let mut parser = SseParser::default();
            loop {
                let chunk = tokio::select! {
                    _ = cancel.cancelled() => return Ok(()),
                    c = stream.next() => c,
                };
                let Some(Ok(chunk)) = chunk else { break };
                for event in parser.push(&chunk) {
                    last = Some(event.id);
                    if tx.send(event).await.is_err() {
                        return Ok(());
                    }
                }
            }
            // Dropped: reconnect after the last id seen.
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }
    }
}

fn event_from(v: &Value) -> Option<ServerEvent> {
    Some(ServerEvent {
        id: v["id"].as_u64()?,
        kind: v["type"].as_str()?.to_owned(),
        data: v.clone(),
    })
}

/// Server-sent events: `id: N`, `event: type`, `data: {...}`, a blank line.
#[derive(Default)]
pub struct SseParser {
    buf: String,
}

impl SseParser {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<ServerEvent> {
        self.buf.push_str(&String::from_utf8_lossy(bytes));
        let mut out = Vec::new();
        while let Some(end) = self.buf.find("\n\n") {
            let block: String = self.buf.drain(..end + 2).collect();
            let (mut id, mut kind, mut data) = (None, None, String::new());
            for line in block.lines() {
                let line = line.trim_end_matches('\r');
                if let Some(v) = line.strip_prefix("id:") {
                    id = v.trim().parse().ok();
                } else if let Some(v) = line.strip_prefix("event:") {
                    kind = Some(v.trim().to_owned());
                } else if let Some(v) = line.strip_prefix("data:") {
                    data.push_str(v.trim_start());
                }
            }
            let value: Value = serde_json::from_str(&data).unwrap_or(Value::Null);
            let id = id.or_else(|| value["id"].as_u64());
            let kind = kind.or_else(|| value["type"].as_str().map(str::to_owned));
            if let (Some(id), Some(kind)) = (id, kind) {
                out.push(ServerEvent {
                    id,
                    kind,
                    data: value,
                });
            }
        }
        out
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_blocks_split_across_chunks() {
        let mut p = SseParser::default();
        assert!(p.push(b"id: 41\nevent: chat\nda").is_empty());
        let events = p.push(
            b"ta: {\"id\":41,\"type\":\"chat\",\"name\":\"Steve\",\"message\":\"hi\"}\n\nid: 42\n",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 41);
        assert_eq!(events[0].kind, "chat");
        assert_eq!(events[0].data["message"], "hi");
        let events = p.push(b"event: log\ndata: {\"line\":\"[INFO] Done\"}\n\n");
        assert_eq!(events[0].id, 42);
        assert_eq!(events[0].data["line"], "[INFO] Done");
    }

    #[test]
    fn metrics_from_a_status_sample() {
        let status: Status = serde_json::from_value(json!({
            "phase": "running", "version": "x", "pid": 1, "uptime_seconds": 5,
            "sample": {"pid": 2, "rss_bytes": 1000, "cpu_percent": 12.5,
                       "game": {"tps": 20.0, "mspt": {"average": 1.5, "p95": 3.1}, "players": [{"name": "A"}]}}
        }))
        .unwrap();
        let m = status.metrics();
        assert_eq!(m.tps, Some(20.0));
        assert_eq!(m.mspt_p95, Some(3.1));
        assert_eq!(m.players_online, Some(1));
        assert_eq!(m.rss_bytes, Some(1000));
        let unknown: Status = serde_json::from_value(json!({"phase": "hibernating"})).unwrap();
        assert_eq!(unknown.phase, Phase::Unknown);
    }

    #[test]
    fn ban_ip_takes_an_address_or_a_name() {
        let (_, body) = PlayerAction::BanIp {
            target: "203.0.113.5".into(),
            reason: None,
        }
        .route();
        assert_eq!(body, json!({"ip": "203.0.113.5"}));
        let (_, body) = PlayerAction::BanIp {
            target: "Steve".into(),
            reason: Some("x".into()),
        }
        .route();
        assert_eq!(body, json!({"name": "Steve", "reason": "x"}));
    }
}
