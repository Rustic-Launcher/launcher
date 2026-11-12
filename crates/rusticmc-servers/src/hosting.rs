//! Local-server types. The provider abstraction over local, VPS and Rustic Cloud servers is
//! `rusticmc_hosting::HostingProvider` (the hosting engineer's crate); [`crate::provider`]
//! implements it for this computer's servers.

use crate::control::{Metrics, Status};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostingKind {
    Local,
    Ssh,
    Cloud,
}

/// The engine a server runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerKind {
    /// Horizon's own game (`horizon-client --dedicated` under `horizon-server`).
    Horizon,
    /// Rustic Server: `horizon-server` with `engine.path` = the `rustic-server` program, and
    /// `.rplugin` plugins.
    RusticServer,
}

/// What creating a server takes, wherever it runs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateServer {
    pub name: String,
    pub kind: ServerKind,
    /// The Horizon build (installed version) whose `horizon-server` and game files it uses.
    pub build: String,
    /// The player agreed to the Minecraft EULA in the launcher.
    pub eula_accepted: bool,
    #[serde(default)]
    pub world: Option<String>,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "yes")]
    pub whitelist: bool,
    #[serde(default)]
    pub players: Vec<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default = "yes")]
    pub online_mode: bool,
    /// A `rustic-server` program for this server only.
    #[serde(default)]
    pub engine_path: Option<std::path::PathBuf>,
}

fn yes() -> bool {
    true
}

/// A server as the server list shows it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HostedServer {
    pub id: String,
    pub name: String,
    pub kind: ServerKind,
    pub hosting: HostingKind,
    pub build: String,
    pub port: u16,
    pub created_at: u64,
    /// `None` when not known (unreachable).
    pub running: Option<bool>,
}

/// A server's live state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ServerState {
    pub running: bool,
    pub status: Option<Status>,
    pub metrics: Metrics,
}
