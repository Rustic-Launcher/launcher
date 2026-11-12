//! Servers, local and on a VPS, through one `rusticmc_hosting::HostingProvider` per place. Local
//! ids are the server folder names; VPS servers are `vps~<profile>~<name>`.

use crate::dto::*;
use crate::state::{AppState, Res, ServerMeta, State};
use crate::util::{build_label, default_build, files_in, iso, not_available_yet};
use rusticmc_hosting::HostingProvider;
use rusticmc_hosting::model::{
    ConsoleEvent, EngineSource, PlayerAction as HPlayerAction, ServerSpec, UninstallOptions,
};
use rusticmc_launcher_core::{IpcError, IpcErrorCode, time};
use rusticmc_servers::plugins::PluginManager;
use rusticmc_servers::{CreateServer, ServerKind};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;

pub enum Place {
    Local(String),
    Remote { profile: String, name: String },
}

pub fn place(id: &str) -> Res<Place> {
    match id.strip_prefix("vps~") {
        Some(rest) => {
            let (profile, name) = rest
                .split_once('~')
                .ok_or_else(|| IpcError::invalid("bad server id"))?;
            Ok(Place::Remote {
                profile: profile.into(),
                name: name.into(),
            })
        }
        None => Ok(Place::Local(id.into())),
    }
}

pub fn remote_id(profile: &str, name: &str) -> String {
    format!("vps~{profile}~{name}")
}

pub fn hosting_err(e: rusticmc_hosting::HostingError) -> IpcError {
    let p = e.to_payload();
    let code = match e.ipc_code() {
        "not_found" => IpcErrorCode::NotFound,
        "invalid_argument" => IpcErrorCode::InvalidArgument,
        "conflict" => IpcErrorCode::Conflict,
        "network" => IpcErrorCode::Network,
        "unauthorized" => IpcErrorCode::Unauthorized,
        "unavailable" => IpcErrorCode::Unavailable,
        "cancelled" => IpcErrorCode::Cancelled,
        _ => IpcErrorCode::Internal,
    };
    IpcError {
        code,
        message: e.to_string(),
        details: serde_json::to_value(&p)
            .ok()
            .map(|v| serde_json::json!({ "hosting_code": e.code(), "payload": v })),
    }
}

/// The provider and the provider-side name of a server.
pub fn provider(state: &AppState, id: &str) -> Res<(Arc<dyn HostingProvider>, String)> {
    match place(id)? {
        Place::Local(name) => Ok((state.local.clone(), name)),
        Place::Remote { profile, name } => {
            Ok((crate::cmd::hosting::remote(state, &profile)?, name))
        }
    }
}

fn read_toml(path: &std::path::Path) -> toml::Table {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

fn toml_str(t: &toml::Table, section: &str, key: &str) -> Option<String> {
    t.get(section)?.get(key).map(|v| match v {
        toml::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

fn state_name(phase: &str) -> &'static str {
    match phase {
        "running" => "running",
        "starting" | "restarting" => "starting",
        "stopping" => "stopping",
        "crashed" => "crashed",
        _ => "stopped",
    }
}

/// When the running game started: from the supervisor's engine uptime when it reports one (a
/// restart, or an automatic restart after a crash, starts a new game process), else the last
/// start the launcher asked for.
fn started_at(last_start: Option<u64>, engine_uptime: Option<u64>, now: u64) -> Option<u64> {
    engine_uptime
        .map(|up| now.saturating_sub(up))
        .or(last_start)
}

async fn local_dto(state: &AppState, p: &rusticmc_servers::ServerProfile) -> Server {
    let dir = state.servers.dir(&p.id);
    let toml = read_toml(&dir.join("server.toml"));
    let status = state.servers.live_status(&p.id).await;
    let metrics = status.as_ref().map(|s| s.metrics()).unwrap_or_default();
    let phase = status
        .as_ref()
        .map(|s| {
            serde_json::to_value(s.phase)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        })
        .unwrap_or_else(|| "stopped".into());
    let source = state
        .instances
        .builds
        .get(&p.build)
        .map(|b| b.source)
        .unwrap_or_default();
    let relay = state.servers.relay_hostnames(&p.id).into_iter().next();
    let lan = rusticmc_servers::network::primary_lan_ip().map(|ip| {
        if p.port == 25565 {
            ip.to_string()
        } else {
            format!("{ip}:{}", p.port)
        }
    });
    Server {
        id: p.id.clone(),
        name: p.name.clone(),
        icon: p.icon.clone(),
        engine: match p.kind {
            ServerKind::RusticServer => "rustic",
            ServerKind::Horizon => "horizon",
        },
        horizon_version_label: build_label(&p.build, &source),
        horizon_version: p.build.clone(),
        hosting: ServerHosting::Local,
        state: state_name(&phase),
        players_online: metrics.players_online.unwrap_or(0) as u64,
        max_players: toml_str(&toml, "server", "max_players")
            .and_then(|v| v.parse().ok())
            .unwrap_or(20),
        port: p.port,
        motd: toml_str(&toml, "server", "motd").unwrap_or_else(|| "A Minecraft Server".into()),
        world_name: toml_str(&toml, "world", "name").unwrap_or_else(|| "world".into()),
        memory_mb: p.memory_mb.unwrap_or(
            state
                .settings
                .lock()
                .expect("settings")
                .game
                .default_memory_mb,
        ),
        created_at: iso(p.created_at),
        last_started_at: started_at(
            p.last_started_at,
            status.as_ref().and_then(|s| s.engine_uptime_seconds),
            time::now(),
        )
        .map(iso),
        share_address: p.hostname.clone().or(relay).or(lan),
        plugin_count: if p.kind == ServerKind::RusticServer {
            PluginManager::new(&dir)
                .list()
                .0
                .iter()
                .filter(|x| x.enabled)
                .count()
        } else {
            0
        },
        folder: dir.display().to_string(),
    }
}

async fn remote_dto(state: &AppState, meta: &ServerMeta) -> Server {
    let status = match crate::cmd::hosting::remote(state, &meta.profile) {
        Ok(p) => tokio::time::timeout(Duration::from_secs(8), p.status(&meta.name))
            .await
            .ok()
            .and_then(Result::ok),
        Err(_) => None,
    };
    let host = state
        .registry
        .profile(&meta.profile)
        .map(|p| p.target.host)
        .unwrap_or_default();
    Server {
        id: remote_id(&meta.profile, &meta.name),
        name: meta.display_name.clone(),
        icon: meta.icon.clone(),
        engine: if meta.rustic { "rustic" } else { "horizon" },
        horizon_version: meta.horizon_version.clone(),
        horizon_version_label: build_label(&meta.horizon_version, "horizon"),
        hosting: ServerHosting::Vps {
            connection_id: meta.profile.clone(),
            host: host.clone(),
        },
        state: match &status {
            Some(s) => state_name(&s.phase),
            None => "unreachable",
        },
        players_online: status.as_ref().map_or(0, |s| s.players_online.len() as u64),
        max_players: status
            .as_ref()
            .and_then(|s| s.metrics.max_players)
            .map_or(meta.max_players as u64, u64::from),
        port: meta.port,
        motd: String::new(),
        world_name: meta.world.clone(),
        memory_mb: meta.memory_mb,
        created_at: iso(meta.created_at),
        last_started_at: None,
        share_address: Some(if meta.port == 25565 {
            host
        } else {
            format!("{host}:{}", meta.port)
        }),
        plugin_count: 0,
        folder: String::new(),
    }
}

pub async fn server_dto(state: &AppState, id: &str) -> Res<Server> {
    match place(id)? {
        Place::Local(name) => {
            let p = state.servers.profile(&name)?;
            Ok(local_dto(state, &p).await)
        }
        Place::Remote { profile, name } => {
            let meta = state
                .server_meta(&profile, &name)
                .ok_or_else(|| IpcError::not_found("no such server"))?;
            Ok(remote_dto(state, &meta).await)
        }
    }
}

async fn all(state: &AppState) -> Vec<Server> {
    let mut out = Vec::new();
    for p in state.servers.profiles() {
        out.push(local_dto(state, &p).await);
    }
    let metas = state.server_metas();
    let remote = futures_join(metas.iter().map(|m| remote_dto(state, m))).await;
    out.extend(remote);
    out
}

async fn futures_join<F: std::future::Future<Output = Server>>(
    futs: impl Iterator<Item = F>,
) -> Vec<Server> {
    let mut out = Vec::new();
    for f in futs {
        out.push(f.await);
    }
    out
}

pub async fn servers_changed(state: &AppState) {
    let list = all(state).await;
    let _ = state.app.emit("servers-changed", list);
}

#[tauri::command]
pub async fn servers_list(state: State<'_>) -> Res<Vec<Server>> {
    Ok(all(&state).await)
}

#[tauri::command]
pub async fn server_get(state: State<'_>, args: IdArgs) -> Res<Server> {
    server_dto(&state, &args.id).await
}

/// The settings a wizard's world setup turns into (`section.key` → value).
pub fn world_settings(req: &CreateServerRequest) -> BTreeMap<String, String> {
    let mut s = BTreeMap::new();
    s.insert("server.max_players".into(), req.max_players.to_string());
    if let WorldSetup::New {
        level_type,
        generate_structures,
        gamemode,
        difficulty,
        hardcore,
        ..
    } = &req.world
    {
        s.insert("world.game_mode".into(), gamemode.clone());
        s.insert("world.difficulty".into(), difficulty.clone());
        s.insert("world.hardcore".into(), hardcore.to_string());
        let lt = match level_type.as_str() {
            "flat" => "minecraft:flat",
            "large_biomes" => "minecraft:large_biomes",
            "amplified" => "minecraft:amplified",
            _ => "minecraft:normal",
        };
        s.insert("properties.level-type".into(), lt.into());
        s.insert(
            "properties.generate-structures".into(),
            generate_structures.to_string(),
        );
    }
    s
}

/// The player who owns the new server: the active Microsoft account (whitelisted and op).
fn owner_name(state: &AppState) -> Option<String> {
    match state.accounts.active().ok()? {
        rusticmc_auth::ActiveIdentity::Microsoft(uuid) => state
            .accounts
            .snapshot()
            .ok()?
            .accounts
            .into_iter()
            .find(|a| a.uuid == uuid)
            .map(|a| a.name),
        rusticmc_auth::ActiveIdentity::None => None,
    }
}

/// Official `.rplugin` files bundled beside the launcher (`starter-plugins/`).
pub fn starter_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("starter-plugins");
    dir.is_dir().then_some(dir)
}

#[tauri::command]
pub async fn server_create(state: State<'_>, args: CreateServerRequest) -> Res<CreateServerResult> {
    if !args.eula_accepted {
        return Err(IpcError::invalid(
            "Accept the Minecraft EULA to create a server.",
        ));
    }
    if let CreateHosting::Vps { connection_id } = &args.hosting {
        let connection_id = connection_id.clone();
        return crate::cmd::hosting::install_on_vps(&state, connection_id, args, false).await;
    }
    let owner = owner_name(&state);
    let seed = match &args.world {
        WorldSetup::New { seed, .. } => seed.clone().filter(|s| !s.is_empty()),
        _ => None,
    };
    let profile = state
        .servers
        .create(CreateServer {
            name: args.name.clone(),
            kind: if args.engine == "rustic" {
                ServerKind::RusticServer
            } else {
                ServerKind::Horizon
            },
            build: if args.horizon_version.is_empty() {
                default_build(&state)
                    .map(|b| b.version)
                    .ok_or_else(|| IpcError::not_found("Install a Horizon build first."))?
            } else {
                args.horizon_version.clone()
            },
            eula_accepted: true,
            world: Some("world".into()),
            seed,
            port: None,
            whitelist: true,
            players: owner.iter().cloned().collect(),
            op: owner,
            online_mode: true,
            engine_path: None,
        })
        .await?;
    let (task, _) = state.tasks.start(
        "server",
        format!("Server {}", profile.name),
        None,
        Some(profile.id.clone()),
    );
    let dir = state.servers.dir(&profile.id);
    // A world from elsewhere goes where init expects it (`world/`); the server reads it at start.
    let source = match &args.world {
        WorldSetup::Import { path } => Some(PathBuf::from(path)),
        WorldSetup::FromInstance {
            instance_id,
            world_id,
        } => {
            if world_id.contains(['/', '\\']) || world_id.starts_with('.') {
                return Err(IpcError::invalid("bad world id"));
            }
            Some(
                state
                    .instances
                    .store
                    .game_dir(instance_id)
                    .join("saves")
                    .join(world_id),
            )
        }
        WorldSetup::New { .. } => None,
    };
    if let Some(src) = source {
        if !src.join("level.dat").is_file() {
            return Err(IpcError::invalid(
                "That folder is not a world (no level.dat).",
            ));
        }
        rusticmc_launcher_core::fsutil::copy_dir(&src, &dir.join("world"))
            .map_err(|e| IpcError::internal(e.to_string()))?;
    }
    state
        .servers
        .set_config(&profile.id, world_settings(&args).into_iter().collect())
        .await?;
    state.servers.update_profile(&profile.id, |p| {
        p.icon = args.icon.clone();
        p.memory_mb = Some(args.memory_mb);
        p.start_with_launcher = args.start_with_launcher;
        p.relay = args.access.relay;
    })?;
    if profile.kind == ServerKind::RusticServer && !args.starter_plugins.is_empty() {
        let pm = state.servers.plugins(&profile.id)?;
        if let Some(dir) = starter_dir() {
            for file in files_in(&dir) {
                if let Ok(m) = rusticmc_servers::plugins::PluginManifest::read_archive(&file)
                    && args.starter_plugins.contains(&m.id)
                {
                    pm.install(&file, false)?;
                }
            }
        }
    }
    if args.access.invite
        && let Ok(code) = state.servers.invite_code(&profile.id)
    {
        state
            .invites
            .lock()
            .expect("invites")
            .insert(profile.id.clone(), code);
    }
    state.tasks.finish(&task);
    let server = server_dto(&state, &profile.id).await?;
    servers_changed(&state).await;
    Ok(CreateServerResult {
        server,
        task_id: task,
    })
}

#[tauri::command]
pub async fn server_delete(state: State<'_>, args: ServerDeleteArgs) -> Res<()> {
    let (p, name) = provider(&state, &args.id)?;
    p.uninstall(
        &name,
        &UninstallOptions {
            delete_files: args.delete_files,
            remove_runtime: false,
        },
        &rusticmc_hosting::model::NoProgress,
    )
    .await
    .map_err(hosting_err)?;
    if let Place::Remote { profile, name } = place(&args.id)? {
        state.remove_server_meta(&profile, &name);
    }
    servers_changed(&state).await;
    Ok(())
}

/// Follows a server's console into `server-console` events and the history buffer.
pub fn follow_console(state: Arc<AppState>, id: String) {
    tauri::async_runtime::spawn(async move {
        let Ok((p, name)) = provider(&state, &id) else {
            return;
        };
        let Ok(mut sub) = p.console_subscribe(&name).await else {
            return;
        };
        while let Some(e) = sub.events.recv().await {
            let (level, text) = match e {
                ConsoleEvent::Log { line } => {
                    let level = if line.contains("ERROR") || line.contains("/ERROR") {
                        "error"
                    } else if line.contains("WARN") {
                        "warn"
                    } else {
                        "info"
                    };
                    (level, line)
                }
                ConsoleEvent::Game { kind, data } => {
                    if matches!(kind.as_str(), "join" | "leave" | "kick" | "death") {
                        let _ = state.app.emit(
                            "server-players-changed",
                            serde_json::json!({ "server_id": id }),
                        );
                    }
                    if kind == "chat" {
                        (
                            "chat",
                            format!(
                                "<{}> {}",
                                data["name"].as_str().unwrap_or("?"),
                                data["message"].as_str().unwrap_or("")
                            ),
                        )
                    } else {
                        continue;
                    }
                }
                ConsoleEvent::Phase { phase } => {
                    let _ = state.app.emit(
                        "server-state",
                        serde_json::json!({ "server_id": id, "state": state_name(&phase) }),
                    );
                    continue;
                }
                ConsoleEvent::Closed { .. } => break,
            };
            push_console(&state, &id, level, text);
        }
    });
}

pub fn push_console(state: &AppState, id: &str, level: &'static str, text: String) {
    let line = ConsoleLine {
        server_id: id.to_owned(),
        ts: iso(time::now()),
        level,
        text,
    };
    {
        let mut all = state.console.lock().expect("console");
        let buf = all.entry(id.to_owned()).or_default();
        buf.push_back(line.clone());
        while buf.len() > 2000 {
            buf.pop_front();
        }
    }
    let _ = state.app.emit("server-console", line);
}

#[tauri::command]
pub async fn server_start(state: State<'_>, args: IdArgs) -> Res<Server> {
    let (p, name) = provider(&state, &args.id)?;
    let _ = state.app.emit(
        "server-state",
        serde_json::json!({ "server_id": args.id, "state": "starting" }),
    );
    p.start(&name).await.map_err(hosting_err)?;
    follow_console(state.inner().clone(), args.id.clone());
    let s = server_dto(&state, &args.id).await?;
    servers_changed(&state).await;
    Ok(s)
}

#[tauri::command]
pub async fn server_stop(state: State<'_>, args: IdArgs) -> Res<Server> {
    let (p, name) = provider(&state, &args.id)?;
    let _ = state.app.emit(
        "server-state",
        serde_json::json!({ "server_id": args.id, "state": "stopping" }),
    );
    p.stop(&name).await.map_err(hosting_err)?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let s = server_dto(&state, &args.id).await?;
    servers_changed(&state).await;
    Ok(s)
}

#[tauri::command]
pub async fn server_restart(state: State<'_>, args: IdArgs) -> Res<Server> {
    let (p, name) = provider(&state, &args.id)?;
    p.restart(&name).await.map_err(hosting_err)?;
    server_dto(&state, &args.id).await
}

#[tauri::command]
pub async fn server_command(state: State<'_>, args: ServerCommandArgs) -> Res<()> {
    let (p, name) = provider(&state, &args.server_id)?;
    push_console(
        &state,
        &args.server_id,
        "command",
        format!("> {}", args.line.trim()),
    );
    let out = p
        .send_command(&name, &args.line)
        .await
        .map_err(hosting_err)?;
    for line in out.output {
        push_console(&state, &args.server_id, "info", line);
    }
    Ok(())
}

#[tauri::command]
pub async fn server_console_history(
    state: State<'_>,
    args: ConsoleHistoryArgs,
) -> Res<Vec<ConsoleLine>> {
    let buffered: Vec<ConsoleLine> = state
        .console
        .lock()
        .expect("console")
        .get(&args.server_id)
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default();
    if !buffered.is_empty() {
        let skip = buffered.len().saturating_sub(args.limit);
        return Ok(buffered.into_iter().skip(skip).collect());
    }
    let (p, name) = provider(&state, &args.server_id)?;
    let lines = p
        .logs(&name, args.limit.max(1))
        .await
        .map_err(hosting_err)?;
    let now = iso(time::now());
    Ok(lines
        .into_iter()
        .map(|text| ConsoleLine {
            server_id: args.server_id.clone(),
            ts: now.clone(),
            level: if text.contains("ERROR") {
                "error"
            } else if text.contains("WARN") {
                "warn"
            } else {
                "info"
            },
            text,
        })
        .collect())
}

#[tauri::command]
pub async fn server_complete(state: State<'_>, args: CompleteArgs) -> Res<Vec<String>> {
    match place(&args.server_id)? {
        Place::Local(id) => Ok(state
            .servers
            .complete(&id, &args.partial)
            .await?
            .into_iter()
            .filter(|c| c.kind != "argument")
            .map(|c| c.text)
            .collect()),
        Place::Remote { .. } => Ok(Vec::new()),
    }
}

async fn players(state: &AppState, id: &str) -> Res<Vec<ServerPlayer>> {
    let (p, name) = provider(state, id)?;
    let snap = p.players(&name).await.map_err(hosting_err)?;
    let mut names: Vec<String> = snap
        .online
        .iter()
        .map(|o| o.name.clone())
        .chain(snap.ops.iter().cloned())
        .chain(snap.whitelist.iter().cloned())
        .chain(snap.bans.iter().cloned())
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    let has = |list: &[String], n: &str| list.iter().any(|x| x.eq_ignore_ascii_case(n));
    Ok(names
        .into_iter()
        .map(|n| {
            let online = snap.online.iter().find(|o| o.name.eq_ignore_ascii_case(&n));
            ServerPlayer {
                uuid: online.and_then(|o| o.uuid.clone()).unwrap_or_default(),
                online: online.is_some(),
                op: has(&snap.ops, &n),
                whitelisted: has(&snap.whitelist, &n),
                banned: has(&snap.bans, &n),
                ping_ms: online.and_then(|o| o.ping_ms),
                joined_at: None,
                playtime_s: 0,
                name: n,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn server_players(state: State<'_>, args: ServerIdArgs) -> Res<Vec<ServerPlayer>> {
    players(&state, &args.server_id).await
}

#[tauri::command]
pub async fn server_player_action(
    state: State<'_>,
    args: PlayerActionArgs,
) -> Res<Vec<ServerPlayer>> {
    let name = args.player.clone();
    let reason = args.reason.clone().filter(|r| !r.is_empty());
    let action = match args.action.as_str() {
        "kick" => HPlayerAction::Kick { name, reason },
        "ban" => HPlayerAction::Ban { name, reason },
        "pardon" => HPlayerAction::Pardon { name },
        "op" => HPlayerAction::Op { name },
        "deop" => HPlayerAction::Deop { name },
        "whitelist_add" => HPlayerAction::WhitelistAdd { name },
        "whitelist_remove" => HPlayerAction::WhitelistRemove { name },
        other => return Err(IpcError::invalid(format!("unknown action {other}"))),
    };
    let (p, server) = provider(&state, &args.server_id)?;
    p.player_action(&server, &action)
        .await
        .map_err(hosting_err)?;
    let _ = state.app.emit(
        "server-players-changed",
        serde_json::json!({ "server_id": args.server_id }),
    );
    players(&state, &args.server_id).await
}

#[tauri::command]
pub async fn server_whitelist_set_enabled(state: State<'_>, args: WhitelistArgs) -> Res<()> {
    let (p, server) = provider(&state, &args.server_id)?;
    p.player_action(
        &server,
        &HPlayerAction::WhitelistEnabled {
            enabled: args.enabled,
        },
    )
    .await
    .map_err(hosting_err)?;
    Ok(())
}

/// The settings the config page shows (horizon/docs/api/server-reference.md, `server.toml`).
/// `(key, group, type, default, options, min, max, live)`: `live` ones a running server takes
/// at once (difficulty, game mode, whitelist, motd, max players).
type FieldSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<i64>,
    Option<i64>,
    bool,
);
const FIELDS: &[FieldSpec] = &[
    (
        "server.motd",
        "general",
        "string",
        "A Minecraft Server",
        &[],
        None,
        None,
        true,
    ),
    (
        "server.max_players",
        "players",
        "integer",
        "20",
        &[],
        Some(1),
        Some(1000),
        true,
    ),
    (
        "server.online_mode",
        "players",
        "boolean",
        "true",
        &[],
        None,
        None,
        false,
    ),
    (
        "server.whitelist",
        "players",
        "boolean",
        "true",
        &[],
        None,
        None,
        true,
    ),
    (
        "server.enforce_whitelist",
        "players",
        "boolean",
        "false",
        &[],
        None,
        None,
        false,
    ),
    (
        "server.port",
        "network",
        "integer",
        "25565",
        &[],
        Some(1),
        Some(65535),
        false,
    ),
    (
        "server.bind",
        "network",
        "string",
        "",
        &[],
        None,
        None,
        false,
    ),
    (
        "server.view_distance",
        "performance",
        "integer",
        "10",
        &[],
        Some(2),
        Some(32),
        false,
    ),
    (
        "server.simulation_distance",
        "performance",
        "integer",
        "10",
        &[],
        Some(2),
        Some(32),
        false,
    ),
    (
        "server.max_view_distance",
        "performance",
        "integer",
        "0",
        &[],
        Some(0),
        Some(64),
        false,
    ),
    (
        "server.accept_older_clients",
        "network",
        "boolean",
        "false",
        &[],
        None,
        None,
        false,
    ),
    (
        "world.name",
        "world",
        "string",
        "world",
        &[],
        None,
        None,
        false,
    ),
    ("world.seed", "world", "string", "", &[], None, None, false),
    (
        "world.game_mode",
        "world",
        "enum",
        "survival",
        &["survival", "creative", "adventure", "spectator"],
        None,
        None,
        true,
    ),
    (
        "world.difficulty",
        "world",
        "enum",
        "easy",
        &["peaceful", "easy", "normal", "hard"],
        None,
        None,
        true,
    ),
    (
        "world.hardcore",
        "world",
        "boolean",
        "false",
        &[],
        None,
        None,
        false,
    ),
    (
        "backups.every",
        "general",
        "string",
        "6h",
        &[],
        None,
        None,
        false,
    ),
    (
        "backups.keep",
        "general",
        "integer",
        "12",
        &[],
        Some(0),
        Some(1000),
        false,
    ),
];

fn typed(kind: &str, text: &str) -> serde_json::Value {
    match kind {
        "integer" => text
            .parse::<i64>()
            .map(Into::into)
            .unwrap_or(serde_json::Value::Null),
        "boolean" => (text == "true").into(),
        _ => text.into(),
    }
}

#[tauri::command]
pub async fn server_config_get(state: State<'_>, args: ServerIdArgs) -> Res<ServerConfig> {
    let (p, name) = provider(&state, &args.server_id)?;
    let config = p.config_read(&name).await.map_err(hosting_err)?;
    let config = if config.get("config").is_some() {
        config["config"].clone()
    } else {
        config
    };
    Ok(ServerConfig {
        fields: FIELDS
            .iter()
            .map(|(key, group, kind, default, options, min, max, live)| {
                let (section, field) = key.split_once('.').expect("section.key");
                let value = match &config[section][field] {
                    serde_json::Value::Null => typed(kind, default),
                    v => v.clone(),
                };
                ConfigField {
                    key: (*key).into(),
                    group,
                    kind,
                    value,
                    default: typed(kind, default),
                    options: (!options.is_empty())
                        .then(|| options.iter().map(|o| (*o).to_owned()).collect()),
                    min: *min,
                    max: *max,
                    restart_required: !live,
                }
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn server_config_set(state: State<'_>, args: ServerConfigSetArgs) -> Res<ServerConfig> {
    let changes: Vec<(String, String)> = args
        .values
        .into_iter()
        .map(|(k, v)| {
            let text = match v {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            (k, text)
        })
        .collect();
    if changes
        .iter()
        .any(|(k, _)| !FIELDS.iter().any(|f| f.0 == k) && !k.starts_with("properties."))
    {
        return Err(IpcError::invalid("unknown setting"));
    }
    let (p, name) = provider(&state, &args.server_id)?;
    p.config_write(&name, &changes).await.map_err(hosting_err)?;
    server_config_get(
        state,
        ServerIdArgs {
            server_id: args.server_id,
        },
    )
    .await
}

fn local_only(id: &str) -> Res<String> {
    match place(id)? {
        Place::Local(name) => Ok(name),
        Place::Remote { .. } => Err(IpcError::with_reason(
            IpcErrorCode::Unavailable,
            "Not available for a VPS server from the launcher yet.",
            "hosting.unsupported",
        )),
    }
}

#[tauri::command]
pub fn server_config_raw_get(state: State<'_>, args: ServerIdArgs) -> Res<RawConfig> {
    let id = local_only(&args.server_id)?;
    state.servers.profile(&id)?;
    let path = state.servers.dir(&id).join("server.toml");
    Ok(RawConfig {
        content: std::fs::read_to_string(&path).map_err(|e| IpcError::internal(e.to_string()))?,
        path: path.display().to_string(),
        format: "toml",
    })
}

/// Replaces server.toml after checking it with `horizon-server config --json` (unknown keys are
/// refused); the old file comes back on failure. `engine.*` stays the launcher's.
#[tauri::command]
pub async fn server_config_raw_set(state: State<'_>, args: RawConfigSetArgs) -> Res<RawConfig> {
    let id = local_only(&args.server_id)?;
    let profile = state.servers.profile(&id)?;
    let parsed: toml::Table =
        toml::from_str(&args.content).map_err(|e| IpcError::invalid(e.to_string()))?;
    let path = state.servers.dir(&id).join("server.toml");
    let old = std::fs::read_to_string(&path).map_err(|e| IpcError::internal(e.to_string()))?;
    let old_table: toml::Table = toml::from_str(&old).unwrap_or_default();
    if parsed.get("engine") != old_table.get("engine") {
        return Err(IpcError::invalid("[engine] is managed by the launcher"));
    }
    std::fs::write(&path, &args.content).map_err(|e| IpcError::internal(e.to_string()))?;
    if let Err(e) = state.servers.supervisor(&profile)?.config().await {
        let _ = std::fs::write(&path, old);
        return Err(IpcError::invalid(e.to_string()));
    }
    server_config_raw_get(
        state,
        ServerIdArgs {
            server_id: args.server_id,
        },
    )
}

fn plugin_dto(p: &rusticmc_servers::plugins::InstalledPlugin) -> ServerPlugin {
    let m = &p.manifest;
    ServerPlugin {
        id: m.id.clone(),
        name: m.name.clone(),
        version: m.version.to_string(),
        api: m.api.clone(),
        authors: m.authors.clone(),
        license: m.license.clone(),
        description: m.description.clone().unwrap_or_default(),
        state: if p.enabled { "enabled" } else { "disabled" },
        error: None,
        update_available: None,
        capabilities: PluginCapabilities {
            network: match &m.network {
                None => false.into(),
                Some(list) => list.clone().into(),
            },
            filesystem: m.filesystem.clone(),
        },
        project_id: None,
    }
}

#[tauri::command]
pub fn server_plugins_list(state: State<'_>, args: ServerIdArgs) -> Res<Vec<ServerPlugin>> {
    let id = local_only(&args.server_id)?;
    let pm = state.servers.plugins(&id)?;
    let (ok, bad) = pm.list();
    let problems = pm.check_dependencies();
    let mut out: Vec<ServerPlugin> = ok
        .iter()
        .map(|p| {
            let mut dto = plugin_dto(p);
            if let Some(problem) = problems.iter().find(|pr| match pr {
                rusticmc_servers::plugins::DependencyProblem::Missing { plugin, .. }
                | rusticmc_servers::plugins::DependencyProblem::Disabled { plugin, .. }
                | rusticmc_servers::plugins::DependencyProblem::WrongVersion { plugin, .. }
                | rusticmc_servers::plugins::DependencyProblem::Duplicate { plugin } => {
                    *plugin == p.manifest.id
                }
                rusticmc_servers::plugins::DependencyProblem::Cycle { plugins } => {
                    plugins.contains(&p.manifest.id)
                }
            }) && p.enabled
            {
                dto.state = "errored";
                dto.error = serde_json::to_string(problem).ok();
            }
            dto
        })
        .collect();
    for b in bad {
        let file = b
            .file
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(ServerPlugin {
            id: file.clone(),
            name: file,
            version: String::new(),
            api: String::new(),
            authors: Vec::new(),
            license: String::new(),
            description: String::new(),
            state: "errored",
            error: Some(b.error),
            update_available: None,
            capabilities: PluginCapabilities {
                network: false.into(),
                filesystem: "none".into(),
            },
            project_id: None,
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn server_plugin_install(state: State<'_>, args: PluginInstallArgs) -> Res<ServerPlugin> {
    let path = match (&args.path, &args.project_id) {
        (Some(p), _) => PathBuf::from(p),
        (None, Some(project)) => {
            // From the catalogue: checked (origin, hashes, the .rplugin rules), then installed by
            // the plugin manager. Rustic's catalogue serves `.rplugin`s; Modrinth's plugins are
            // Bukkit/Paper (Java) and are refused by the content crate.
            let target = crate::cmd::content::InstallTarget::Server {
                server_id: args.server_id.clone(),
            };
            let done = crate::cmd::content::install_project(
                &state,
                project,
                args.version_id.as_deref(),
                &target,
                false,
            )
            .await?;
            servers_changed(&state).await;
            let id = local_only(&args.server_id)?;
            let pm = state.servers.plugins(&id)?;
            let (ok, _) = pm.list();
            let name = done.first().map(|d| d.name.clone()).unwrap_or_default();
            return ok
                .iter()
                .find(|p| p.manifest.name == name)
                .map(plugin_dto)
                .ok_or_else(|| IpcError::internal("plugin installed but not listed"));
        }
        (None, None) => return Err(IpcError::invalid("path or project_id is required")),
    };
    match place(&args.server_id)? {
        Place::Local(id) => {
            let p = state.servers.plugins(&id)?.install(&path, false)?;
            servers_changed(&state).await;
            Ok(plugin_dto(&p))
        }
        Place::Remote { .. } => {
            let manifest = rusticmc_servers::plugins::PluginManifest::read_archive(&path)?;
            let (p, name) = provider(&state, &args.server_id)?;
            let info = p.plugin_upload(&name, &path).await.map_err(hosting_err)?;
            Ok(plugin_dto(&rusticmc_servers::plugins::InstalledPlugin {
                manifest,
                file: PathBuf::from(info.file),
                enabled: info.enabled,
                dev: false,
            }))
        }
    }
}

#[tauri::command]
pub fn server_plugin_set_enabled(state: State<'_>, args: PluginToggleArgs) -> Res<ServerPlugin> {
    let id = local_only(&args.server_id)?;
    let pm = state.servers.plugins(&id)?;
    if !args.enabled {
        let dependents = pm.dependents(&args.plugin_id);
        if !dependents.is_empty() {
            return Err(IpcError {
                code: IpcErrorCode::Conflict,
                message: format!("{} need it: disable them first.", dependents.join(", ")),
                details: Some(serde_json::json!({ "dependents": dependents })),
            });
        }
    }
    Ok(plugin_dto(&pm.set_enabled(&args.plugin_id, args.enabled)?))
}

#[tauri::command]
pub fn server_plugin_update(_args: PluginArgs) -> Res<ServerPlugin> {
    // Updating needs a source for newer versions (the content catalogue); a newer file
    // installs over the old one with `server_plugin_install { path }`.
    Err(not_available_yet(
        "Plugin updates from the catalogue (rusticmc-content wiring)",
    ))
}

#[tauri::command]
pub async fn server_plugin_remove(state: State<'_>, args: PluginArgs) -> Res<()> {
    let (p, name) = provider(&state, &args.server_id)?;
    p.plugin_remove(&name, &args.plugin_id)
        .await
        .map_err(hosting_err)?;
    servers_changed(&state).await;
    Ok(())
}

fn world_datapacks(state: &AppState, id: &str) -> Res<PathBuf> {
    let dir = state.servers.dir(id);
    let toml = read_toml(&dir.join("server.toml"));
    let world = toml_str(&toml, "world", "name").unwrap_or_else(|| "world".into());
    Ok(dir.join(world).join("datapacks"))
}

#[tauri::command]
pub async fn server_datapacks_list(
    state: State<'_>,
    args: ServerIdArgs,
) -> Res<Vec<ServerDatapack>> {
    let mut out = Vec::new();
    let (p, name) = provider(&state, &args.server_id)?;
    for d in p.datapacks_list(&name).await.map_err(hosting_err)? {
        out.push(ServerDatapack {
            name: d.file.trim_end_matches(".zip").to_owned(),
            id: d.file,
            version: String::new(),
            enabled: true,
            description: String::new(),
        });
    }
    if let Place::Local(id) = place(&args.server_id)? {
        for f in files_in(&world_datapacks(&state, &id)?.join(".disabled")) {
            let file = f
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(ServerDatapack {
                name: file.trim_end_matches(".zip").to_owned(),
                id: file,
                version: String::new(),
                enabled: false,
                description: String::new(),
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn server_datapack_set_enabled(
    state: State<'_>,
    args: DatapackToggleArgs,
) -> Res<ServerDatapack> {
    let id = local_only(&args.server_id)?;
    if args.datapack_id.contains(['/', '\\']) || args.datapack_id.starts_with('.') {
        return Err(IpcError::invalid("bad datapack id"));
    }
    let dir = world_datapacks(&state, &id)?;
    let (from, to) = if args.enabled {
        (
            dir.join(".disabled").join(&args.datapack_id),
            dir.join(&args.datapack_id),
        )
    } else {
        (
            dir.join(&args.datapack_id),
            dir.join(".disabled").join(&args.datapack_id),
        )
    };
    if from.exists() {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| IpcError::internal(e.to_string()))?;
        }
        std::fs::rename(&from, &to).map_err(|e| IpcError::internal(e.to_string()))?;
    } else if !to.exists() {
        return Err(IpcError::not_found("no such datapack"));
    }
    Ok(ServerDatapack {
        name: args.datapack_id.trim_end_matches(".zip").to_owned(),
        id: args.datapack_id,
        version: String::new(),
        enabled: args.enabled,
        description: String::new(),
    })
}

#[tauri::command]
pub async fn server_datapack_remove(state: State<'_>, args: DatapackArgs) -> Res<()> {
    let (p, name) = provider(&state, &args.server_id)?;
    p.datapack_remove(&name, &args.datapack_id)
        .await
        .map_err(hosting_err)
}

/// `2026-11-10_12-00-00-<name>.zip` → (unix time, name).
fn parse_backup_name(name: &str) -> (Option<u64>, String) {
    let stem = name.trim_end_matches(".zip");
    let stamp = stem.get(..19);
    let time = stamp.and_then(|s| {
        let n = |a: usize, b: usize| s.get(a..b)?.parse::<u32>().ok();
        rusticmc_launcher_core::time::from_civil(
            i64::from(n(0, 4)?),
            n(5, 7)?,
            n(8, 10)?,
            n(11, 13)?,
            n(14, 16)?,
            n(17, 19)?,
        )
    });
    let rest = stem.get(20..).unwrap_or(stem).to_owned();
    (time, rest)
}

fn backup_dto(name: String, bytes: u64, modified: Option<u64>) -> Backup {
    let (t, label) = parse_backup_name(&name);
    let kind = match label.as_str() {
        "auto" => "scheduled",
        l if l.contains("restore") => "pre_restore",
        _ => "manual",
    };
    Backup {
        created_at: iso(t.or(modified).unwrap_or(0)),
        size_bytes: bytes,
        kind,
        note: (!matches!(label.as_str(), "auto" | "manual")).then_some(label),
        id: name,
    }
}

#[tauri::command]
pub async fn server_backups_list(state: State<'_>, args: ServerIdArgs) -> Res<Vec<Backup>> {
    let (p, name) = provider(&state, &args.server_id)?;
    Ok(p.backups_list(&name)
        .await
        .map_err(hosting_err)?
        .into_iter()
        .map(|b| backup_dto(b.name, b.bytes, b.modified))
        .collect())
}

#[tauri::command]
pub async fn server_backup_create(state: State<'_>, args: BackupCreateArgs) -> Res<Backup> {
    let (p, name) = provider(&state, &args.server_id)?;
    let note = args
        .note
        .as_deref()
        .map(rusticmc_launcher_core::fsutil::slug);
    let file = p
        .backup_create(&name, note.as_deref())
        .await
        .map_err(hosting_err)?;
    let file = file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&file)
        .trim()
        .to_owned();
    let list = p.backups_list(&name).await.map_err(hosting_err)?;
    let b = list
        .into_iter()
        .find(|b| b.name == file)
        .map(|b| backup_dto(b.name, b.bytes, b.modified))
        .unwrap_or_else(|| backup_dto(file, 0, Some(time::now())));
    Ok(b)
}

#[tauri::command]
pub async fn server_backup_restore(state: State<'_>, args: BackupArgs) -> Res<()> {
    let (p, name) = provider(&state, &args.server_id)?;
    p.backup_restore(&name, &args.backup_id, None)
        .await
        .map_err(hosting_err)
}

#[tauri::command]
pub async fn server_backup_delete(state: State<'_>, args: BackupArgs) -> Res<()> {
    let id = local_only(&args.server_id)?;
    if args.backup_id.contains(['/', '\\']) || args.backup_id.starts_with('.') {
        return Err(IpcError::invalid("bad backup id"));
    }
    match state.servers.delete_backup(&id, &args.backup_id).await {
        Ok(()) => Ok(()),
        Err(rusticmc_servers::ServerError::NotRunning) => {
            let path = state.servers.dir(&id).join("backups").join(&args.backup_id);
            std::fs::remove_file(path).map_err(|e| IpcError::not_found(e.to_string()))
        }
        Err(e) => Err(e.into()),
    }
}

fn every_to_hours(every: &str) -> Option<f64> {
    let every = every.trim();
    if every == "0" || every.is_empty() {
        return None;
    }
    let (n, unit) = every.split_at(every.len() - 1);
    let n: f64 = n.parse().ok()?;
    Some(match unit {
        "m" => n / 60.0,
        "h" => n,
        "d" => n * 24.0,
        _ => return None,
    })
}

#[tauri::command]
pub async fn server_backup_schedule_get(
    state: State<'_>,
    args: ServerIdArgs,
) -> Res<BackupSchedule> {
    let (p, name) = provider(&state, &args.server_id)?;
    let c = p.config_read(&name).await.map_err(hosting_err)?;
    let c = if c.get("config").is_some() {
        c["config"].clone()
    } else {
        c
    };
    let every = c["backups"]["every"].as_str().unwrap_or("6h").to_owned();
    let hours = every_to_hours(&every);
    Ok(BackupSchedule {
        enabled: hours.is_some(),
        interval_hours: hours.unwrap_or(6.0),
        keep: c["backups"]["keep"].as_u64().unwrap_or(12) as u32,
    })
}

#[tauri::command]
pub async fn server_backup_schedule_set(
    state: State<'_>,
    args: BackupScheduleSetArgs,
) -> Res<BackupSchedule> {
    let every = if !args.enabled {
        "0".to_owned()
    } else if args.interval_hours < 1.0 {
        format!("{}m", ((args.interval_hours * 60.0).round() as u64).max(5))
    } else if args.interval_hours.fract() == 0.0 && (args.interval_hours as u64).is_multiple_of(24)
    {
        format!("{}d", args.interval_hours as u64 / 24)
    } else {
        format!("{}h", args.interval_hours.round() as u64)
    };
    let (p, name) = provider(&state, &args.server_id)?;
    p.config_write(
        &name,
        &[
            ("backups.every".into(), every),
            ("backups.keep".into(), args.keep.to_string()),
        ],
    )
    .await
    .map_err(hosting_err)?;
    server_backup_schedule_get(
        state,
        ServerIdArgs {
            server_id: args.server_id,
        },
    )
    .await
}

fn ram_max(state: &AppState, id: &str) -> u64 {
    let mb = state
        .servers
        .profile(id)
        .ok()
        .and_then(|p| p.memory_mb)
        .unwrap_or(0);
    if mb > 0 {
        return u64::from(mb);
    }
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() / (1024 * 1024)
}

#[tauri::command]
pub async fn server_stats_history(
    state: State<'_>,
    args: StatsHistoryArgs,
) -> Res<Vec<ServerStats>> {
    match place(&args.server_id)? {
        Place::Local(id) => {
            let since = time::now().saturating_sub(args.minutes * 60);
            let max = ram_max(&state, &id);
            Ok(state
                .servers
                .metrics_history(&id, Some(since))
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|s| ServerStats {
                    server_id: args.server_id.clone(),
                    ts: iso(s.t),
                    cpu_percent: s.cpu_percent.unwrap_or(0.0),
                    ram_used_mb: s.rss_bytes.unwrap_or(0) / (1024 * 1024),
                    ram_max_mb: max,
                    tps: s.tps.unwrap_or(0.0),
                    mspt: s.mspt.unwrap_or(0.0),
                    players: s.players.unwrap_or(0),
                })
                .collect())
        }
        Place::Remote { .. } => {
            let (p, name) = provider(&state, &args.server_id)?;
            let m = p.metrics(&name).await.map_err(hosting_err)?;
            Ok(vec![ServerStats {
                server_id: args.server_id.clone(),
                ts: iso(time::now()),
                cpu_percent: m.cpu_percent.unwrap_or(0.0),
                ram_used_mb: m.ram_bytes.unwrap_or(0) / (1024 * 1024),
                ram_max_mb: 0,
                tps: m.tps.unwrap_or(0.0),
                mspt: m.mspt_avg.unwrap_or(0.0),
                players: u64::from(m.players.unwrap_or(0)),
            }])
        }
    }
}

// ---- access ---------------------------------------------------------------------------------

pub fn access(state: &AppState, id: &str) -> Res<ServerAccess> {
    let profile = state.servers.profile(id)?;
    let lan_ip = rusticmc_servers::network::primary_lan_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "127.0.0.1".into());
    let relay = state.servers.relay_status(id);
    let mapping = state.servers.port_mapping(id);
    Ok(ServerAccess {
        lan: LanAccess {
            address: if profile.port == 25565 {
                lan_ip.clone()
            } else {
                format!("{lan_ip}:{}", profile.port)
            },
            ip: lan_ip,
            port: profile.port,
        },
        relay: RelayAccess {
            enabled: profile.relay,
            hostname: relay
                .as_ref()
                .and_then(|s| s.routes.hostnames.first().cloned()),
            label: profile.relay_label.clone(),
            status: match &relay {
                None => "off",
                Some(s) if s.last_error.is_some() => "error",
                Some(s) if !s.routes.hostnames.is_empty() => "online",
                Some(_) => "connecting",
            },
            error: relay.and_then(|s| s.last_error),
        },
        invite: InviteAccess {
            code: state.invites.lock().expect("invites").get(id).cloned(),
            expires_at: None,
        },
        upnp: UpnpAccess {
            enabled: profile.upnp,
            status: match (&mapping, profile.upnp) {
                (Some(_), _) => "mapped",
                (None, true) => "failed",
                (None, false) => "off",
            },
            external_address: mapping
                .as_ref()
                .and_then(|m| m.external_ip.map(|ip| format!("{ip}:{}", m.external_port))),
            error: None,
        },
        public_ip: mapping.and_then(|m| m.external_ip.map(|ip| ip.to_string())),
    })
}

#[tauri::command]
pub fn server_access_get(state: State<'_>, args: ServerIdArgs) -> Res<ServerAccess> {
    let id = local_only(&args.server_id)?;
    access(&state, &id)
}

#[tauri::command]
pub async fn server_relay_set(state: State<'_>, args: RelaySetArgs) -> Res<ServerAccess> {
    let id = local_only(&args.server_id)?;
    if args.enabled {
        // Tunnel tokens come from Rustic Cloud (HANDOFF relay → cloud); its client isn't wired.
        let tokens = crate::relay::token_source(&state)?;
        let relay_addr = state
            .config()
            .cloud
            .relay_base
            .trim_start_matches("https://")
            .to_owned();
        let provider = rusticmc_tunnel::RelayTunnelProvider::new(format!("{relay_addr}:7443"));
        state.servers.start_relay(&id, &provider, tokens)?;
    } else {
        state.servers.stop_relay(&id).await;
    }
    state.servers.update_profile(&id, |p| {
        p.relay = args.enabled;
        p.relay_label = args.label.clone();
    })?;
    access(&state, &id)
}

#[tauri::command]
pub fn server_invite_create(state: State<'_>, args: ServerIdArgs) -> Res<ServerAccess> {
    let id = local_only(&args.server_id)?;
    let code = state.servers.invite_code(&id)?;
    state
        .invites
        .lock()
        .expect("invites")
        .insert(id.clone(), code);
    access(&state, &id)
}

#[tauri::command]
pub fn server_invite_revoke(state: State<'_>, args: ServerIdArgs) -> Res<ServerAccess> {
    let id = local_only(&args.server_id)?;
    state.invites.lock().expect("invites").remove(&id);
    access(&state, &id)
}

#[tauri::command]
pub async fn server_upnp_set(state: State<'_>, args: UpnpSetArgs) -> Res<ServerAccess> {
    let id = local_only(&args.server_id)?;
    state
        .servers
        .update_profile(&id, |p| p.upnp = args.enabled)?;
    if args.enabled {
        if let Err(e) = state.servers.map_port(&id).await {
            let mut a = access(&state, &id)?;
            a.upnp.status = "failed";
            a.upnp.error = Some(e.to_string());
            return Ok(a);
        }
    } else {
        state.servers.unmap_port(&id).await?;
    }
    access(&state, &id)
}

#[tauri::command]
pub fn servers_starter_plugins() -> Vec<StarterPlugin> {
    let Some(dir) = starter_dir() else {
        return Vec::new();
    };
    files_in(&dir)
        .into_iter()
        .filter_map(|f| rusticmc_servers::plugins::PluginManifest::read_archive(&f).ok())
        .map(|m| StarterPlugin {
            recommended: matches!(m.id.as_str(), "essentials-lite" | "perms"),
            capabilities: PluginCapabilities {
                network: match &m.network {
                    None => false.into(),
                    Some(l) => l.clone().into(),
                },
                filesystem: m.filesystem.clone(),
            },
            id: m.id,
            name: m.name,
            version: m.version.to_string(),
            description: m.description.unwrap_or_default(),
        })
        .collect()
}

/// Every few seconds: local servers' stats and state for the UI.
pub fn watch_servers(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let mut last: BTreeMap<String, &'static str> = BTreeMap::new();
        let mut procs = sysinfo::System::new();
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            for p in state.servers.profiles() {
                let status = state.servers.live_status(&p.id).await;
                let now = match &status {
                    Some(s) => state_name(
                        serde_json::to_value(s.phase)
                            .ok()
                            .and_then(|v| v.as_str().map(str::to_owned))
                            .as_deref()
                            .unwrap_or("stopped"),
                    ),
                    None => "stopped",
                };
                if last.get(&p.id) != Some(&now) {
                    last.insert(p.id.clone(), now);
                    let _ = state.app.emit(
                        "server-state",
                        serde_json::json!({ "server_id": p.id, "state": now }),
                    );
                }
                if let Some(s) = status {
                    let mut m = s.metrics();
                    let pid = s
                        .engine_pid
                        .or_else(|| s.sample["pid"].as_u64().map(|p| p as u32));
                    fill_process_metrics(&mut m, pid, &mut procs);
                    let _ = state.app.emit(
                        "server-stats",
                        ServerStats {
                            server_id: p.id.clone(),
                            ts: iso(time::now()),
                            cpu_percent: m.cpu_percent.unwrap_or(0.0),
                            ram_used_mb: m.rss_bytes.unwrap_or(0) / (1024 * 1024),
                            ram_max_mb: ram_max(&state, &p.id),
                            tps: m.tps.unwrap_or(0.0),
                            mspt: m.mspt.unwrap_or(0.0),
                            players: m.players_online.unwrap_or(0) as u64,
                        },
                    );
                }
            }
        }
    });
}

/// Horizon's supervisor measures RSS and CPU only on Linux and macOS (`/proc`, `ps`): elsewhere
/// (Windows) they come back empty, so the launcher samples the engine process itself rather
/// than showing "0 GB / 0 %" for a running server. CPU needs two samples (the loop's previous
/// refresh), like the supervisor's own figure: percent of one core.
fn fill_process_metrics(
    m: &mut rusticmc_servers::control::Metrics,
    pid: Option<u32>,
    sys: &mut sysinfo::System,
) {
    if m.rss_bytes.is_some() && m.cpu_percent.is_some() {
        return;
    }
    let Some(pid) = pid.map(sysinfo::Pid::from_u32) else {
        return;
    };
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[pid]),
        true,
        sysinfo::ProcessRefreshKind::nothing()
            .with_memory()
            .with_cpu(),
    );
    if let Some(p) = sys.process(pid) {
        m.rss_bytes.get_or_insert(p.memory());
        m.cpu_percent.get_or_insert(f64::from(p.cpu_usage()));
    }
}

/// Servers set to start with the launcher.
pub fn autostart(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        for p in state
            .servers
            .profiles()
            .into_iter()
            .filter(|p| p.start_with_launcher)
        {
            if let Err(e) = state.servers.start_server(&p.id).await {
                tracing::warn!(server = %p.id, error = %e, "autostart failed");
            } else {
                follow_console(state.clone(), p.id.clone());
            }
        }
    });
}

/// The engine for a VPS install: Rustic Server from this computer's `rustic-server`, checked by
/// SHA-256 on the VPS; Horizon otherwise. The program is looked up for the default build, the one
/// whose prepared resources go along (`spec_for`): the engine must come from the same Horizon
/// commit as those resources (ADR 0152).
pub fn engine_for(state: &AppState, engine: &str) -> Res<EngineSource> {
    if engine != "rustic" {
        return Ok(EngineSource::Horizon);
    }
    let build = default_build(state);
    let exe = state
        .servers
        .rustic_server_for_build(build.as_ref().map(|b| b.version.as_str()))
        .ok_or_else(|| IpcError::from(rusticmc_servers::ServerError::NoRusticServer))?
        .path;
    let sha256 =
        rusticmc_launcher_core::download::hash_file(&exe, rusticmc_launcher_core::HashAlgo::Sha256)
            .map_err(|e| IpcError::internal(e.to_string()))?;
    Ok(EngineSource::RusticUpload { path: exe, sha256 })
}

/// A `ServerSpec` from the wizard's request.
pub fn spec_for(state: &AppState, req: &CreateServerRequest, name: &str) -> Res<ServerSpec> {
    let owner = owner_name(state);
    Ok(ServerSpec {
        name: name.to_owned(),
        port: 25565,
        world: "world".into(),
        seed: match &req.world {
            WorldSetup::New { seed, .. } => seed.clone().filter(|s| !s.is_empty()),
            _ => None,
        },
        whitelist: true,
        players: owner.iter().cloned().collect(),
        op: owner,
        online_mode: true,
        eula_accepted: req.eula_accepted,
        engine: engine_for(state, &req.engine)?,
        // Only a folder the launcher really prepared (else the VPS install stops before starting).
        prepared_resources: default_build(state)
            .map(|b| state.instances.prepared_dir(&b.version))
            .filter(|d| d.join("horizon-resources.json").is_file()),
        settings: world_settings(req),
    })
}

#[cfg(test)]
mod tests {
    use super::fill_process_metrics;
    use rusticmc_servers::control::Metrics;

    #[test]
    fn samples_the_engine_when_the_supervisor_did_not() {
        // Regression (QA on Windows): RSS and CPU read "0 GB / 0 %" for a running server.
        let mut sys = sysinfo::System::new();
        let mut m = Metrics::default();
        fill_process_metrics(&mut m, Some(std::process::id()), &mut sys);
        assert!(m.rss_bytes.is_some_and(|b| b > 0));
        assert!(m.cpu_percent.is_some());
    }

    #[test]
    fn uptime_follows_the_game_process() {
        // Regression (QA): right after a restart the header still said "up 13m".
        use super::started_at;
        assert_eq!(started_at(Some(1_000), Some(5), 2_000), Some(1_995));
        assert_eq!(started_at(Some(1_000), None, 2_000), Some(1_000));
        assert_eq!(started_at(None, None, 2_000), None);
    }

    #[test]
    fn keeps_the_supervisor_figures() {
        let mut sys = sysinfo::System::new();
        let mut m = Metrics {
            rss_bytes: Some(7),
            cpu_percent: Some(1.5),
            ..Metrics::default()
        };
        fill_process_metrics(&mut m, Some(std::process::id()), &mut sys);
        assert_eq!((m.rss_bytes, m.cpu_percent), (Some(7), Some(1.5)));
        let mut empty = Metrics::default();
        fill_process_metrics(&mut empty, None, &mut sys);
        assert_eq!(empty, Metrics::default());
    }
}
