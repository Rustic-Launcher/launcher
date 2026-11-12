//! `LocalProvider`: a server on this PC. The desktop engineer owns local servers
//! (`rusticmc-servers`: folders, the local `horizon-server`, the game files prepared by the
//! launcher). This adapter only forwards to the backend they register, so the UI and the IPC layer
//! see one `HostingProvider` interface for every mode (coordination: launcher/docs/HANDOFF.md).
//!
//! Until a backend is registered every call answers `Unsupported`, with a clear message.
//! What is local-only (how friends reach the server) lives in [`crate::access`].

use crate::error::{HostingError, Result};
use crate::model::*;
use crate::provider::{Capabilities, ConsoleSubscription, HostingProvider};
use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct LocalProvider {
    backend: RwLock<Option<Arc<dyn HostingProvider>>>,
}

impl LocalProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wire the desktop's implementation (`rusticmc-servers`).
    pub fn with_backend(backend: Arc<dyn HostingProvider>) -> Self {
        let p = Self::default();
        p.set_backend(backend);
        p
    }

    pub fn set_backend(&self, backend: Arc<dyn HostingProvider>) {
        *self.backend.write().unwrap() = Some(backend);
    }

    fn backend(&self) -> Result<Arc<dyn HostingProvider>> {
        self.backend.read().unwrap().clone().ok_or_else(|| {
            HostingError::Unsupported(
                "local servers are not wired yet (rusticmc-servers registers its backend)".into(),
            )
        })
    }
}

#[async_trait]
impl HostingProvider for LocalProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
    }
    fn capabilities(&self) -> Capabilities {
        match self.backend.read().unwrap().as_ref() {
            Some(b) => b.capabilities(),
            None => Capabilities::NONE,
        }
    }
    async fn install(&self, spec: &ServerSpec, progress: &dyn ProgressSink) -> Result<ServerRef> {
        self.backend()?.install(spec, progress).await
    }
    async fn start(&self, server: &str) -> Result<()> {
        self.backend()?.start(server).await
    }
    async fn stop(&self, server: &str) -> Result<()> {
        self.backend()?.stop(server).await
    }
    async fn restart(&self, server: &str) -> Result<()> {
        self.backend()?.restart(server).await
    }
    async fn status(&self, server: &str) -> Result<ServerStatus> {
        self.backend()?.status(server).await
    }
    async fn metrics(&self, server: &str) -> Result<Metrics> {
        self.backend()?.metrics(server).await
    }
    async fn console_subscribe(&self, server: &str) -> Result<ConsoleSubscription> {
        self.backend()?.console_subscribe(server).await
    }
    async fn send_command(&self, server: &str, line: &str) -> Result<CommandOutcome> {
        self.backend()?.send_command(server, line).await
    }
    async fn logs(&self, server: &str, lines: usize) -> Result<Vec<String>> {
        self.backend()?.logs(server, lines).await
    }
    async fn players(&self, server: &str) -> Result<PlayersSnapshot> {
        self.backend()?.players(server).await
    }
    async fn player_action(&self, server: &str, action: &PlayerAction) -> Result<CommandOutcome> {
        self.backend()?.player_action(server, action).await
    }
    async fn config_read(&self, server: &str) -> Result<serde_json::Value> {
        self.backend()?.config_read(server).await
    }
    async fn config_write(
        &self,
        server: &str,
        changes: &[(String, String)],
    ) -> Result<ConfigOutcome> {
        self.backend()?.config_write(server, changes).await
    }
    async fn plugins_list(&self, server: &str) -> Result<Vec<PluginInfo>> {
        self.backend()?.plugins_list(server).await
    }
    async fn plugin_upload(&self, server: &str, rplugin: &Path) -> Result<PluginInfo> {
        // Same check as for a VPS before anything is copied.
        crate::rplugin::read_manifest(rplugin)?;
        self.backend()?.plugin_upload(server, rplugin).await
    }
    async fn plugin_set_enabled(&self, server: &str, id: &str, enabled: bool) -> Result<()> {
        self.backend()?
            .plugin_set_enabled(server, id, enabled)
            .await
    }
    async fn plugin_remove(&self, server: &str, id: &str) -> Result<()> {
        self.backend()?.plugin_remove(server, id).await
    }
    async fn datapacks_list(&self, server: &str) -> Result<Vec<DatapackInfo>> {
        self.backend()?.datapacks_list(server).await
    }
    async fn datapack_upload(&self, server: &str, zip: &Path) -> Result<DatapackInfo> {
        self.backend()?.datapack_upload(server, zip).await
    }
    async fn datapack_remove(&self, server: &str, file: &str) -> Result<()> {
        self.backend()?.datapack_remove(server, file).await
    }
    async fn backup_create(&self, server: &str, name: Option<&str>) -> Result<String> {
        self.backend()?.backup_create(server, name).await
    }
    async fn backups_list(&self, server: &str) -> Result<Vec<BackupInfo>> {
        self.backend()?.backups_list(server).await
    }
    async fn backup_restore(
        &self,
        server: &str,
        backup: &str,
        dimension: Option<&str>,
    ) -> Result<()> {
        self.backend()?
            .backup_restore(server, backup, dimension)
            .await
    }
    async fn backup_download(&self, server: &str, backup: &str, dest: &Path) -> Result<u64> {
        self.backend()?.backup_download(server, backup, dest).await
    }
    async fn update_engine(
        &self,
        server: &str,
        request: &UpdateRequest,
        progress: &dyn ProgressSink,
    ) -> Result<()> {
        self.backend()?
            .update_engine(server, request, progress)
            .await
    }
    async fn uninstall(
        &self,
        server: &str,
        options: &UninstallOptions,
        progress: &dyn ProgressSink,
    ) -> Result<()> {
        self.backend()?.uninstall(server, options, progress).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::RusticCloudProvider;

    #[tokio::test]
    async fn unwired_is_a_clear_error() {
        let p = LocalProvider::new();
        assert_eq!(p.capabilities(), Capabilities::NONE);
        let e = p.status("x").await.unwrap_err();
        assert_eq!(e.code(), "unsupported");
    }

    #[tokio::test]
    async fn delegates_to_the_backend() {
        // Any provider works as a backend; the cloud stub makes delegation visible.
        let p = LocalProvider::with_backend(Arc::new(RusticCloudProvider::new()));
        let e = p.status("x").await.unwrap_err();
        assert_eq!(e.code(), "not_available_yet");
        assert_eq!(p.kind(), ProviderKind::Local);
    }
}
