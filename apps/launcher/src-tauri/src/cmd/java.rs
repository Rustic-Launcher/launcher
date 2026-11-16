//! The Java Edition launcher and the Bedrock panel (ADR 0172): Mojang's versions, Java profiles
//! (kept apart from Rustic instances), installs as tasks, the owner gate, launch with logs and
//! crash detection, Modrinth mods for a profile's loader, Bedrock detection and start on Windows.
//!
//! Commands `java_*` and `bedrock_*`; events `java-state` (a profile's run state), `java-log` (its
//! output lines) and `java-profiles-changed`.

use crate::state::{AppState, Res, State};
use rusticmc_auth::{ActiveIdentity, LaunchAuth};
use rusticmc_content::fairplay::FairPlayPolicy;
use rusticmc_content::model::{ContentKind, DependencyKind, FairPlay};
use rusticmc_content::sources::modrinth::{ALLOWED_HOSTS, Modrinth, ModrinthConfig, SearchQuery};
use rusticmc_java::install::{Installed, Installer, Step};
use rusticmc_java::launch::{self, Auth, LaunchOptions};
use rusticmc_java::loaders::{self, LoaderKind, LoaderVersion};
use rusticmc_java::manifest::VersionFilter;
use rusticmc_java::platform::{Platform, os_version_from};
use rusticmc_java::process::{self, LogLine};
use rusticmc_java::profiles::{JavaProfile, NewProfile, ProfilePatch, ProfileStore};
use rusticmc_java::{Hosts, JavaError, bedrock, runtime};
use rusticmc_launcher_core::{
    CancellationToken, Checksum, DownloadManager, DownloadRequest, IpcError, IpcErrorCode,
    LauncherPaths, time,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

/// Lines of output kept per profile.
const LOG_LINES: usize = 2000;

/// The Java launcher's state, one per app.
pub struct JavaLauncher {
    pub installer: Installer,
    pub profiles: ProfileStore,
    run_dir: PathBuf,
    natives_dir: PathBuf,
    games: Mutex<HashMap<String, CancellationToken>>,
    busy: Mutex<HashSet<String>>,
    states: Mutex<HashMap<String, JavaRunState>>,
    logs: Mutex<HashMap<String, VecDeque<JavaLogLine>>>,
    bedrock: tokio::sync::Mutex<Option<bedrock::BedrockStatus>>,
    /// Official edition icons fetched this session, as `data:` URLs.
    icons: tokio::sync::Mutex<HashMap<String, Option<String>>>,
}

impl JavaLauncher {
    /// Game files in `<data>/minecraft` (the official layout, shared with the Rustic edition's
    /// own game files, ADR 0147), profiles and run files in `<data>/java`.
    pub fn new(paths: &LauncherPaths, http: rusticmc_launcher_core::reqwest::Client) -> Self {
        let java_dir = paths.data().join("java");
        let run_dir = java_dir.join("run");
        process::clean_run_dir(&run_dir);
        let os_version = if cfg!(target_os = "macos") {
            sysinfo::System::os_version()
        } else {
            sysinfo::System::kernel_version()
        };
        let platform = Platform::current();
        let platform = platform
            .clone()
            .with_os_version(os_version_from(platform.os, os_version.as_deref()));
        Self {
            installer: Installer {
                root: paths.game_files(),
                downloads: DownloadManager::with_client(
                    http.clone(),
                    paths.download_cache(),
                    16,
                    3,
                ),
                http,
                hosts: Hosts::default(),
                extra_hosts: Vec::new(),
                platform,
            },
            profiles: ProfileStore::new(&java_dir),
            natives_dir: java_dir.join("natives"),
            run_dir,
            games: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            states: Mutex::new(HashMap::new()),
            logs: Mutex::new(HashMap::new()),
            bedrock: tokio::sync::Mutex::new(None),
            icons: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    fn running(&self, id: &str) -> bool {
        self.games.lock().expect("games").contains_key(id)
    }
}

// ---- DTOs ----

#[derive(Clone, Debug, Serialize)]
pub struct JavaGameVersion {
    pub id: String,
    /// `release`, `snapshot`, `old_beta`, `old_alpha`.
    pub kind: String,
    pub release_time: String,
    pub installed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct JavaProfileView {
    #[serde(flatten)]
    pub profile: JavaProfile,
    /// The resolved game folder.
    pub game_dir_path: String,
    /// Installed and ready to start.
    pub installed: bool,
    pub running: bool,
}

/// What a profile is doing.
#[derive(Clone, Debug, Serialize)]
pub struct JavaRunState {
    pub profile_id: String,
    /// `idle`, `installing`, `launching`, `running`, `crashed`.
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<Step>,
    /// 0..1 while installing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash_report: Option<String>,
    /// The launch played offline (no network; the owner's last check is recent enough).
    #[serde(default)]
    pub offline: bool,
}

impl JavaRunState {
    fn idle(id: &str) -> Self {
        Self {
            profile_id: id.to_owned(),
            state: "idle",
            step: None,
            progress: None,
            task_id: None,
            message: None,
            reason: None,
            started_at: None,
            exit_code: None,
            crash_report: None,
            offline: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct JavaLogLine {
    pub profile_id: String,
    /// `info`, `warn`, `error`, `debug`.
    pub level: String,
    pub thread: Option<String>,
    pub text: String,
    pub time: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct JavaModHit {
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    /// Fair in single player, not on every server (installable with a notice).
    pub flagged: bool,
    pub installed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct JavaModSearch {
    pub hits: Vec<JavaModHit>,
    pub total: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct JavaModFile {
    pub file_name: String,
    pub enabled: bool,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ModIndex {
    /// File name (without `.disabled`) → Modrinth project.
    files: HashMap<String, String>,
}

// ---- helpers ----

fn java_err(e: JavaError) -> IpcError {
    let code = match &e {
        JavaError::UnknownVersion(_) | JavaError::UnknownProfile(_) => IpcErrorCode::NotFound,
        JavaError::Invalid(_) => IpcErrorCode::InvalidArgument,
        JavaError::AlreadyRunning | JavaError::NotRunning => IpcErrorCode::Conflict,
        JavaError::Network(_) | JavaError::Http { .. } | JavaError::Download(_) => {
            IpcErrorCode::Network
        }
        JavaError::Cancelled => IpcErrorCode::Cancelled,
        JavaError::NoRuntime { .. } | JavaError::Bedrock(_) => IpcErrorCode::Unavailable,
        _ => IpcErrorCode::Internal,
    };
    if e.is_cancelled() {
        return IpcError::with_reason(IpcErrorCode::Cancelled, e.to_string(), "java.cancelled");
    }
    IpcError::with_reason(code, e.to_string(), e.reason())
}

fn view(java: &JavaLauncher, p: JavaProfile) -> JavaProfileView {
    let installed = p
        .version_id
        .as_deref()
        .is_some_and(|v| java.installer.version_file(v).is_file());
    JavaProfileView {
        game_dir_path: java.profiles.game_dir(&p).display().to_string(),
        running: java.running(&p.id),
        installed,
        profile: p,
    }
}

fn views(java: &JavaLauncher) -> Res<Vec<JavaProfileView>> {
    let mut list = java.profiles.list().map_err(java_err)?;
    list.sort_by(|a, b| {
        b.last_played_at
            .unwrap_or(b.created_at)
            .cmp(&a.last_played_at.unwrap_or(a.created_at))
    });
    Ok(list.into_iter().map(|p| view(java, p)).collect())
}

fn profiles_changed(state: &AppState) {
    if let Ok(v) = views(&state.java) {
        let _ = state.app.emit("java-profiles-changed", v);
    }
}

fn set_state(state: &AppState, s: JavaRunState) {
    state
        .java
        .states
        .lock()
        .expect("states")
        .insert(s.profile_id.clone(), s.clone());
    let _ = state.app.emit("java-state", s);
}

fn current_state(state: &AppState, id: &str) -> JavaRunState {
    state
        .java
        .states
        .lock()
        .expect("states")
        .get(id)
        .cloned()
        .unwrap_or_else(|| JavaRunState::idle(id))
}

fn push_log(state: &AppState, id: &str, line: LogLine) {
    let level = match line.level.as_deref() {
        Some("ERROR") | Some("FATAL") => "error",
        Some("WARN") => "warn",
        Some("DEBUG") | Some("TRACE") => "debug",
        Some(_) => "info",
        None if line.stream == process::Stream::Stderr => "warn",
        None => "info",
    };
    let entry = JavaLogLine {
        profile_id: id.to_owned(),
        level: level.into(),
        thread: line.thread,
        text: line.text,
        time: time::now(),
    };
    {
        let mut logs = state.java.logs.lock().expect("logs");
        let buf = logs.entry(id.to_owned()).or_default();
        if buf.len() >= LOG_LINES {
            buf.pop_front();
        }
        buf.push_back(entry.clone());
    }
    let _ = state.app.emit("java-log", entry);
}

fn system_line(state: &AppState, id: &str, text: String) {
    push_log(
        state,
        id,
        LogLine {
            stream: process::Stream::Stdout,
            level: Some("INFO".into()),
            thread: Some("Rustic".into()),
            text,
        },
    );
}

fn step_name(step: Step) -> &'static str {
    match step {
        Step::Metadata => "Reading the version files",
        Step::Loader => "Installing the mod loader",
        Step::Libraries => "Downloading the game and its libraries",
        Step::Assets => "Downloading the assets",
        Step::Java => "Downloading Java",
    }
}

// ---- versions & loaders ----

#[derive(Deserialize)]
pub struct VersionsArgs {
    #[serde(default)]
    pub snapshots: bool,
    #[serde(default)]
    pub old: bool,
}

#[tauri::command]
pub async fn java_versions_list(state: State<'_>, args: VersionsArgs) -> Res<Vec<JavaGameVersion>> {
    let m = state.java.installer.manifest().await.map_err(java_err)?;
    let filter = VersionFilter {
        snapshots: args.snapshots,
        old: args.old,
    };
    Ok(m.filtered(filter)
        .into_iter()
        .map(|v| JavaGameVersion {
            installed: state.java.installer.version_file(&v.id).is_file(),
            id: v.id.clone(),
            kind: v.kind.clone(),
            release_time: v.release_time.clone(),
        })
        .collect())
}

#[derive(Deserialize)]
pub struct LoaderVersionsArgs {
    pub loader: LoaderKind,
    pub game_version: String,
}

#[tauri::command]
pub async fn java_loader_versions(
    state: State<'_>,
    args: LoaderVersionsArgs,
) -> Res<Vec<LoaderVersion>> {
    let inst = &state.java.installer;
    match args.loader {
        LoaderKind::Vanilla => Ok(Vec::new()),
        LoaderKind::Fabric | LoaderKind::Quilt => {
            loaders::fabric::versions(inst, args.loader, &args.game_version)
                .await
                .map_err(java_err)
        }
        LoaderKind::Forge | LoaderKind::NeoForge => {
            loaders::forge::versions(inst, args.loader, &args.game_version)
                .await
                .map_err(java_err)
        }
    }
}

// ---- profiles ----

#[tauri::command]
pub fn java_profiles_list(state: State<'_>) -> Res<Vec<JavaProfileView>> {
    views(&state.java)
}

#[tauri::command]
pub fn java_profile_create(state: State<'_>, args: NewProfile) -> Res<JavaProfileView> {
    let memory = state
        .settings
        .lock()
        .expect("settings")
        .game
        .default_memory_mb;
    let p = state
        .java
        .profiles
        .create(args, memory, time::now())
        .map_err(java_err)?;
    let v = view(&state.java, p);
    profiles_changed(&state);
    Ok(v)
}

#[derive(Deserialize)]
pub struct UpdateArgs {
    pub id: String,
    pub patch: ProfilePatch,
}

#[tauri::command]
pub fn java_profile_update(state: State<'_>, args: UpdateArgs) -> Res<JavaProfileView> {
    let p = state
        .java
        .profiles
        .update(&args.id, args.patch)
        .map_err(java_err)?;
    let v = view(&state.java, p);
    profiles_changed(&state);
    Ok(v)
}

#[derive(Deserialize)]
pub struct DeleteArgs {
    pub id: String,
    #[serde(default)]
    pub delete_files: bool,
}

#[tauri::command]
pub fn java_profile_delete(state: State<'_>, args: DeleteArgs) -> Res<()> {
    if state.java.running(&args.id) || state.java.busy.lock().expect("busy").contains(&args.id) {
        return Err(java_err(JavaError::AlreadyRunning));
    }
    state
        .java
        .profiles
        .delete(&args.id, args.delete_files)
        .map_err(java_err)?;
    state.java.states.lock().expect("states").remove(&args.id);
    state.java.logs.lock().expect("logs").remove(&args.id);
    profiles_changed(&state);
    Ok(())
}

#[derive(Deserialize)]
pub struct IdArgs {
    pub id: String,
}

#[derive(Serialize)]
pub struct TaskStarted {
    pub task_id: String,
}

/// Installs a profile (game files, loader, Java) as a task. Mojang's files are downloaded for a
/// verified owner only (ADR 0005, 0147): the same owner gate as a launch.
#[tauri::command]
pub async fn java_profile_install(state: State<'_>, args: IdArgs) -> Res<TaskStarted> {
    let profile = state.java.profiles.get(&args.id).map_err(java_err)?;
    owner_auth(&state).await?;
    let st = state.inner().clone();
    let (task_id, cancel) = begin(&st, &profile)?;
    let tid = task_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = ensure_installed(&st, &profile, &tid, &cancel).await;
        finish_busy(&st, &profile.id);
        match result {
            Ok(_) => {
                st.tasks.finish(&tid);
                set_state(&st, JavaRunState::idle(&profile.id));
            }
            Err(e) => fail_run(&st, &profile.id, &tid, e),
        }
        profiles_changed(&st);
    });
    Ok(TaskStarted { task_id })
}

/// Marks a profile busy and starts its task.
fn begin(state: &Arc<AppState>, profile: &JavaProfile) -> Res<(String, CancellationToken)> {
    if state.java.running(&profile.id) {
        return Err(java_err(JavaError::AlreadyRunning));
    }
    if !state
        .java
        .busy
        .lock()
        .expect("busy")
        .insert(profile.id.clone())
    {
        return Err(IpcError::with_reason(
            IpcErrorCode::Conflict,
            "This profile is already being prepared.",
            "java.busy",
        ));
    }
    let (task_id, cancel) = state.tasks.start(
        "java",
        format!("Minecraft: Java Edition · {}", profile.name),
        None,
        None,
    );
    set_state(
        state,
        JavaRunState {
            state: "installing",
            task_id: Some(task_id.clone()),
            progress: Some(0.0),
            step: Some(Step::Metadata),
            ..JavaRunState::idle(&profile.id)
        },
    );
    Ok((task_id, cancel))
}

fn finish_busy(state: &AppState, id: &str) {
    state.java.busy.lock().expect("busy").remove(id);
}

fn fail_run(state: &AppState, id: &str, task_id: &str, e: IpcError) {
    let reason = e
        .details
        .as_ref()
        .and_then(|d| d.get("reason"))
        .and_then(|r| r.as_str())
        .map(str::to_owned);
    if reason.as_deref() == Some("java.cancelled") {
        // The player's own cancel: no error to show.
        state.tasks.fail(task_id, "cancelled");
        system_line(state, id, "Cancelled.".into());
        set_state(state, JavaRunState::idle(id));
        return;
    }
    state.tasks.fail(task_id, e.message.clone());
    system_line(state, id, format!("Failed: {}", e.message));
    set_state(
        state,
        JavaRunState {
            message: Some(e.message),
            reason,
            ..JavaRunState::idle(id)
        },
    );
}

/// One bar over an install, in the order it runs: the game's files (metadata, libraries,
/// assets), Mojang's Java, then the loader and its own files. Never goes back.
#[derive(Default)]
struct Overall {
    after_java: bool,
    last: f64,
}

impl Overall {
    fn at(&mut self, step: Step, done: u64, total: u64) -> f64 {
        let f = if total == 0 {
            1.0
        } else {
            (done as f64 / total as f64).clamp(0.0, 1.0)
        };
        if step == Step::Java {
            self.after_java = true;
        }
        let (start, span) = match (step, self.after_java) {
            (Step::Java, _) => (0.75, 0.15),
            (Step::Loader, _) => (0.9, 0.05),
            (_, true) => (0.95, 0.05),
            (Step::Metadata, false) => (0.0, 0.02),
            (Step::Libraries, false) => (0.02, 0.18),
            (Step::Assets, false) => (0.2, 0.55),
        };
        self.last = self.last.max((start + span * f).min(1.0));
        self.last
    }
}

/// The Java executable and major a profile uses: its own, else Mojang's runtime for `version`.
async fn java_for(
    state: &AppState,
    profile: &JavaProfile,
    version: &rusticmc_java::version::VersionJson,
    cancel: &CancellationToken,
    progress: &(dyn Fn(Step, u64, u64) + Send + Sync),
) -> Res<(PathBuf, Option<u32>)> {
    let (component, major) = version.runtime_component();
    if let Some(custom) = profile.java_path.as_deref() {
        let exe = runtime::custom_java(Path::new(custom)).map_err(java_err)?;
        let m = runtime::java_major(&exe);
        return Ok((exe, m));
    }
    let exe = state
        .java
        .installer
        .ensure_runtime(&component, cancel, progress)
        .await
        .map_err(java_err)?;
    Ok((exe.clone(), runtime::java_major(&exe).or(Some(major))))
}

/// Everything a profile needs, installed; returns the launchable version and its Java.
async fn ensure_installed(
    state: &Arc<AppState>,
    profile: &JavaProfile,
    task_id: &str,
    cancel: &CancellationToken,
) -> Res<(Installed, PathBuf, Option<u32>)> {
    let id = profile.id.clone();
    let st = state.clone();
    let tid = task_id.to_owned();
    let last = Mutex::new((Step::Metadata, -1.0f64, Overall::default()));
    let progress = move |step: Step, done: u64, total: u64| {
        let p = {
            let mut l = last.lock().expect("progress");
            let p = l.2.at(step, done, total);
            if l.0 == step && p - l.1 < 0.005 && done < total {
                return;
            }
            if l.0 != step {
                st.tasks.detail(&tid, step_name(step));
            }
            l.0 = step;
            l.1 = p;
            p
        };
        st.tasks.fraction(&tid, p);
        let mut s = current_state(&st, &id);
        s.step = Some(step);
        s.progress = Some(p);
        set_state(&st, s);
    };
    let inst = &state.java.installer;
    let features = launch::features_for(&launch_options_stub(profile));
    // The game itself first (a loader's installer reads its client jar).
    let vanilla = inst
        .install(&profile.game_version, &features, cancel, &progress)
        .await
        .map_err(java_err)?;
    let (java, major) = java_for(state, profile, &vanilla.version, cancel, &progress).await?;
    let mut picked_loader: Option<String> = None;
    let version_id = match profile.loader {
        LoaderKind::Vanilla => profile.game_version.clone(),
        loader => {
            let installed = profile
                .version_id
                .as_deref()
                .filter(|v| inst.version_file(v).is_file());
            match installed {
                Some(v) => v.to_owned(),
                None => {
                    progress(Step::Loader, 0, 1);
                    let lv = match profile.loader_version.clone() {
                        Some(v) => v,
                        None => {
                            let list = match loader {
                                LoaderKind::Fabric | LoaderKind::Quilt => {
                                    loaders::fabric::versions(inst, loader, &profile.game_version)
                                        .await
                                }
                                _ => {
                                    loaders::forge::versions(inst, loader, &profile.game_version)
                                        .await
                                }
                            }
                            .map_err(java_err)?;
                            loaders::pick_default(&list)
                                .map(|v| v.version.clone())
                                .ok_or_else(|| {
                                    java_err(JavaError::NoLoaderVersion {
                                        loader: loader.label().into(),
                                        game: profile.game_version.clone(),
                                    })
                                })?
                        }
                    };
                    system_line(
                        state,
                        &profile.id,
                        format!("Installing {} {lv}", loader.label()),
                    );
                    picked_loader = Some(lv.clone());
                    let st = state.clone();
                    let pid = profile.id.clone();
                    let id = match loader {
                        LoaderKind::Fabric | LoaderKind::Quilt => {
                            loaders::fabric::install(inst, loader, &profile.game_version, &lv).await
                        }
                        _ => {
                            loaders::forge::install(
                                inst,
                                loader,
                                &profile.game_version,
                                &lv,
                                &java,
                                cancel,
                                &move |l: &str| {
                                    push_log(
                                        &st,
                                        &pid,
                                        LogLine {
                                            stream: process::Stream::Stdout,
                                            level: None,
                                            thread: Some("installer".into()),
                                            text: l.to_owned(),
                                        },
                                    )
                                },
                            )
                            .await
                        }
                    }
                    .map_err(java_err)?;
                    progress(Step::Loader, 1, 1);
                    id
                }
            }
        }
    };
    if profile.version_id.as_deref() != Some(version_id.as_str())
        || (profile.loader != LoaderKind::Vanilla && profile.loader_version.is_none())
    {
        let vid = version_id.clone();
        state
            .java
            .profiles
            .modify(&profile.id, |p| {
                p.version_id = Some(vid);
                if p.loader_version.is_none() {
                    p.loader_version = picked_loader;
                }
                Ok(())
            })
            .map_err(java_err)?;
    }
    let installed = if version_id == profile.game_version {
        vanilla
    } else {
        inst.install(&version_id, &features, cancel, &progress)
            .await
            .map_err(java_err)?
    };
    Ok((installed, java, major))
}

fn launch_options_stub(profile: &JavaProfile) -> LaunchOptions {
    LaunchOptions {
        game_dir: PathBuf::new(),
        java: PathBuf::new(),
        java_major: None,
        memory_min_mb: profile.memory_min_mb,
        memory_max_mb: profile.memory_max_mb,
        resolution: profile.resolution.as_ref().map(|r| (r.width, r.height)),
        fullscreen: profile.fullscreen,
        jvm_args: profile.jvm_args.clone(),
        launcher_name: "rustic-launcher".into(),
        launcher_version: rusticmc_launcher_core::VERSION.into(),
        quick_play_server: None,
    }
}

// ---- launch ----

#[derive(Deserialize)]
pub struct LaunchArgs {
    pub id: String,
    /// Join this server on start (`host[:port]`), on versions with quick play.
    #[serde(default)]
    pub server: Option<String>,
}

/// The owner gate (the same as the Rustic edition's): the selected Microsoft account must own
/// the game; offline only within the grace window after a successful check.
async fn owner_auth(state: &AppState) -> Res<(Auth, bool)> {
    let uuid = match state.accounts.active()? {
        ActiveIdentity::Microsoft(uuid) => uuid,
        ActiveIdentity::None => {
            return Err(IpcError::with_reason(
                IpcErrorCode::OwnershipRequired,
                "Sign in with a Microsoft account that owns Minecraft: Java Edition to play.",
                "auth.no_account",
            ));
        }
    };
    let (session, offline) = match state.accounts.launch_session(Some(&uuid)).await? {
        LaunchAuth::Online(s) => (s, false),
        LaunchAuth::Offline { session, .. } => (session, true),
    };
    Ok((
        Auth {
            name: session.name,
            uuid: session.uuid.replace('-', ""),
            access_token: session.access_token,
            xuid: session.xuid,
        },
        offline,
    ))
}

#[tauri::command]
pub async fn java_profile_launch(state: State<'_>, args: LaunchArgs) -> Res<JavaRunState> {
    let profile = state.java.profiles.get(&args.id).map_err(java_err)?;
    if let Some(s) = &args.server
        && (s.is_empty()
            || s.len() > 255
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '_')))
    {
        return Err(IpcError::invalid("bad server address"));
    }
    // The owner gate first: no download, no start without ownership.
    let (auth, offline) = owner_auth(&state).await?;
    let st = state.inner().clone();
    let (task_id, cancel) = begin(&st, &profile)?;
    let tid = task_id.clone();
    st.java.logs.lock().expect("logs").remove(&profile.id);
    system_line(
        &st,
        &profile.id,
        format!(
            "Preparing {} ({} {})",
            profile.name,
            profile.loader.label(),
            profile.game_version
        ),
    );
    let server = args.server.clone();
    tauri::async_runtime::spawn(async move {
        let result = launch_task(&st, &profile, auth, offline, server, &tid, &cancel).await;
        finish_busy(&st, &profile.id);
        if let Err(e) = result {
            fail_run(&st, &profile.id, &tid, e);
        }
        profiles_changed(&st);
    });
    let s = current_state(&state, &args.id);
    Ok(JavaRunState { offline, ..s })
}

async fn launch_task(
    state: &Arc<AppState>,
    profile: &JavaProfile,
    auth: Auth,
    offline: bool,
    server: Option<String>,
    task_id: &str,
    cancel: &CancellationToken,
) -> Res<()> {
    let (installed, java, major) = ensure_installed(state, profile, task_id, cancel).await?;
    let profile = state.java.profiles.get(&profile.id).map_err(java_err)?;
    set_state(
        state,
        JavaRunState {
            state: "launching",
            task_id: Some(task_id.to_owned()),
            progress: Some(1.0),
            offline,
            ..JavaRunState::idle(&profile.id)
        },
    );
    let root = state.java.installer.root.clone();
    let game_dir = state.java.profiles.game_dir(&profile);
    let natives = state.java.natives_dir.join(&profile.id);
    {
        let (inst, root, natives, game_dir) = (
            installed.clone(),
            root.clone(),
            natives.clone(),
            game_dir.clone(),
        );
        tokio::task::spawn_blocking(move || launch::prepare(&inst, &root, &natives, &game_dir))
            .await
            .map_err(|e| IpcError::internal(e.to_string()))?
            .map_err(java_err)?;
    }
    let _ = std::fs::create_dir_all(game_dir.join("mods"));
    let mut opts = launch_options_stub(&profile);
    opts.game_dir = game_dir.clone();
    opts.java = java;
    opts.java_major = major;
    opts.quick_play_server = server;
    let cmd = launch::build(
        &installed,
        &root,
        &natives,
        &auth,
        &opts,
        &state.java.installer.platform,
    )
    .map_err(java_err)?;
    tracing::info!(profile = %profile.id, version = %installed.version.id, "starting Java Edition");
    tracing::debug!(command = %cmd.display(), "Java Edition command");
    system_line(
        state,
        &profile.id,
        format!("Starting {} as {}", installed.version.id, auth.name),
    );
    let st = state.clone();
    let pid = profile.id.clone();
    let handle = process::spawn(
        &cmd,
        &state.java.run_dir,
        Arc::new(move |line| push_log(&st, &pid, line)),
    )
    .map_err(java_err)?;
    state.tasks.finish(task_id);
    let started = time::now();
    state
        .java
        .games
        .lock()
        .expect("games")
        .insert(profile.id.clone(), handle.stop_token());
    set_state(
        state,
        JavaRunState {
            state: "running",
            started_at: Some(started),
            offline,
            ..JavaRunState::idle(&profile.id)
        },
    );
    let _ = state.java.profiles.modify(&profile.id, |p| {
        p.last_played_at = Some(started);
        Ok(())
    });
    profiles_changed(state);
    let exit = handle.done.await.unwrap_or(process::Exit {
        code: None,
        crashed: true,
        killed: false,
        crash_report: None,
    });
    state.java.games.lock().expect("games").remove(&profile.id);
    let played = time::now().saturating_sub(started);
    let _ = state.java.profiles.modify(&profile.id, |p| {
        p.play_time += played;
        Ok(())
    });
    system_line(
        state,
        &profile.id,
        format!(
            "The game stopped (exit code {}).",
            exit.code.map_or("none".into(), |c| c.to_string())
        ),
    );
    set_state(
        state,
        JavaRunState {
            state: if exit.crashed { "crashed" } else { "idle" },
            exit_code: exit.code,
            crash_report: exit.crash_report.map(|p| p.display().to_string()),
            ..JavaRunState::idle(&profile.id)
        },
    );
    Ok(())
}

#[tauri::command]
pub fn java_profile_stop(state: State<'_>, args: IdArgs) -> Res<JavaRunState> {
    if let Some(token) = state.java.games.lock().expect("games").get(&args.id) {
        token.cancel();
        return Ok(current_state(&state, &args.id));
    }
    // Still preparing: cancel its task.
    let task = current_state(&state, &args.id).task_id;
    match task {
        Some(t) if state.java.busy.lock().expect("busy").contains(&args.id) => {
            state.tasks.cancel(&t, &state.downloads);
            Ok(current_state(&state, &args.id))
        }
        _ => Err(java_err(JavaError::NotRunning)),
    }
}

#[tauri::command]
pub fn java_run_states(state: State<'_>) -> Vec<JavaRunState> {
    state
        .java
        .states
        .lock()
        .expect("states")
        .values()
        .cloned()
        .collect()
}

#[tauri::command]
pub fn java_profile_log(state: State<'_>, args: IdArgs) -> Vec<JavaLogLine> {
    state
        .java
        .logs
        .lock()
        .expect("logs")
        .get(&args.id)
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default()
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaFolder {
    Game,
    Mods,
    Logs,
    CrashReports,
    Screenshots,
}

#[derive(Deserialize)]
pub struct FolderArgs {
    pub id: String,
    pub folder: JavaFolder,
}

#[tauri::command]
pub fn java_profile_open_folder(
    app: tauri::AppHandle,
    state: State<'_>,
    args: FolderArgs,
) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    let p = state.java.profiles.get(&args.id).map_err(java_err)?;
    let base = state.java.profiles.game_dir(&p);
    let dir = match args.folder {
        JavaFolder::Game => base,
        JavaFolder::Mods => base.join("mods"),
        JavaFolder::Logs => base.join("logs"),
        JavaFolder::CrashReports => base.join("crash-reports"),
        JavaFolder::Screenshots => base.join("screenshots"),
    };
    std::fs::create_dir_all(&dir).map_err(|e| IpcError::internal(e.to_string()))?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| IpcError::internal(e.to_string()))
}

// ---- mods (Modrinth, filtered by the profile's game version and loader) ----

fn modrinth() -> Res<Modrinth> {
    // TODO_HUMAN C1: Modrinth asks for a contact in the User-Agent.
    Modrinth::new(ModrinthConfig {
        contact: "https://github.com/rustic-mc/rustic".into(),
        ..Default::default()
    })
    .map_err(|e| IpcError::with_reason(IpcErrorCode::Network, e.to_string(), "content"))
}

fn mods_dir(state: &AppState, p: &JavaProfile) -> PathBuf {
    state.java.profiles.game_dir(p).join("mods")
}

fn read_index(dir: &Path) -> ModIndex {
    rusticmc_launcher_core::fsutil::read_json(&dir.join(".rustic-mods.json"))
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn write_index(dir: &Path, index: &ModIndex) {
    let _ = rusticmc_launcher_core::fsutil::write_json(&dir.join(".rustic-mods.json"), index);
}

fn modded(p: &JavaProfile) -> Res<&'static str> {
    p.loader.modrinth().ok_or_else(|| {
        IpcError::with_reason(
            IpcErrorCode::InvalidArgument,
            "Pick a mod loader for this profile first.",
            "java.vanilla_mods",
        )
    })
}

#[derive(Deserialize)]
pub struct ModSearchArgs {
    pub profile_id: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[tauri::command]
pub async fn java_mods_search(state: State<'_>, args: ModSearchArgs) -> Res<JavaModSearch> {
    let p = state
        .java
        .profiles
        .get(&args.profile_id)
        .map_err(java_err)?;
    let loader = modded(&p)?;
    let page = modrinth()?
        .search(&SearchQuery {
            query: args.query.chars().take(100).collect(),
            kind: Some(ContentKind::Mod),
            facets: vec![vec![format!("categories:{loader}")]],
            game_version: Some(p.game_version.clone()),
            offset: args.offset,
            limit: args.limit.unwrap_or(20).min(50),
            index: Some("relevance".into()),
        })
        .await
        .map_err(|e| IpcError::with_reason(IpcErrorCode::Network, e.to_string(), "content"))?;
    let policy = FairPlayPolicy::default();
    let installed: HashSet<String> = read_index(&mods_dir(&state, &p))
        .files
        .into_values()
        .collect();
    let hits = page
        .hits
        .into_iter()
        .filter_map(|h| {
            let verdict = policy.judge(&h);
            if matches!(verdict, FairPlay::Blocked { .. }) {
                return None;
            }
            Some(JavaModHit {
                installed: installed.contains(&h.id),
                flagged: matches!(verdict, FairPlay::Flagged { .. }),
                project_id: h.id,
                slug: h.slug,
                name: h.name,
                summary: h.summary,
                author: h.authors.first().cloned().unwrap_or_default(),
                icon_url: h.icon_url,
                downloads: h.downloads,
            })
        })
        .collect();
    Ok(JavaModSearch {
        hits,
        total: page.total,
    })
}

#[tauri::command]
pub fn java_mods_list(state: State<'_>, args: ProfileArgs) -> Res<Vec<JavaModFile>> {
    let p = state
        .java
        .profiles
        .get(&args.profile_id)
        .map_err(java_err)?;
    let dir = mods_dir(&state, &p);
    let index = read_index(&dir);
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let (base, enabled) = match name.strip_suffix(".disabled") {
                Some(b) => (b.to_owned(), false),
                None => (name.clone(), true),
            };
            if !base.ends_with(".jar") {
                continue;
            }
            out.push(JavaModFile {
                project_id: index.files.get(&base).cloned(),
                file_name: base,
                enabled,
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    out.sort_by_key(|a| a.file_name.to_lowercase());
    Ok(out)
}

#[derive(Deserialize)]
pub struct ProfileArgs {
    pub profile_id: String,
}

#[derive(Deserialize)]
pub struct ModInstallArgs {
    pub profile_id: String,
    pub project_id: String,
}

/// A safe mod file name (no folders).
fn safe_file_name(name: &str) -> Option<String> {
    let ok = !name.is_empty()
        && name.len() <= 200
        && name.ends_with(".jar")
        && !name.starts_with('.')
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | ' ' | '(' | ')')
        });
    ok.then(|| name.to_owned())
}

/// Installs the project's newest version for the profile's game version and loader, and its
/// required dependencies (fair-play blocked projects refused).
#[tauri::command]
pub async fn java_mod_install(state: State<'_>, args: ModInstallArgs) -> Res<JavaModFile> {
    let p = state
        .java
        .profiles
        .get(&args.profile_id)
        .map_err(java_err)?;
    let loader = modded(&p)?;
    let dir = mods_dir(&state, &p);
    std::fs::create_dir_all(&dir).map_err(|e| IpcError::internal(e.to_string()))?;
    let mr = modrinth()?;
    let policy = FairPlayPolicy::default();
    let mut index = read_index(&dir);
    let mut queue = vec![args.project_id.clone()];
    let mut done: HashSet<String> = HashSet::new();
    let mut first: Option<JavaModFile> = None;
    while let Some(project) = queue.pop() {
        if !done.insert(project.clone()) || done.len() > 25 {
            continue;
        }
        let net = |e: rusticmc_content::ContentError| {
            IpcError::with_reason(IpcErrorCode::Network, e.to_string(), "content")
        };
        let raw = mr.project(&project).await.map_err(net)?;
        let blocked = std::iter::once(raw.title.as_str())
            .chain(std::iter::once(raw.slug.as_str()))
            .chain(raw.categories.iter().map(String::as_str))
            .find_map(|t| policy.matches(t));
        if blocked.is_some() || policy.denied.contains(&raw.id.to_lowercase()) {
            return Err(IpcError::with_reason(
                IpcErrorCode::Conflict,
                format!("{} is not allowed by Rustic's fair-play policy.", raw.title),
                "content.fair_play",
            ));
        }
        if index.files.values().any(|v| *v == raw.id) && project != args.project_id {
            continue;
        }
        let versions = mr
            .versions(&project, &[loader], &[p.game_version.as_str()], None)
            .await
            .map_err(net)?;
        let Some(version) = versions.into_iter().next() else {
            return Err(IpcError::with_reason(
                IpcErrorCode::Conflict,
                format!(
                    "{} has no version for {} {}.",
                    raw.title,
                    p.loader.label(),
                    p.game_version
                ),
                "java.mod_incompatible",
            ));
        };
        let file = version
            .primary_file()
            .cloned()
            .ok_or_else(|| IpcError::not_found("the version has no file"))?;
        let name = safe_file_name(&file.filename)
            .ok_or_else(|| IpcError::invalid("the mod's file name is not safe"))?;
        let host = file
            .url
            .strip_prefix("https://")
            .and_then(|r| r.split('/').next())
            .unwrap_or_default();
        if !ALLOWED_HOSTS.contains(&host) {
            return Err(IpcError::invalid(
                "the mod's file is not on Modrinth's hosts",
            ));
        }
        let mut req = DownloadRequest::new(&file.url, dir.join(&name)).label(raw.title.clone());
        if let Some(c) = file
            .hashes
            .sha512
            .as_deref()
            .and_then(Checksum::sha512)
            .or_else(|| file.hashes.sha1.as_deref().and_then(Checksum::sha1))
        {
            req = req.checksum(c);
        }
        if let Some(s) = file.size {
            req = req.size(s);
        }
        state.downloads.download(req).await?;
        // An older file of the same project goes.
        let old: Vec<String> = index
            .files
            .iter()
            .filter(|(f, v)| **v == raw.id && **f != name)
            .map(|(f, _)| f.clone())
            .collect();
        for f in old {
            let _ = std::fs::remove_file(dir.join(&f));
            let _ = std::fs::remove_file(dir.join(format!("{f}.disabled")));
            index.files.remove(&f);
        }
        index.files.insert(name.clone(), raw.id.clone());
        if first.is_none() {
            first = Some(JavaModFile {
                size: file.size.unwrap_or(0),
                file_name: name,
                enabled: true,
                project_id: Some(raw.id.clone()),
            });
        }
        for d in &version.dependencies {
            if d.kind == DependencyKind::Required
                && let Some(pid) = &d.project_id
            {
                queue.push(pid.clone());
            }
        }
    }
    write_index(&dir, &index);
    first.ok_or_else(|| IpcError::internal("nothing installed"))
}

#[derive(Deserialize)]
pub struct ModFileArgs {
    pub profile_id: String,
    pub file_name: String,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[tauri::command]
pub fn java_mod_set_enabled(state: State<'_>, args: ModFileArgs) -> Res<JavaModFile> {
    let p = state
        .java
        .profiles
        .get(&args.profile_id)
        .map_err(java_err)?;
    let dir = mods_dir(&state, &p);
    let name = safe_file_name(&args.file_name).ok_or_else(|| IpcError::invalid("bad file name"))?;
    let on = dir.join(&name);
    let off = dir.join(format!("{name}.disabled"));
    let enabled = args.enabled.unwrap_or(true);
    let (from, to) = if enabled { (&off, &on) } else { (&on, &off) };
    if from.is_file() {
        std::fs::rename(from, to).map_err(|e| IpcError::internal(e.to_string()))?;
    } else if !to.is_file() {
        return Err(IpcError::not_found("no such mod"));
    }
    Ok(JavaModFile {
        size: std::fs::metadata(to).map(|m| m.len()).unwrap_or(0),
        project_id: read_index(&dir).files.get(&name).cloned(),
        file_name: name,
        enabled,
    })
}

#[tauri::command]
pub fn java_mod_remove(state: State<'_>, args: ModFileArgs) -> Res<()> {
    let p = state
        .java
        .profiles
        .get(&args.profile_id)
        .map_err(java_err)?;
    let dir = mods_dir(&state, &p);
    let name = safe_file_name(&args.file_name).ok_or_else(|| IpcError::invalid("bad file name"))?;
    let _ = std::fs::remove_file(dir.join(&name));
    let _ = std::fs::remove_file(dir.join(format!("{name}.disabled")));
    let mut index = read_index(&dir);
    if index.files.remove(&name).is_some() {
        write_index(&dir, &index);
    }
    Ok(())
}

// ---- official edition icons ----

#[derive(Deserialize)]
pub struct IconArgs {
    pub edition: String,
}

/// The official Java or Bedrock edition icon from minecraft.net, as a `data:` URL (the webview
/// can't load it directly: the CDN refuses automated browsers); `None` offline or on any failure
/// (the UI shows a neutral icon). Fetched once per session, never stored on disk.
#[tauri::command]
pub async fn edition_icon(state: State<'_>, args: IconArgs) -> Res<Option<String>> {
    use base64::Engine;
    let Some((_, url)) = rusticmc_java::OFFICIAL_ICONS
        .iter()
        .find(|(e, _)| *e == args.edition)
    else {
        return Err(IpcError::invalid("unknown edition"));
    };
    let mut cache = state.java.icons.lock().await;
    if let Some(hit) = cache.get(&args.edition) {
        return Ok(hit.clone());
    }
    let fetched = async {
        let resp = state
            .http
            .get(*url)
            .header(
                rusticmc_launcher_core::reqwest::header::USER_AGENT,
                rusticmc_java::icon_user_agent(),
            )
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let mime = resp
            .headers()
            .get(rusticmc_launcher_core::reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(';').next().unwrap_or(v).trim().to_owned())
            .filter(|m| {
                matches!(
                    m.as_str(),
                    "image/png" | "image/webp" | "image/jpeg" | "image/gif"
                )
            })?;
        let bytes = resp.bytes().await.ok()?;
        (bytes.len() <= 256 * 1024).then(|| {
            format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            )
        })
    }
    .await;
    match &fetched {
        // Kept for the session; a failure is asked again next time (the network may be back).
        Some(_) => {
            cache.insert(args.edition, fetched.clone());
        }
        None => tracing::debug!(edition = %args.edition, "official edition icon unavailable"),
    }
    Ok(fetched)
}

// ---- Bedrock (Windows) ----

#[tauri::command]
pub async fn bedrock_status(state: State<'_>) -> Res<bedrock::BedrockStatus> {
    let s = bedrock::detect().await;
    *state.java.bedrock.lock().await = Some(s.clone());
    Ok(s)
}

#[tauri::command]
pub async fn bedrock_launch(state: State<'_>) -> Res<()> {
    let status = {
        let cached = state.java.bedrock.lock().await.clone();
        match cached {
            Some(s) if s.installed => s,
            _ => bedrock::detect().await,
        }
    };
    bedrock::launch(&status).await.map_err(java_err)
}

#[tauri::command]
pub fn bedrock_open_store(app: tauri::AppHandle) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    if !cfg!(windows) {
        return Err(IpcError::unavailable("Windows only"));
    }
    app.opener()
        .open_url(bedrock::STORE_URL, None::<&str>)
        .or_else(|_| app.opener().open_url(bedrock::STORE_WEB_URL, None::<&str>))
        .map_err(|e| IpcError::internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_follows_the_install_order() {
        // A loader profile: the game's files, Java, the loader, the loader's files.
        let order = [
            Step::Metadata,
            Step::Libraries,
            Step::Assets,
            Step::Java,
            Step::Loader,
            Step::Metadata,
            Step::Libraries,
            Step::Assets,
        ];
        let mut o = Overall::default();
        let mut last = 0.0;
        for s in order {
            for (d, t) in [(0, 10), (5, 10), (10, 10)] {
                let p = o.at(s, d, t);
                assert!(p >= last, "{s:?} {d}/{t}");
                last = p;
            }
        }
        assert!((last - 1.0).abs() < 1e-9);
        // The assets are most of the bar.
        let mut o = Overall::default();
        assert!(o.at(Step::Assets, 5, 10) > 0.4);
        // A custom Java (no Java step): never backwards either.
        let mut o = Overall::default();
        let a = o.at(Step::Assets, 10, 10);
        assert!(o.at(Step::Libraries, 0, 10) >= a);
    }

    #[test]
    fn mod_file_names() {
        assert!(safe_file_name("sodium-fabric-0.6.0+mc1.21.1.jar").is_some());
        assert!(safe_file_name("../evil.jar").is_none());
        assert!(safe_file_name("a/b.jar").is_none());
        assert!(safe_file_name("a.zip").is_none());
        assert!(safe_file_name(".hidden.jar").is_none());
    }
}
