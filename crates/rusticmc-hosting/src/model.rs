//! The data every provider speaks. Serialised as camelCase for the frontend (IPC.md, "Hosting").
//! Shapes that mirror Horizon's supervisor (`status --json`, `players --json`, `backups --json`,
//! `config set --json`) are read leniently: a field Horizon adds later is ignored, one it drops
//! becomes a default.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Which provider a server lives on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Local,
    RemoteSsh,
    RusticCloud,
}

/// The engine the supervisor runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EngineSource {
    /// Vanilla Horizon: the game program shipped in the Horizon release.
    Horizon,
    /// Rustic Server from a local file (built or downloaded by the launcher), uploaded by SFTP and
    /// checked against `sha256` on the VPS.
    RusticUpload { path: PathBuf, sha256: String },
    /// Rustic Server downloaded by the VPS itself from a release URL, checked against `sha256`.
    RusticRelease { url: String, sha256: String },
}

/// What to create.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSpec {
    /// Folder, unit and display key: `[a-z][a-z0-9-]{0,31}`.
    pub name: String,
    pub port: u16,
    pub world: String,
    #[serde(default)]
    pub seed: Option<String>,
    pub whitelist: bool,
    /// Players to whitelist (Minecraft names).
    #[serde(default)]
    pub players: Vec<String>,
    #[serde(default)]
    pub op: Option<String>,
    /// `false` turns online mode off (docs/hosting.md: keep such a server private).
    pub online_mode: bool,
    /// The user ticked "I agree to the Minecraft EULA" (https://aka.ms/MinecraftEULA) for this
    /// server. Without it nothing is installed: `horizon-server init --eula` writes `eula=true`.
    pub eula_accepted: bool,
    pub engine: EngineSource,
    /// The prepared game files (the folder holding `horizon-resources.json`, prepared on this
    /// computer from the user's own game). Uploaded and imported with
    /// `horizon-server import-resources`. `None`: the server is installed but can't start until they
    /// are imported (the UI offers it later).
    #[serde(default)]
    pub prepared_resources: Option<PathBuf>,
    /// Extra `config set` changes (`section.key` = value), applied after `init`.
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

/// A server a provider manages.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRef {
    pub provider: ProviderKind,
    /// The connection profile (VPS) for remote servers; empty for local ones.
    pub target: String,
    pub name: String,
}

/// A supervisor phase as `status --json` reports it (`running`, `starting`, `stopping`,
/// `stopped`, `crashed`...). Kept as text: Horizon may add phases.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub phase: String,
    pub version: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub crashes: u64,
    pub last_crash: Option<String>,
    pub metrics: Metrics,
    pub players_online: Vec<String>,
    /// A newer Horizon release, when the supervisor's check found one.
    pub update_available: Option<String>,
}

impl ServerStatus {
    pub fn is_running(&self) -> bool {
        self.phase == "running"
    }

    /// Read `horizon-server status --json` (or `GET /api/v1/status`).
    pub fn from_horizon(v: &serde_json::Value) -> Self {
        let sample = &v["sample"];
        let game = &sample["game"];
        let update = &v["update"];
        Self {
            phase: v["phase"].as_str().unwrap_or("unknown").to_owned(),
            version: v["version"].as_str().map(str::to_owned),
            uptime_seconds: v["engine_uptime_seconds"]
                .as_u64()
                .or(v["uptime_seconds"].as_u64()),
            crashes: v["crashes"].as_u64().unwrap_or(0),
            last_crash: v["last_crash"].as_str().map(str::to_owned),
            metrics: Metrics::from_horizon_sample(sample),
            players_online: game["players"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|p| p["name"].as_str().map(str::to_owned))
                .collect(),
            update_available: update["available"]
                .as_str()
                .or(update["latest"].as_str())
                .map(str::to_owned),
        }
    }
}

/// RAM, CPU, TPS, MSPT, players.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub ram_bytes: Option<u64>,
    /// Percent of one core.
    pub cpu_percent: Option<f64>,
    pub tps: Option<f64>,
    pub mspt_avg: Option<f64>,
    pub mspt_p95: Option<f64>,
    pub players: Option<u32>,
    pub max_players: Option<u32>,
    pub chunks: Option<u64>,
}

impl Metrics {
    /// Read one `sample` of the supervisor's status.
    pub fn from_horizon_sample(sample: &serde_json::Value) -> Self {
        let game = &sample["game"];
        Self {
            ram_bytes: sample["rss_bytes"].as_u64(),
            cpu_percent: sample["cpu_percent"].as_f64(),
            tps: game["tps"].as_f64(),
            mspt_avg: game["mspt"]["average"].as_f64(),
            mspt_p95: game["mspt"]["p95"].as_f64(),
            players: game["players"].as_array().map(|a| a.len() as u32),
            max_players: game["max_players"].as_u64().map(|n| n as u32),
            chunks: game["chunks"].as_u64(),
        }
    }
}

/// A line or event from the server's console stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ConsoleEvent {
    /// A log line.
    Log { line: String },
    /// The supervisor's phase changed.
    Phase { phase: String },
    /// A typed game event (`join`, `leave`, `kick`, `chat`, `death`) with its JSON data.
    Game {
        kind: String,
        data: serde_json::Value,
    },
    /// The stream ended (server stopped, connection lost); the UI resubscribes.
    Closed { reason: String },
}

/// Moderation and list actions (`horizon-server` commands of the same names).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
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
    WhitelistEnabled {
        enabled: bool,
    },
}

impl PlayerAction {
    /// The `horizon-server` arguments for this action.
    pub fn horizon_args(&self) -> Vec<String> {
        fn with_reason(cmd: &str, who: &str, reason: &Option<String>) -> Vec<String> {
            let mut v = vec![cmd.to_owned(), who.to_owned()];
            if let Some(r) = reason.as_ref().filter(|r| !r.is_empty()) {
                v.push(r.clone());
            }
            v
        }
        match self {
            Self::Kick { name, reason } => with_reason("kick", name, reason),
            Self::Ban { name, reason } => with_reason("ban", name, reason),
            Self::Pardon { name } => vec!["pardon".into(), name.clone()],
            Self::BanIp { target, reason } => with_reason("ban-ip", target, reason),
            Self::PardonIp { ip } => vec!["pardon-ip".into(), ip.clone()],
            Self::Op { name } => vec!["op".into(), name.clone()],
            Self::Deop { name } => vec!["deop".into(), name.clone()],
            Self::WhitelistAdd { name } => vec!["whitelist".into(), "add".into(), name.clone()],
            Self::WhitelistRemove { name } => {
                vec!["whitelist".into(), "remove".into(), name.clone()]
            }
            Self::WhitelistEnabled { enabled } => vec![
                "whitelist".into(),
                if *enabled { "on" } else { "off" }.into(),
            ],
        }
    }
}

/// `players --json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersSnapshot {
    pub online: Vec<OnlinePlayer>,
    pub max_players: Option<u32>,
    pub ops: Vec<String>,
    pub whitelist_enabled: bool,
    pub whitelist: Vec<String>,
    pub bans: Vec<String>,
    pub ip_bans: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlinePlayer {
    pub name: String,
    pub uuid: Option<String>,
    pub ping_ms: Option<u64>,
    pub op_level: Option<u64>,
}

impl PlayersSnapshot {
    pub fn from_horizon(v: &serde_json::Value) -> Self {
        let names = |key: &str, field: &str| -> Vec<String> {
            v[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| e[field].as_str().map(str::to_owned))
                .collect()
        };
        Self {
            online: v["online"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| OnlinePlayer {
                    name: p["name"].as_str().unwrap_or_default().to_owned(),
                    uuid: p["uuid"].as_str().map(str::to_owned),
                    ping_ms: p["ping_ms"].as_u64(),
                    op_level: p["op_level"].as_u64(),
                })
                .collect(),
            max_players: v["max_players"].as_u64().map(|n| n as u32),
            ops: names("ops", "name"),
            whitelist_enabled: v["whitelist"]["enabled"].as_bool().unwrap_or(false),
            whitelist: v["whitelist"]["players"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| e["name"].as_str().map(str::to_owned))
                .collect(),
            bans: names("bans", "name"),
            ip_bans: names("ip_bans", "ip"),
        }
    }
}

/// What a moderation command did (`{"changed", "output"}`), or the console's answer to `cmd`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutcome {
    pub output: Vec<String>,
}

/// `config set --json`: which keys were saved, applied at once, or wait for a restart.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOutcome {
    pub saved: Vec<String>,
    pub applied: Vec<String>,
    pub restart: Vec<String>,
}

impl ConfigOutcome {
    pub fn from_horizon(v: &serde_json::Value) -> Self {
        let list = |k: &str| -> Vec<String> {
            v[k].as_array()
                .into_iter()
                .flatten()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        };
        Self {
            saved: list("saved"),
            applied: list("applied"),
            restart: list("restart"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub bytes: u64,
    /// Seconds since 1970.
    pub modified: Option<u64>,
}

impl BackupInfo {
    /// `backups --json` is an array of `{name, bytes, modified}`.
    pub fn list_from_horizon(v: &serde_json::Value) -> Vec<Self> {
        v.as_array()
            .or(v["backups"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|b| {
                Some(Self {
                    name: b["name"].as_str()?.to_owned(),
                    bytes: b["bytes"].as_u64().unwrap_or(0),
                    modified: b["modified"].as_u64(),
                })
            })
            .collect()
    }
}

/// A `.rplugin` in `<server>/plugins/` (or disabled beside them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    /// The manifest id (`[plugin] id`), which is also the file name `<id>.rplugin`.
    pub id: String,
    pub file: String,
    pub enabled: bool,
    pub bytes: u64,
}

/// A datapack in `<server>/<world>/datapacks/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatapackInfo {
    pub file: String,
    pub bytes: u64,
}

/// What `update_engine` should do.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    /// Update Horizon (`horizon-server update [--version TAG]`): `Some("")` = newest.
    pub horizon_version: Option<String>,
    /// Replace the Rustic Server engine.
    pub engine: Option<EngineSource>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallOptions {
    /// Delete the server folder (world, backups). Off: only the service, firewall rule and
    /// engine go; the folder stays for a later reinstall.
    pub delete_files: bool,
    /// Also remove the Horizon install and the dedicated account when no other server uses them.
    pub remove_runtime: bool,
}

/// One step of a long operation, for progress bars (`hosting-progress` event).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub operation: String,
    pub step: String,
    pub title: String,
    pub index: usize,
    pub total: usize,
    pub state: StepState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Bytes done / total for transfers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Running,
    /// Found already done (idempotent re-run): nothing changed.
    Skipped,
    Done,
    Failed,
    RollingBack,
    RolledBack,
    /// A note (warning) attached to the step.
    Warning,
}

/// The frontend's coarser install step ids (`InstallStepId` in `apps/launcher/src/ipc/types/hosting.ts`)
/// for a step of the VPS install plan.
pub fn ui_step(step: &str) -> &'static str {
    match step {
        "connect" => "connect",
        "requirements" => "check",
        "service-account" | "folders" => "user",
        "install-horizon" => "download",
        "install-engine" => "engine",
        "init-server" | "configure" | "import-resources" => "world",
        "service" => "service",
        "firewall" => "firewall",
        _ => "start",
    }
}

/// Where progress goes (the Tauri layer forwards it as `hosting-install` events).
pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

/// Drops every event.
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn emit(&self, _event: ProgressEvent) {}
}

impl<F: Fn(ProgressEvent) + Send + Sync> ProgressSink for F {
    fn emit(&self, event: ProgressEvent) {
        self(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_from_horizon() {
        let s = ServerStatus::from_horizon(&json!({
            "phase": "running",
            "version": "nightly-2026-11-01-abc1234",
            "engine_uptime_seconds": 7380,
            "crashes": 1,
            "last_crash": "signal: 9 (SIGKILL)",
            "sample": {
                "rss_bytes": 1_420_000_000u64,
                "cpu_percent": 12.4,
                "game": {
                    "tps": 20.0,
                    "mspt": { "average": 3.12, "p95": 7.9 },
                    "players": [{ "name": "Alex" }, { "name": "Steve" }],
                    "max_players": 20,
                    "chunks": 1234,
                    "paused": false
                }
            }
        }));
        assert!(s.is_running());
        assert_eq!(s.uptime_seconds, Some(7380));
        assert_eq!(s.players_online, vec!["Alex", "Steve"]);
        assert_eq!(s.metrics.ram_bytes, Some(1_420_000_000));
        assert_eq!(s.metrics.tps, Some(20.0));
        assert_eq!(s.metrics.mspt_p95, Some(7.9));
        assert_eq!(s.metrics.players, Some(2));
        assert_eq!(s.metrics.max_players, Some(20));
        let stopped = ServerStatus::from_horizon(&json!({ "phase": "stopped" }));
        assert!(!stopped.is_running());
        assert_eq!(stopped.metrics, Metrics::default());
    }

    #[test]
    fn players_from_horizon() {
        let p = PlayersSnapshot::from_horizon(&json!({
            "online": [{ "name": "Alex", "uuid": "u1", "ping_ms": 40, "op_level": 4 }],
            "max_players": 20,
            "ops": [{ "name": "Alex", "uuid": "u1", "level": 4 }],
            "whitelist": { "enabled": true, "players": [{ "name": "Alex", "uuid": "u1" }] },
            "bans": [{ "name": "Griefer", "uuid": "u2", "reason": "x" }],
            "ip_bans": [{ "ip": "203.0.113.9" }]
        }));
        assert_eq!(p.online[0].ping_ms, Some(40));
        assert_eq!(p.ops, vec!["Alex"]);
        assert!(p.whitelist_enabled);
        assert_eq!(p.bans, vec!["Griefer"]);
        assert_eq!(p.ip_bans, vec!["203.0.113.9"]);
    }

    #[test]
    fn actions_map_to_horizon_commands() {
        assert_eq!(
            PlayerAction::Ban {
                name: "G".into(),
                reason: Some("grief ing".into())
            }
            .horizon_args(),
            vec!["ban", "G", "grief ing"]
        );
        assert_eq!(
            PlayerAction::Kick {
                name: "G".into(),
                reason: Some(String::new())
            }
            .horizon_args(),
            vec!["kick", "G"]
        );
        assert_eq!(
            PlayerAction::WhitelistEnabled { enabled: false }.horizon_args(),
            vec!["whitelist", "off"]
        );
        assert_eq!(
            PlayerAction::WhitelistAdd { name: "A".into() }.horizon_args(),
            vec!["whitelist", "add", "A"]
        );
    }

    #[test]
    fn backups_and_config() {
        let b = BackupInfo::list_from_horizon(&json!([
            { "name": "a.zip", "bytes": 10, "modified": 5 },
            { "bytes": 3 }
        ]));
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].modified, Some(5));
        let api =
            BackupInfo::list_from_horizon(&json!({ "backups": [{ "name": "b.zip", "bytes": 1 }] }));
        assert_eq!(api[0].name, "b.zip");
        let c = ConfigOutcome::from_horizon(
            &json!({ "saved": ["world.difficulty"], "applied": ["world.difficulty"], "restart": [] }),
        );
        assert_eq!(c.applied, vec!["world.difficulty"]);
    }

    #[test]
    fn ui_steps() {
        assert_eq!(ui_step("install-horizon"), "download");
        assert_eq!(ui_step("import-resources"), "world");
        assert_eq!(ui_step("health-check"), "start");
        assert_eq!(ui_step("folders"), "user");
    }

    #[test]
    fn camel_case_on_the_wire() {
        let ev = ProgressEvent {
            operation: "install".into(),
            step: "probe".into(),
            title: "Detect the system".into(),
            index: 1,
            total: 10,
            state: StepState::Done,
            detail: None,
            bytes: None,
        };
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["state"], "done");
        assert!(v.get("detail").is_none());
        let src = serde_json::to_value(EngineSource::RusticRelease {
            url: "https://x".into(),
            sha256: "ab".into(),
        })
        .unwrap();
        assert_eq!(src["kind"], "rustic_release");
    }
}
