//! Ties instances, builds, preparation and launches together, as the app uses them.

use crate::builds::{BuildManager, InstalledBuild};
use crate::error::{InstanceError, Result};
use crate::launch::{GameEvent, LaunchRequest, RunningGame, launch};
use crate::launch_api::{ClientLaunchApi, PrepareLaunchApi, probe_client, probe_preparer};
use crate::model::{Instance, InstanceStore};
use crate::prepare::{self, CheckResult, GameFiles, PrepareEvent, PrepareRequest};
use rusticmc_auth::GameSession;
use rusticmc_launcher_core::{CancellationToken, time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// An instance's game event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InstanceGameEvent {
    pub instance_id: String,
    pub event: GameEvent,
}

/// Preparation progress of a build.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrepareProgress {
    pub build: String,
    pub event: PrepareEvent,
}

/// How a launch authenticates.
#[derive(Clone, Debug)]
pub enum LaunchIdentity {
    /// Online with this session (stdin).
    Online(GameSession),
    /// No session: the game is offline (an owner without network, or an offline profile the owner
    /// gate allowed). Horizon's Launch API has no offline-name option.
    Offline,
}

/// The instance side of the launcher.
pub struct InstanceManager {
    pub store: InstanceStore,
    pub builds: BuildManager,
    prepared_root: PathBuf,
    running: RunningMap,
    events: broadcast::Sender<InstanceGameEvent>,
    prepare_events: broadcast::Sender<PrepareProgress>,
    preparing: Mutex<HashMap<String, CancellationToken>>,
}

impl InstanceManager {
    pub fn new(store: InstanceStore, builds: BuildManager, prepared_root: PathBuf) -> Self {
        Self {
            store,
            builds,
            prepared_root,
            running: Arc::new(Mutex::new(HashMap::new())),
            events: broadcast::channel(1024).0,
            prepare_events: broadcast::channel(256).0,
            preparing: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<InstanceGameEvent> {
        self.events.subscribe()
    }

    pub fn subscribe_prepare(&self) -> broadcast::Receiver<PrepareProgress> {
        self.prepare_events.subscribe()
    }

    /// Where a build's prepared resources go (shared by every instance of the build).
    pub fn prepared_dir(&self, version: &str) -> PathBuf {
        self.prepared_root.join(version)
    }

    /// The build an instance plays: its pinned version, else the newest installed of its source.
    pub fn build_for(&self, instance: &Instance) -> Result<InstalledBuild> {
        match &instance.build.version {
            Some(v) => self.builds.get(v),
            None => self
                .builds
                .installed()
                .into_iter()
                .find(|b| b.source == instance.build.source)
                .ok_or_else(|| InstanceError::BuildNotInstalled(instance.build.source.clone())),
        }
    }

    pub async fn client_api(&self, build: &InstalledBuild) -> Result<ClientLaunchApi> {
        probe_client(&build.client()).await
    }

    pub async fn preparer_api(&self, build: &InstalledBuild) -> Result<PrepareLaunchApi> {
        probe_preparer(&build.preparer()).await
    }

    /// `--check` for a build's resources.
    pub async fn check_prepared(&self, version: &str) -> Result<CheckResult> {
        let build = self.builds.get(version)?;
        prepare::check(&build.preparer(), &self.prepared_dir(version)).await
    }

    /// Prepares a build's resources, with progress on [`Self::subscribe_prepare`].
    pub async fn prepare_build(
        &self,
        version: &str,
        files: GameFiles,
        server_jar: Option<PathBuf>,
        java_home: Option<PathBuf>,
    ) -> Result<()> {
        let build = self.builds.get(version)?;
        let cancel = CancellationToken::new();
        {
            let mut preparing = self.preparing.lock().expect("preparing");
            if preparing.contains_key(version) {
                return Err(InstanceError::Prepare(
                    "already preparing this build".into(),
                ));
            }
            preparing.insert(version.into(), cancel.clone());
        }
        let req = PrepareRequest {
            preparer: build.preparer(),
            output: self.prepared_dir(version),
            files,
            server_jar,
            java_home,
        };
        let events = self.prepare_events.clone();
        let build_name = version.to_owned();
        let result = prepare::prepare(
            &req,
            move |event| {
                let _ = events.send(PrepareProgress {
                    build: build_name.clone(),
                    event,
                });
            },
            &cancel,
        )
        .await;
        self.preparing.lock().expect("preparing").remove(version);
        result
    }

    pub fn cancel_prepare(&self, version: &str) {
        if let Some(c) = self.preparing.lock().expect("preparing").remove(version) {
            c.cancel();
        }
    }

    /// Starts an instance. The caller has passed the owner gate (`identity`). Fails with
    /// [`InstanceError::NotPrepared`] when `--check` finds gaps.
    pub async fn launch(&self, id: &str, identity: LaunchIdentity) -> Result<Option<u32>> {
        self.launch_with(id, identity, None).await
    }

    /// [`Self::launch`], opening a world or server at start (overrides the instance's setting).
    pub async fn launch_with(
        &self,
        id: &str,
        identity: LaunchIdentity,
        quick_play: Option<crate::launch::QuickPlay>,
    ) -> Result<Option<u32>> {
        self.launch_with_client(id, identity, quick_play, None)
            .await
    }

    /// [`Self::launch_with`] with another game program than the build's: the client compiled for
    /// the instance with its native mods (ADR 0157), which reads the build's prepared resources.
    pub async fn launch_with_client(
        &self,
        id: &str,
        identity: LaunchIdentity,
        quick_play: Option<crate::launch::QuickPlay>,
        client: Option<PathBuf>,
    ) -> Result<Option<u32>> {
        if self.running.lock().expect("running").contains_key(id) {
            return Err(InstanceError::AlreadyRunning);
        }
        let instance = self.store.get(id)?;
        let build = self.build_for(&instance)?;
        let client = client.unwrap_or_else(|| build.client());
        let api = probe_client(&client).await?;
        let resources = self.prepared_dir(&build.version);
        let check = prepare::check(&build.preparer(), &resources).await?;
        if !check.current {
            return Err(InstanceError::NotPrepared);
        }
        let game_dir = self.store.game_dir(id);
        let req = LaunchRequest {
            client,
            resources,
            game_dir: game_dir.clone(),
            home: game_dir,
            window: instance.settings.window.clone(),
            quick_play: quick_play.or_else(|| instance.settings.quick_play.clone()),
            extra_args: instance
                .game_args
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
            session: match &identity {
                LaunchIdentity::Online(s) => Some(s.to_json()),
                LaunchIdentity::Offline => None,
            },
        };
        let events = self.events.clone();
        let instance_id = id.to_owned();
        let game = Arc::new(
            launch(&api, req, move |event| {
                let _ = events.send(InstanceGameEvent {
                    instance_id: instance_id.clone(),
                    event: event.clone(),
                });
            })
            .await?,
        );
        let pid = game.pid;
        self.running
            .lock()
            .expect("running")
            .insert(id.to_owned(), game.clone());
        // Playtime and the running list follow the exit.
        let store = self.store.clone();
        let id = id.to_owned();
        let me = self.running.clone();
        tokio::spawn(async move {
            if let GameEvent::Exited { played_seconds, .. } = game.wait().await
                && let Err(e) = store.record_play(&id, played_seconds, time::now())
            {
                tracing::warn!(instance = %id, error = %e, "could not record playtime");
            }
            me.lock().expect("running").remove(&id);
        });
        Ok(pid)
    }

    /// Instances running now.
    pub fn running(&self) -> Vec<String> {
        self.running
            .lock()
            .expect("running")
            .keys()
            .cloned()
            .collect()
    }

    /// Stops a running instance.
    pub async fn kill(&self, id: &str) -> Result<()> {
        let game = self
            .running
            .lock()
            .expect("running")
            .get(id)
            .cloned()
            .ok_or(InstanceError::NotRunning)?;
        game.kill().await;
        Ok(())
    }
}

type RunningMap = Arc<Mutex<HashMap<String, Arc<RunningGame>>>>;
