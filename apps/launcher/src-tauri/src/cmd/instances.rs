//! Horizon versions, instances, their worlds and content folders, and launching.

use crate::dto::*;
use crate::state::{AppState, Res, State};
use crate::util::{build_label, default_build, files_in, iso, mtime, not_available_yet};
use rusticmc_auth::{ActiveIdentity, LaunchAuth};
use rusticmc_instances::{
    BuildChoice, BuildRelease, ExitOutcome, GameEvent, InstancePatch, InstanceSettings,
    LaunchIdentity, QuickPlay, WindowOptions,
};
use rusticmc_launcher_core::{IpcError, IpcErrorCode, time};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Emitter;
use tauri_plugin_opener::OpenerExt;

const MODS: &str = "horizon-mods";
const PACKS: &str = "resourcepacks";
const SHADERS: &str = "shaderpacks";
const DISABLED: &str = ".disabled";

fn count_files(dir: &Path) -> usize {
    files_in(dir).len()
}

fn worlds_with_datapacks(game: &Path) -> usize {
    files_in(&game.join("saves"))
        .iter()
        .filter(|w| count_files(&w.join("datapacks")) > 0)
        .count()
}

/// The UI's view of an instance.
pub fn instance_dto(state: &AppState, i: &rusticmc_instances::Instance) -> Instance {
    let game = state.instances.store.game_dir(&i.id);
    let build = state.instances.build_for(i).ok();
    let version = build
        .as_ref()
        .map(|b| b.version.clone())
        .or_else(|| i.build.version.clone())
        .unwrap_or_default();
    let running = state.instances.running().contains(&i.id);
    let newest = state
        .instances
        .builds
        .installed()
        .into_iter()
        .find(|b| b.source == i.build.source)
        .map(|b| b.version);
    Instance {
        id: i.id.clone(),
        name: i.name.clone(),
        icon: i.icon.clone(),
        horizon_version_label: build_label(&version, &i.build.source),
        horizon_version: version.clone(),
        state: if running {
            "running"
        } else if build.is_none() {
            "needs_update"
        } else {
            "ready"
        },
        update_available: newest.filter(|n| i.build.version.is_some() && *n != version),
        mod_count: count_files(&game.join(MODS)),
        resource_pack_count: count_files(&game.join(PACKS)),
        shader_count: count_files(&game.join(SHADERS)),
        datapack_world_count: worlds_with_datapacks(&game),
        playtime_s: i.playtime_seconds,
        last_played_at: i.last_played_at.map(iso),
        last_server: i.last_server.as_ref().map(|s| LastServer {
            name: s.name.clone(),
            address: s.address.clone(),
        }),
        created_at: iso(i.created_at),
        memory_mb: i.settings.memory_mb,
        game_args: i.game_args.clone(),
        resolution: match (i.settings.window.width, i.settings.window.height) {
            (Some(width), Some(height)) => Some(Resolution {
                width,
                height,
                fullscreen: i.settings.window.fullscreen,
            }),
            _ => None,
        },
        folder: game.display().to_string(),
        source: i.source.as_ref().map(|s| InstanceSourceDto {
            kind: s.kind.clone(),
            project_id: s.project_id.clone(),
            name: s.name.clone(),
            version: s.version.clone(),
        }),
        pinned: i.pinned,
    }
}

fn all(state: &AppState) -> Vec<Instance> {
    let mut list: Vec<Instance> = state
        .instances
        .store
        .list()
        .iter()
        .map(|i| instance_dto(state, i))
        .collect();
    list.sort_by_key(|i| !i.pinned);
    list
}

pub fn instances_changed(state: &AppState) {
    let _ = state.app.emit("instances-changed", all(state));
}

#[tauri::command]
pub async fn horizon_versions_list(state: State<'_>) -> Res<Vec<HorizonVersion>> {
    let installed = state.instances.builds.installed();
    let default_source = state.config().builds.default_source;
    let mut out: Vec<HorizonVersion> = Vec::new();
    let mut errors = Vec::new();
    for source in state.sources() {
        match state.instances.builds.releases(&source).await {
            Ok(releases) => {
                for (n, r) in releases.into_iter().enumerate() {
                    out.push(version_dto(
                        &r,
                        &installed,
                        n == 0 && source.config.id == default_source,
                    ));
                }
            }
            Err(e) => errors.push(e.to_string()),
        }
    }
    // Installed builds a source no longer lists (a Nightly older than seven days) stay.
    for b in &installed {
        if !out.iter().any(|v| v.id == b.version) {
            out.push(HorizonVersion {
                id: b.version.clone(),
                label: build_label(&b.version, &b.source),
                channel: if b.version.starts_with("nightly") {
                    "nightly"
                } else {
                    "release"
                },
                rustic_edition: b.source == "rustic-edition",
                released_at: iso(b.installed_at),
                installed: true,
                recommended: false,
                size_bytes: rusticmc_launcher_core::fsutil::dir_size(&b.dir),
            });
        }
    }
    // The default source publishes nothing (no channel yet): recommend the newest build found.
    if !out.iter().any(|v| v.recommended)
        && let Some(first) = out.iter_mut().find(|v| !v.id.is_empty())
    {
        first.recommended = true;
    }
    if out.is_empty() && !errors.is_empty() {
        return Err(IpcError::new(IpcErrorCode::Network, errors.join("; ")));
    }
    Ok(out)
}

fn version_dto(
    r: &BuildRelease,
    installed: &[rusticmc_instances::InstalledBuild],
    recommended: bool,
) -> HorizonVersion {
    HorizonVersion {
        id: r.version.clone(),
        label: build_label(&r.version, &r.source),
        channel: if r.version.starts_with("nightly") {
            "nightly"
        } else {
            "release"
        },
        rustic_edition: r.source == "rustic-edition",
        released_at: r.published_at.clone(),
        installed: installed.iter().any(|b| b.version == r.version),
        recommended,
        size_bytes: r.asset.as_ref().and_then(|a| a.size).unwrap_or(0),
    }
}

#[tauri::command]
pub fn instances_list(state: State<'_>) -> Vec<Instance> {
    all(&state)
}

#[tauri::command]
pub fn instance_get(state: State<'_>, args: IdArgs) -> Res<Instance> {
    let i = state.instances.store.get(&args.id)?;
    Ok(instance_dto(&state, &i))
}

/// The source of an installed build, or the default source for one not installed yet.
fn source_of(state: &AppState, version: &str) -> String {
    state
        .instances
        .builds
        .get(version)
        .map(|b| b.source)
        .unwrap_or_else(|_| state.config().builds.default_source)
}

#[tauri::command]
pub fn instance_create(state: State<'_>, args: CreateInstanceRequest) -> Res<Instance> {
    let source = source_of(&state, &args.horizon_version);
    let i = state.instances.store.create(
        &args.name,
        BuildChoice {
            source,
            version: Some(args.horizon_version.clone()).filter(|v| !v.is_empty()),
        },
    )?;
    let i = state.instances.store.update(
        &i.id,
        InstancePatch {
            icon: Some(args.icon),
            settings: Some(InstanceSettings {
                memory_mb: args.memory_mb,
                ..InstanceSettings::default()
            }),
            ..InstancePatch::default()
        },
    )?;
    // Install the build in the background when it isn't yet.
    if state.instances.builds.get(&args.horizon_version).is_err()
        && !args.horizon_version.is_empty()
    {
        let _ = start_build_install(
            state.inner().clone(),
            args.horizon_version,
            Some(i.id.clone()),
        );
    }
    instances_changed(&state);
    Ok(instance_dto(&state, &i))
}

#[tauri::command]
pub async fn instance_import(
    state: State<'_>,
    args: ImportInstanceRequest,
) -> Res<serde_json::Value> {
    match args {
        ImportInstanceRequest::Folder { path, name } => {
            let src = PathBuf::from(&path);
            if !src.is_dir() {
                return Err(IpcError::invalid("not a folder"));
            }
            let name = name.unwrap_or_else(|| {
                src.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Imported".into())
            });
            let build = default_build(&state);
            let i = state.instances.store.create(
                &name,
                BuildChoice {
                    source: build
                        .as_ref()
                        .map(|b| b.source.clone())
                        .unwrap_or_else(|| state.config().builds.default_source),
                    version: build.map(|b| b.version),
                },
            )?;
            let (task, _) = state.tasks.start(
                "modpack",
                format!("Import {name}"),
                Some(i.id.clone()),
                None,
            );
            let dest = state.instances.store.game_dir(&i.id);
            let st = state.inner().clone();
            let task2 = task.clone();
            tauri::async_runtime::spawn_blocking(move || {
                match rusticmc_launcher_core::fsutil::copy_dir(&src, &dest) {
                    Ok(()) => st.tasks.finish(&task2),
                    Err(e) => st.tasks.fail(&task2, e.to_string()),
                }
                instances_changed(&st);
            });
            Ok(serde_json::json!({ "instance": instance_dto(&state, &i), "task_id": task }))
        }
        ImportInstanceRequest::Mrpack { path, name } => {
            let src = PathBuf::from(&path);
            let index = rusticmc_content::mrpack::read_index(&src)
                .map_err(|e| IpcError::invalid(e.to_string()))?;
            let name = name.unwrap_or_else(|| index.name.clone());
            let build = default_build(&state);
            let i = state.instances.store.create(
                &name,
                BuildChoice {
                    source: build
                        .as_ref()
                        .map(|b| b.source.clone())
                        .unwrap_or_else(|| state.config().builds.default_source),
                    version: build.map(|b| b.version),
                },
            )?;
            let _ = state.instances.store.set_source(
                &i.id,
                Some(rusticmc_instances::InstanceSource {
                    kind: "modpack".into(),
                    project_id: String::new(),
                    name: index.name.clone(),
                    version: index.version_id.clone(),
                }),
            );
            let (task, _) = state.tasks.start(
                "modpack",
                format!("Import {name}"),
                Some(i.id.clone()),
                None,
            );
            let st = state.inner().clone();
            let task2 = task.clone();
            let id = i.id.clone();
            tauri::async_runtime::spawn(async move {
                match crate::cmd::content::import_mrpack(&st, src, &id).await {
                    Ok(_) => st.tasks.finish(&task2),
                    Err(e) => st.tasks.fail(&task2, e.message),
                }
                instances_changed(&st);
            });
            Ok(serde_json::json!({ "instance": instance_dto(&state, &i), "task_id": task }))
        }
        // CurseForge zips and catalogue modpacks: the content crate has no importer for them yet.
        ImportInstanceRequest::CurseforgeZip { .. } => {
            Err(not_available_yet("CurseForge modpack import"))
        }
        ImportInstanceRequest::Modpack {
            project_id,
            version_id,
            name,
        } => {
            let build = default_build(&state);
            let label = name.unwrap_or_else(|| project_id.clone());
            let i = state.instances.store.create(
                &label,
                BuildChoice {
                    source: build
                        .as_ref()
                        .map(|b| b.source.clone())
                        .unwrap_or_else(|| state.config().builds.default_source),
                    version: build.map(|b| b.version),
                },
            )?;
            let _ = state.instances.store.set_source(
                &i.id,
                Some(rusticmc_instances::InstanceSource {
                    kind: "modpack".into(),
                    project_id: project_id.clone(),
                    name: label.clone(),
                    version: version_id.clone().unwrap_or_default(),
                }),
            );
            let (task, _) = state.tasks.start(
                "modpack",
                format!("Import {label}"),
                Some(i.id.clone()),
                None,
            );
            let st = state.inner().clone();
            let task2 = task.clone();
            let id = i.id.clone();
            tauri::async_runtime::spawn(async move {
                let target = crate::cmd::content::InstallTarget::Instance {
                    instance_id: id.clone(),
                };
                match crate::cmd::content::install_project(
                    &st,
                    &project_id,
                    version_id.as_deref(),
                    &target,
                    true,
                )
                .await
                {
                    Ok(_) => st.tasks.finish(&task2),
                    Err(e) => st.tasks.fail(&task2, e.message),
                }
                instances_changed(&st);
            });
            Ok(serde_json::json!({ "instance": instance_dto(&state, &i), "task_id": task }))
        }
    }
}

#[tauri::command]
pub fn instance_update(state: State<'_>, args: InstanceUpdateArgs) -> Res<Instance> {
    let current = state.instances.store.get(&args.id)?;
    let mut settings = current.settings.clone();
    if let Some(m) = args.patch.memory_mb {
        settings.memory_mb = m;
    }
    if let Some(r) = args.patch.resolution {
        settings.window = match r {
            Some(r) => WindowOptions {
                width: Some(r.width),
                height: Some(r.height),
                fullscreen: r.fullscreen,
            },
            None => WindowOptions::default(),
        };
    }
    let i = state.instances.store.update(
        &args.id,
        InstancePatch {
            name: args.patch.name,
            icon: args.patch.icon,
            pinned: args.patch.pinned,
            game_args: args.patch.game_args,
            settings: Some(settings),
            ..InstancePatch::default()
        },
    )?;
    instances_changed(&state);
    Ok(instance_dto(&state, &i))
}

#[tauri::command]
pub fn instance_delete(state: State<'_>, args: IdArgs) -> Res<()> {
    if state.instances.running().contains(&args.id) {
        return Err(IpcError::conflict("Stop the game first."));
    }
    state.instances.store.delete(&args.id)?;
    instances_changed(&state);
    Ok(())
}

#[tauri::command]
pub fn instance_duplicate(state: State<'_>, args: IdArgs) -> Res<Instance> {
    let src = state.instances.store.get(&args.id)?;
    let copy = state
        .instances
        .store
        .duplicate(&args.id, &format!("{} (copy)", src.name))?;
    instances_changed(&state);
    Ok(instance_dto(&state, &copy))
}

/// Installs a build in the background as a task.
pub fn start_build_install(
    state: Arc<AppState>,
    version: String,
    instance: Option<String>,
) -> String {
    let (task, cancel) = state
        .tasks
        .start("game", format!("Horizon {version}"), instance, None);
    let task_id = task.clone();
    tauri::async_runtime::spawn(async move {
        let result = async {
            for source in state.sources() {
                let releases = match state.instances.builds.releases(&source).await {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if let Some(r) = releases.into_iter().find(|r| r.version == version) {
                    let tasks = state.tasks.clone();
                    let t = task_id.clone();
                    return tokio::select! {
                        _ = cancel.cancelled() => Err(IpcError::new(IpcErrorCode::Cancelled, "cancelled")),
                        r = state.instances.builds.install(&source, &r, move |stage| {
                            let f = match stage {
                                rusticmc_instances::InstallStage::Verifying => 0.05,
                                rusticmc_instances::InstallStage::Downloading => 0.1,
                                rusticmc_instances::InstallStage::Unpacking => 0.9,
                                rusticmc_instances::InstallStage::Done { .. } => 1.0,
                            };
                            tasks.fraction(&t, f);
                        }) => r.map(|_| ()).map_err(IpcError::from),
                    };
                }
            }
            Err(IpcError::not_found(format!("no source publishes {version}")))
        }
        .await;
        match result {
            Ok(()) => state.tasks.finish(&task_id),
            Err(e) => state.tasks.fail(&task_id, e.message),
        }
        instances_changed(&state);
    });
    task
}

#[tauri::command]
pub fn instance_set_version(state: State<'_>, args: InstanceSetVersionArgs) -> Res<TaskRef> {
    let source = source_of(&state, &args.horizon_version);
    state.instances.store.update(
        &args.id,
        InstancePatch {
            build: Some(BuildChoice {
                source,
                version: Some(args.horizon_version.clone()),
            }),
            ..InstancePatch::default()
        },
    )?;
    let task_id = if state.instances.builds.get(&args.horizon_version).is_ok() {
        let (t, _) = state.tasks.start(
            "game",
            format!("Horizon {}", args.horizon_version),
            Some(args.id.clone()),
            None,
        );
        state.tasks.finish(&t);
        t
    } else {
        start_build_install(state.inner().clone(), args.horizon_version, Some(args.id))
    };
    instances_changed(&state);
    Ok(TaskRef { task_id })
}

#[tauri::command]
pub fn instance_open_folder(state: State<'_>, args: InstanceFolderArgs) -> Res<()> {
    state.instances.store.get(&args.id)?;
    let game = state.instances.store.game_dir(&args.id);
    let sub = match args.folder.as_str() {
        "root" => "",
        "mods" => MODS,
        "resourcepacks" => PACKS,
        "shaderpacks" => SHADERS,
        "saves" => "saves",
        "screenshots" => "screenshots",
        "logs" => "logs",
        _ => return Err(IpcError::invalid("unknown folder")),
    };
    let dir = game.join(sub);
    std::fs::create_dir_all(&dir).map_err(|e| IpcError::internal(e.to_string()))?;
    state
        .app
        .opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| IpcError::internal(e.to_string()))
}

#[tauri::command]
pub fn instance_worlds_list(state: State<'_>, args: InstanceIdArgs) -> Res<Vec<World>> {
    state.instances.store.get(&args.instance_id)?;
    let saves = state
        .instances
        .store
        .game_dir(&args.instance_id)
        .join("saves");
    Ok(files_in(&saves)
        .into_iter()
        .filter(|w| w.join("level.dat").is_file())
        .map(|w| {
            let id = w
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            World {
                name: id.clone(),
                id,
                // level.dat is NBT; the mode isn't read yet (RISKS.md).
                mode: "survival",
                last_played_at: mtime(&w.join("level.dat")).map(iso),
                size_bytes: rusticmc_launcher_core::fsutil::dir_size(&w),
                datapack_count: count_files(&w.join("datapacks")),
            }
        })
        .collect())
}

fn content_dirs(game: &Path, kind: Option<&str>) -> Vec<(&'static str, PathBuf, Option<String>)> {
    let mut out = Vec::new();
    let want = |k: &str| kind.is_none_or(|w| w == k);
    if want("mod") {
        out.push(("mod", game.join(MODS), None));
    }
    if want("resourcepack") {
        out.push(("resourcepack", game.join(PACKS), None));
    }
    if want("shader") {
        out.push(("shader", game.join(SHADERS), None));
    }
    if want("datapack") {
        for w in files_in(&game.join("saves")) {
            let world = w.file_name().map(|n| n.to_string_lossy().into_owned());
            out.push(("datapack", w.join("datapacks"), world));
        }
    }
    out
}

/// Content found in the instance's folders (enabled ones, and disabled ones under `.disabled/`).
fn list_content(game: &Path, kind: Option<&str>) -> Vec<InstalledContent> {
    let mut out = Vec::new();
    for (k, dir, world) in content_dirs(game, kind) {
        for (enabled, folder) in [(true, dir.clone()), (false, dir.join(DISABLED))] {
            for (n, f) in files_in(&folder).into_iter().enumerate() {
                let filename = f
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let rel = f
                    .strip_prefix(game)
                    .unwrap_or(&f)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(InstalledContent {
                    id: rel,
                    project_id: None,
                    kind: k.into(),
                    name: filename
                        .rsplit_once('.')
                        .map(|(a, _)| a.to_owned())
                        .unwrap_or_else(|| filename.clone()),
                    version: String::new(),
                    filename,
                    enabled,
                    source: "local".into(),
                    icon_url: None,
                    update_available: None,
                    compatible: true,
                    incompatibility: None,
                    order: (k == "resourcepack").then_some(n),
                    world_id: world.clone(),
                });
            }
        }
    }
    out
}

/// Content found in the folders, with what Rustic's ledger knows of the files it installed
/// (source, project, version, and the switch state Horizon reads).
fn list_with_ledger(game: &Path, kind: Option<&str>) -> Vec<InstalledContent> {
    let ledger = rusticmc_content::ledger::Ledger::load(game).unwrap_or_default();
    let mut out = list_content(game, kind);
    // Files Rustic put in new-world-datapacks/ are listed too (they are no world's yet).
    if kind.is_none_or(|k| k == "datapack") {
        for e in ledger.entries.iter().filter(|e| e.target == "new_worlds") {
            for f in &e.files {
                out.push(InstalledContent {
                    id: f.path.clone(),
                    project_id: None,
                    kind: "datapack".into(),
                    name: e.name.clone(),
                    version: e.version.clone(),
                    filename: f.path.rsplit('/').next().unwrap_or("").to_owned(),
                    enabled: e.enabled,
                    source: "local".into(),
                    icon_url: None,
                    update_available: None,
                    compatible: true,
                    incompatibility: None,
                    order: None,
                    world_id: None,
                });
            }
        }
    }
    for c in &mut out {
        if let Some(e) = ledger
            .entries
            .iter()
            .find(|e| e.files.iter().any(|f| f.path == c.id))
        {
            c.project_id = Some(match e.source {
                rusticmc_content::SourceKind::Horizon => format!("horizon:{}", e.project_id),
                rusticmc_content::SourceKind::Rustic => format!("rustic:{}", e.project_id),
                _ => e.project_id.clone(),
            });
            c.name = e.name.clone();
            c.version = e.version.clone();
            c.enabled = e.enabled;
            c.source = match e.source {
                rusticmc_content::SourceKind::Horizon => "horizon",
                rusticmc_content::SourceKind::Modrinth => "modrinth",
                rusticmc_content::SourceKind::Rustic => "rustic",
                rusticmc_content::SourceKind::Local => "local",
            }
            .into();
        }
    }
    out
}

#[tauri::command]
pub fn instance_content_list(
    state: State<'_>,
    args: InstanceContentListArgs,
) -> Res<Vec<InstalledContent>> {
    state.instances.store.get(&args.instance_id)?;
    let game = state.instances.store.game_dir(&args.instance_id);
    Ok(list_with_ledger(&game, args.kind.as_deref()))
}

/// The content's file under the game folder, refusing paths that leave it.
fn content_path(game: &Path, id: &str) -> Res<PathBuf> {
    if id.split(['/', '\\']).any(|c| c == ".." || c.is_empty()) {
        return Err(IpcError::invalid("bad content id"));
    }
    let p = game.join(id);
    if !p.exists() {
        return Err(IpcError::not_found("no such content"));
    }
    Ok(p)
}

#[tauri::command]
pub fn instance_content_set_enabled(
    state: State<'_>,
    args: InstanceContentToggleArgs,
) -> Res<InstalledContent> {
    let game = state.instances.store.game_dir(&args.instance_id);
    let path = content_path(&game, &args.content_id)?;
    if let Some(e) = crate::cmd::content::ledger_entry(&game, &args.content_id) {
        crate::cmd::content::set_entry_enabled(&state, &game, &e, args.enabled)?;
        return list_with_ledger(&game, None)
            .into_iter()
            .find(|c| c.id == args.content_id)
            .ok_or_else(|| IpcError::internal("content switched but not listed"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| IpcError::invalid("bad content id"))?;
    let is_disabled = parent.file_name().is_some_and(|n| n == DISABLED);
    let name = path
        .file_name()
        .ok_or_else(|| IpcError::invalid("bad content id"))?;
    let target = match (args.enabled, is_disabled) {
        (true, true) => parent.parent().unwrap_or(parent).join(name),
        (false, false) => parent.join(DISABLED).join(name),
        _ => path.clone(),
    };
    if target != path {
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| IpcError::internal(e.to_string()))?;
        }
        std::fs::rename(&path, &target).map_err(|e| IpcError::internal(e.to_string()))?;
    }
    let rel = target
        .strip_prefix(&game)
        .unwrap_or(&target)
        .to_string_lossy()
        .replace('\\', "/");
    list_with_ledger(&game, None)
        .into_iter()
        .find(|c| c.id == rel)
        .ok_or_else(|| IpcError::internal("content moved but not listed"))
}

#[tauri::command]
pub fn instance_content_remove(state: State<'_>, args: InstanceContentArgs) -> Res<()> {
    let game = state.instances.store.game_dir(&args.instance_id);
    let id = args
        .content_id
        .ok_or_else(|| IpcError::invalid("content_id is required"))?;
    let path = content_path(&game, &id)?;
    if let Some(e) = crate::cmd::content::ledger_entry(&game, &id) {
        crate::cmd::content::uninstall_entry(&state, &game, &e)?;
        instances_changed(&state);
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(&path)
    } else {
        std::fs::remove_file(&path)
    }
    .map_err(|e| IpcError::internal(e.to_string()))?;
    instances_changed(&state);
    Ok(())
}

#[tauri::command]
pub async fn instance_content_update(state: State<'_>, args: InstanceContentArgs) -> Res<TaskRef> {
    state.instances.store.get(&args.instance_id)?;
    let task_id =
        crate::cmd::content::update_instance(&state, &args.instance_id, args.content_id.clone())
            .await?;
    Ok(TaskRef { task_id })
}

/// Resource pack order: Horizon (like vanilla) keeps it in `options.txt` `resourcePacks`; the
/// launcher records the order the player chose in `resourcepacks/.order` and lists in it.
#[tauri::command]
pub fn instance_resource_packs_reorder(
    state: State<'_>,
    args: ReorderArgs,
) -> Res<Vec<InstalledContent>> {
    let game = state.instances.store.game_dir(&args.instance_id);
    let mut packs = list_content(&game, Some("resourcepack"));
    for p in &mut packs {
        p.order = args
            .ordered_ids
            .iter()
            .position(|id| *id == p.id)
            .or(p.order);
    }
    packs.sort_by_key(|p| p.order.unwrap_or(usize::MAX));
    let order: Vec<&str> = packs.iter().map(|p| p.filename.as_str()).collect();
    std::fs::create_dir_all(game.join(PACKS)).map_err(|e| IpcError::internal(e.to_string()))?;
    std::fs::write(game.join(PACKS).join(".order"), order.join("\n"))
        .map_err(|e| IpcError::internal(e.to_string()))?;
    Ok(packs)
}

// ---- launch ---------------------------------------------------------------------------------

pub(crate) fn set_launch(state: &AppState, status: LaunchStatus) {
    state
        .launches
        .lock()
        .expect("launches")
        .insert(status.instance_id.clone(), status.clone());
    let _ = state.app.emit("launch-state", status);
}

pub(crate) fn phase(id: &str, phase: &'static str, message: Option<String>) -> LaunchStatus {
    LaunchStatus {
        instance_id: id.to_owned(),
        phase,
        progress: None,
        message,
        reason: None,
        started_at: None,
    }
}

#[tauri::command]
pub async fn instance_launch(state: State<'_>, args: InstanceLaunchArgs) -> Res<LaunchStatus> {
    let id = args.instance_id.clone();
    state.instances.store.get(&id)?;
    if state.instances.running().contains(&id) {
        return Err(IpcError::with_reason(
            IpcErrorCode::Conflict,
            "This instance is already running.",
            "launch.already_running",
        ));
    }
    // Updates first (ADR 0162): the edition build this instance follows, then the launcher,
    // which restarts and resumes this launch. Never blocks the launch when they fail.
    set_launch(&state, phase(&id, "updating", None));
    crate::update::before_play(
        state.inner(),
        PendingLaunch {
            instance_id: id.clone(),
            options: args
                .options
                .as_ref()
                .and_then(|o| serde_json::to_value(o).ok()),
            created_at: time::now(),
        },
    )
    .await;
    // The edition update may have moved the instance to a new build.
    let instance = state.instances.store.get(&id)?;
    set_launch(&state, phase(&id, "checking", None));
    let result = launch_inner(&state, &instance, args.options.unwrap_or_default()).await;
    match result {
        Ok(()) => {
            let status = LaunchStatus {
                started_at: Some(iso(time::now())),
                ..phase(&id, "running", None)
            };
            set_launch(&state, status.clone());
            instances_changed(&state);
            Ok(status)
        }
        Err(e) => {
            let reason = e
                .details
                .as_ref()
                .and_then(|d| d.get("reason"))
                .and_then(|r| r.as_str())
                .map(str::to_owned);
            set_launch(
                &state,
                LaunchStatus {
                    reason,
                    ..phase(&id, "idle", Some(e.message.clone()))
                },
            );
            Err(e)
        }
    }
}

async fn launch_inner(
    state: &Arc<AppState>,
    instance: &rusticmc_instances::Instance,
    options: LaunchOptions,
) -> Res<()> {
    let id = &instance.id;
    // The owner gate first.
    let identity = match state.accounts.active()? {
        ActiveIdentity::Microsoft(uuid) => {
            match state.accounts.launch_session(Some(&uuid)).await? {
                LaunchAuth::Online(s) => LaunchIdentity::Online(s),
                // No network: still the owner's official name and UUID; the game goes offline.
                LaunchAuth::Offline { session, .. } => LaunchIdentity::Online(session),
            }
        }
        ActiveIdentity::None => {
            return Err(IpcError::with_reason(
                IpcErrorCode::OwnershipRequired,
                "Sign in with a Microsoft account that owns Minecraft: Java Edition to play.",
                "auth.no_account",
            ));
        }
    };
    // The build.
    if state.instances.build_for(instance).is_err() {
        let version =
            instance.build.version.clone().ok_or_else(|| {
                IpcError::not_found("No Horizon build installed for this instance.")
            })?;
        set_launch(state, phase(id, "downloading", None));
        let task = start_build_install(state.clone(), version.clone(), Some(id.clone()));
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let t = state.tasks.list().into_iter().find(|t| t.id == task);
            match t.map(|t| t.state) {
                Some("done") => break,
                Some("running" | "queued") => continue,
                _ => {
                    return Err(IpcError::new(
                        IpcErrorCode::Network,
                        format!("Horizon {version} could not be installed."),
                    ));
                }
            }
        }
    }
    // The game files: prepared once per build from the player's own install (first launch, or a
    // new build), with the same checks and owner gate as Settings > Game files.
    let build = state.instances.build_for(instance)?;
    let prepared = state
        .instances
        .check_prepared(&build.version)
        .await
        .is_ok_and(|c| c.current);
    if !prepared {
        set_launch(state, phase(id, "preparing", None));
        let job = crate::cmd::app::prepare_job(state, build, None).await?;
        let task = crate::cmd::app::start_prepare(state.clone(), job);
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            match state.tasks.list().into_iter().find(|t| t.id == task) {
                Some(t) if t.state == "done" => break,
                Some(t) if t.state == "running" || t.state == "queued" => {
                    let progress = t
                        .total_bytes
                        .filter(|&total| total > 0)
                        .map(|total| t.received_bytes as f64 / total as f64);
                    set_launch(
                        state,
                        LaunchStatus {
                            progress,
                            ..phase(id, "preparing", None)
                        },
                    );
                }
                t => {
                    let detail = t.and_then(|t| t.error).unwrap_or_default();
                    return Err(IpcError::with_reason(
                        IpcErrorCode::Unavailable,
                        format!("The game files could not be prepared. {detail}")
                            .trim_end()
                            .to_owned(),
                        "prepare.failed",
                    ));
                }
            }
        }
    }
    // Native mods: the instance's own client, compiled first when its mods changed (ADR 0157);
    // none turned on, the build's prebuilt client.
    let client = crate::cmd::native::client_for_launch(state, id, |f| {
        set_launch(
            state,
            LaunchStatus {
                progress: Some(f),
                ..phase(id, "building", None)
            },
        );
    })
    .await?;
    set_launch(state, phase(id, "launching", None));
    let quick_play = match (options.join, options.world_id) {
        (Some(join), _) => {
            let _ = state.instances.store.set_last_server(
                id,
                Some(rusticmc_instances::LastServer {
                    name: join.name.clone().unwrap_or_else(|| join.address.clone()),
                    address: join.address.clone(),
                }),
            );
            crate::cmd::multiplayer::mark_joined(state, &join.address);
            Some(QuickPlay::Multiplayer(join.address))
        }
        (None, Some(world)) => Some(QuickPlay::Singleplayer(world)),
        _ => None,
    };
    state
        .instances
        .launch_with_client(id, identity, quick_play, client)
        .await
        .map_err(|e| {
            if matches!(e, rusticmc_instances::InstanceError::NotPrepared) {
                IpcError::with_reason(IpcErrorCode::Unavailable, e.to_string(), "prepare.needed")
            } else {
                e.into()
            }
        })?;
    Ok(())
}

#[tauri::command]
pub fn instance_launch_cancel(state: State<'_>, args: InstanceIdArgs) -> Res<LaunchStatus> {
    let status = phase(&args.instance_id, "idle", None);
    set_launch(&state, status.clone());
    Ok(status)
}

#[tauri::command]
pub async fn instance_stop(state: State<'_>, args: InstanceIdArgs) -> Res<LaunchStatus> {
    set_launch(&state, phase(&args.instance_id, "stopping", None));
    state.instances.kill(&args.instance_id).await?;
    Ok(phase(&args.instance_id, "stopping", None))
}

#[tauri::command]
pub fn launch_status(state: State<'_>) -> Vec<LaunchStatus> {
    state
        .launches
        .lock()
        .expect("launches")
        .values()
        .cloned()
        .collect()
}

/// Forwards game events to the UI (`game-log`, `game-exited`, `launch-state`).
pub fn forward_game_events(state: Arc<AppState>) {
    let mut rx = state.instances.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(e) = rx.recv().await {
            match e.event {
                GameEvent::Log { stream, line } => {
                    if let Some(place) = crate::discord::place_from_game_line(&line) {
                        state.discord.set_place(place);
                    }
                    let lower = line.to_ascii_lowercase();
                    let level = if lower.contains("error") || lower.contains("panicked") {
                        "error"
                    } else if lower.contains("warn")
                        || stream == rusticmc_instances::launch::Stream::Stderr
                    {
                        "warn"
                    } else {
                        "info"
                    };
                    let _ = state.app.emit(
                        "game-log",
                        GameLogLine {
                            instance_id: e.instance_id.clone(),
                            level,
                            line,
                        },
                    );
                }
                GameEvent::Exited {
                    outcome,
                    crash_report,
                    played_seconds,
                } => {
                    let (code, crashed) = match &outcome {
                        ExitOutcome::Quit => (0, false),
                        ExitOutcome::Crashed { code } => (*code, true),
                        ExitOutcome::RefusedCommandLine { .. } => (2, true),
                        ExitOutcome::Killed => (-1, false),
                        ExitOutcome::Other { code } => (code.unwrap_or(-1), code.unwrap_or(0) != 0),
                    };
                    let _ = state.app.emit(
                        "game-exited",
                        GameExited {
                            instance_id: e.instance_id.clone(),
                            code,
                            crashed,
                            crash_report_path: crash_report.map(|p| p.display().to_string()),
                            played_s: played_seconds,
                        },
                    );
                    set_launch(
                        &state,
                        phase(
                            &e.instance_id,
                            if crashed { "crashed" } else { "idle" },
                            None,
                        ),
                    );
                    state.discord.set(crate::discord::Presence {
                        place: crate::discord::Place::Launcher,
                        game: None,
                        since: rusticmc_launcher_core::time::now(),
                    });
                    crate::update::on_game_exit(state.clone());
                    let st = state.clone();
                    // The playtime is written by the exit task; refresh the list after it.
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        instances_changed(&st);
                    });
                }
                GameEvent::SessionRefused { message } => {
                    let _ = state.app.emit(
                        "game-log",
                        GameLogLine {
                            instance_id: e.instance_id.clone(),
                            level: "warn",
                            line: format!("Not signed in for online servers: {message}"),
                        },
                    );
                }
                GameEvent::Started { .. } => {
                    // "Horizon 26.3": the game version the instance's build is pinned to.
                    let game = state
                        .instances
                        .store
                        .get(&e.instance_id)
                        .and_then(|i| state.instances.build_for(&i))
                        .ok()
                        .and_then(|b| rusticmc_instances::game_files::Pins::of_build(&b.dir))
                        .map_or_else(
                            || "Horizon".to_owned(),
                            |p| format!("Horizon {}", p.version),
                        );
                    state.discord.set(crate::discord::Presence {
                        place: crate::discord::Place::Menu,
                        game: Some(game),
                        since: rusticmc_launcher_core::time::now(),
                    });
                }
            }
        }
    });
}
