//! `RusticCloudProvider`: servers on Rustic's machines. **Coming soon**: every call answers
//! [`HostingError::NotAvailableYet`]. The API shape below is the contract the cloud engineer will
//! serve (`/v1/hosting/*`, launcher/docs/HOSTING.md "Rustic Cloud (later)"), so the UI can be built
//! against it now. Nothing here talks to the network.

use crate::error::{HostingError, Result};
use crate::model::*;
use crate::provider::{Capabilities, ConsoleSubscription, HostingProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A plan (`GET /v1/hosting/plans`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPlan {
    pub id: String,
    pub name: String,
    pub ram_mb: u32,
    pub vcpus: f32,
    pub disk_gb: u32,
    pub max_players: u32,
    /// Monthly price in the smallest currency unit, VAT included (EU consumer law: prices shown
    /// include VAT).
    pub price_cents: u32,
    pub currency: String,
}

/// A region (`GET /v1/hosting/regions`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudRegion {
    pub id: String,
    pub name: String,
    pub country: String,
    pub available: bool,
}

/// `POST /v1/hosting/servers`: allocate a server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationRequest {
    pub plan: String,
    pub region: String,
    pub spec: ServerSpec,
    /// The user ticked the immediate-execution consent and the withdrawal-right waiver for this
    /// digital service (launcher/docs/LEGAL.md).
    pub immediate_execution_consent: bool,
}

/// The allocation's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub id: String,
    pub state: AllocationState,
    /// `<name>.<region>.play.<rustic domain>:port` once running.
    pub address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocationState {
    Queued,
    Provisioning,
    Running,
    Suspended,
    Deleting,
}

/// The waitlist entry the "coming soon" screen sends (`POST /v1/hosting/waitlist`, cloud owner).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitlistEntry {
    pub region: Option<String>,
    pub expected_players: Option<u32>,
}

#[derive(Debug, Default)]
pub struct RusticCloudProvider;

impl RusticCloudProvider {
    pub fn new() -> Self {
        Self
    }

    fn not_yet<T>(&self) -> Result<T> {
        Err(HostingError::NotAvailableYet {
            provider: "Rustic Cloud",
            detail: "hosting on Rustic's machines is coming soon; join the waitlist from the Servers page".into(),
        })
    }

    pub async fn plans(&self) -> Result<Vec<CloudPlan>> {
        self.not_yet()
    }
    pub async fn regions(&self) -> Result<Vec<CloudRegion>> {
        self.not_yet()
    }
    pub async fn allocate(&self, _request: &AllocationRequest) -> Result<Allocation> {
        self.not_yet()
    }
}

#[async_trait]
impl HostingProvider for RusticCloudProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::RusticCloud
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::NONE
    }
    async fn install(&self, _: &ServerSpec, _: &dyn ProgressSink) -> Result<ServerRef> {
        self.not_yet()
    }
    async fn start(&self, _: &str) -> Result<()> {
        self.not_yet()
    }
    async fn stop(&self, _: &str) -> Result<()> {
        self.not_yet()
    }
    async fn restart(&self, _: &str) -> Result<()> {
        self.not_yet()
    }
    async fn status(&self, _: &str) -> Result<ServerStatus> {
        self.not_yet()
    }
    async fn console_subscribe(&self, _: &str) -> Result<ConsoleSubscription> {
        self.not_yet()
    }
    async fn send_command(&self, _: &str, _: &str) -> Result<CommandOutcome> {
        self.not_yet()
    }
    async fn logs(&self, _: &str, _: usize) -> Result<Vec<String>> {
        self.not_yet()
    }
    async fn players(&self, _: &str) -> Result<PlayersSnapshot> {
        self.not_yet()
    }
    async fn player_action(&self, _: &str, _: &PlayerAction) -> Result<CommandOutcome> {
        self.not_yet()
    }
    async fn config_read(&self, _: &str) -> Result<serde_json::Value> {
        self.not_yet()
    }
    async fn config_write(&self, _: &str, _: &[(String, String)]) -> Result<ConfigOutcome> {
        self.not_yet()
    }
    async fn plugins_list(&self, _: &str) -> Result<Vec<PluginInfo>> {
        self.not_yet()
    }
    async fn plugin_upload(&self, _: &str, _: &Path) -> Result<PluginInfo> {
        self.not_yet()
    }
    async fn plugin_set_enabled(&self, _: &str, _: &str, _: bool) -> Result<()> {
        self.not_yet()
    }
    async fn plugin_remove(&self, _: &str, _: &str) -> Result<()> {
        self.not_yet()
    }
    async fn datapacks_list(&self, _: &str) -> Result<Vec<DatapackInfo>> {
        self.not_yet()
    }
    async fn datapack_upload(&self, _: &str, _: &Path) -> Result<DatapackInfo> {
        self.not_yet()
    }
    async fn datapack_remove(&self, _: &str, _: &str) -> Result<()> {
        self.not_yet()
    }
    async fn backup_create(&self, _: &str, _: Option<&str>) -> Result<String> {
        self.not_yet()
    }
    async fn backups_list(&self, _: &str) -> Result<Vec<BackupInfo>> {
        self.not_yet()
    }
    async fn backup_restore(&self, _: &str, _: &str, _: Option<&str>) -> Result<()> {
        self.not_yet()
    }
    async fn backup_download(&self, _: &str, _: &str, _: &Path) -> Result<u64> {
        self.not_yet()
    }
    async fn update_engine(&self, _: &str, _: &UpdateRequest, _: &dyn ProgressSink) -> Result<()> {
        self.not_yet()
    }
    async fn uninstall(&self, _: &str, _: &UninstallOptions, _: &dyn ProgressSink) -> Result<()> {
        self.not_yet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn everything_is_not_available_yet() {
        let c = RusticCloudProvider::new();
        assert!(!c.capabilities().available);
        assert_eq!(c.start("x").await.unwrap_err().code(), "not_available_yet");
        assert_eq!(c.plans().await.unwrap_err().code(), "not_available_yet");
        let e = c.metrics("x").await.unwrap_err();
        assert!(e.to_string().contains("coming soon"));
    }
}
