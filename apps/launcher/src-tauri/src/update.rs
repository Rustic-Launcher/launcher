//! Automatic updates of the launcher itself and of the Rustic edition build (ADR 0162).
//!
//! Checks run at start, before a launch (Play), when a game exits, every few hours and on
//! demand. What they find is installed without a prompt when "Install updates automatically" is
//! on (Settings > Launcher), else offered as "Update and restart":
//!
//! - **Rustic edition**: the channel's newest stable build is installed as a new build folder (the
//!   running game's folder is never touched); instances that follow the newest build of the
//!   source (`BuildChoice.version == None`) play it from their next launch, pinned ones stay.
//! - **Launcher**: Tauri's updater (minisign-signed NSIS installer, static `latest.json`); the
//!   installer runs passive and starts the new launcher, which resumes a launch the update
//!   interrupted (`pending-launch.json`, taken by the UI with `app_take_pending_launch`).
//!
//! **A running game is never closed**: while one runs (tracked by this launcher, still starting,
//! or any `rustic-client`/`horizon-client` process, such as one started before a restart), the
//! updates wait and are applied as soon as it exits. Failures never block playing: they are
//! logged, shown in Settings, and retried later.

use crate::dto::{PendingLaunch, UpdateItem, UpdateStatus};
use crate::state::AppState;
use rusticmc_instances::{BuildRelease, BuildSource};
use rusticmc_launcher_core::{DownloadEvent, fsutil, time};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tauri_plugin_updater::UpdaterExt;

/// The build source the edition updates follow.
pub const EDITION_SOURCE: &str = "rustic-edition";
/// Overrides the launcher's update endpoint (a staging feed; release builds accept https only).
pub const UPDATE_URL_ENV: &str = "RUSTIC_LAUNCHER_UPDATE_URL";
/// Lets a debug build update itself (never by default: `tauri dev` would install the release).
pub const DEV_UPDATES_ENV: &str = "RUSTIC_UPDATE_IN_DEV";

/// Between two background checks.
const PERIOD: Duration = Duration::from_secs(4 * 3600);
/// After a failed check.
const RETRY: Duration = Duration::from_secs(30 * 60);
/// A launcher version whose install did not complete is not retried automatically before this.
const FAILED_INSTALL_BACKOFF: u64 = 6 * 3600;
/// A launch interrupted by an update is resumed only this soon after.
const PENDING_LAUNCH_TTL: u64 = 30 * 60;
/// The feeds' answer, at most (Play waits for it).
const FEED_TIMEOUT: Duration = Duration::from_secs(20);
const PLAY_FEED_TIMEOUT: Duration = Duration::from_secs(8);
/// A check this recent is reused by Play instead of asking the feeds again.
const FRESH: u64 = 60;
/// Game programs: while one runs, nothing is installed.
const GAME_PROGRAMS: [&str; 2] = ["rustic-client", "horizon-client"];

const STATE_FILE: &str = "updates.json";
const PENDING_FILE: &str = "pending-launch.json";

/// What starts a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Play,
    GameExit,
    Periodic,
    /// "Check now" in Settings: checks and, automatic updates on, applies.
    Manual,
    /// "Update and restart": applies whatever was found, automatic updates on or off.
    Apply,
}

impl Trigger {
    fn forced(self) -> bool {
        self == Self::Apply
    }
}

/// What survives a restart (`<config>/updates.json`).
#[derive(Default, Serialize, Deserialize)]
struct Persisted {
    #[serde(default)]
    last_check: Option<u64>,
    /// A launcher install handed to the installer: `(from, to, at)`.
    #[serde(default)]
    installing: Option<(String, String, u64)>,
    /// A launcher version whose install did not complete, and when.
    #[serde(default)]
    failed: Option<(String, u64)>,
}

/// The update service, one per app.
pub struct Updates {
    status: Mutex<UpdateStatus>,
    launcher: Mutex<Option<tauri_plugin_updater::Update>>,
    edition: Mutex<Option<(BuildSource, BuildRelease)>>,
    /// One check or install at a time.
    lock: tokio::sync::Mutex<()>,
    /// A launch waiting for the updates (resumed after a restart).
    waiting_play: Mutex<Option<PendingLaunch>>,
    watching: AtomicBool,
    last_emit: Mutex<Option<Instant>>,
    file: PathBuf,
    pending_file: PathBuf,
}

impl Updates {
    /// Reads what the previous run left: a finished self-update (`updated_from`) or one that did
    /// not complete (retried later).
    pub fn new(config_dir: &std::path::Path, current: &str, auto: bool) -> Self {
        let file = config_dir.join(STATE_FILE);
        let mut p: Persisted = fsutil::read_json(&file).ok().flatten().unwrap_or_default();
        let mut updated_from = None;
        if let Some((from, to, at)) = p.installing.take() {
            if to == current {
                tracing::info!(%from, %to, "launcher updated");
                updated_from = Some(from);
                p.failed = None;
            } else {
                tracing::warn!(%to, %current, "the launcher update did not complete: retried later");
                p.failed = Some((to, at));
            }
            let _ = fsutil::write_json(&file, &p);
        }
        Self {
            status: Mutex::new(UpdateStatus {
                phase: "idle",
                current_version: current.to_owned(),
                channel: "stable",
                auto,
                last_check: p.last_check.map(crate::util::iso),
                launcher: None,
                edition: None,
                edition_current: None,
                working_on: None,
                progress: None,
                error: None,
                updated_from,
                resume_instance: None,
            }),
            launcher: Mutex::new(None),
            edition: Mutex::new(None),
            lock: tokio::sync::Mutex::new(()),
            waiting_play: Mutex::new(None),
            watching: AtomicBool::new(false),
            last_emit: Mutex::new(None),
            pending_file: config_dir.join(PENDING_FILE),
            file,
        }
    }

    fn persisted(&self) -> Persisted {
        fsutil::read_json(&self.file)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn save(&self, p: &Persisted) {
        if let Err(e) = fsutil::write_json(&self.file, p) {
            tracing::warn!(error = %e, "could not save the update state");
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.status.lock().expect("updates").clone()
    }

    /// The launch an update interrupted, once, if recent.
    pub fn take_pending_launch(&self) -> Option<PendingLaunch> {
        let p: Option<PendingLaunch> = fsutil::read_json(&self.pending_file).ok().flatten();
        let _ = std::fs::remove_file(&self.pending_file);
        self.status.lock().expect("updates").resume_instance = None;
        p.filter(|p| time::now().saturating_sub(p.created_at) <= PENDING_LAUNCH_TTL)
    }

    fn peek_pending_launch(&self) -> Option<String> {
        fsutil::read_json::<PendingLaunch>(&self.pending_file)
            .ok()
            .flatten()
            .filter(|p| time::now().saturating_sub(p.created_at) <= PENDING_LAUNCH_TTL)
            .map(|p| p.instance_id)
    }
}

fn set(state: &AppState, f: impl FnOnce(&mut UpdateStatus)) {
    let status = {
        let mut s = state.updates.status.lock().expect("updates");
        f(&mut s);
        s.clone()
    };
    *state.updates.last_emit.lock().expect("updates") = Some(Instant::now());
    let _ = state.app.emit("update-state", status);
}

/// Progress, at most five events a second.
fn progress(state: &AppState, working_on: &'static str, phase: &'static str, value: Option<f64>) {
    {
        let mut s = state.updates.status.lock().expect("updates");
        s.working_on = Some(working_on);
        s.phase = phase;
        s.progress = value;
    }
    let due = state
        .updates
        .last_emit
        .lock()
        .expect("updates")
        .is_none_or(|t| t.elapsed() >= Duration::from_millis(200));
    if due {
        set(state, |_| {});
        // The launch waiting for this update shows its progress on the Play button.
        let waiting = state
            .updates
            .waiting_play
            .lock()
            .expect("updates")
            .as_ref()
            .map(|p| p.instance_id.clone());
        if let Some(id) = waiting {
            use crate::cmd::instances::{phase as launch_phase, set_launch};
            set_launch(
                state,
                crate::dto::LaunchStatus {
                    progress: value,
                    ..launch_phase(&id, "updating", None)
                },
            );
        }
    }
}

/// Settings > Launcher > "Install updates automatically" changed: turned on, what waits is
/// applied now.
pub fn auto_changed(state: &Arc<AppState>, auto: bool) {
    let was = state.updates.status().auto;
    set(state, |s| s.auto = auto);
    if auto && !was {
        let st = state.clone();
        tauri::async_runtime::spawn(async move { run(&st, Trigger::Manual, None).await });
    }
}

fn auto_enabled(state: &AppState) -> bool {
    state
        .settings
        .lock()
        .expect("settings")
        .launcher
        .auto_update
}

/// Whether the launcher may replace itself: release builds, or a debug build that asks for it.
fn self_update_allowed() -> bool {
    !cfg!(debug_assertions) || std::env::var_os(DEV_UPDATES_ENV).is_some()
}

/// A game is running or starting (other than `except`, the launch waiting for this check).
pub fn game_running(state: &AppState, except: Option<&str>) -> bool {
    let other = |id: &str| except != Some(id);
    if state.instances.running().iter().any(|id| other(id)) {
        return true;
    }
    let starting = state.launches.lock().expect("launches").values().any(|l| {
        other(&l.instance_id)
            && matches!(
                l.phase,
                "checking" | "downloading" | "preparing" | "building" | "launching" | "updating"
            )
    });
    starting || game_process_running()
}

/// Any `rustic-client` or `horizon-client` process: a game this launcher no longer tracks (it was
/// restarted), a native-mods client, or one started another way.
fn game_process_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .values()
        .any(|p| is_game_program(&p.name().to_string_lossy()))
}

fn is_game_program(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    GAME_PROGRAMS.contains(&stem)
}

fn updater(state: &AppState) -> Result<tauri_plugin_updater::Updater, String> {
    let mut b = state
        .app
        .updater_builder()
        .timeout(Duration::from_secs(120));
    if let Some(url) = std::env::var(UPDATE_URL_ENV)
        .ok()
        .filter(|u| !u.trim().is_empty())
    {
        let url = url
            .trim()
            .parse()
            .map_err(|e| format!("{UPDATE_URL_ENV}: {e}"))?;
        b = b.endpoints(vec![url]).map_err(|e| e.to_string())?;
    }
    b.build().map_err(|e| e.to_string())
}

/// The edition source, when configured.
fn edition_source(state: &AppState) -> Option<BuildSource> {
    state
        .sources()
        .into_iter()
        .find(|s| s.config.id == EDITION_SOURCE)
}

/// The newest installed edition build.
fn edition_current(state: &AppState) -> Option<String> {
    state
        .instances
        .builds
        .installed()
        .into_iter()
        .find(|b| b.source == EDITION_SOURCE)
        .map(|b| b.version)
}

/// Whether a new edition build is worth installing: an instance follows the source's newest
/// build, or the edition is the default source and already in use (servers and new instances
/// take its newest build).
fn edition_wanted(state: &AppState) -> bool {
    let follows = state
        .instances
        .store
        .list()
        .iter()
        .any(|i| i.build.source == EDITION_SOURCE && i.build.version.is_none());
    follows
        || (state.config().builds.default_source == EDITION_SOURCE
            && edition_current(state).is_some())
}

/// Asks both feeds. Errors are joined for the log and Settings.
async fn check(state: &Arc<AppState>, timeout: Duration) -> Result<(), String> {
    set(state, |s| {
        s.phase = "checking";
        s.error = None;
        s.edition_current = edition_current(state);
    });
    let mut errors = Vec::new();
    let launcher = async {
        if !self_update_allowed() {
            return Ok(None);
        }
        let u = updater(state)?;
        u.check().await.map_err(|e| e.to_string())
    };
    let edition = async {
        let Some(source) = edition_source(state) else {
            return Ok(None);
        };
        if !edition_wanted(state) {
            return Ok(None);
        }
        let installed = state.instances.builds.installed();
        let latest = state
            .instances
            .builds
            .latest(&source)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(
            latest
                .filter(|r| !installed.iter().any(|b| b.version == r.version))
                .map(|r| (source, r)),
        )
    };
    let (l, e) =
        match tokio::time::timeout(timeout, async { tokio::join!(launcher, edition) }).await {
            Ok(r) => r,
            Err(_) => (
                Err("the update server did not answer in time".into()),
                Err("the edition channel did not answer in time".into()),
            ),
        };
    let launcher = l.unwrap_or_else(|e| {
        errors.push(format!("launcher: {e}"));
        None
    });
    let edition = e.unwrap_or_else(|e| {
        errors.push(format!("Rustic edition: {e}"));
        None
    });
    let now = time::now();
    let mut p = state.updates.persisted();
    p.last_check = Some(now);
    state.updates.save(&p);
    let launcher_item = launcher.as_ref().map(|u| UpdateItem {
        version: u.version.clone(),
        notes: u.body.clone(),
        size: None,
        published_at: u
            .raw_json
            .get("pub_date")
            .and_then(|d| d.as_str())
            .map(str::to_owned),
    });
    let edition_item = edition.as_ref().map(|(_, r)| UpdateItem {
        version: r.version.clone(),
        notes: Some(r.notes.clone()).filter(|n| !n.is_empty()),
        size: r.asset.as_ref().and_then(|a| a.size),
        published_at: Some(r.published_at.clone()).filter(|d| !d.is_empty()),
    });
    *state.updates.launcher.lock().expect("updates") = launcher;
    *state.updates.edition.lock().expect("updates") = edition;
    if !errors.is_empty() {
        tracing::warn!(errors = %errors.join("; "), "update check");
    }
    set(state, |s| {
        s.last_check = Some(crate::util::iso(now));
        s.launcher = launcher_item;
        s.edition = edition_item;
        s.phase = if errors.is_empty() { "idle" } else { "error" };
        s.working_on = None;
        s.progress = None;
        s.error = (!errors.is_empty()).then(|| errors.join("; "));
    });
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Checks (unless `Play` finds a fresh check) and applies what it may; failures are reported in
/// the status.
pub async fn run(state: &Arc<AppState>, trigger: Trigger, play: Option<PendingLaunch>) {
    let except = play.as_ref().map(|p| p.instance_id.clone());
    if let Some(p) = &play {
        *state.updates.waiting_play.lock().expect("updates") = Some(p.clone());
    }
    let _guard = state.updates.lock.lock().await;
    let auto = auto_enabled(state);
    set(state, |s| s.auto = auto);
    let fresh = state
        .updates
        .persisted()
        .last_check
        .is_some_and(|t| time::now().saturating_sub(t) <= FRESH);
    if !(trigger == Trigger::Apply || (trigger == Trigger::Play && fresh)) {
        let timeout = if trigger == Trigger::Play {
            PLAY_FEED_TIMEOUT
        } else {
            FEED_TIMEOUT
        };
        // One feed failing (logged, shown in Settings) never holds back what the other found.
        let _ = check(state, timeout).await;
    }
    apply(state, trigger, auto, except.as_deref()).await;
    *state.updates.waiting_play.lock().expect("updates") = None;
}

async fn apply(state: &Arc<AppState>, trigger: Trigger, auto: bool, except: Option<&str>) {
    let has_launcher = state.updates.launcher.lock().expect("updates").is_some();
    let has_edition = state.updates.edition.lock().expect("updates").is_some();
    if !has_launcher && !has_edition {
        return;
    }
    if !auto && !trigger.forced() {
        set(state, |s| s.phase = "available");
        return;
    }
    // The launching instance needs the edition build it follows now; anything else waits for
    // the games to exit (no install competes with a running game).
    let busy = game_running(state, except);
    if has_edition && (!busy || trigger == Trigger::Play) && !install_edition(state).await {
        return;
    }
    if !has_launcher {
        return;
    }
    if busy {
        tracing::info!("launcher update deferred: a game is running");
        set(state, |s| s.phase = "deferred");
        watch_games(state.clone());
        return;
    }
    let version = state
        .updates
        .launcher
        .lock()
        .expect("updates")
        .as_ref()
        .map(|u| u.version.clone())
        .unwrap_or_default();
    if !trigger.forced()
        && let Some((failed, at)) = state.updates.persisted().failed
        && failed == version
        && time::now().saturating_sub(at) < FAILED_INSTALL_BACKOFF
    {
        tracing::info!(%version, "launcher update not retried yet (its last install did not complete)");
        set(state, |s| s.phase = "available");
        return;
    }
    install_launcher(state).await;
}

/// Installs the edition build found; false when it failed.
async fn install_edition(state: &Arc<AppState>) -> bool {
    let Some((source, release)) = state.updates.edition.lock().expect("updates").clone() else {
        return true;
    };
    tracing::info!(version = %release.version, "installing the Rustic edition build");
    progress(state, "edition", "downloading", Some(0.0));
    let (task, cancel) = state.tasks.start(
        "update",
        format!("Horizon {} (Rustic edition)", release.version),
        None,
        None,
    );
    // The archive's bytes, from the download manager's events for its label.
    let label = format!("Horizon {}", release.version);
    let mut events = state.downloads.subscribe();
    let st = state.clone();
    let task2 = task.clone();
    let follow = tauri::async_runtime::spawn(async move {
        let mut id = None;
        while let Ok(e) = events.recv().await {
            match e {
                DownloadEvent::Queued {
                    id: i, label: l, ..
                } if l.as_deref() == Some(&label) => id = Some(i),
                DownloadEvent::Progress {
                    id: i,
                    downloaded,
                    total: Some(total),
                    ..
                } if Some(i) == id && total > 0 => {
                    let f = downloaded as f64 / total as f64;
                    st.tasks.fraction(&task2, 0.05 + f * 0.85);
                    progress(&st, "edition", "downloading", Some(0.05 + f * 0.85));
                }
                _ => {}
            }
        }
    });
    let st = state.clone();
    let task3 = task.clone();
    let result = tokio::select! {
        _ = cancel.cancelled() => Err("cancelled".to_owned()),
        r = state.instances.builds.install(&source, &release, move |stage| {
            let (phase, f) = match stage {
                rusticmc_instances::InstallStage::Verifying => ("downloading", 0.02),
                rusticmc_instances::InstallStage::Downloading => ("downloading", 0.05),
                rusticmc_instances::InstallStage::Unpacking => ("installing", 0.92),
                rusticmc_instances::InstallStage::Done { .. } => ("installing", 1.0),
            };
            st.tasks.fraction(&task3, f);
            progress(&st, "edition", phase, Some(f));
        }) => r.map(|_| ()).map_err(|e| e.to_string()),
    };
    follow.abort();
    match result {
        Ok(()) => {
            state.tasks.finish(&task);
            tracing::info!(version = %release.version, "Rustic edition build installed");
            *state.updates.edition.lock().expect("updates") = None;
            crate::cmd::instances::instances_changed(state);
            let _ = state.app.emit(
                "update-installed",
                serde_json::json!({ "kind": "edition", "version": release.version }),
            );
            set(state, |s| {
                s.edition = None;
                s.edition_current = edition_current(state);
                s.phase = "idle";
                s.working_on = None;
                s.progress = None;
            });
            true
        }
        Err(e) => {
            state.tasks.fail(&task, e.clone());
            tracing::warn!(version = %release.version, error = %e, "the Rustic edition build was not installed");
            set(state, |s| {
                s.phase = "error";
                s.working_on = None;
                s.progress = None;
                s.error = Some(format!("Rustic edition {}: {e}", release.version));
            });
            false
        }
    }
}

/// Downloads and runs the launcher's installer; on success the process ends (Windows) or
/// restarts. A launch waiting for the update is written down first, to resume after.
async fn install_launcher(state: &Arc<AppState>) {
    let Some(update) = state.updates.launcher.lock().expect("updates").clone() else {
        return;
    };
    let version = update.version.clone();
    tracing::info!(%version, "downloading the launcher update");
    progress(state, "launcher", "downloading", Some(0.0));
    let (task, _) = state
        .tasks
        .start("update", format!("Rustic Launcher {version}"), None, None);
    let st = state.clone();
    let task2 = task.clone();
    let mut received = 0u64;
    let bytes = update
        .download(
            move |chunk, total| {
                received += chunk as u64;
                st.tasks.bytes(&task2, received, total);
                let f = total.filter(|&t| t > 0).map(|t| received as f64 / t as f64);
                progress(&st, "launcher", "downloading", f);
            },
            || {},
        )
        .await;
    let bytes = match bytes {
        Ok(b) => b,
        Err(e) => {
            state.tasks.fail(&task, e.to_string());
            tracing::warn!(%version, error = %e, "the launcher update was not downloaded");
            set(state, |s| {
                s.phase = "error";
                s.working_on = None;
                s.progress = None;
                s.error = Some(format!("Rustic Launcher {version}: {e}"));
            });
            return;
        }
    };
    state.tasks.finish(&task);
    // A game started during the download: install when it exits (the bytes are fetched again).
    if game_running(
        state,
        state
            .updates
            .waiting_play
            .lock()
            .expect("updates")
            .as_ref()
            .map(|p| p.instance_id.as_str()),
    ) {
        set(state, |s| s.phase = "deferred");
        watch_games(state.clone());
        return;
    }
    let waiting = state.updates.waiting_play.lock().expect("updates").clone();
    if let Some(p) = waiting
        && let Err(e) = fsutil::write_json(&state.updates.pending_file, &p)
    {
        tracing::warn!(error = %e, "could not keep the launch to resume");
    }
    let mut p = state.updates.persisted();
    p.installing = Some((
        state.updates.status().current_version,
        version.clone(),
        time::now(),
    ));
    state.updates.save(&p);
    set(state, |s| {
        s.phase = "restarting";
        s.progress = Some(1.0);
    });
    // Let the window show "Restarting…" before the installer takes over.
    tokio::time::sleep(Duration::from_millis(600)).await;
    tracing::info!(%version, "installing the launcher update and restarting");
    match update.install(bytes) {
        // Windows never gets here (the installer is started and this process exits).
        Ok(()) => state.app.restart(),
        Err(e) => {
            tracing::warn!(%version, error = %e, "the launcher update was not installed");
            let mut p = state.updates.persisted();
            p.installing = None;
            p.failed = Some((version.clone(), time::now()));
            state.updates.save(&p);
            let _ = std::fs::remove_file(&state.updates.pending_file);
            set(state, |s| {
                s.phase = "error";
                s.working_on = None;
                s.progress = None;
                s.error = Some(format!("Rustic Launcher {version}: {e}"));
            });
        }
    }
}

/// While an update waits for the games, looks every few seconds for them to be gone.
fn watch_games(state: Arc<AppState>) {
    if state.updates.watching.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let still = tauri::async_runtime::spawn_blocking({
                let st = state.clone();
                move || game_running(&st, None)
            })
            .await
            .unwrap_or(true);
            if !still {
                break;
            }
        }
        state.updates.watching.store(false, Ordering::SeqCst);
        tracing::info!("the games exited: applying the waiting update");
        run(&state, Trigger::GameExit, None).await;
    });
}

/// The background schedule: a check shortly after start, then every few hours (sooner after a
/// failure).
pub fn start(state: Arc<AppState>) {
    if let Some(id) = state.updates.peek_pending_launch() {
        set(&state, |s| s.resume_instance = Some(id));
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(4)).await;
        let mut trigger = Trigger::Startup;
        loop {
            run(&state, trigger, None).await;
            let failed = state.updates.status().error.is_some();
            tokio::time::sleep(if failed { RETRY } else { PERIOD }).await;
            trigger = Trigger::Periodic;
        }
    });
}

/// A game exited: check again (and apply what waited for it).
pub fn on_game_exit(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        if !game_running(&state, None) {
            run(&state, Trigger::GameExit, None).await;
        }
    });
}

/// Before a launch: the edition build the instance follows and the launcher, updated first when
/// automatic updates are on. Returns once the launch may go on; when the launcher replaces
/// itself, never (the launch resumes in the new launcher).
pub async fn before_play(state: &Arc<AppState>, play: PendingLaunch) {
    if !auto_enabled(state) {
        return;
    }
    run(state, Trigger::Play, Some(play)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_programs() {
        assert!(is_game_program("rustic-client.exe"));
        assert!(is_game_program("Horizon-Client.EXE"));
        assert!(is_game_program("rustic-client"));
        assert!(!is_game_program("rustic-server.exe"));
        assert!(!is_game_program("horizon-server"));
        assert!(!is_game_program("rustic-launcher.exe"));
    }

    #[test]
    fn a_finished_update_is_reported_once_and_a_failed_one_remembered() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(STATE_FILE);
        let write = |p: &Persisted| fsutil::write_json(&file, p).unwrap();
        write(&Persisted {
            installing: Some(("0.2.0".into(), "0.2.1".into(), 10)),
            ..Persisted::default()
        });
        let u = Updates::new(dir.path(), "0.2.1", true);
        assert_eq!(u.status().updated_from.as_deref(), Some("0.2.0"));
        assert!(u.persisted().installing.is_none() && u.persisted().failed.is_none());
        let u = Updates::new(dir.path(), "0.2.1", true);
        assert_eq!(u.status().updated_from, None, "reported once");

        write(&Persisted {
            installing: Some(("0.2.0".into(), "0.2.1".into(), 10)),
            ..Persisted::default()
        });
        let u = Updates::new(dir.path(), "0.2.0", true);
        assert_eq!(u.status().updated_from, None);
        assert_eq!(u.persisted().failed, Some(("0.2.1".into(), 10)));
    }

    #[test]
    fn a_pending_launch_is_taken_once_and_expires() {
        let dir = tempfile::tempdir().unwrap();
        let u = Updates::new(dir.path(), "0.2.0", true);
        let p = PendingLaunch {
            instance_id: "survival".into(),
            options: None,
            created_at: time::now(),
        };
        fsutil::write_json(&u.pending_file, &p).unwrap();
        assert_eq!(u.peek_pending_launch().as_deref(), Some("survival"));
        assert_eq!(u.take_pending_launch().unwrap().instance_id, "survival");
        assert!(u.take_pending_launch().is_none());
        fsutil::write_json(
            &u.pending_file,
            &PendingLaunch {
                created_at: time::now() - PENDING_LAUNCH_TTL - 1,
                ..p
            },
        )
        .unwrap();
        assert!(u.take_pending_launch().is_none(), "too old to resume");
    }
}
