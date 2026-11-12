//! The `HostingProvider` abstraction: one interface for a server on this PC, on the user's VPS, or
//! (later) on Rustic Cloud. The UI and the Tauri commands only see this trait (ADR 0121).

use crate::error::Result;
use crate::model::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::sync::mpsc;

/// What a provider can do, so the UI greys out the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub available: bool,
    pub install: bool,
    pub console: bool,
    pub plugins: bool,
    pub backups_download: bool,
    pub update_engine: bool,
    pub firewall: bool,
}

impl Capabilities {
    pub const ALL: Self = Self {
        available: true,
        install: true,
        console: true,
        plugins: true,
        backups_download: true,
        update_engine: true,
        firewall: true,
    };
    pub const NONE: Self = Self {
        available: false,
        install: false,
        console: false,
        plugins: false,
        backups_download: false,
        update_engine: false,
        firewall: false,
    };
}

/// A live console: events arrive on `events`; dropping the subscription stops the stream.
pub struct ConsoleSubscription {
    pub events: mpsc::Receiver<ConsoleEvent>,
    /// Aborts the background task when dropped.
    pub(crate) _guard: Option<AbortOnDrop>,
}

impl ConsoleSubscription {
    pub fn new(events: mpsc::Receiver<ConsoleEvent>, task: tokio::task::JoinHandle<()>) -> Self {
        Self {
            events,
            _guard: Some(AbortOnDrop(task)),
        }
    }

    /// A subscription fed by someone else (the local provider's backend).
    pub fn from_receiver(events: mpsc::Receiver<ConsoleEvent>) -> Self {
        Self {
            events,
            _guard: None,
        }
    }
}

pub(crate) struct AbortOnDrop(pub tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// One hosting backend. Every method takes the server's name (unique per provider and target);
/// long operations report steps through `progress`.
#[async_trait]
pub trait HostingProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn capabilities(&self) -> Capabilities;

    /// Create and install a server (idempotent: re-running resumes where a failed run stopped).
    async fn install(&self, spec: &ServerSpec, progress: &dyn ProgressSink) -> Result<ServerRef>;
    async fn start(&self, server: &str) -> Result<()>;
    async fn stop(&self, server: &str) -> Result<()>;
    async fn restart(&self, server: &str) -> Result<()>;
    async fn status(&self, server: &str) -> Result<ServerStatus>;
    async fn metrics(&self, server: &str) -> Result<Metrics> {
        Ok(self.status(server).await?.metrics)
    }

    /// Live log lines and game events.
    async fn console_subscribe(&self, server: &str) -> Result<ConsoleSubscription>;
    /// Run one console command; returns what it logged.
    async fn send_command(&self, server: &str, line: &str) -> Result<CommandOutcome>;
    /// The last `lines` lines of the log.
    async fn logs(&self, server: &str, lines: usize) -> Result<Vec<String>>;

    async fn players(&self, server: &str) -> Result<PlayersSnapshot>;
    async fn player_action(&self, server: &str, action: &PlayerAction) -> Result<CommandOutcome>;

    /// `server.toml` as JSON (sections of keys).
    async fn config_read(&self, server: &str) -> Result<serde_json::Value>;
    /// `section.key = value` changes, checked by the supervisor before anything is written.
    async fn config_write(
        &self,
        server: &str,
        changes: &[(String, String)],
    ) -> Result<ConfigOutcome>;

    async fn plugins_list(&self, server: &str) -> Result<Vec<PluginInfo>>;
    /// Install a `.rplugin` into `<server>/plugins/<id>.rplugin` (takes effect at the next start).
    async fn plugin_upload(&self, server: &str, rplugin: &Path) -> Result<PluginInfo>;
    async fn plugin_set_enabled(&self, server: &str, id: &str, enabled: bool) -> Result<()>;
    async fn plugin_remove(&self, server: &str, id: &str) -> Result<()>;

    async fn datapacks_list(&self, server: &str) -> Result<Vec<DatapackInfo>>;
    async fn datapack_upload(&self, server: &str, zip: &Path) -> Result<DatapackInfo>;
    async fn datapack_remove(&self, server: &str, file: &str) -> Result<()>;

    async fn backup_create(&self, server: &str, name: Option<&str>) -> Result<String>;
    async fn backups_list(&self, server: &str) -> Result<Vec<BackupInfo>>;
    async fn backup_restore(
        &self,
        server: &str,
        backup: &str,
        dimension: Option<&str>,
    ) -> Result<()>;
    async fn backup_download(&self, server: &str, backup: &str, dest: &Path) -> Result<u64>;

    async fn update_engine(
        &self,
        server: &str,
        request: &UpdateRequest,
        progress: &dyn ProgressSink,
    ) -> Result<()>;

    async fn uninstall(
        &self,
        server: &str,
        options: &UninstallOptions,
        progress: &dyn ProgressSink,
    ) -> Result<()>;
}

/// The provider list the UI shows (`hosting_providers`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDescriptor {
    pub kind: ProviderKind,
    pub title: String,
    pub description: String,
    pub capabilities: Capabilities,
    /// `"coming_soon"` for Rustic Cloud, `"ready"` otherwise.
    pub status: String,
}

/// The three providers, as the "New server" dialog lists them.
pub fn descriptors() -> Vec<ProviderDescriptor> {
    vec![
        ProviderDescriptor {
            kind: ProviderKind::Local,
            title: "This computer".into(),
            description: "Runs on your PC while it is on. Friends join on your LAN, through a temporary Rustic Relay address, or with an invite code.".into(),
            capabilities: Capabilities::ALL,
            status: "ready".into(),
        },
        ProviderDescriptor {
            kind: ProviderKind::RemoteSsh,
            title: "Your VPS (SSH)".into(),
            description: "Rustic connects to your Linux VPS over SSH, installs Horizon's server and the Rustic engine as a service, then manages it from here.".into(),
            capabilities: Capabilities::ALL,
            status: "ready".into(),
        },
        ProviderDescriptor {
            kind: ProviderKind::RusticCloud,
            title: "Rustic Cloud".into(),
            description: "Servers on Rustic's machines. Coming soon: join the waitlist.".into(),
            capabilities: Capabilities::NONE,
            status: "coming_soon".into(),
        },
    ]
}
