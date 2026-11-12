//! `rusticmc_hosting::HostingProvider` for this computer's servers (HANDOFF.md, hosting → desktop):
//! the same interface the VPS provider implements, so the app's `server_*` commands treat every
//! server alike. Register it with `rusticmc_hosting::LocalProvider::set_backend`.
//!
//! A server's "name" in the trait is its local id (its folder under `<data>/servers/`).

use crate::control::PlayerAction as ApiAction;
use crate::error::ServerError;
use crate::hosting::{CreateServer, ServerKind};
use crate::manager::ServerManager;
use async_trait::async_trait;
use rusticmc_hosting::model::{
    BackupInfo, CommandOutcome, ConfigOutcome, ConsoleEvent, DatapackInfo, EngineSource, Metrics,
    PlayerAction, PlayersSnapshot, PluginInfo, ProgressEvent, ProgressSink, ProviderKind,
    ServerRef, ServerSpec, ServerStatus, StepState, UninstallOptions, UpdateRequest,
};
use rusticmc_hosting::provider::{Capabilities, ConsoleSubscription, HostingProvider};
use rusticmc_hosting::{HostingError, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// `settings` key a spec may carry to pick the Horizon build (stripped before `config set`).
pub const BUILD_SETTING: &str = "launcher.build";

/// Local servers behind the hosting trait.
#[derive(Clone)]
pub struct LocalServers {
    pub manager: Arc<ServerManager>,
    /// The build new servers use when the spec names none.
    pub default_build: Arc<dyn Fn() -> Option<String> + Send + Sync>,
}

impl From<ServerError> for HostingError {
    fn from(e: ServerError) -> Self {
        match e {
            ServerError::UnknownServer(s) => HostingError::UnknownServer(s),
            ServerError::Invalid(s) => HostingError::Invalid(s),
            ServerError::EulaNotAccepted => HostingError::Precondition(e.to_string()),
            ServerError::NotRunning => HostingError::Precondition(e.to_string()),
            ServerError::Io { source, .. } => HostingError::Io(source),
            other => HostingError::Precondition(other.to_string()),
        }
    }
}

fn step(progress: &dyn ProgressSink, step: &str, title: &str, index: usize, state: StepState) {
    progress.emit(ProgressEvent {
        operation: "install".into(),
        step: step.into(),
        title: title.into(),
        index,
        total: 3,
        state,
        detail: None,
        bytes: None,
    });
}

impl LocalServers {
    fn world_dir(&self, server: &str) -> Result<PathBuf> {
        let dir = self.manager.dir(server);
        let text = std::fs::read_to_string(dir.join("server.toml")).unwrap_or_default();
        let world = toml::from_str::<toml::Table>(&text)
            .ok()
            .and_then(|t| {
                t.get("world")
                    .and_then(|w| w.get("name"))
                    .and_then(|n| n.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "world".into());
        if world.contains(['/', '\\']) || world.starts_with('.') {
            return Err(HostingError::Invalid("bad world name".into()));
        }
        Ok(dir.join(world))
    }

    async fn cli(&self, server: &str, args: Vec<String>) -> Result<String> {
        let profile = self.manager.profile(server)?;
        let out = self
            .manager
            .supervisor(&profile)?
            .run(&args, Duration::from_secs(120))
            .await?;
        if out.code == Some(0) {
            Ok(out.stdout)
        } else {
            Err(HostingError::RemoteCommand {
                command: args.join(" "),
                status: out.code.unwrap_or(-1) as u32,
                stderr: out.stderr.trim().to_owned(),
            })
        }
    }
}

fn api_action(a: &PlayerAction) -> ApiAction {
    match a.clone() {
        PlayerAction::Kick { name, reason } => ApiAction::Kick { name, reason },
        PlayerAction::Ban { name, reason } => ApiAction::Ban { name, reason },
        PlayerAction::Pardon { name } => ApiAction::Pardon { name },
        PlayerAction::BanIp { target, reason } => ApiAction::BanIp { target, reason },
        PlayerAction::PardonIp { ip } => ApiAction::PardonIp { ip },
        PlayerAction::Op { name } => ApiAction::Op { name },
        PlayerAction::Deop { name } => ApiAction::Deop { name },
        PlayerAction::WhitelistAdd { name } => ApiAction::WhitelistAdd { name },
        PlayerAction::WhitelistRemove { name } => ApiAction::WhitelistRemove { name },
        PlayerAction::WhitelistEnabled { enabled } => ApiAction::Whitelist { enabled },
    }
}

#[async_trait]
impl HostingProvider for LocalServers {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            firewall: false,
            ..Capabilities::ALL
        }
    }

    async fn install(&self, spec: &ServerSpec, progress: &dyn ProgressSink) -> Result<ServerRef> {
        let mut settings = spec.settings.clone();
        let build = settings
            .remove(BUILD_SETTING)
            .or_else(|| (self.default_build)())
            .ok_or_else(|| HostingError::Precondition("install a Horizon build first".into()))?;
        let (kind, engine_path) = match &spec.engine {
            EngineSource::Horizon => (ServerKind::Horizon, None),
            EngineSource::RusticUpload { path, .. } => {
                (ServerKind::RusticServer, Some(path.clone()))
            }
            EngineSource::RusticRelease { .. } => (ServerKind::RusticServer, None),
        };
        step(
            progress,
            "init-server",
            "Set up the server",
            0,
            StepState::Running,
        );
        let profile = self
            .manager
            .create(CreateServer {
                name: spec.name.clone(),
                kind,
                build,
                eula_accepted: spec.eula_accepted,
                world: Some(spec.world.clone()),
                seed: spec.seed.clone(),
                port: Some(spec.port),
                whitelist: spec.whitelist,
                players: spec.players.clone(),
                op: spec.op.clone(),
                online_mode: spec.online_mode,
                engine_path,
            })
            .await?;
        step(
            progress,
            "init-server",
            "Set up the server",
            0,
            StepState::Done,
        );
        step(progress, "configure", "Settings", 1, StepState::Running);
        if !settings.is_empty() {
            self.manager
                .set_config(&profile.id, settings.into_iter().collect())
                .await?;
        }
        step(progress, "configure", "Settings", 1, StepState::Done);
        Ok(ServerRef {
            provider: ProviderKind::Local,
            target: String::new(),
            name: profile.id,
        })
    }

    async fn start(&self, server: &str) -> Result<()> {
        Ok(self.manager.start_server(server).await?)
    }

    async fn stop(&self, server: &str) -> Result<()> {
        Ok(self.manager.stop_server(server).await?)
    }

    async fn restart(&self, server: &str) -> Result<()> {
        Ok(self.manager.restart(server).await?)
    }

    async fn status(&self, server: &str) -> Result<ServerStatus> {
        self.manager.profile(server)?;
        match self.manager.live_status(server).await {
            Some(s) => {
                let v =
                    serde_json::to_value(&s).map_err(|e| HostingError::Protocol(e.to_string()))?;
                Ok(ServerStatus::from_horizon(&v))
            }
            None => Ok(ServerStatus {
                phase: "stopped".into(),
                ..ServerStatus::default()
            }),
        }
    }

    async fn metrics(&self, server: &str) -> Result<Metrics> {
        Ok(self.status(server).await?.metrics)
    }

    async fn console_subscribe(&self, server: &str) -> Result<ConsoleSubscription> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(256);
        self.manager.stream_events(server, None, tx)?;
        let (out_tx, out_rx) = tokio::sync::mpsc::channel(256);
        let task = tokio::spawn(async move {
            while let Some(e) = rx.recv().await {
                let event = match e.kind.as_str() {
                    "log" => ConsoleEvent::Log {
                        line: e.data["line"].as_str().unwrap_or_default().to_owned(),
                    },
                    "phase" => ConsoleEvent::Phase {
                        phase: e.data["phase"].as_str().unwrap_or_default().to_owned(),
                    },
                    kind => ConsoleEvent::Game {
                        kind: kind.to_owned(),
                        data: e.data,
                    },
                };
                if out_tx.send(event).await.is_err() {
                    return;
                }
            }
            let _ = out_tx
                .send(ConsoleEvent::Closed {
                    reason: "the event stream ended".into(),
                })
                .await;
        });
        Ok(ConsoleSubscription::new(out_rx, task))
    }

    async fn send_command(&self, server: &str, line: &str) -> Result<CommandOutcome> {
        Ok(CommandOutcome {
            output: self.manager.command(server, line).await?,
        })
    }

    async fn logs(&self, server: &str, lines: usize) -> Result<Vec<String>> {
        match self.manager.control(server) {
            Ok(c) => {
                let (_, all) = c.logs(0).await?;
                Ok(all.into_iter().rev().take(lines).rev().collect())
            }
            Err(ServerError::NotRunning) => {
                let path = self.manager.dir(server).join("logs").join("latest.log");
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let all: Vec<String> = text.lines().map(str::to_owned).collect();
                Ok(all.into_iter().rev().take(lines).rev().collect())
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn players(&self, server: &str) -> Result<PlayersSnapshot> {
        match self.manager.control(server) {
            Ok(c) => Ok(PlayersSnapshot::from_horizon(&c.players().await?)),
            Err(ServerError::NotRunning) => {
                let out = self
                    .cli(server, vec!["players".into(), "--json".into()])
                    .await?;
                let v: serde_json::Value = serde_json::from_str(out.trim())
                    .map_err(|e| HostingError::Protocol(e.to_string()))?;
                Ok(PlayersSnapshot::from_horizon(&v))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn player_action(&self, server: &str, action: &PlayerAction) -> Result<CommandOutcome> {
        match self.manager.control(server) {
            Ok(c) => Ok(CommandOutcome {
                output: c.player_action(&api_action(action)).await?.output,
            }),
            // Stopped: the CLI edits the lists themselves (server-reference.md).
            Err(ServerError::NotRunning) => {
                let out = self.cli(server, action.horizon_args()).await?;
                Ok(CommandOutcome {
                    output: out.lines().map(str::to_owned).collect(),
                })
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn config_read(&self, server: &str) -> Result<serde_json::Value> {
        Ok(self.manager.config(server).await?)
    }

    async fn config_write(
        &self,
        server: &str,
        changes: &[(String, String)],
    ) -> Result<ConfigOutcome> {
        let o = self.manager.set_config(server, changes.to_vec()).await?;
        Ok(ConfigOutcome {
            saved: o.saved,
            applied: o.applied,
            restart: o.restart,
        })
    }

    async fn plugins_list(&self, server: &str) -> Result<Vec<PluginInfo>> {
        let (plugins, _) = self.manager.plugins(server)?.list();
        Ok(plugins
            .into_iter()
            .map(|p| PluginInfo {
                id: p.manifest.id,
                file: p
                    .file
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                enabled: p.enabled,
                bytes: std::fs::metadata(&p.file).map_or(0, |m| m.len()),
            })
            .collect())
    }

    async fn plugin_upload(&self, server: &str, rplugin: &Path) -> Result<PluginInfo> {
        let p = self.manager.plugins(server)?.install(rplugin, false)?;
        Ok(PluginInfo {
            bytes: std::fs::metadata(&p.file).map_or(0, |m| m.len()),
            file: p
                .file
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            id: p.manifest.id,
            enabled: p.enabled,
        })
    }

    async fn plugin_set_enabled(&self, server: &str, id: &str, enabled: bool) -> Result<()> {
        self.manager.plugins(server)?.set_enabled(id, enabled)?;
        Ok(())
    }

    async fn plugin_remove(&self, server: &str, id: &str) -> Result<()> {
        Ok(self.manager.plugins(server)?.remove(id)?)
    }

    async fn datapacks_list(&self, server: &str) -> Result<Vec<DatapackInfo>> {
        let dir = self.world_dir(server)?.join("datapacks");
        let mut out: Vec<DatapackInfo> = std::fs::read_dir(&dir)
            .map(|e| {
                e.flatten()
                    .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                    .map(|e| DatapackInfo {
                        file: e.file_name().to_string_lossy().into_owned(),
                        bytes: if e.path().is_dir() {
                            rusticmc_launcher_core::fsutil::dir_size(&e.path())
                        } else {
                            e.metadata().map_or(0, |m| m.len())
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.file.cmp(&b.file));
        Ok(out)
    }

    async fn datapack_upload(&self, server: &str, zip: &Path) -> Result<DatapackInfo> {
        let dir = self.world_dir(server)?.join("datapacks");
        std::fs::create_dir_all(&dir)?;
        let name = zip
            .file_name()
            .ok_or_else(|| HostingError::Invalid("no file name".into()))?;
        let target = dir.join(name);
        let bytes = std::fs::copy(zip, &target)?;
        Ok(DatapackInfo {
            file: name.to_string_lossy().into_owned(),
            bytes,
        })
    }

    async fn datapack_remove(&self, server: &str, file: &str) -> Result<()> {
        if file.contains(['/', '\\']) || file.starts_with('.') {
            return Err(HostingError::Invalid("bad datapack name".into()));
        }
        let path = self.world_dir(server)?.join("datapacks").join(file);
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    async fn backup_create(&self, server: &str, name: Option<&str>) -> Result<String> {
        Ok(self.manager.backup(server, name.map(str::to_owned)).await?)
    }

    async fn backups_list(&self, server: &str) -> Result<Vec<BackupInfo>> {
        let dir = self.manager.dir(server).join("backups");
        Ok(self
            .manager
            .backups(server)
            .await?
            .into_iter()
            .map(|b| BackupInfo {
                modified: std::fs::metadata(dir.join(&b.name))
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
                name: b.name,
                bytes: b.bytes,
            })
            .collect())
    }

    async fn backup_restore(
        &self,
        server: &str,
        backup: &str,
        dimension: Option<&str>,
    ) -> Result<()> {
        Ok(self
            .manager
            .restore(server, backup, dimension.map(str::to_owned))
            .await?)
    }

    async fn backup_download(&self, server: &str, backup: &str, dest: &Path) -> Result<u64> {
        if backup.contains(['/', '\\']) || backup.starts_with('.') {
            return Err(HostingError::Invalid("bad backup name".into()));
        }
        let src = self.manager.dir(server).join("backups").join(backup);
        Ok(std::fs::copy(src, dest)?)
    }

    async fn update_engine(
        &self,
        server: &str,
        request: &UpdateRequest,
        progress: &dyn ProgressSink,
    ) -> Result<()> {
        step(progress, "update", "Update", 0, StepState::Running);
        if let Some(EngineSource::RusticUpload { path, .. }) = &request.engine {
            let path = path.clone();
            self.manager.update_profile(server, |p| {
                p.kind = ServerKind::RusticServer;
                p.engine = Some(path);
            })?;
        }
        let profile = self.manager.profile(server)?;
        let build = match request.horizon_version.as_deref() {
            Some("") => (self.default_build)().unwrap_or(profile.build.clone()),
            Some(v) => v.to_owned(),
            None => profile.build.clone(),
        };
        self.manager.change_build(server, &build).await?;
        step(progress, "update", "Update", 0, StepState::Done);
        Ok(())
    }

    async fn uninstall(
        &self,
        server: &str,
        options: &UninstallOptions,
        _progress: &dyn ProgressSink,
    ) -> Result<()> {
        Ok(self.manager.delete(server, !options.delete_files).await?)
    }
}
