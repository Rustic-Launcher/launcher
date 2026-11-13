//! Native mods compiled on the player's computer (ADR 0157, `rusticmc-native`): an instance's
//! status, the trust notice, Rust's installation, the build (a task in the downloads panel with a
//! `native-build-progress` event per step), the compile cache in Settings, and the client a
//! launch uses.

use crate::dto::TaskRef;
use crate::state::{AppState, Res, State};
use rusticmc_launcher_core::{IpcError, IpcErrorCode};
use rusticmc_native::instance::{self, State as BuildState};
use rusticmc_native::{
    BuildRequest, Builder, Consent, LocalSource, NativeError, NativeRoot, SourceSpec,
    ToolchainStatus, cache, toolchain,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;

/// Only the Rustic edition is built with native mods (its source is what the pipeline knows).
const EDITION_SOURCE: &str = "rustic-edition";

pub fn root(state: &AppState) -> NativeRoot {
    NativeRoot::new(state.paths.data())
}

fn cache_limit(state: &AppState) -> u64 {
    state.config.read().expect("config").native.cache_limit_gib
}

fn local_source(state: &AppState) -> Option<LocalSource> {
    state
        .config
        .read()
        .expect("config")
        .native
        .local_source()
        .map(|l| LocalSource {
            horizon: l.horizon,
            launcher: l.launcher,
            horizon_commit: l.horizon_commit,
        })
}

/// A pipeline error for the UI: its message, and `details.reason` = `native.<code>`.
pub fn native_err(e: NativeError) -> IpcError {
    let code = match &e {
        NativeError::ConsentRequired => IpcErrorCode::Unauthorized,
        NativeError::ToolchainMissing { .. } | NativeError::DiskSpace { .. } => {
            IpcErrorCode::Unavailable
        }
        NativeError::Busy => IpcErrorCode::Conflict,
        NativeError::Cancelled => IpcErrorCode::Cancelled,
        NativeError::Download(_) => IpcErrorCode::Network,
        NativeError::NoMods => IpcErrorCode::NotFound,
        _ => IpcErrorCode::Internal,
    };
    let mut err = IpcError::with_reason(code, e.to_string(), &format!("native.{}", e.code()));
    if let (NativeError::Compile { mods, .. }, Some(d)) = (&e, err.details.as_mut()) {
        d["mods"] = serde_json::json!(mods);
    }
    if let (NativeError::Mixins { mod_id, .. }, Some(d)) = (&e, err.details.as_mut()) {
        d["mods"] = serde_json::json!(mod_id.iter().collect::<Vec<_>>());
    }
    err
}

#[derive(Deserialize)]
pub struct NativeInstanceArgs {
    pub instance_id: String,
}

#[derive(Deserialize)]
pub struct NativeConsentArgs {
    pub instance_id: String,
    pub accept: bool,
}

#[derive(Deserialize)]
pub struct NativeCacheLimitArgs {
    pub cache_limit_gib: u64,
}

#[derive(Serialize, Clone)]
pub struct NativeModDto {
    pub id: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Serialize, Clone)]
pub struct NativeLeftOutDto {
    pub id: String,
    pub version: String,
    pub why: String,
    pub detail: String,
}

#[derive(Serialize, Clone)]
pub struct NativeBuildDto {
    pub built_at: u64,
    pub seconds: f64,
    pub compiled: u32,
    pub mods: Vec<NativeModDto>,
    pub left_out: Vec<NativeLeftOutDto>,
    pub client_bytes: u64,
    pub base_build: String,
    pub rust_version: String,
}

#[derive(Serialize, Clone)]
pub struct NativeToolchainDto {
    /// `ready` | `missing`.
    pub state: &'static str,
    /// `managed` | `rustup` | `system`, when ready.
    pub kind: Option<toolchain::Kind>,
    pub version: String,
    pub reason: Option<String>,
    pub installable: bool,
    pub prerequisites: Vec<String>,
}

impl From<ToolchainStatus> for NativeToolchainDto {
    fn from(s: ToolchainStatus) -> Self {
        match s {
            ToolchainStatus::Ready { toolchain } => Self {
                state: "ready",
                kind: Some(toolchain.kind),
                version: toolchain.version,
                reason: None,
                installable: false,
                prerequisites: vec![],
            },
            ToolchainStatus::Missing {
                reason,
                installable,
                prerequisites,
            } => Self {
                state: "missing",
                kind: None,
                version: toolchain::RUST_VERSION.into(),
                reason: Some(reason),
                installable,
                prerequisites,
            },
        }
    }
}

/// An instance's native mods, for its page and the install flow.
#[derive(Serialize, Clone)]
pub struct NativeStatusDto {
    pub instance_id: String,
    /// `not_needed` | `not_built` | `needs_rebuild` | `up_to_date`.
    pub state: BuildState,
    /// The instance plays a build that native mods can't go into (a plain Horizon build).
    pub supported: bool,
    pub mods: Vec<NativeModDto>,
    pub consent: bool,
    pub consent_at: Option<u64>,
    pub build: Option<NativeBuildDto>,
    /// The running build's task, if any.
    pub task_id: Option<String>,
    pub toolchain: NativeToolchainDto,
}

/// What a native build of `instance_id` needs, read from disk.
struct Target {
    instance_dir: PathBuf,
    game_dir: PathBuf,
    build: rusticmc_instances::InstalledBuild,
}

fn target(state: &AppState, instance_id: &str) -> Res<Target> {
    let instance = state.instances.store.get(instance_id)?;
    let build = state.instances.build_for(&instance)?;
    Ok(Target {
        instance_dir: state.instances.store.dir(instance_id),
        game_dir: state.instances.store.game_dir(instance_id),
        build,
    })
}

fn running_task(state: &AppState, instance_id: &str) -> Option<String> {
    state
        .tasks
        .list()
        .into_iter()
        .find(|t| {
            t.kind == "native-build"
                && t.instance_id.as_deref() == Some(instance_id)
                && matches!(t.state, "running" | "queued")
        })
        .map(|t| t.id)
}

pub async fn status_of(state: &AppState, instance_id: &str) -> Res<NativeStatusDto> {
    let t = target(state, instance_id)?;
    let s = instance::status(&t.instance_dir, &t.game_dir, &t.build.version).map_err(native_err)?;
    let toolchain = toolchain::detect(&root(state)).await.into();
    Ok(NativeStatusDto {
        instance_id: instance_id.to_owned(),
        state: s.state,
        supported: t.build.source == EDITION_SOURCE,
        mods: s
            .mods
            .iter()
            .map(|m| NativeModDto {
                id: m.id.clone(),
                version: m.version.clone(),
                enabled: m.enabled,
            })
            .collect(),
        consent: s.consent.is_some(),
        consent_at: s.consent.map(|c| c.accepted_at),
        build: s.record.map(|r| NativeBuildDto {
            built_at: r.built_at,
            seconds: r.seconds,
            compiled: r.compiled,
            mods: r
                .built
                .iter()
                .map(|m| NativeModDto {
                    id: m.id.clone(),
                    version: m.version.clone(),
                    enabled: true,
                })
                .collect(),
            left_out: r
                .left_out
                .iter()
                .map(|l| NativeLeftOutDto {
                    id: l.id.clone(),
                    version: l.version.clone(),
                    why: l.why.clone(),
                    detail: l.detail.clone(),
                })
                .collect(),
            client_bytes: r.client_bytes,
            base_build: r.base_build,
            rust_version: r.rust_version,
        }),
        task_id: running_task(state, instance_id),
        toolchain,
    })
}

#[tauri::command]
pub async fn native_status(state: State<'_>, args: NativeInstanceArgs) -> Res<NativeStatusDto> {
    status_of(&state, &args.instance_id).await
}

/// One instance's native mods at a glance, for the Profiles page's cards (ADR 0159).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct NativeOverviewDto {
    pub instance_id: String,
    /// `not_needed` | `not_built` | `needs_rebuild` | `up_to_date`.
    pub state: BuildState,
    /// Native mods installed in the instance.
    pub mods: u32,
    /// Of which turned on.
    pub enabled: u32,
    /// False when the instance plays a plain Horizon build.
    pub supported: bool,
    /// A build of this instance is running.
    pub building: bool,
}

/// The card line of an instance's native mods; `None` without any (most instances).
fn overview_of(
    instance_id: &str,
    s: &instance::Status,
    supported: bool,
    building: bool,
) -> Option<NativeOverviewDto> {
    if s.mods.is_empty() {
        return None;
    }
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    Some(NativeOverviewDto {
        instance_id: instance_id.to_owned(),
        state: s.state,
        mods: count(s.mods.len()),
        enabled: count(s.mods.iter().filter(|m| m.enabled).count()),
        supported,
        building,
    })
}

/// Every instance with native mods, read from disk only (no Rust detection, unlike
/// `native_status`), so the Profiles page lists them in one call. An instance whose build is not
/// installed or whose mods can't be read is left out: its own page says why.
#[tauri::command]
pub async fn native_overview(state: State<'_>) -> Res<Vec<NativeOverviewDto>> {
    let st = Arc::clone(&state);
    tokio::task::spawn_blocking(move || {
        st.instances
            .store
            .list()
            .iter()
            .filter_map(|inst| {
                let t = target(&st, &inst.id).ok()?;
                let s = instance::status(&t.instance_dir, &t.game_dir, &t.build.version).ok()?;
                overview_of(
                    &inst.id,
                    &s,
                    t.build.source == EDITION_SOURCE,
                    running_task(&st, &inst.id).is_some(),
                )
            })
            .collect()
    })
    .await
    .map_err(|e| IpcError::internal(e.to_string()))
}

/// The player accepts (or withdraws) the trust notice for an instance. Only the UI's notice
/// dialog calls this, on the player's click.
#[tauri::command]
pub async fn native_consent(state: State<'_>, args: NativeConsentArgs) -> Res<NativeStatusDto> {
    let dir = state.instances.store.dir(&args.instance_id);
    state.instances.store.get(&args.instance_id)?;
    if args.accept {
        Consent::accept(&dir).map_err(native_err)?;
        tracing::info!(instance = %args.instance_id, "native mods: trust notice accepted");
    } else {
        Consent::revoke(&dir).map_err(native_err)?;
    }
    status_of(&state, &args.instance_id).await
}

/// Installs the launcher's own Rust (after the player agreed in the UI) as a task.
#[tauri::command]
pub async fn native_toolchain_install(state: State<'_>) -> Res<TaskRef> {
    if let Some(t) = state
        .tasks
        .list()
        .into_iter()
        .find(|t| t.kind == "native-toolchain" && matches!(t.state, "running" | "queued"))
    {
        return Ok(TaskRef { task_id: t.id });
    }
    let (task, cancel) = state.tasks.start(
        "native-toolchain",
        format!("Rust {}", toolchain::RUST_VERSION),
        None,
        None,
    );
    let st = Arc::clone(&state);
    let id = task.clone();
    tokio::spawn(async move {
        let progress = |f: f64, msg: &str| {
            st.tasks.fraction(&id, f);
            st.tasks.detail(&id, msg);
        };
        match toolchain::install(&root(&st), &st.downloads, &cancel, &progress).await {
            Ok(t) => {
                tracing::info!(version = %t.version, "native mods: Rust installed");
                st.tasks.finish(&id);
            }
            Err(e) => {
                tracing::warn!("native mods: installing Rust failed: {e}");
                st.tasks.fail(
                    &id,
                    if matches!(e, NativeError::Cancelled) {
                        "cancelled".to_owned()
                    } else {
                        e.to_string()
                    },
                );
            }
        }
        let _ = st.app.emit("native-changed", serde_json::json!({}));
    });
    Ok(TaskRef { task_id: task })
}

/// The `native-build-progress` event.
#[derive(Serialize, Clone)]
pub struct NativeProgressEvent {
    pub instance_id: String,
    pub task_id: String,
    pub phase: rusticmc_native::Phase,
    pub fraction: f64,
    pub message: String,
    pub units: Option<(u32, u32)>,
}

/// Checks what a build needs and starts it as a task; its id. Refuses (with `native.<code>`)
/// without consent, without Rust, for a build that isn't the Rustic edition, while the instance
/// plays.
pub async fn start_build(state: &Arc<AppState>, instance_id: &str) -> Res<String> {
    if let Some(task) = running_task(state, instance_id) {
        return Ok(task);
    }
    if state.instances.running().iter().any(|r| r == instance_id) {
        return Err(IpcError::with_reason(
            IpcErrorCode::Conflict,
            "Close the game before rebuilding its native mods.",
            "native.running",
        ));
    }
    let t = target(state, instance_id)?;
    if t.build.source != EDITION_SOURCE {
        return Err(IpcError::with_reason(
            IpcErrorCode::Unavailable,
            "Native mods are built into the Rustic edition: switch this instance to it first.",
            "native.not_edition",
        ));
    }
    if Consent::load(&t.instance_dir).is_none() {
        return Err(native_err(NativeError::ConsentRequired));
    }
    let root = root(state);
    let rust = match toolchain::detect(&root).await {
        ToolchainStatus::Ready { toolchain } => toolchain,
        ToolchainStatus::Missing { reason, .. } => {
            return Err(native_err(NativeError::ToolchainMissing {
                version: toolchain::RUST_VERSION.into(),
                reason,
            }));
        }
    };
    let source =
        SourceSpec::for_build(&t.build.dir, local_source(state).as_ref()).map_err(native_err)?;
    let req = BuildRequest {
        instance_dir: t.instance_dir,
        game_dir: t.game_dir,
        base_build: t.build.version.clone(),
        base_build_dir: t.build.dir.clone(),
        source,
        toolchain: rust,
    };
    let mut builder = Builder::new(root, state.downloads.clone());
    builder.cache_limit_gib = cache_limit(state);
    let label = state
        .instances
        .store
        .get(instance_id)
        .map(|i| i.name)
        .unwrap_or_else(|_| instance_id.to_owned());
    let (task, cancel) = state.tasks.start(
        "native-build",
        format!("Native mods: {label}"),
        Some(instance_id.to_owned()),
        None,
    );
    let st = Arc::clone(state);
    let id = task.clone();
    let inst = instance_id.to_owned();
    tokio::spawn(async move {
        let on = |p: rusticmc_native::Progress| {
            st.tasks.fraction(&id, p.fraction);
            st.tasks.detail(&id, &p.message);
            let _ = st.app.emit(
                "native-build-progress",
                NativeProgressEvent {
                    instance_id: inst.clone(),
                    task_id: id.clone(),
                    phase: p.phase,
                    fraction: p.fraction,
                    message: p.message,
                    units: p.units,
                },
            );
        };
        match builder.build(&req, &on, &cancel).await {
            Ok(r) => {
                tracing::info!(
                    instance = %inst,
                    seconds = r.seconds,
                    compiled = r.compiled,
                    left_out = r.left_out.len(),
                    "native mods: built"
                );
                st.tasks.finish(&id);
            }
            Err(e) => {
                tracing::warn!(instance = %inst, "native mods: build failed: {e}");
                st.tasks.fail(
                    &id,
                    if matches!(e, NativeError::Cancelled) {
                        "cancelled".to_owned()
                    } else {
                        e.to_string()
                    },
                );
            }
        }
        let _ = st
            .app
            .emit("native-changed", serde_json::json!({ "instance_id": inst }));
    });
    Ok(task)
}

#[tauri::command]
pub async fn native_build(state: State<'_>, args: NativeInstanceArgs) -> Res<TaskRef> {
    let task = start_build(&state, &args.instance_id).await?;
    Ok(TaskRef { task_id: task })
}

/// The client a launch of the instance uses: `None` for the prebuilt one (no native mod turned
/// on), the instance's own when up to date; else it builds first (`on_progress` hears the
/// fraction) and errors with `native.<code>` when it can't.
pub async fn client_for_launch(
    state: &Arc<AppState>,
    instance_id: &str,
    on_progress: impl Fn(f64),
) -> Res<Option<PathBuf>> {
    let t = target(state, instance_id)?;
    let s = instance::status(&t.instance_dir, &t.game_dir, &t.build.version).map_err(native_err)?;
    match s.state {
        BuildState::NotNeeded => return Ok(None),
        BuildState::UpToDate => return Ok(s.client),
        BuildState::NotBuilt | BuildState::NeedsRebuild => {}
    }
    let task = start_build(state, instance_id).await?;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match state.tasks.list().into_iter().find(|x| x.id == task) {
            Some(x) if matches!(x.state, "running" | "queued") => {
                if let Some(total) = x.total_bytes.filter(|&t| t > 0) {
                    on_progress(x.received_bytes as f64 / total as f64);
                }
            }
            Some(x) if x.state == "done" => break,
            x => {
                let detail = x.and_then(|x| x.error).unwrap_or_default();
                return Err(IpcError::with_reason(
                    if detail == "cancelled" {
                        IpcErrorCode::Cancelled
                    } else {
                        IpcErrorCode::Internal
                    },
                    format!("The native mods could not be built. {detail}")
                        .trim_end()
                        .to_owned(),
                    "native.build_failed",
                ));
            }
        }
    }
    let s = instance::status(&t.instance_dir, &t.game_dir, &t.build.version).map_err(native_err)?;
    Ok(s.client)
}

/// Settings: the compile cache, the limit, Rust.
#[derive(Serialize, Clone)]
pub struct NativeCacheDto {
    pub usage: cache::Usage,
    pub limit_gib: u64,
    pub toolchain: NativeToolchainDto,
    pub building: bool,
}

#[tauri::command]
pub async fn native_cache_info(state: State<'_>) -> Res<NativeCacheDto> {
    let root = root(&state);
    let limit = cache_limit(&state);
    let r = root.clone();
    let usage = tokio::task::spawn_blocking(move || cache::usage(&r, limit))
        .await
        .map_err(|e| IpcError::internal(e.to_string()))?;
    Ok(NativeCacheDto {
        usage,
        limit_gib: limit,
        toolchain: toolchain::detect(&root).await.into(),
        building: rusticmc_native::pipeline::running(),
    })
}

/// Clears the compile cache (refused while a build runs).
#[tauri::command]
pub async fn native_cache_clear(state: State<'_>) -> Res<u64> {
    if rusticmc_native::pipeline::running() {
        return Err(native_err(NativeError::Busy));
    }
    let root = root(&state);
    tokio::task::spawn_blocking(move || cache::clear(&root))
        .await
        .map_err(|e| IpcError::internal(e.to_string()))?
        .map_err(native_err)
}

#[tauri::command]
pub async fn native_cache_set_limit(
    state: State<'_>,
    args: NativeCacheLimitArgs,
) -> Res<NativeCacheDto> {
    if args.cache_limit_gib > 4096 {
        return Err(IpcError::invalid("The cache limit is at most 4096 GiB."));
    }
    {
        let mut config = state.config.write().expect("config");
        config.native.cache_limit_gib = args.cache_limit_gib;
        config.save(&state.paths.config_file())?;
    }
    native_cache_info(state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_errors_carry_their_reason_and_mods() {
        let e = native_err(NativeError::Compile {
            mods: vec!["zoom".into()],
            excerpt: "error[E0308]".into(),
        });
        let d = e.details.unwrap();
        assert_eq!(d["reason"], "native.compile");
        assert_eq!(d["mods"][0], "zoom");
        let e = native_err(NativeError::ConsentRequired);
        assert_eq!(e.code, IpcErrorCode::Unauthorized);
        assert_eq!(e.details.unwrap()["reason"], "native.consent_required");
        let e = native_err(NativeError::ToolchainMissing {
            version: "1.99.1".into(),
            reason: "Rust is not installed".into(),
        });
        assert_eq!(e.details.unwrap()["reason"], "native.toolchain_missing");
    }

    fn status(state: BuildState, mods: &[(&str, bool)]) -> instance::Status {
        instance::Status {
            state,
            mods: mods
                .iter()
                .map(|(id, enabled)| rusticmc_native::instance::NativeModFile {
                    id: (*id).into(),
                    version: "1.0.0".into(),
                    sha256: String::new(),
                    file: PathBuf::from(format!("{id}.hnmod")),
                    hooks: None,
                    enabled: *enabled,
                })
                .collect(),
            consent: None,
            record: None,
            client: None,
            key: None,
        }
    }

    #[test]
    fn the_overview_counts_native_mods_and_skips_instances_without() {
        assert_eq!(
            overview_of("a", &status(BuildState::NotNeeded, &[]), true, false),
            None
        );
        let o = overview_of(
            "b",
            &status(
                BuildState::NeedsRebuild,
                &[("zoom", true), ("minimap", false)],
            ),
            true,
            true,
        )
        .unwrap();
        assert_eq!(o.instance_id, "b");
        assert_eq!((o.mods, o.enabled), (2, 1));
        assert!(o.supported && o.building);
        let json = serde_json::to_value(&o).unwrap();
        assert_eq!(json["state"], "needs_rebuild");
        // Every mod off: still listed (the card says the prebuilt game plays).
        let off = overview_of(
            "c",
            &status(BuildState::NotNeeded, &[("zoom", false)]),
            false,
            false,
        )
        .unwrap();
        assert_eq!((off.mods, off.enabled, off.supported), (1, 0, false));
    }

    #[test]
    fn a_missing_toolchain_reads_for_the_ui() {
        let dto: NativeToolchainDto = ToolchainStatus::Missing {
            reason: "Rust is not installed".into(),
            installable: true,
            prerequisites: vec![],
        }
        .into();
        assert_eq!(dto.state, "missing");
        assert!(dto.installable);
        assert_eq!(dto.version, toolchain::RUST_VERSION);
    }
}
