//! App, system, settings, downloads, game preparation.

use crate::dto::*;
use crate::state::{AppState, Res, State};
use crate::util::{default_build, iso};
use rusticmc_auth::LaunchAuth;
use rusticmc_instances::{GameFiles, PrepareEvent, game_files, java, prepare};
use rusticmc_launcher_core::{IpcError, IpcErrorCode};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub fn app_info(state: State<'_>) -> AppInfo {
    AppInfo {
        version: rusticmc_launcher_core::VERSION.into(),
        channel: "stable",
        os: match std::env::consts::OS {
            "windows" => "windows",
            "macos" => "macos",
            _ => "linux",
        },
        arch: std::env::consts::ARCH.into(),
        native: true,
        data_dir: state.paths.data().display().to_string(),
    }
}

#[tauri::command]
pub fn system_info() -> SystemInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
    SystemInfo {
        total_memory_mb: sys.total_memory() / (1024 * 1024),
        available_memory_mb: sys.available_memory() / (1024 * 1024),
        cpu_model: sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_owned())
            .unwrap_or_default(),
        cpu_threads: sys.cpus().len(),
    }
}

#[tauri::command]
pub fn settings_get(state: State<'_>) -> Settings {
    state.settings.lock().expect("settings").clone()
}

fn merge(target: &mut serde_json::Value, patch: serde_json::Map<String, serde_json::Value>) {
    if let Some(obj) = target.as_object_mut() {
        for (k, v) in patch {
            obj.insert(k, v);
        }
    }
}

#[tauri::command]
pub fn settings_set(state: State<'_>, args: SettingsPatch) -> Res<Settings> {
    let current = state.settings.lock().expect("settings").clone();
    let mut v = serde_json::to_value(&current).map_err(|e| IpcError::internal(e.to_string()))?;
    if let Some(p) = args.game {
        merge(&mut v["game"], p);
    }
    if let Some(p) = args.launcher {
        merge(&mut v["launcher"], p);
    }
    if let Some(p) = args.privacy {
        merge(&mut v["privacy"], p);
    }
    if let Some(done) = args.onboarding_completed {
        v["onboarding_completed"] = done.into();
    }
    let settings: Settings =
        serde_json::from_value(v).map_err(|e| IpcError::invalid(e.to_string()))?;
    state.discord.configure(settings.privacy.discord());
    if settings.launcher.concurrent_downloads == 0 || settings.launcher.concurrent_downloads > 16 {
        return Err(IpcError::invalid("concurrent downloads: 1 to 16"));
    }
    // What the Rust side uses lives in launcher.toml too.
    let mut config = state.config();
    config.downloads.concurrency = settings.launcher.concurrent_downloads as usize;
    config.privacy.telemetry = settings.privacy.telemetry;
    config.java.preferred_home = settings
        .game
        .prepare_java_custom_path
        .as_ref()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| settings.game.prepare_java_id.as_ref().map(PathBuf::from));
    config.game_files.minecraft_dir = Some(settings.game.game_dir.clone())
        .filter(|d| !d.is_empty())
        .map(PathBuf::from);
    state.save_config(config)?;
    state.save_settings(&settings)?;
    *state.settings.lock().expect("settings") = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn java_runtimes_list(state: State<'_>) -> Vec<JavaRuntime> {
    let config = state.config();
    let configured: Vec<PathBuf> = config
        .java
        .preferred_home
        .iter()
        .cloned()
        .chain(config.java.extra_homes.iter().cloned())
        .collect();
    let minecraft = config
        .game_files
        .minecraft_dir
        .clone()
        .or_else(java::default_minecraft_dir);
    java::discover(&configured, minecraft.as_deref())
        .into_iter()
        .map(|r| JavaRuntime {
            id: r.home.display().to_string(),
            path: r.home.display().to_string(),
            vendor: std::fs::read_to_string(r.home.join("release"))
                .ok()
                .and_then(|t| {
                    t.lines()
                        .find_map(|l| l.strip_prefix("IMPLEMENTOR="))
                        .map(|v| v.trim_matches('"').to_owned())
                })
                .unwrap_or_default(),
            version: r.version,
            source: match r.source {
                java::JavaSource::Configured => "custom",
                java::JavaSource::MinecraftLauncher => "bundled",
                _ => "system",
            },
        })
        .collect()
}

#[tauri::command]
pub fn app_open_path(app: AppHandle, args: PathArgs) -> Res<()> {
    app.opener()
        .open_path(args.path, None::<&str>)
        .map_err(|e| IpcError::internal(e.to_string()))
}

#[tauri::command]
pub fn app_open_url(app: AppHandle, args: UrlArgs) -> Res<()> {
    if !(args.url.starts_with("https://") || args.url.starts_with("http://")) {
        return Err(IpcError::invalid("only web links open"));
    }
    app.opener()
        .open_url(args.url, None::<&str>)
        .map_err(|e| IpcError::internal(e.to_string()))
}

/// The `rustic://content/...` or `rustic://server/...` link that started (or reached) the launcher
/// before the UI listened, taken once: the UI opens the project's page with the install prompt,
/// or the server's page in the directory.
#[tauri::command]
pub fn app_take_deep_link() -> Option<serde_json::Value> {
    crate::take_pending_content_link()
}

/// What the Settings row shows: the `rustic-server` a new Rustic Server of the default build
/// would run, where it comes from, and its version line.
async fn rustic_server_status_of(state: &AppState) -> RusticServerStatus {
    let build = default_build(state).map(|b| b.version);
    let found = state.servers.rustic_server_for_build(build.as_deref());
    let version = match &found {
        Some(f) => rusticmc_servers::engine::rustic_server_version(&f.path)
            .await
            .ok(),
        None => None,
    };
    RusticServerStatus {
        path: found.as_ref().map(|f| f.path.display().to_string()),
        origin: found.map(|f| f.origin),
        configured: state
            .servers
            .configured_rustic_server()
            .map(|p| p.display().to_string()),
        build,
        version,
    }
}

#[tauri::command]
pub async fn rustic_server_status(state: State<'_>) -> Res<RusticServerStatus> {
    Ok(rustic_server_status_of(&state).await)
}

/// Sets the `rustic-server` override (`[rustic_server] executable` in launcher.toml) after
/// checking the file answers `--rustic-version`; `None` clears it (automatic lookup).
#[tauri::command]
pub async fn rustic_server_set(
    state: State<'_>,
    args: RusticServerSetArgs,
) -> Res<RusticServerStatus> {
    let path = match args.path.filter(|p| !p.trim().is_empty()) {
        Some(p) => {
            let p = std::path::absolute(PathBuf::from(p.trim()))
                .map_err(|e| IpcError::invalid(e.to_string()))?;
            rusticmc_servers::engine::rustic_server_version(&p).await?;
            Some(p)
        }
        None => None,
    };
    let mut config = state.config();
    config.rustic_server.executable = path.clone();
    state.save_config(config)?;
    state.servers.set_rustic_server(path);
    Ok(rustic_server_status_of(&state).await)
}

#[tauri::command]
pub async fn app_pick_path(app: AppHandle, args: PickPathRequest) -> Res<Option<String>> {
    let mut dialog = app.dialog().file();
    if let Some(title) = &args.title {
        dialog = dialog.set_title(title);
    }
    for f in args.filters.unwrap_or_default() {
        let exts: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&f.name, &exts);
    }
    let folder = args.kind == "folder";
    let picked = tauri::async_runtime::spawn_blocking(move || {
        if folder {
            dialog.blocking_pick_folder()
        } else {
            dialog.blocking_pick_file()
        }
    })
    .await
    .map_err(|e| IpcError::internal(e.to_string()))?;
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string()))
}

#[tauri::command]
pub async fn app_check_update(app: AppHandle) -> Res<LauncherUpdate> {
    let updater = app
        .updater()
        .map_err(|e| IpcError::unavailable(e.to_string()))?;
    match updater.check().await {
        Ok(Some(u)) => Ok(LauncherUpdate {
            available: true,
            version: Some(u.version.clone()),
            notes: u.body.clone(),
        }),
        Ok(None) => Ok(LauncherUpdate {
            available: false,
            version: None,
            notes: None,
        }),
        Err(e) => Err(IpcError::new(IpcErrorCode::Network, e.to_string())),
    }
}

#[tauri::command]
pub async fn app_install_update(app: AppHandle, state: State<'_>) -> Res<()> {
    let updater = app
        .updater()
        .map_err(|e| IpcError::unavailable(e.to_string()))?;
    let update = updater
        .check()
        .await
        .map_err(|e| IpcError::new(IpcErrorCode::Network, e.to_string()))?
        .ok_or_else(|| IpcError::not_found("no update available"))?;
    let (task, _) = state.tasks.start(
        "update",
        format!("Rustic Launcher {}", update.version),
        None,
        None,
    );
    let tasks = state.tasks.clone();
    let task2 = task.clone();
    let mut received = 0u64;
    let result = update
        .download_and_install(
            move |chunk, total| {
                received += chunk as u64;
                tasks.bytes(&task2, received, total);
            },
            || {},
        )
        .await;
    match result {
        Ok(()) => {
            state.tasks.finish(&task);
            app.restart();
        }
        Err(e) => {
            state.tasks.fail(&task, e.to_string());
            Err(IpcError::new(IpcErrorCode::Network, e.to_string()))
        }
    }
}

#[tauri::command]
pub fn downloads_list(state: State<'_>) -> Vec<DownloadTask> {
    state.tasks.list()
}

#[tauri::command]
pub fn download_cancel(state: State<'_>, args: IdArgs) -> Res<()> {
    if state.tasks.cancel(&args.id, &state.downloads) {
        Ok(())
    } else {
        Err(IpcError::not_found(format!("no running task {}", args.id)))
    }
}

#[tauri::command]
pub fn game_detect_install(state: State<'_>) -> GameInstallDetection {
    let dir = state
        .config()
        .game_files
        .minecraft_dir
        .filter(|d| d.is_dir())
        .or_else(java::default_minecraft_dir);
    match dir {
        Some(dir) => {
            let mut versions: Vec<String> = std::fs::read_dir(dir.join("versions"))
                .map(|e| {
                    e.flatten()
                        .filter(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            e.path().join(format!("{name}.json")).is_file()
                        })
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            versions.sort();
            GameInstallDetection {
                found: !versions.is_empty(),
                path: Some(dir.display().to_string()),
                versions,
            }
        }
        None => GameInstallDetection {
            found: false,
            path: None,
            versions: Vec::new(),
        },
    }
}

#[tauri::command]
pub async fn game_prepare_status(state: State<'_>) -> Res<GamePreparation> {
    let source_path = state
        .config()
        .game_files
        .minecraft_dir
        .map(|p| p.display().to_string());
    let Some(build) = default_build(&state) else {
        return Ok(GamePreparation {
            state: "missing",
            game_version: String::new(),
            prepared_at: None,
            source_path,
        });
    };
    let game_version = state
        .instances
        .preparer_api(&build)
        .await
        .ok()
        .and_then(|a| a.minecraft)
        .unwrap_or_default();
    if state.preparing.lock().expect("preparing").as_deref() == Some(build.version.as_str()) {
        return Ok(GamePreparation {
            state: "preparing",
            game_version,
            prepared_at: None,
            source_path,
        });
    }
    let dir = state.instances.prepared_dir(&build.version);
    let check = state.instances.check_prepared(&build.version).await;
    let prepared_at = std::fs::metadata(&dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| iso(d.as_secs()));
    Ok(GamePreparation {
        state: match check {
            Ok(c) if c.current => "ready",
            Ok(_)
                if dir.is_dir()
                    && std::fs::read_dir(&dir)
                        .map(|mut d| d.next().is_some())
                        .unwrap_or(false) =>
            {
                "outdated"
            }
            _ => "missing",
        },
        game_version,
        prepared_at,
        source_path,
    })
}

#[derive(serde::Deserialize)]
pub struct PlayerSkinArgs {
    pub uuid: String,
}

/// A player's skin URL (Mojang's session service, cached), for their head in lists; `null` for
/// an offline profile, no custom skin or no network.
#[tauri::command]
pub async fn player_skin_url(state: State<'_>, args: PlayerSkinArgs) -> Res<Option<String>> {
    Ok(state.skins.skin_url(&state.http, &args.uuid).await)
}

/// Prepares the default build's game files from the player's own install. The first
/// preparation needs the server jar, downloaded only for a signed-in owner (Horizon launch.md).
#[tauri::command]
pub async fn game_prepare(state: State<'_>, args: GamePrepareArgs) -> Res<TaskRef> {
    let build = default_build(&state).ok_or_else(|| {
        IpcError::with_reason(
            IpcErrorCode::NotFound,
            "Install a Horizon build first.",
            "build.not_installed",
        )
    })?;
    let job = prepare_job(&state, build, args.source_path.map(PathBuf::from)).await?;
    Ok(TaskRef {
        task_id: start_prepare(state.inner().clone(), job),
    })
}

/// A checked game-files preparation: source install, Java and a verified owner.
pub(crate) struct PrepareJob {
    build: rusticmc_instances::InstalledBuild,
    minecraft: String,
    minecraft_dir: PathBuf,
    java_home: PathBuf,
    proof: rusticmc_auth::VerifiedOwner,
    /// The game files to download first (into `minecraft_dir`), when no folder has them whole.
    download: Option<GameDownload>,
}

/// The pinned game files to download from Mojang, seeded from the player's own folder.
pub(crate) struct GameDownload {
    pins: game_files::Pins,
    seed: Option<PathBuf>,
}

/// Checks everything a preparation of `build` needs (also used by the launch when the files are
/// missing). `source`: the player's game directory, else the configured or default one.
pub(crate) async fn prepare_job(
    state: &crate::state::AppState,
    build: rusticmc_instances::InstalledBuild,
    source: Option<PathBuf>,
) -> Res<PrepareJob> {
    let config = state.config();
    let seed = source
        .or(config.game_files.minecraft_dir.clone())
        .or_else(java::default_minecraft_dir);
    // The player's folder when it holds the pinned version whole; else Rustic's own copy,
    // downloaded from Mojang for the verified owner below (ADR 0147).
    let pins = game_files::Pins::of_build(&build.dir);
    let own = state.paths.game_files();
    let (minecraft_dir, download) = {
        let seed = seed.clone();
        tokio::task::spawn_blocking(move || {
            let Some(pins) = pins else {
                // A build without pins: the player's folder as it is.
                return seed.map(|s| (s, None));
            };
            if let Some(s) = seed.as_ref().filter(|s| game_files::is_complete(s, &pins)) {
                return Some((s.clone(), None));
            }
            let download =
                (!game_files::is_complete(&own, &pins)).then_some(GameDownload { pins, seed });
            Some((own, download))
        })
        .await
        .map_err(|e| IpcError::new(IpcErrorCode::Internal, e.to_string()))?
        .ok_or_else(|| {
            IpcError::with_reason(
                IpcErrorCode::NotFound,
                rusticmc_instances::InstanceError::NoMinecraftDir.to_string(),
                "prepare.no_minecraft_dir",
            )
        })?
    };
    let api = state.instances.preparer_api(&build).await?;
    let minecraft = api.minecraft.clone().unwrap_or_default();
    let java_major = api.java_major.unwrap_or(25);
    let configured: Vec<PathBuf> = config.java.preferred_home.iter().cloned().collect();
    // The official launcher's Java runtimes live in the player's own folder.
    let java_home = java::find(
        java_major,
        &configured,
        Some(seed.as_deref().unwrap_or(&minecraft_dir)),
    )
    .map(|r| r.home)
    .ok_or_else(|| {
        IpcError::with_reason(
            IpcErrorCode::Unavailable,
            rusticmc_instances::InstanceError::NoJava(java_major).to_string(),
            "prepare.no_java",
        )
    })?;
    // The owner gate: a verified owner, before the server jar is downloaded.
    let owner = match state.accounts.launch_session(None).await? {
        LaunchAuth::Online(s) => s,
        LaunchAuth::Offline { reason, .. } => {
            return Err(IpcError::new(IpcErrorCode::Network, reason));
        }
    };
    let proof = state.accounts.verified_owner(Some(&owner.uuid)).await?;
    Ok(PrepareJob {
        build,
        minecraft,
        minecraft_dir,
        java_home,
        proof,
        download,
    })
}

/// Runs a preparation in the background as a task (`prepare-progress` events); returns its id.
pub(crate) fn start_prepare(
    state: std::sync::Arc<crate::state::AppState>,
    job: PrepareJob,
) -> String {
    let PrepareJob {
        build,
        minecraft,
        minecraft_dir,
        java_home,
        proof,
        download,
    } = job;
    let (task, cancel) =
        state
            .tasks
            .start("prepare", format!("Game files {minecraft}"), None, None);
    *state.preparing.lock().expect("preparing") = Some(build.version.clone());
    let st = state.clone();
    let task_id = task.clone();
    tauri::async_runtime::spawn(async move {
        let emit = |step: &'static str, progress: f64, message: Option<String>| {
            let _ = st.app.emit(
                "prepare-progress",
                PrepareProgress {
                    step,
                    progress,
                    message,
                },
            );
        };
        emit("detect", 0.0, None);
        // Without a whole copy of the game on this computer, download it first (half the bar).
        let share = if download.is_some() { 0.5 } else { 0.0 };
        if let Some(GameDownload { pins, seed }) = download {
            emit("download", 0.0, None);
            let st2 = st.clone();
            let task2 = task_id.clone();
            let progress = move |done: u64, total: u64| {
                let f = if total == 0 {
                    0.0
                } else {
                    done as f64 / total as f64
                };
                st2.tasks.fraction(&task2, f * share);
                let _ = st2.app.emit(
                    "prepare-progress",
                    PrepareProgress {
                        step: "download",
                        progress: f,
                        message: None,
                    },
                );
            };
            let got = game_files::download(
                &proof,
                &pins,
                &minecraft_dir,
                seed.as_deref(),
                &st.downloads,
                &cancel,
                &progress,
            )
            .await;
            if let Err(e) = got {
                let e = format!("The game files could not be downloaded from Mojang: {e}");
                st.tasks.fail(&task_id, e.clone());
                emit("done", 0.0, Some(e));
                *st.preparing.lock().expect("preparing") = None;
                return;
            }
        }
        let version_json = prepare::version_json_in(&minecraft_dir, &minecraft);
        let server_jar = if version_json.is_file() {
            match prepare::download_server_jar(
                &proof,
                &version_json,
                &st.paths.owner_downloads(),
                &st.downloads,
            )
            .await
            {
                Ok(p) => Some(p),
                Err(e) => {
                    st.tasks.fail(&task_id, e.to_string());
                    emit("done", 0.0, Some(e.to_string()));
                    *st.preparing.lock().expect("preparing") = None;
                    return;
                }
            }
        } else {
            None
        };
        emit("verify", 0.05, None);
        let mut events = st.instances.subscribe_prepare();
        let st2 = st.clone();
        let task2 = task_id.clone();
        let forward = tauri::async_runtime::spawn(async move {
            while let Ok(p) = events.recv().await {
                match p.event {
                    PrepareEvent::Fraction { value } => {
                        st2.tasks.fraction(&task2, share + value * (1.0 - share));
                        let _ = st2.app.emit(
                            "prepare-progress",
                            PrepareProgress {
                                step: "export",
                                progress: value,
                                message: None,
                            },
                        );
                    }
                    PrepareEvent::Message { text } => {
                        let _ = st2.app.emit(
                            "prepare-progress",
                            PrepareProgress {
                                step: "export",
                                progress: -1.0,
                                message: Some(text),
                            },
                        );
                    }
                    _ => {}
                }
            }
        });
        let result = tokio::select! {
            _ = cancel.cancelled() => {
                st.instances.cancel_prepare(&build.version);
                Err(rusticmc_instances::InstanceError::Prepare("cancelled".into()))
            }
            r = st.instances.prepare_build(
                &build.version,
                GameFiles { minecraft_dir: Some(minecraft_dir), ..GameFiles::default() },
                server_jar,
                Some(java_home),
            ) => r,
        };
        forward.abort();
        *st.preparing.lock().expect("preparing") = None;
        match result {
            Ok(()) => {
                st.tasks.finish(&task_id);
                emit("done", 1.0, None);
            }
            Err(e) => {
                st.tasks.fail(&task_id, e.to_string());
                emit("done", 0.0, Some(e.to_string()));
            }
        }
    });
    task
}
