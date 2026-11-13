//! Local servers: one folder per server under `<data>/servers/<id>/`, which is `horizon-server`'s
//! `--dir` (server.toml, the world, logs, backups, plugins), plus the launcher's own
//! `.rustic/profile.toml`. The supervisor program comes from the server's Horizon build;
//! `engine.resources` points at that build's prepared game files; a Rustic Server's
//! `engine.path` points at the `rustic-server` program ([`crate::engine`] says which one).

use crate::console::{CommandTree, Completion};
use crate::control::{
    ActionReply, Backup, ConfigOutcome, ControlClient, Endpoint, MetricSample, Phase, PlayerAction,
    ServerEvent,
};
use crate::engine::{EngineSearch, RusticServerProgram, find_rustic_server};
use crate::error::{Result, ServerError};
use crate::hosting::{CreateServer, HostedServer, HostingKind, ServerKind, ServerState};
use crate::invite::{Endpoint as InviteEndpoint, Invite};
use crate::network::{self, LocalAddress, PortMapping};
use crate::plugins::PluginManager;
use crate::supervisor::{InitOptions, Supervisor};
use rusticmc_instances::BuildManager;
use rusticmc_launcher_core::{CancellationToken, fsutil, reqwest, time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;

/// The launcher's record of a local server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub kind: ServerKind,
    pub build: String,
    pub port: u16,
    pub created_at: u64,
    /// Back the world up before switching builds.
    #[serde(default = "yes")]
    pub backup_before_update: bool,
    /// Ask the router to forward the port while it runs.
    #[serde(default)]
    pub upnp: bool,
    /// The server's hostname when a domain is configured.
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub icon: rusticmc_instances::InstanceIcon,
    /// Memory the owner wants for it (shown; Horizon's server has no memory option, RISKS.md).
    #[serde(default)]
    pub memory_mb: Option<u32>,
    /// Start with the launcher.
    #[serde(default)]
    pub start_with_launcher: bool,
    #[serde(default)]
    pub last_started_at: Option<u64>,
    /// Keep it reachable through Rustic Relay while it runs.
    #[serde(default)]
    pub relay: bool,
    /// The temporary relay label asked for, if any.
    #[serde(default)]
    pub relay_label: Option<String>,
    /// This server's id in Rustic Cloud, once registered there (the relay's tokens are signed
    /// for it).
    #[serde(default)]
    pub cloud_id: Option<String>,
    /// A `rustic-server` program for this server only (an uploaded or downloaded engine), else
    /// the launcher's configured one.
    #[serde(default)]
    pub engine: Option<PathBuf>,
}

fn yes() -> bool {
    true
}

const PROFILE: &str = ".rustic/profile.toml";

/// Removes a server's folder. A server that was just stopped may still hold files open for a few
/// seconds while the game and the supervisor exit (on Windows they cannot be deleted until then):
/// tries again until `patience` runs out.
async fn remove_dir_patiently(dir: &Path, patience: std::time::Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + patience;
    loop {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err(e) => return Err(ServerError::io(dir, e)),
        }
    }
}

/// Local servers.
pub struct ServerManager {
    root: PathBuf,
    http: reqwest::Client,
    builds: BuildManager,
    prepared_root: PathBuf,
    /// The launcher's manual override of the `rustic-server` program (Settings).
    rustic_server: RwLock<Option<PathBuf>>,
    mappings: Mutex<HashMap<String, PortMapping>>,
    tunnels: Mutex<HashMap<String, rusticmc_tunnel::TunnelHandle>>,
    streams: Mutex<HashMap<String, CancellationToken>>,
}

impl ServerManager {
    pub fn new(
        root: PathBuf,
        http: reqwest::Client,
        builds: BuildManager,
        prepared_root: PathBuf,
        rustic_server: Option<PathBuf>,
    ) -> Self {
        Self {
            root,
            http,
            builds,
            prepared_root,
            rustic_server: RwLock::new(rustic_server),
            mappings: Mutex::new(HashMap::new()),
            tunnels: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
        }
    }

    pub fn dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    fn check_id(id: &str) -> Result<()> {
        if id.is_empty() || id.contains(['/', '\\', '.']) {
            return Err(ServerError::UnknownServer(id.into()));
        }
        Ok(())
    }

    pub fn profiles(&self) -> Vec<ServerProfile> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut out: Vec<ServerProfile> = entries
            .flatten()
            .filter_map(|e| fsutil::read_toml(&e.path().join(PROFILE)).ok().flatten())
            .collect();
        out.sort_by_key(|a| a.name.to_lowercase());
        out
    }

    pub fn profile(&self, id: &str) -> Result<ServerProfile> {
        Self::check_id(id)?;
        fsutil::read_toml(&self.dir(id).join(PROFILE))?
            .ok_or_else(|| ServerError::UnknownServer(id.into()))
    }

    fn save_profile(&self, p: &ServerProfile) -> Result<()> {
        Ok(fsutil::write_toml(&self.dir(&p.id).join(PROFILE), p)?)
    }

    /// Edits the launcher-side settings of a server.
    pub fn update_profile(
        &self,
        id: &str,
        edit: impl FnOnce(&mut ServerProfile),
    ) -> Result<ServerProfile> {
        let mut p = self.profile(id)?;
        edit(&mut p);
        p.id = id.to_owned();
        self.save_profile(&p)?;
        Ok(p)
    }

    pub fn supervisor(&self, profile: &ServerProfile) -> Result<Supervisor> {
        let build = self.builds.get(&profile.build)?;
        let program = build.server_supervisor();
        if !program.is_file() {
            return Err(ServerError::Supervisor(format!(
                "build {} has no horizon-server",
                profile.build
            )));
        }
        Ok(Supervisor::new(program, self.dir(&profile.id)))
    }

    /// The control API of a running supervisor.
    pub fn control(&self, id: &str) -> Result<ControlClient> {
        Self::check_id(id)?;
        ControlClient::open(self.http.clone(), &self.dir(id))
    }

    /// The launcher's manual override of the `rustic-server` program, if any.
    pub fn configured_rustic_server(&self) -> Option<PathBuf> {
        self.rustic_server.read().expect("rustic_server").clone()
    }

    /// Changes the manual override (Settings); `None` goes back to the automatic lookup. Servers
    /// pick it up at their next start.
    pub fn set_rustic_server(&self, configured: Option<PathBuf>) {
        *self.rustic_server.write().expect("rustic_server") = configured;
    }

    /// The `rustic-server` a server runs: its own engine, the override, its build's, beside the
    /// launcher, the PATH ([`crate::engine`]).
    pub fn rustic_server_for(&self, profile: &ServerProfile) -> Option<RusticServerProgram> {
        let configured = self.configured_rustic_server();
        let build = self.builds.get(&profile.build).ok();
        find_rustic_server(EngineSearch {
            server: profile.engine.as_deref(),
            configured: configured.as_deref(),
            build_dir: build.as_ref().map(|b| b.dir.as_path()),
        })
    }

    /// The `rustic-server` a new server of `build` would run (the Settings row, VPS installs).
    pub fn rustic_server_for_build(&self, build: Option<&str>) -> Option<RusticServerProgram> {
        let configured = self.configured_rustic_server();
        let build = build.and_then(|b| self.builds.get(b).ok());
        find_rustic_server(EngineSearch {
            server: None,
            configured: configured.as_deref(),
            build_dir: build.as_ref().map(|b| b.dir.as_path()),
        })
    }

    /// The settings that tie a server to its build and engine.
    fn engine_settings(&self, profile: &ServerProfile) -> Result<Vec<(String, String)>> {
        let mut changes = vec![(
            "engine.resources".to_owned(),
            self.prepared_root
                .join(&profile.build)
                .display()
                .to_string(),
        )];
        match profile.kind {
            ServerKind::RusticServer => {
                let exe = self
                    .rustic_server_for(profile)
                    .ok_or(ServerError::NoRusticServer)?;
                changes.push(("engine.path".into(), exe.path.display().to_string()));
            }
            ServerKind::Horizon => changes.push(("engine.path".into(), String::new())),
        }
        Ok(changes)
    }

    /// Creates a server: `horizon-server init` with the player's answers, then the engine settings.
    pub async fn create(&self, req: CreateServer) -> Result<ServerProfile> {
        if !req.eula_accepted {
            return Err(ServerError::EulaNotAccepted);
        }
        if req.name.trim().is_empty() {
            return Err(ServerError::Invalid("A server needs a name.".into()));
        }
        let mut id = fsutil::slug(&req.name);
        let base = id.clone();
        let mut n = 2;
        while self.dir(&id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        let profile = ServerProfile {
            id: id.clone(),
            name: req.name.trim().into(),
            kind: req.kind,
            build: req.build.clone(),
            port: req.port.unwrap_or(25565),
            created_at: time::now(),
            backup_before_update: true,
            upnp: false,
            hostname: None,
            icon: rusticmc_instances::InstanceIcon::default(),
            memory_mb: None,
            start_with_launcher: false,
            last_started_at: None,
            relay: false,
            relay_label: None,
            cloud_id: None,
            engine: req.engine_path.clone(),
        };
        let engine = self.engine_settings(&profile)?;
        let supervisor = self.supervisor(&profile)?;
        let result = async {
            supervisor
                .init(&InitOptions {
                    eula: true,
                    world: req.world.clone(),
                    seed: req.seed.clone(),
                    port: Some(profile.port),
                    whitelist: req.whitelist,
                    players: req.players.clone(),
                    op: req.op.clone(),
                    online_mode: req.online_mode,
                })
                .await?;
            supervisor.config_set(&engine).await?;
            self.save_profile(&profile)
        }
        .await;
        if let Err(e) = result {
            let _ = std::fs::remove_dir_all(self.dir(&id));
            return Err(e);
        }
        Ok(profile)
    }

    /// Whether a supervisor answers for this server, and its status.
    pub async fn live_status(&self, id: &str) -> Option<crate::control::Status> {
        let control = self.control(id).ok()?;
        control.status().await.ok()
    }

    /// Starts the server (resumes a stopped supervisor through the API, else `horizon-server
    /// start`). Maps the port when the profile asks for it.
    pub async fn start_server(&self, id: &str) -> Result<()> {
        let profile = self.profile(id)?;
        match self.live_status(id).await {
            Some(s) if s.phase == Phase::Stopped => self.control(id)?.start().await?,
            Some(_) => return Ok(()),
            None => {
                // The program and resources may have moved since creation (a rebuilt engine).
                let supervisor = self.supervisor(&profile)?;
                supervisor
                    .config_set(&self.engine_settings(&profile)?)
                    .await?;
                supervisor.start().await?;
            }
        }
        let _ = self.update_profile(id, |p| p.last_started_at = Some(time::now()));
        if profile.upnp
            && let Err(e) = self.map_port(id).await
        {
            tracing::warn!(server = id, error = %e, "port mapping failed");
        }
        Ok(())
    }

    /// Stops the game and the supervisor.
    pub async fn stop_server(&self, id: &str) -> Result<()> {
        if let Some(cancel) = self.streams.lock().expect("streams").remove(id) {
            cancel.cancel();
        }
        let mapping = self.mappings.lock().expect("mappings").remove(id);
        if let Some(m) = mapping {
            let _ = network::unmap_port(&m).await;
        }
        match self.control(id) {
            Ok(c) => c.stop().await,
            Err(ServerError::NotRunning) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Switches a server to another installed build: backup first (when set), stop, new
    /// program and resources, start again if it was running.
    pub async fn change_build(&self, id: &str, build: &str) -> Result<ServerProfile> {
        self.builds.get(build)?;
        let mut profile = self.profile(id)?;
        let was_running = self
            .live_status(id)
            .await
            .is_some_and(|s| s.phase != Phase::Stopped);
        if profile.backup_before_update {
            if was_running {
                self.control(id)?.backup(Some("before-update")).await?;
            } else {
                // `horizon-server backup NAME` works on a stopped server too.
                self.supervisor(&profile)?
                    .run(
                        &["backup".into(), "before-update".into()],
                        std::time::Duration::from_secs(600),
                    )
                    .await?;
            }
        }
        if was_running {
            self.stop_server(id).await?;
        }
        profile.build = build.into();
        self.save_profile(&profile)?;
        self.supervisor(&profile)?
            .config_set(&self.engine_settings(&profile)?)
            .await?;
        if was_running {
            self.start_server(id).await?;
        }
        Ok(profile)
    }

    /// Console completion: the command tree of the server's prepared files, and online players.
    pub async fn complete(&self, id: &str, line: &str) -> Result<Vec<Completion>> {
        let profile = self.profile(id)?;
        let tree = CommandTree::load(&self.prepared_root.join(&profile.build)).unwrap_or_default();
        let players: Vec<String> = match self.control(id) {
            Ok(c) => c
                .players()
                .await
                .ok()
                .and_then(|v| {
                    v["online"].as_array().map(|a| {
                        a.iter()
                            .filter_map(|p| p["name"].as_str().map(str::to_owned))
                            .collect()
                    })
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Ok(tree.complete(line, &players))
    }

    /// Streams a server's events (logs, joins, chat...) into `tx` until stopped or replaced.
    pub fn stream_events(
        &self,
        id: &str,
        since: Option<u64>,
        tx: mpsc::Sender<ServerEvent>,
    ) -> Result<()> {
        let control = self.control(id)?;
        let cancel = CancellationToken::new();
        if let Some(old) = self
            .streams
            .lock()
            .expect("streams")
            .insert(id.to_owned(), cancel.clone())
        {
            old.cancel();
        }
        let id = id.to_owned();
        tokio::spawn(async move {
            if let Err(e) = control.stream_events(since, tx, cancel).await {
                tracing::debug!(server = %id, error = %e, "event stream ended");
            }
        });
        Ok(())
    }

    pub fn stop_events(&self, id: &str) {
        if let Some(c) = self.streams.lock().expect("streams").remove(id) {
            c.cancel();
        }
    }

    pub async fn metrics_history(&self, id: &str, since: Option<u64>) -> Result<Vec<MetricSample>> {
        self.control(id)?.metrics(since).await
    }

    /// Automatic backups: every (`30m`, `6h`, `1d`, `0` off) and how many to keep.
    pub async fn set_backup_schedule(
        &self,
        id: &str,
        every: &str,
        keep: u32,
    ) -> Result<ConfigOutcome> {
        let profile = self.profile(id)?;
        self.supervisor(&profile)?
            .config_set(&[
                ("backups.every".into(), every.into()),
                ("backups.keep".into(), keep.to_string()),
            ])
            .await
    }

    pub async fn delete_backup(&self, id: &str, name: &str) -> Result<()> {
        self.control(id)?.delete_backup(name).await
    }

    pub fn plugins(&self, id: &str) -> Result<PluginManager> {
        let profile = self.profile(id)?;
        if profile.kind != ServerKind::RusticServer {
            return Err(ServerError::Plugin("plugins need a Rustic Server".into()));
        }
        Ok(PluginManager::new(&self.dir(id)))
    }

    // ---- access ----------------------------------------------------------------------------

    /// The addresses players on the LAN (or a VPN) use.
    pub fn lan_addresses(&self, id: &str) -> Result<Vec<(LocalAddress, String)>> {
        let port = self.profile(id)?.port;
        Ok(network::local_addresses()
            .into_iter()
            .map(|a| {
                let shown = match a.ip {
                    std::net::IpAddr::V6(v6) => format!("[{v6}]:{port}"),
                    ip if port == 25565 => ip.to_string(),
                    ip => format!("{ip}:{port}"),
                };
                (a, shown)
            })
            .collect())
    }

    /// Asks the router to forward the server's port (UPnP, then NAT-PMP).
    pub async fn map_port(&self, id: &str) -> Result<PortMapping> {
        let profile = self.profile(id)?;
        let mapping =
            network::map_port(profile.port, &format!("Rustic server {}", profile.name)).await?;
        self.mappings
            .lock()
            .expect("mappings")
            .insert(id.to_owned(), mapping.clone());
        Ok(mapping)
    }

    pub async fn unmap_port(&self, id: &str) -> Result<()> {
        let mapping = self.mappings.lock().expect("mappings").remove(id);
        match mapping {
            Some(m) => network::unmap_port(&m).await,
            None => Ok(()),
        }
    }

    pub fn port_mapping(&self, id: &str) -> Option<PortMapping> {
        self.mappings.lock().expect("mappings").get(id).cloned()
    }

    /// Exposes the server through Rustic Relay (`rusticmc-tunnel`).
    pub fn start_relay(
        &self,
        id: &str,
        provider: &dyn rusticmc_tunnel::TunnelProvider,
        tokens: Arc<dyn rusticmc_tunnel::TokenSource>,
    ) -> Result<()> {
        let port = self.profile(id)?.port;
        let handle = provider
            .start(rusticmc_tunnel::TunnelRequest {
                local_addr: SocketAddr::from(([127, 0, 0, 1], port)),
                token_source: tokens,
                proxy_protocol: false,
            })
            .map_err(|e| ServerError::Relay(e.to_string()))?;
        self.tunnels
            .lock()
            .expect("tunnels")
            .insert(id.to_owned(), handle);
        Ok(())
    }

    /// The relay hostnames now (empty until connected).
    pub fn relay_hostnames(&self, id: &str) -> Vec<String> {
        self.tunnels
            .lock()
            .expect("tunnels")
            .get(id)
            .map(|h| h.status().routes.hostnames)
            .unwrap_or_default()
    }

    pub fn relay_status(&self, id: &str) -> Option<rusticmc_tunnel::TunnelStatus> {
        self.tunnels
            .lock()
            .expect("tunnels")
            .get(id)
            .map(|h| h.status())
    }

    pub async fn stop_relay(&self, id: &str) {
        let handle = self.tunnels.lock().expect("tunnels").remove(id);
        if let Some(h) = handle {
            h.shutdown().await;
        }
    }

    /// An invite code: LAN/VPN addresses, the router mapping, the relay hostnames.
    pub fn invite_code(&self, id: &str) -> Result<String> {
        let profile = self.profile(id)?;
        let mut endpoints: Vec<InviteEndpoint> = Vec::new();
        if let Some(m) = self.port_mapping(id)
            && let Some(ip) = m.external_ip
        {
            endpoints.push(InviteEndpoint::Ip(SocketAddr::new(ip, m.external_port)));
        }
        for host in self.relay_hostnames(id) {
            endpoints.push(InviteEndpoint::Host(host, 25565));
        }
        if let Some(h) = &profile.hostname {
            endpoints.push(InviteEndpoint::Host(h.clone(), profile.port));
        }
        for a in network::local_addresses().into_iter().take(4) {
            endpoints.push(InviteEndpoint::Ip(SocketAddr::new(a.ip, profile.port)));
        }
        if endpoints.is_empty() {
            return Err(ServerError::Network("no address to share".into()));
        }
        Invite {
            port: profile.port,
            endpoints,
        }
        .encode()
    }

    fn hosted(&self, p: &ServerProfile, running: Option<bool>) -> HostedServer {
        HostedServer {
            id: p.id.clone(),
            name: p.name.clone(),
            kind: p.kind,
            hosting: HostingKind::Local,
            build: p.build.clone(),
            port: p.port,
            created_at: p.created_at,
            running,
        }
    }
}

/// The operations every local server has, wherever the UI calls them from.
impl ServerManager {
    pub async fn list(&self) -> Result<Vec<HostedServer>> {
        let mut out = Vec::new();
        for p in self.profiles() {
            let running = if Endpoint::read(&self.dir(&p.id)).is_some() {
                self.live_status(&p.id)
                    .await
                    .map(|s| s.phase != Phase::Stopped)
                    .or(Some(false))
            } else {
                Some(false)
            };
            out.push(self.hosted(&p, running));
        }
        Ok(out)
    }
    pub async fn create_hosted(&self, request: CreateServer) -> Result<HostedServer> {
        let p = self.create(request).await?;
        Ok(self.hosted(&p, Some(false)))
    }
    pub async fn delete(&self, id: &str, keep_files: bool) -> Result<()> {
        self.profile(id)?;
        let _ = self.stop_server(id).await;
        self.stop_relay(id).await;
        let dir = self.dir(id);
        if keep_files {
            let p = dir.join(PROFILE);
            std::fs::remove_file(&p).map_err(|e| ServerError::io(&p, e))
        } else {
            remove_dir_patiently(&dir, std::time::Duration::from_secs(30)).await
        }
    }
    pub async fn start(&self, id: &str) -> Result<()> {
        self.start_server(id).await
    }
    pub async fn stop(&self, id: &str) -> Result<()> {
        self.stop_server(id).await
    }
    pub async fn restart(&self, id: &str) -> Result<()> {
        self.control(id)?.restart().await
    }
    pub async fn state(&self, id: &str) -> Result<ServerState> {
        self.profile(id)?;
        Ok(match self.live_status(id).await {
            Some(status) => ServerState {
                running: status.phase != Phase::Stopped,
                metrics: status.metrics(),
                status: Some(status),
            },
            None => ServerState {
                running: false,
                status: None,
                metrics: Default::default(),
            },
        })
    }
    pub async fn command(&self, id: &str, line: &str) -> Result<Vec<String>> {
        self.control(id)?.command(line).await
    }
    pub async fn players(&self, id: &str) -> Result<serde_json::Value> {
        self.control(id)?.players().await
    }
    pub async fn player_action(&self, id: &str, action: PlayerAction) -> Result<ActionReply> {
        self.control(id)?.player_action(&action).await
    }
    pub async fn backups(&self, id: &str) -> Result<Vec<Backup>> {
        match self.control(id) {
            Ok(c) => c.backups().await,
            Err(ServerError::NotRunning) => {
                // Stopped: the backups folder itself.
                let dir = self.dir(id).join("backups");
                let mut out: Vec<Backup> = std::fs::read_dir(&dir)
                    .map(|e| {
                        e.flatten()
                            .filter(|e| e.path().extension().is_some_and(|x| x == "zip"))
                            .map(|e| Backup {
                                name: e.file_name().to_string_lossy().into_owned(),
                                bytes: e.metadata().map_or(0, |m| m.len()),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                out.sort_by(|a, b| b.name.cmp(&a.name));
                Ok(out)
            }
            Err(e) => Err(e),
        }
    }
    pub async fn backup(&self, id: &str, name: Option<String>) -> Result<String> {
        match self.control(id) {
            Ok(c) => c.backup(name.as_deref()).await,
            Err(ServerError::NotRunning) => {
                let profile = self.profile(id)?;
                let mut args = vec!["backup".to_owned()];
                args.extend(name);
                let out = self
                    .supervisor(&profile)?
                    .run(&args, std::time::Duration::from_secs(600))
                    .await?;
                Ok(out.stdout.trim().to_owned())
            }
            Err(e) => Err(e),
        }
    }
    pub async fn restore(&self, id: &str, backup: &str, dimension: Option<String>) -> Result<()> {
        match self.control(id) {
            Ok(c) => c.restore(backup, dimension.as_deref()).await,
            Err(ServerError::NotRunning) => {
                let profile = self.profile(id)?;
                let mut args = vec!["restore".to_owned(), backup.to_owned()];
                if let Some(d) = dimension {
                    args.extend(["--dimension".into(), d]);
                }
                args.push("--yes".into());
                self.supervisor(&profile)?
                    .run(&args, std::time::Duration::from_secs(600))
                    .await
                    .and_then(|o| {
                        if o.code == Some(0) {
                            Ok(())
                        } else {
                            Err(ServerError::Supervisor(o.stderr.trim().to_owned()))
                        }
                    })
            }
            Err(e) => Err(e),
        }
    }
    pub async fn config(&self, id: &str) -> Result<serde_json::Value> {
        let profile = self.profile(id)?;
        match self.control(id) {
            Ok(c) => c.config().await,
            Err(_) => self.supervisor(&profile)?.config().await,
        }
    }
    pub async fn set_config(
        &self,
        id: &str,
        changes: Vec<(String, String)>,
    ) -> Result<ConfigOutcome> {
        let profile = self.profile(id)?;
        if changes.iter().any(|(k, _)| k.starts_with("engine.")) {
            return Err(ServerError::Invalid(
                "engine.* is managed by the launcher (the server's type and build)".into(),
            ));
        }
        if changes.iter().any(|(k, _)| k == "server.port")
            && let Some((_, v)) = changes.iter().find(|(k, _)| k == "server.port")
            && let Ok(port) = v.parse::<u16>()
        {
            self.update_profile(id, |p| p.port = port)?;
        }
        self.supervisor(&profile)?.config_set(&changes).await
    }
}
