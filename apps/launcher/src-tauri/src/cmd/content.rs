//! The content catalogue over `rusticmc-content` (launcher/docs/CONTENT.md): search, details,
//! installs (dependencies resolved) and updates from Horizon's registry, Modrinth and Rustic's
//! catalogue, judged against the target's Horizon (`horizon-client --launch-api`), with the
//! fair-play filter applied everywhere.
//!
//! Project ids carry their source: `horizon:<id>` (Horizon's registry), `rustic:<id>` (Rustic's
//! catalogue), and Modrinth's own ids bare.

use crate::cmd::servers::{Place, place};
use crate::state::{AppState, Res, State};
use crate::util::not_available_yet;
use rusticmc_content::download::{CoreDownloader, Downloader, FileRequest};
use rusticmc_content::fairplay::FairPlayPolicy;
use rusticmc_content::install::{InstallOptions, Installer, check_origin};
use rusticmc_content::ledger::{Entry, Ledger};
use rusticmc_content::resolve::{self, DependencySource, ResolveContext};
use rusticmc_content::sources::horizon::{HorizonRegistry, Index, IndexSource};
use rusticmc_content::sources::modrinth::{
    Modrinth, ModrinthConfig, SearchQuery, kind_of as mr_kind,
};
use rusticmc_content::sources::rustic::{
    HttpRusticRegistry, PriceFilter, RusticProject, RusticRegistry, RusticSearch, RusticSort,
    RusticVersion,
};
use rusticmc_content::{
    Badge, Compatibility, ContentKind, ContentVersion, Dependency, DependencyKind, FairPlay,
    HorizonEnv, ProjectSummary, SourceKind, Target, Verdict, compat, updates,
};
use rusticmc_launcher_core::{IpcError, IpcErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

#[derive(Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallTarget {
    Instance {
        instance_id: String,
    },
    Server {
        server_id: String,
    },
    World {
        instance_id: String,
        world_id: String,
    },
    ServerWorld {
        server_id: String,
    },
}

#[derive(Deserialize)]
pub struct ContentSearchQuery {
    pub query: String,
    pub kind: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub target: Option<InstallTarget>,
    #[serde(default)]
    pub compatible_only: bool,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    /// `free` or `paid` (only Rustic's marketplace has paid projects).
    #[serde(default)]
    pub price: Option<String>,
    /// Only projects a Rustic admin verified (Rustic's catalogue only).
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

#[derive(Deserialize)]
pub struct ContentGetArgs {
    pub project_id: String,
    #[serde(default)]
    pub target: Option<InstallTarget>,
}

#[derive(Deserialize)]
pub struct KindArgs {
    pub kind: String,
}

#[derive(Deserialize)]
pub struct InstallRequest {
    pub project_id: String,
    #[serde(default)]
    pub version_id: Option<String>,
    pub target: InstallTarget,
    #[serde(default)]
    pub with_dependencies: bool,
}

#[derive(Serialize)]
pub struct ContentCategory {
    pub id: String,
    pub kind: String,
}

/// Where a project id comes from.
enum Src<'a> {
    Modrinth(&'a str),
    Horizon(&'a str),
    Rustic(&'a str),
}

fn src(id: &str) -> Src<'_> {
    if let Some(h) = id.strip_prefix("horizon:") {
        Src::Horizon(h)
    } else if let Some(r) = id.strip_prefix("rustic:") {
        Src::Rustic(r)
    } else {
        Src::Modrinth(id)
    }
}

fn ui_id(source: SourceKind, id: &str) -> String {
    match source {
        SourceKind::Horizon => format!("horizon:{id}"),
        SourceKind::Rustic => format!("rustic:{id}"),
        _ => id.to_owned(),
    }
}

fn kind_of(k: &str) -> Res<ContentKind> {
    Ok(match k {
        "mod" => ContentKind::Mod,
        "modpack" => ContentKind::Modpack,
        "resourcepack" => ContentKind::ResourcePack,
        "shader" => ContentKind::ShaderPack,
        "datapack" => ContentKind::DataPack,
        "plugin" => ContentKind::Plugin,
        other => return Err(IpcError::invalid(format!("unknown kind {other}"))),
    })
}

/// The UI's kind; Horizon's two shader formats and native mods fold into it.
fn kind_name(k: ContentKind) -> &'static str {
    match k {
        ContentKind::Mod | ContentKind::NativeMod => "mod",
        ContentKind::Modpack => "modpack",
        ContentKind::ResourcePack => "resourcepack",
        ContentKind::Shader | ContentKind::ShaderPack => "shader",
        ContentKind::DataPack => "datapack",
        ContentKind::Plugin => "plugin",
    }
}

/// The kinds the UI's kind covers.
fn kinds_for(k: ContentKind) -> Vec<ContentKind> {
    match k {
        ContentKind::Mod => vec![ContentKind::Mod, ContentKind::NativeMod],
        ContentKind::ShaderPack | ContentKind::Shader => {
            vec![ContentKind::Shader, ContentKind::ShaderPack]
        }
        other => vec![other],
    }
}

fn shader_format(k: ContentKind) -> Option<&'static str> {
    match k {
        ContentKind::Shader => Some("horizon"),
        ContentKind::ShaderPack => Some("shaders_dir"),
        _ => None,
    }
}

fn content_err(e: rusticmc_content::ContentError) -> IpcError {
    use rusticmc_content::ContentError as E;
    match e {
        E::NotFound(m) => IpcError::not_found(m),
        E::Incompatible(m) | E::Refused(m) | E::Dependency(m) | E::Plugin(m) => {
            IpcError::with_reason(IpcErrorCode::Conflict, m, "content")
        }
        other => IpcError::with_reason(IpcErrorCode::Network, other.to_string(), "content"),
    }
}

fn modrinth() -> Res<Modrinth> {
    // TODO_HUMAN C1: Modrinth asks for a contact in the User-Agent.
    Modrinth::new(ModrinthConfig {
        contact: "https://github.com/Rustic-Launcher/launcher".into(),
        ..Default::default()
    })
    .map_err(content_err)
}

fn rustic_registry(state: &AppState) -> Res<Arc<HttpRusticRegistry>> {
    HttpRusticRegistry::new(crate::cloud_link::api_base(&state.config().cloud.api_base))
        .map(Arc::new)
        .map_err(content_err)
}

/// A Rustic project the API does not show (private, pending, taken down, or no such id).
fn rustic_not_found(e: rusticmc_content::ContentError) -> IpcError {
    match e {
        rusticmc_content::ContentError::NotFound(_) => IpcError::with_reason(
            IpcErrorCode::NotFound,
            "This project is not available",
            "content.not_available",
        ),
        other => content_err(other),
    }
}

/// Rustic versions per project, kept two minutes (search verdicts ask for every hit's).
type VersionCache = HashMap<String, (Instant, Arc<Vec<RusticVersion>>)>;

static RUSTIC_VERSIONS: LazyLock<tokio::sync::Mutex<VersionCache>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

async fn rustic_versions(
    reg: &HttpRusticRegistry,
    id: &str,
    fresh: bool,
) -> rusticmc_content::Result<Arc<Vec<RusticVersion>>> {
    if !fresh
        && let Some((at, v)) = RUSTIC_VERSIONS.lock().await.get(id)
        && at.elapsed() < Duration::from_secs(120)
    {
        return Ok(v.clone());
    }
    let v = Arc::new(reg.versions(id).await?);
    let mut cache = RUSTIC_VERSIONS.lock().await;
    if cache.len() > 512 {
        cache.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(120));
    }
    cache.insert(id.to_owned(), (Instant::now(), v.clone()));
    Ok(v)
}

/// The newest version that installs on `t`, else the newest one.
fn best_version<'a>(
    versions: &'a [RusticVersion],
    t: Option<&Target>,
    env: &HorizonEnv,
) -> Option<&'a RusticVersion> {
    versions
        .iter()
        .find(|v| {
            let c = compat::check(&v.version, t, env);
            // A native mod that builds here counts: the launcher compiles it (ADR 0157).
            c.installable() || c.verdict == Verdict::NeedsBuild
        })
        .or_else(|| versions.first())
}

/// The stricter of two fair-play standings (the API's and this launcher's policy).
fn stricter(a: FairPlay, b: FairPlay) -> FairPlay {
    let rank = |f: &FairPlay| match f {
        FairPlay::Clean => 0,
        FairPlay::Flagged { .. } => 1,
        FairPlay::Blocked { .. } => 2,
    };
    if rank(&b) > rank(&a) { b } else { a }
}

/// The verdict a Rustic project gets on `t`: from its best version's manifest facts.
fn rustic_compat(
    p: &RusticProject,
    versions: Option<&[RusticVersion]>,
    t: Option<&Target>,
    env: &HorizonEnv,
) -> Value {
    let off_server = !matches!(t, Some(Target::Server { .. }) | None);
    if p.summary.kind == ContentKind::Plugin && off_server {
        return json!({ "status": "incompatible", "reason": "needs_rustic_server",
                       "detail": "A plugin goes into a Rustic Server server." });
    }
    match versions.and_then(|v| best_version(v, t, env)) {
        Some(v) => compatibility(
            &compat::check(&v.version, t, env),
            matches!(
                p.summary.kind,
                ContentKind::Mod
                    | ContentKind::NativeMod
                    | ContentKind::Modpack
                    | ContentKind::Plugin
            ),
        ),
        None => json!({ "status": "unknown" }),
    }
}

/// The marketplace's card fields on a summary (`verified`, `price`, banner, size...).
fn rustic_extras(v: &mut Value, p: &RusticProject) {
    v["author"] = json!(p.author());
    v["verified"] = json!(p.verified);
    v["price"] = match &p.price {
        Some(pr) if pr.is_paid() => {
            json!({ "amount_minor": pr.amount_cents, "currency": pr.currency })
        }
        _ => Value::Null,
    };
    v["banner_url"] = json!(p.gallery.first().map(|g| g.url.clone()));
    v["size_bytes"] = json!(p.size);
    v["latest_version"] = json!(p.latest_version);
    v["created_at"] = json!(p.created_at);
    if let Some(u) = &p.updated_at {
        v["updated_at"] = json!(u);
    }
}

/// Horizon's registry index, fetched at most every 10 minutes.
type CachedIndex = Option<(Instant, Arc<Index>)>;

static INDEX: LazyLock<tokio::sync::Mutex<CachedIndex>> =
    LazyLock::new(|| tokio::sync::Mutex::new(None));

async fn horizon_index(env: &HorizonEnv) -> Res<Arc<Index>> {
    let mut slot = INDEX.lock().await;
    if let Some((at, i)) = slot.as_ref()
        && at.elapsed() < Duration::from_secs(600)
    {
        return Ok(i.clone());
    }
    let i = Arc::new(
        HorizonRegistry::new(None)
            .map_err(content_err)?
            .fetch(env)
            .await
            .map_err(content_err)?,
    );
    *slot = Some((Instant::now(), i.clone()));
    Ok(i)
}

/// The Horizon a target plays (its build's `--launch-api`), with the folder content goes to.
async fn env_for(state: &AppState, target: &InstallTarget) -> Res<(HorizonEnv, Target)> {
    let (build, t) = match target {
        InstallTarget::Instance { instance_id } => {
            let i = state.instances.store.get(instance_id)?;
            (
                state.instances.build_for(&i)?,
                Target::Instance {
                    game_dir: state.instances.store.game_dir(instance_id),
                },
            )
        }
        InstallTarget::World {
            instance_id,
            world_id,
        } => {
            let i = state.instances.store.get(instance_id)?;
            (
                state.instances.build_for(&i)?,
                Target::World {
                    game_dir: state.instances.store.game_dir(instance_id),
                    world: world_id.clone(),
                },
            )
        }
        InstallTarget::Server { server_id } | InstallTarget::ServerWorld { server_id } => {
            let Place::Local(id) = place(server_id)? else {
                return Err(not_available_yet(
                    "Installing catalogue content on a VPS server (upload the file instead)",
                ));
            };
            let p = state.servers.profile(&id)?;
            (
                state.instances.builds.get(&p.build)?,
                Target::Server {
                    server_dir: state.servers.dir(&id),
                },
            )
        }
    };
    let mut cmd = tokio::process::Command::new(build.client());
    cmd.arg("--launch-api");
    // A console program: without this Windows flashes a console window on every query.
    rusticmc_instances::process::no_window(&mut cmd);
    let out = cmd
        .output()
        .await
        .map_err(|e| IpcError::internal(e.to_string()))?;
    let env =
        HorizonEnv::from_launch_api(&String::from_utf8_lossy(&out.stdout)).map_err(content_err)?;
    Ok((env, t))
}

/// The target's environment, or Horizon's defaults when there is none (or its build can't say).
async fn env_or_default(
    state: &AppState,
    t: Option<&InstallTarget>,
) -> (HorizonEnv, Option<Target>) {
    match t {
        Some(t) => match env_for(state, t).await {
            Ok((e, t)) => (e, Some(t)),
            Err(e) => {
                tracing::debug!("content: no launch API for the target: {}", e.message);
                (HorizonEnv::default(), None)
            }
        },
        None => (HorizonEnv::default(), None),
    }
}

/// The UI's compatibility for a verdict.
fn compatibility(c: &Compatibility, ships_horizon_build: bool) -> Value {
    match c.verdict {
        Verdict::Compatible => {
            json!({ "status": "compatible", "builds_on_horizon": ships_horizon_build })
        }
        Verdict::Partial => json!({ "status": "compatible", "builds_on_horizon": false }),
        // A native mod that fits the instance's Horizon: compiled on the player's PC (ADR 0157).
        Verdict::NeedsBuild => json!({
            "status": "compatible",
            "builds_on_horizon": false,
            "needs_build": true,
            "detail": c.reasons.join("; "),
        }),
        Verdict::Incompatible | Verdict::Blocked => {
            let reason = match c.badge {
                Badge::NeedsPort => "java_only",
                Badge::NeedsNewerHorizon | Badge::DoesntBuildOnYourHorizon => "horizon_version",
                Badge::WrongTarget if c.reasons.iter().any(|r| r.contains("Rustic Server")) => {
                    "needs_rustic_server"
                }
                Badge::FairPlayBlocked => "not_fair_play",
                _ if c.verdict == Verdict::Blocked => "not_fair_play",
                _ => "unsupported_format",
            };
            json!({ "status": "incompatible", "reason": reason, "detail": c.reasons.join("; ") })
        }
    }
}

/// A Modrinth hit's verdict without a version: what its kind and loaders say.
fn modrinth_hit_compat(h: &ProjectSummary, env: &HorizonEnv) -> Value {
    let probe = ContentVersion {
        source: SourceKind::Modrinth,
        project_id: h.id.clone(),
        id: String::new(),
        name: h.name.clone(),
        version: String::new(),
        kind: h.kind,
        game_versions: h.game_versions.clone(),
        loaders: h.loaders.clone(),
        files: vec![],
        dependencies: vec![],
        published: String::new(),
        horizon: None,
    };
    compatibility(&compat::check(&probe, None, env), false)
}

fn installed_in(
    target: Option<&(InstallTarget, Target)>,
    source: SourceKind,
    id: &str,
) -> Vec<String> {
    let Some((ui, t)) = target else {
        return vec![];
    };
    let ledger = Ledger::load(t.root()).unwrap_or_default();
    if ledger.find_project(source, id).is_none() {
        return vec![];
    }
    vec![match ui {
        InstallTarget::Instance { instance_id } | InstallTarget::World { instance_id, .. } => {
            instance_id.clone()
        }
        InstallTarget::Server { server_id } | InstallTarget::ServerWorld { server_id } => {
            server_id.clone()
        }
    }]
}

/// Promoted (the "featured" rows): clean by the fair-play policy and widely used.
fn featured(p: &ProjectSummary) -> bool {
    p.fair_play == FairPlay::Clean && p.downloads >= 100_000
}

fn summary_json(h: &ProjectSummary, compat: Value, installed: Vec<String>, updated: &str) -> Value {
    let source = match h.source {
        SourceKind::Horizon => "horizon",
        SourceKind::Rustic => "rustic",
        _ => "modrinth",
    };
    let mut v = json!({
        "id": ui_id(h.source, &h.id), "slug": h.slug, "kind": kind_name(h.kind),
        "source": source, "name": h.name, "summary": h.summary,
        "author": h.authors.first().cloned().unwrap_or_default(),
        "icon_url": h.icon_url, "downloads": h.downloads, "follows": 0,
        "updated_at": updated, "categories": h.categories, "compatibility": compat,
        "installed_in": installed, "featured": featured(h),
        "game_versions": h.game_versions, "native": h.kind == ContentKind::NativeMod,
        "flagged": matches!(h.fair_play, FairPlay::Flagged { .. }),
        "verified": false, "price": null
    });
    if let Some(f) = shader_format(h.kind) {
        v["shader_format"] = json!(f);
    }
    v
}

fn blocked(p: &ProjectSummary) -> bool {
    matches!(p.fair_play, FairPlay::Blocked { .. })
}

#[tauri::command]
pub async fn content_search(state: State<'_>, args: ContentSearchQuery) -> Res<Value> {
    let kind = kind_of(&args.kind)?;
    let want = |s: &str| args.sources.is_empty() || args.sources.iter().any(|x| x == s);
    let limit = if args.limit == 0 {
        20
    } else {
        args.limit.min(100)
    };
    let (env, target) = env_or_default(&state, args.target.as_ref()).await;
    let target = args.target.clone().zip(target);
    let t = target.as_ref().map(|(_, t)| t);
    let policy = FairPlayPolicy::default();
    let mut hits: Vec<Value> = Vec::new();
    let mut total: u64 = 0;
    // Whether any source has results past this page (the UI's "Load more").
    let mut more = false;
    let incompatible = |c: &Value| c["status"] == "incompatible";
    let price = match args.price.as_deref() {
        Some("free") => Some(PriceFilter::Free),
        Some("paid") => Some(PriceFilter::Paid),
        _ => None,
    };
    // Paid and verified projects exist only in Rustic's catalogue: the others hold nothing then.
    let rustic_only = price == Some(PriceFilter::Paid) || args.verified;

    // Horizon's registry: one small index, searched locally (on the first page).
    if want("horizon") && kind != ContentKind::Plugin && !rustic_only {
        match horizon_index(&env).await {
            Ok(index) => {
                let filters: Vec<String> = args.category.iter().cloned().collect();
                let kinds = kinds_for(kind);
                let mut found: Vec<ProjectSummary> = index
                    .search(&args.query, None, &filters)
                    .into_iter()
                    .filter(|p| kinds.contains(&p.kind))
                    .collect();
                policy.apply(&mut found);
                found.retain(|p| !blocked(p));
                if args.sort.as_deref() == Some("downloads") {
                    found.sort_by_key(|a| std::cmp::Reverse(a.downloads));
                }
                // Every match is judged first (a small local index), so "compatible only"
                // filters before paging and the total is exact; then this page's slice.
                let mut judged: Vec<(&ProjectSummary, Value, String)> = found
                    .iter()
                    .map(|p| {
                        let versions = index.versions(&p.id);
                        let c = match versions.first() {
                            Some(v) => {
                                let best = index.installable(&p.id, &env).unwrap_or(v.clone());
                                compatibility(&compat::check(&best, t, &env), true)
                            }
                            None => json!({ "status": "unknown" }),
                        };
                        let updated = versions
                            .first()
                            .map(|v| v.published.clone())
                            .unwrap_or_default();
                        (p, c, updated)
                    })
                    .collect();
                if args.compatible_only {
                    judged.retain(|(_, c, _)| !incompatible(c));
                }
                if matches!(args.sort.as_deref(), Some("updated" | "newest")) {
                    // RFC 3339 dates sort as text; the newest first.
                    judged.sort_by(|a, b| b.2.cmp(&a.2));
                }
                total += judged.len() as u64;
                let (start, end) = page_bounds(judged.len(), args.offset, limit);
                more |= end < judged.len();
                for (p, c, updated) in &judged[start..end] {
                    let inst = installed_in(target.as_ref(), SourceKind::Horizon, &p.id);
                    hits.push(summary_json(p, c.clone(), inst, updated));
                }
            }
            Err(e) => tracing::warn!("Horizon registry unavailable: {}", e.message),
        }
    }

    // Modrinth: Java mods are listed as not running; packs, shaders, data packs and modpacks
    // are judged by kind. Bukkit-family plugins are not searched (they are not Rustic plugins).
    if want("modrinth") && kind != ContentKind::Plugin && !rustic_only {
        let page = modrinth()?
            .search(&SearchQuery {
                query: args.query.clone(),
                kind: Some(kind),
                facets: args
                    .category
                    .iter()
                    .map(|c| vec![format!("categories:{c}")])
                    .collect(),
                game_version: None,
                offset: args.offset,
                limit,
                index: args.sort.clone().map(|s| match s.as_str() {
                    "newest" | "updated" | "downloads" => s,
                    _ => "relevance".to_owned(),
                }),
            })
            .await
            .map_err(content_err)?;
        let mut found = page.hits;
        policy.apply(&mut found);
        let before = found.len();
        found.retain(|p| !blocked(p));
        let blocked_here = (before - found.len()) as u64;
        let mut shown = 0u64;
        for p in &found {
            let c = modrinth_hit_compat(p, &env);
            if args.compatible_only && incompatible(&c) {
                continue;
            }
            shown += 1;
            let inst = installed_in(target.as_ref(), SourceKind::Modrinth, &p.id);
            hits.push(summary_json(p, c, inst, ""));
        }
        let remote_more = u64::from(args.offset) + u64::from(limit) < page.total;
        if args.compatible_only {
            // Modrinth can't filter by Rustic's compatibility: count what this page kept, and
            // stop offering more once a page keeps nothing (e.g. Java mods, which never run).
            total += shown;
            more |= remote_more && shown > 0;
        } else {
            total += page.total.saturating_sub(blocked_here);
            more |= remote_more;
        }
    }

    // Rustic's catalogue: curated entries and creators' marketplace projects (ADR 0155). The
    // UI's kind may cover two of the API's (mods and native mods, both shader formats): one
    // query each, paged alike, as the sources are. Each hit is judged from its versions' manifest
    // facts (cached), as Horizon's registry hits are.
    if want("rustic") {
        let reg = rustic_registry(&state)?;
        let mut pages = tokio::task::JoinSet::new();
        for k in kinds_for(kind) {
            let reg = reg.clone();
            let q = RusticSearch {
                query: args.query.clone(),
                kind: Some(k),
                category: args.category.clone(),
                tag: None,
                sort: RusticSort::parse(args.sort.as_deref().unwrap_or("relevance")),
                price,
                verified: args.verified,
                offset: args.offset,
                limit,
            };
            pages.spawn(async move { reg.search(&q).await });
        }
        let mut found: Vec<RusticProject> = Vec::new();
        while let Some(page) = pages.join_next().await {
            match page {
                Ok(Ok(page)) => {
                    more |= page.has_more;
                    total += page.total;
                    found.extend(page.hits);
                }
                Ok(Err(e)) => tracing::debug!("Rustic catalogue: {e}"),
                Err(e) => tracing::warn!("Rustic catalogue search task: {e}"),
            }
        }
        for p in &mut found {
            let local = policy.judge(&p.summary);
            p.summary.fair_play = stricter(std::mem::take(&mut p.summary.fair_play), local);
        }
        let before = found.len();
        found.retain(|p| !blocked(&p.summary));
        total = total.saturating_sub((before - found.len()) as u64);
        let mut versions = tokio::task::JoinSet::new();
        for (i, p) in found.iter().enumerate() {
            let reg = reg.clone();
            let id = p.summary.id.clone();
            versions.spawn(async move { (i, rustic_versions(&reg, &id, false).await) });
        }
        let mut judged: Vec<Option<Arc<Vec<RusticVersion>>>> = vec![None; found.len()];
        while let Some(Ok((i, v))) = versions.join_next().await {
            match v {
                Ok(v) => judged[i] = Some(v),
                Err(e) => tracing::debug!("Rustic versions: {e}"),
            }
        }
        for (p, v) in found.iter().zip(&judged) {
            let c = rustic_compat(p, v.as_deref().map(Vec::as_slice), t, &env);
            if args.compatible_only && incompatible(&c) {
                total = total.saturating_sub(1);
                continue;
            }
            let inst = installed_in(target.as_ref(), SourceKind::Rustic, &p.summary.id);
            let mut v = summary_json(&p.summary, c, inst, "");
            rustic_extras(&mut v, p);
            hits.push(v);
        }
    }

    Ok(json!({
        "hits": hits, "total": total, "offset": args.offset, "limit": limit, "has_more": more
    }))
}

/// The `[start, end)` slice of `len` local results for a page at `offset` of `limit`.
fn page_bounds(len: usize, offset: u32, limit: u32) -> (usize, usize) {
    let start = (offset as usize).min(len);
    (start, start.saturating_add(limit as usize).min(len))
}

fn channel(version: &str) -> &'static str {
    let v = version.to_lowercase();
    if v.contains("alpha") {
        "alpha"
    } else if v.contains("beta") || v.contains("-pre") || v.contains("-rc") {
        "beta"
    } else {
        "release"
    }
}

fn version_json(v: &ContentVersion, c: Value) -> Value {
    json!({
        "id": ui_id(v.source, &v.id), "version": v.version, "channel": channel(&v.version),
        "horizon_versions": v.game_versions, "published_at": v.published,
        "downloads": 0, "size_bytes": v.primary_file().and_then(|f| f.size).unwrap_or(0),
        "compatibility": c
    })
}

fn gallery(urls: impl Iterator<Item = (String, String)>) -> Value {
    json!(
        urls.enumerate()
            .map(|(i, (url, title))| json!({ "url": url, "title": title, "hue": (i * 47) % 360 }))
            .collect::<Vec<_>>()
    )
}

#[tauri::command]
pub async fn content_get(state: State<'_>, args: ContentGetArgs) -> Res<Value> {
    let (env, target) = env_or_default(&state, args.target.as_ref()).await;
    let target = args.target.clone().zip(target);
    let t = target.as_ref().map(|(_, t)| t);
    let ledger = t.map(|t| Ledger::load(t.root()).unwrap_or_default());
    let has = |source: SourceKind, pid: &str| {
        ledger
            .as_ref()
            .is_some_and(|l| l.find_project(source, pid).is_some())
    };
    match src(&args.project_id) {
        Src::Horizon(id) => {
            let index = horizon_index(&env).await?;
            let p = index
                .project(id)
                .ok_or_else(|| IpcError::not_found(format!("{id} is not in Horizon's registry")))?;
            let versions = index.versions(id);
            let mut summary = index
                .search("", None, &[])
                .into_iter()
                .find(|s| s.id == id)
                .ok_or_else(|| IpcError::not_found(format!("{id} is not offered")))?;
            summary.fair_play = FairPlayPolicy::default().judge(&summary);
            let version_dtos: Vec<Value> = versions
                .iter()
                .map(|v| version_json(v, compatibility(&compat::check(v, t, &env), true)))
                .collect();
            let top = index
                .installable(id, &env)
                .map(|v| compatibility(&compat::check(&v, t, &env), true))
                .or_else(|| version_dtos.first().map(|v| v["compatibility"].clone()))
                .unwrap_or(json!({ "status": "unknown" }));
            let deps: Vec<Value> = versions
                .first()
                .map(|v| v.dependencies.clone())
                .unwrap_or_default()
                .iter()
                .filter_map(|d| d.project_id.clone())
                .map(|pid| {
                    let name = index
                        .project(&pid)
                        .map(|p| p.name.clone())
                        .unwrap_or(pid.clone());
                    json!({ "project_id": format!("horizon:{pid}"), "name": name,
                            "relation": "required", "installed": has(SourceKind::Horizon, &pid) })
                })
                .collect();
            let updated = versions
                .first()
                .map(|v| v.published.clone())
                .unwrap_or_default();
            let mut v = summary_json(
                &summary,
                top,
                installed_in(target.as_ref(), SourceKind::Horizon, id),
                &updated,
            );
            let side = match p.versions.first().and_then(|v| v.side.as_deref()) {
                Some("server") => "server",
                Some("both") => "both",
                _ => "client",
            };
            v["description_md"] = json!(p.description);
            v["license"] = json!(p.license);
            v["side"] = json!(side);
            v["gallery"] = gallery(
                p.screenshots
                    .iter()
                    .enumerate()
                    .map(|(i, u)| (u.clone(), format!("{} {}", p.name, i + 1))),
            );
            v["versions"] = json!(version_dtos);
            v["dependencies"] = json!(deps);
            v["changelog"] = json!(
                p.versions
                    .iter()
                    .map(|x| json!({ "version": x.version, "published_at": x.published, "notes": x.changelog }))
                    .collect::<Vec<_>>()
            );
            v["links"] = json!({ "source": p.source });
            Ok(v)
        }
        Src::Modrinth(id) => {
            let m = modrinth()?;
            let p = m.project(id).await.map_err(content_err)?;
            let kind = mr_kind(&p.project_type, &p.loaders);
            let versions = m
                .versions(id, &[], &[], Some(&p.project_type))
                .await
                .map_err(content_err)?;
            let mut summary = ProjectSummary {
                source: SourceKind::Modrinth,
                id: p.id.clone(),
                slug: p.slug.clone(),
                kind,
                name: p.title.clone(),
                summary: p.description.clone(),
                authors: vec![],
                icon_url: p.icon_url.clone(),
                downloads: p.downloads,
                categories: p.categories.clone(),
                tags: vec![],
                license: p.license.as_ref().map(|l| l.id.clone()),
                loaders: p.loaders.clone(),
                game_versions: p.game_versions.clone(),
                fair_play: FairPlay::Clean,
            };
            summary.fair_play = FairPlayPolicy::default().judge(&summary);
            if let FairPlay::Blocked { reason } = &summary.fair_play {
                return Err(IpcError::with_reason(
                    IpcErrorCode::Conflict,
                    format!("Not offered by Rustic: {reason}"),
                    "not_fair_play",
                ));
            }
            let version_dtos: Vec<Value> = versions
                .iter()
                .take(30)
                .map(|v| version_json(v, compatibility(&compat::check(v, t, &env), false)))
                .collect();
            let top = version_dtos
                .first()
                .map(|v| v["compatibility"].clone())
                .unwrap_or_else(|| modrinth_hit_compat(&summary, &env));
            // The newest version's dependencies, with their names.
            let deps = versions
                .first()
                .map(|v| v.dependencies.clone())
                .unwrap_or_default();
            let ids: Vec<&str> = deps
                .iter()
                .filter_map(|d| d.project_id.as_deref())
                .collect();
            let names = m.projects(&ids).await.unwrap_or_default();
            let deps: Vec<Value> = deps
                .iter()
                .filter(|d| d.kind != DependencyKind::Embedded)
                .filter_map(|d| {
                    let pid = d.project_id.clone()?;
                    let name = names
                        .iter()
                        .find(|n| n.id == pid)
                        .map(|n| n.title.clone())
                        .unwrap_or(pid.clone());
                    let relation = match d.kind {
                        DependencyKind::Required => "required",
                        DependencyKind::Incompatible => "incompatible",
                        _ => "optional",
                    };
                    Some(
                        json!({ "project_id": pid, "name": name, "relation": relation,
                                 "installed": has(SourceKind::Modrinth, &pid) }),
                    )
                })
                .collect();
            let side = match (p.client_side.as_str(), p.server_side.as_str()) {
                ("unsupported", _) => "server",
                (_, "unsupported") => "client",
                _ => "both",
            };
            let mut v = summary_json(
                &summary,
                top,
                installed_in(target.as_ref(), SourceKind::Modrinth, id),
                &p.updated,
            );
            v["follows"] = json!(p.followers);
            v["description_md"] = json!(p.body);
            v["license"] = json!(p.license.map(|l| l.id).unwrap_or_default());
            v["side"] = json!(side);
            v["gallery"] = gallery(
                p.gallery
                    .iter()
                    .map(|g| (g.url.clone(), g.title.clone().unwrap_or_default())),
            );
            v["versions"] = json!(version_dtos);
            v["dependencies"] = json!(deps);
            v["changelog"] = json!([]);
            v["links"] =
                json!({ "source": p.source_url, "issues": p.issues_url, "website": p.wiki_url });
            Ok(v)
        }
        Src::Rustic(id) => {
            let reg = rustic_registry(&state)?;
            let mut p = reg.project(id).await.map_err(rustic_not_found)?;
            let local = FairPlayPolicy::default().judge(&p.summary);
            p.summary.fair_play = stricter(std::mem::take(&mut p.summary.fair_play), local);
            if let FairPlay::Blocked { reason } = &p.summary.fair_play {
                return Err(IpcError::with_reason(
                    IpcErrorCode::Conflict,
                    format!("Not offered by Rustic: {reason}"),
                    "not_fair_play",
                ));
            }
            // The page's id may be a slug: versions and the ledger use the id.
            let pid = p.summary.id.clone();
            let versions = rustic_versions(&reg, &pid, true)
                .await
                .map_err(rustic_not_found)?;
            let builds = matches!(
                p.summary.kind,
                ContentKind::Mod
                    | ContentKind::NativeMod
                    | ContentKind::Modpack
                    | ContentKind::Plugin
            );
            let version_dtos: Vec<Value> = versions
                .iter()
                .map(|v| {
                    let mut j = version_json(
                        &v.version,
                        compatibility(&compat::check(&v.version, t, &env), builds),
                    );
                    j["channel"] = json!(v.channel.as_str());
                    j["downloads"] = json!(v.downloads);
                    j
                })
                .collect();
            let top = rustic_compat(&p, Some(&versions), t, &env);
            // The newest version's dependencies (Rustic project ids), with their names.
            let deps: Vec<Dependency> = versions
                .first()
                .map(|v| v.version.dependencies.clone())
                .unwrap_or_default();
            let mut dep_dtos = Vec::new();
            for d in deps.iter().filter(|d| d.kind != DependencyKind::Embedded) {
                let Some(dep) = d.project_id.clone() else {
                    continue;
                };
                let name = match reg.project(&dep).await {
                    Ok(x) => x.summary.name,
                    Err(_) => dep.clone(),
                };
                let relation = match d.kind {
                    DependencyKind::Required => "required",
                    DependencyKind::Incompatible => "incompatible",
                    _ => "optional",
                };
                dep_dtos.push(json!({
                    "project_id": format!("rustic:{dep}"), "name": name, "relation": relation,
                    "installed": has(SourceKind::Rustic, &dep)
                }));
            }
            let side = match versions
                .first()
                .and_then(|v| v.version.horizon.as_ref())
                .and_then(|h| h.side.as_deref())
            {
                Some("server") => "server",
                Some("both") => "both",
                Some("client") => "client",
                _ if p.summary.kind == ContentKind::Plugin => "server",
                _ => "client",
            };
            let updated = versions
                .first()
                .map(|v| v.version.published.clone())
                .unwrap_or_default();
            let mut v = summary_json(
                &p.summary,
                top,
                installed_in(target.as_ref(), SourceKind::Rustic, &pid),
                &updated,
            );
            rustic_extras(&mut v, &p);
            v["description_md"] = json!(p.description.clone().unwrap_or_default());
            v["license"] = json!(p.summary.license.clone().unwrap_or_default());
            v["side"] = json!(side);
            v["gallery"] = gallery(
                p.gallery
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (g.url.clone(), format!("{} {}", p.summary.name, i + 1))),
            );
            v["versions"] = json!(version_dtos);
            v["dependencies"] = json!(dep_dtos);
            v["changelog"] = json!(
                versions
                    .iter()
                    .filter(|x| !x.changelog.is_empty())
                    .map(|x| json!({ "version": x.version.version, "published_at": x.version.published, "notes": x.changelog }))
                    .collect::<Vec<_>>()
            );
            v["links"] = json!({
                "source": p.links.source, "issues": p.links.issues, "website": p.links.website,
                "wiki": p.links.wiki, "discord": p.links.discord
            });
            v["marketplace"] = json!(p.marketplace);
            Ok(v)
        }
    }
}

#[tauri::command]
pub fn content_categories(args: KindArgs) -> Res<Vec<ContentCategory>> {
    let list: &[&str] = match args.kind.as_str() {
        "mod" => &[
            "adventure",
            "decoration",
            "equipment",
            "magic",
            "optimization",
            "storage",
            "technology",
            "utility",
            "worldgen",
        ],
        "modpack" => &[
            "adventure",
            "challenging",
            "kitchen-sink",
            "lightweight",
            "multiplayer",
            "quests",
        ],
        "resourcepack" => &[
            "16x",
            "32x",
            "64x",
            "128x",
            "realistic",
            "simplistic",
            "themed",
            "vanilla-like",
        ],
        "shader" => &[
            "cartoon",
            "cursed",
            "fantasy",
            "realistic",
            "semi-realistic",
            "vanilla-like",
        ],
        "datapack" => &[
            "adventure",
            "game-mechanics",
            "magic",
            "utility",
            "worldgen",
        ],
        "plugin" => &[
            "chat",
            "economy",
            "management",
            "minigame",
            "moderation",
            "utility",
        ],
        other => return Err(IpcError::invalid(format!("unknown kind {other}"))),
    };
    Ok(list
        .iter()
        .map(|c| ContentCategory {
            id: (*c).into(),
            kind: args.kind.clone(),
        })
        .collect())
}

struct ModrinthDeps(Modrinth);

#[async_trait::async_trait]
impl DependencySource for ModrinthDeps {
    async fn by_version_id(&self, id: &str) -> rusticmc_content::Result<ContentVersion> {
        self.0.version(id).await
    }
    async fn best_for_project(
        &self,
        project: &str,
        ctx: &ResolveContext,
    ) -> rusticmc_content::Result<Option<ContentVersion>> {
        let gv: Vec<&str> = ctx.game_versions.iter().map(String::as_str).collect();
        Ok(self
            .0
            .versions(project, &[], &gv, None)
            .await?
            .into_iter()
            .next())
    }
}

/// Rustic dependencies: other Rustic projects (by id), the newest version that installs.
struct RusticDeps {
    reg: Arc<HttpRusticRegistry>,
    env: HorizonEnv,
}

#[async_trait::async_trait]
impl DependencySource for RusticDeps {
    async fn by_version_id(&self, id: &str) -> rusticmc_content::Result<ContentVersion> {
        let project = id.split('@').next().unwrap_or(id);
        rustic_versions(&self.reg, project, false)
            .await?
            .iter()
            .find(|v| v.version.id == id)
            .map(|v| v.version.clone())
            .ok_or_else(|| rusticmc_content::ContentError::NotFound(id.to_owned()))
    }
    async fn best_for_project(
        &self,
        project: &str,
        _ctx: &ResolveContext,
    ) -> rusticmc_content::Result<Option<ContentVersion>> {
        let versions = rustic_versions(&self.reg, project, false).await?;
        Ok(best_version(&versions, None, &self.env).map(|v| v.version.clone()))
    }
}

fn downloader(state: &AppState) -> CoreDownloader {
    CoreDownloader {
        manager: state.downloads.clone(),
    }
}

/// A scratch folder for one download, removed by the caller.
fn scratch(state: &AppState) -> Res<PathBuf> {
    let dir = state
        .paths
        .cache()
        .join("content-staging")
        .join(uuid::Uuid::new_v4().simple().to_string());
    std::fs::create_dir_all(&dir).map_err(|e| IpcError::internal(e.to_string()))?;
    Ok(dir)
}

/// Downloads a version's primary file into a scratch folder, checked (origin, size, hashes).
async fn fetch_file(state: &AppState, v: &ContentVersion) -> Res<(PathBuf, PathBuf)> {
    let f = v
        .primary_file()
        .ok_or_else(|| IpcError::not_found("this version has no file"))?;
    check_origin(v.source, &f.url, &f.filename).map_err(content_err)?;
    if f.filename.contains(['/', '\\']) || f.filename.starts_with('.') {
        return Err(IpcError::invalid("bad file name"));
    }
    let dir = scratch(state)?;
    let path = downloader(state)
        .fetch(&FileRequest {
            url: f.url.clone(),
            dest: dir.join(&f.filename),
            size: f.size,
            hashes: f.hashes.clone(),
        })
        .await
        .map_err(content_err)?;
    Ok((dir, path))
}

/// One installed project, as the UI lists it after an install.
#[derive(Serialize, Clone)]
pub struct InstalledItem {
    pub project_id: String,
    pub name: String,
    pub version: String,
}

/// What a native mod install needs before anything is downloaded (see `install_project`).
fn native_install_gate(
    state: &AppState,
    versions: &[ContentVersion],
    ui_target: &InstallTarget,
) -> Res<()> {
    if let Some(v) = versions.iter().find(|v| {
        v.kind == ContentKind::NativeMod
            && !matches!(v.source, SourceKind::Horizon | SourceKind::Rustic)
    }) {
        return Err(IpcError::with_reason(
            IpcErrorCode::Conflict,
            format!(
                "{}: native mods install only from Horizon's registry or Rustic's marketplace",
                v.name
            ),
            "native.source",
        ));
    }
    let InstallTarget::Instance { instance_id } = ui_target else {
        return Err(IpcError::invalid("a native mod goes into an instance"));
    };
    let instance = state.instances.store.get(instance_id)?;
    let build = state.instances.build_for(&instance)?;
    if build.source != "rustic-edition" {
        return Err(IpcError::with_reason(
            IpcErrorCode::Unavailable,
            "Native mods are built into the Rustic edition: switch this instance to it first.",
            "native.not_edition",
        ));
    }
    if rusticmc_native::Consent::load(&state.instances.store.dir(instance_id)).is_none() {
        return Err(crate::cmd::native::native_err(
            rusticmc_native::NativeError::ConsentRequired,
        ));
    }
    Ok(())
}

/// Installs `project_id` (any source) into `target`: the shared path of `content_install`, the
/// catalogue modpack import, catalogue plugin installs and updates.
pub async fn install_project(
    state: &AppState,
    project_id: &str,
    version_id: Option<&str>,
    ui_target: &InstallTarget,
    with_dependencies: bool,
) -> Res<Vec<InstalledItem>> {
    let (env, mut target) = env_for(state, ui_target).await?;
    let policy = FairPlayPolicy::default();
    let dl = downloader(state);
    let ctx = ResolveContext {
        game_versions: vec![env.game.clone()],
        loaders: vec![],
    };
    let (versions, index): (Vec<ContentVersion>, Option<Arc<Index>>) = match src(project_id) {
        Src::Horizon(id) => {
            let index = horizon_index(&env).await?;
            let version = match version_id {
                Some(v) => {
                    let v = v.strip_prefix("horizon:").unwrap_or(v);
                    let want = v.rsplit('@').next().unwrap_or(v);
                    index
                        .versions(id)
                        .into_iter()
                        .find(|x| x.version == want)
                        .ok_or_else(|| IpcError::not_found(format!("{id} {want}")))?
                }
                None => index.installable(id, &env).ok_or_else(|| {
                    IpcError::with_reason(
                        IpcErrorCode::Conflict,
                        format!("{id} has no version that runs on this Horizon"),
                        "content.incompatible",
                    )
                })?,
            };
            let versions = if with_dependencies {
                let ledger = Ledger::load(target.root()).map_err(content_err)?;
                let source = IndexSource {
                    index: &index,
                    env: &env,
                };
                resolve::resolve(version, &source, &ledger, &ctx)
                    .await
                    .map_err(content_err)?
            } else {
                vec![version]
            };
            (versions, Some(index))
        }
        Src::Modrinth(id) => {
            let m = modrinth()?;
            let version = match version_id {
                Some(v) => m.version(v).await.map_err(content_err)?,
                None => m
                    .versions(id, &[], &[], None)
                    .await
                    .map_err(content_err)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| IpcError::not_found("this project has no version"))?,
            };
            // A Modrinth modpack is an .mrpack: import what runs on Horizon.
            if version.kind == ContentKind::Modpack {
                let Target::Instance { game_dir } = &target else {
                    return Err(IpcError::invalid("a modpack goes into an instance"));
                };
                let (dir, file) = fetch_file(state, &version).await?;
                let report = rusticmc_content::mrpack::import(
                    &file,
                    game_dir,
                    &dl,
                    None,
                    &Default::default(),
                )
                .await;
                let _ = std::fs::remove_dir_all(&dir);
                let report = report.map_err(content_err)?;
                return Ok(report
                    .lines
                    .iter()
                    .filter(|l| {
                        matches!(
                            l.action,
                            rusticmc_content::mrpack::Action::Imported
                                | rusticmc_content::mrpack::Action::ImportedPartly
                        )
                    })
                    .map(|l| InstalledItem {
                        project_id: id.to_owned(),
                        name: l.path.clone(),
                        version: version.version.clone(),
                    })
                    .collect());
            }
            let versions = if with_dependencies {
                let ledger = Ledger::load(target.root()).map_err(content_err)?;
                resolve::resolve(version, &ModrinthDeps(modrinth()?), &ledger, &ctx)
                    .await
                    .map_err(content_err)?
            } else {
                vec![version]
            };
            (versions, None)
        }
        Src::Rustic(id) => {
            let reg = rustic_registry(state)?;
            let p = reg.project(id).await.map_err(rustic_not_found)?;
            // Creator sales are off (ADR 0155): a paid project is shown, never installed.
            if p.is_paid() {
                return Err(IpcError::with_reason(
                    IpcErrorCode::Conflict,
                    format!(
                        "{} is a paid project: buying it is not available yet",
                        p.summary.name
                    ),
                    "content.paid",
                ));
            }
            let pid = p.summary.id.clone();
            let versions = rustic_versions(&reg, &pid, true)
                .await
                .map_err(rustic_not_found)?;
            let version = match version_id {
                Some(v) => {
                    let v = v.strip_prefix("rustic:").unwrap_or(v);
                    versions
                        .iter()
                        .find(|x| x.version.id == v)
                        .map(|x| x.version.clone())
                        .ok_or_else(|| IpcError::not_found(format!("{v} is not listed")))?
                }
                None => best_version(&versions, Some(&target), &env)
                    .map(|x| x.version.clone())
                    .ok_or_else(|| IpcError::not_found("this project has no version"))?,
            };
            let versions = if with_dependencies {
                let ledger = Ledger::load(target.root()).map_err(content_err)?;
                resolve::resolve(
                    version,
                    &RusticDeps {
                        reg,
                        env: env.clone(),
                    },
                    &ledger,
                    &ctx,
                )
                .await
                .map_err(content_err)?
            } else {
                vec![version]
            };
            (versions, None)
        }
    };
    // A data pack aimed at an instance goes into the packs its new worlds start with.
    if let (Target::Instance { game_dir }, Some(v)) = (&target, versions.last())
        && v.kind == ContentKind::DataPack
    {
        target = Target::NewWorlds {
            game_dir: game_dir.clone(),
        };
    }
    // Plugins go through the server's plugin manager (the rules of `server_plugin_install`).
    if let Some(v) = versions.last()
        && v.kind == ContentKind::Plugin
    {
        let c = compat::check(v, Some(&target), &env);
        if !c.installable() {
            return Err(IpcError::with_reason(
                IpcErrorCode::Conflict,
                c.reasons.join("; "),
                "content.incompatible",
            ));
        }
        let (InstallTarget::Server { server_id } | InstallTarget::ServerWorld { server_id }) =
            ui_target
        else {
            return Err(IpcError::invalid(
                "a plugin goes into a Rustic Server server",
            ));
        };
        let Place::Local(sid) = place(server_id)? else {
            return Err(not_available_yet("Catalogue plugins on a VPS server"));
        };
        let (dir, file) = fetch_file(state, v).await?;
        let checked = rusticmc_content::plugins::read_rplugin(&file).map_err(content_err);
        let r = checked.and_then(|_| {
            state
                .servers
                .plugins(&sid)
                .and_then(|pm| pm.install(&file, false))
                .map_err(IpcError::from)
        });
        let _ = std::fs::remove_dir_all(&dir);
        let p = r?;
        return Ok(vec![InstalledItem {
            project_id: project_id.to_owned(),
            name: p.manifest.name.clone(),
            version: p.manifest.version.to_string(),
        }]);
    }
    // Native mods (ADR 0157): only from Horizon's registry (reviewed, its files pinned) or Rustic's
    // marketplace (admin-approved versions only), into an instance of the Rustic edition, once
    // the player accepted the trust notice for it. The build runs next (`native_build`, or the
    // next launch).
    let native = versions.iter().any(|v| v.kind == ContentKind::NativeMod);
    if native {
        native_install_gate(state, &versions, ui_target)?;
    }
    let installer = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &policy,
        index: index.as_deref(),
    };
    let done = installer
        .install(
            &versions,
            &target,
            &InstallOptions {
                enable: true,
                allow_native: native,
            },
        )
        .await
        .map_err(content_err)?;
    // A native mod's package passes the builder's own checks (only Rust, TOML and data files
    // under `mixins/<id>/`, sizes, no links) before it stays: else it is removed again.
    for d in &done {
        if d.path.extension().is_some_and(|e| e == "hnmod")
            && let Err(e) = rusticmc_native::bundle::Bundle::read(&d.path)
        {
            let _ = std::fs::remove_file(&d.path);
            return Err(IpcError::with_reason(
                IpcErrorCode::Conflict,
                format!("refused: {e}"),
                "native.bundle",
            ));
        }
    }
    Ok(done
        .iter()
        .map(|d| {
            let v = versions.iter().find(|v| v.project_id == d.project_id);
            InstalledItem {
                project_id: ui_id(v.map_or(SourceKind::Modrinth, |v| v.source), &d.project_id),
                name: v.map(|v| v.name.clone()).unwrap_or_default(),
                version: d.version.clone(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn content_install(state: State<'_>, args: InstallRequest) -> Res<Value> {
    let (instance, server) = match &args.target {
        InstallTarget::Instance { instance_id } | InstallTarget::World { instance_id, .. } => {
            (Some(instance_id.clone()), None)
        }
        InstallTarget::Server { server_id } | InstallTarget::ServerWorld { server_id } => {
            (None, Some(server_id.clone()))
        }
    };
    let (task, _) = state.tasks.start(
        "content",
        format!("Install {}", args.project_id),
        instance,
        server,
    );
    match install_project(
        &state,
        &args.project_id,
        args.version_id.as_deref(),
        &args.target,
        args.with_dependencies,
    )
    .await
    {
        Ok(installed) => {
            state.tasks.finish(&task);
            crate::cmd::instances::instances_changed(&state);
            Ok(json!({ "task_id": task, "installed": installed }))
        }
        Err(e) => {
            state.tasks.fail(&task, e.message.clone());
            Err(e)
        }
    }
}

/// `.mrpack` import into a new instance (the Horizon-compatible part, with the report).
pub async fn import_mrpack(state: &AppState, path: PathBuf, instance_id: &str) -> Res<Value> {
    let game = state.instances.store.game_dir(instance_id);
    let report = rusticmc_content::mrpack::import(
        &path,
        &game,
        &downloader(state),
        None,
        &Default::default(),
    )
    .await
    .map_err(content_err)?;
    Ok(serde_json::to_value(&report).unwrap_or_default())
}

/// The ledger entry owning the file `rel` (`/`-separated, under the game folder), if Rustic
/// installed it.
pub fn ledger_entry(game: &Path, rel: &str) -> Option<Entry> {
    Ledger::load(game)
        .ok()?
        .entries
        .into_iter()
        .find(|e| e.files.iter().any(|f| f.path == rel))
}

/// The target an entry was installed to, for the installer.
fn entry_target(game: &Path, e: &Entry) -> Target {
    match e.target.strip_prefix("world:") {
        Some(world) => Target::World {
            game_dir: game.to_path_buf(),
            world: world.to_owned(),
        },
        None if e.target == "new_worlds" => Target::NewWorlds {
            game_dir: game.to_path_buf(),
        },
        None => Target::Instance {
            game_dir: game.to_path_buf(),
        },
    }
}

fn with_installer<R>(state: &AppState, f: impl FnOnce(&Installer<'_>) -> R) -> R {
    let env = HorizonEnv::default();
    let policy = FairPlayPolicy::default();
    let dl = downloader(state);
    f(&Installer {
        downloader: &dl,
        env: &env,
        fairplay: &policy,
        index: None,
    })
}

/// Turns Rustic-installed content on or off with Horizon's own switches.
pub fn set_entry_enabled(state: &AppState, game: &Path, e: &Entry, on: bool) -> Res<()> {
    with_installer(state, |i| {
        i.set_enabled(&entry_target(game, e), e.source, &e.project_id, on)
    })
    .map_err(content_err)
}

/// Removes Rustic-installed content (refused while something installed needs it).
pub fn uninstall_entry(state: &AppState, game: &Path, e: &Entry) -> Res<()> {
    with_installer(state, |i| {
        i.uninstall(&entry_target(game, e), e.source, &e.project_id)
    })
    .map_err(content_err)
}

/// Updates an instance's Rustic-installed content (all, or the item owning one file) in the
/// background; returns the task id.
pub async fn update_instance(
    state: &Arc<AppState>,
    instance_id: &str,
    content_id: Option<String>,
) -> Res<String> {
    let ui_target = InstallTarget::Instance {
        instance_id: instance_id.to_owned(),
    };
    let (env, target) = env_for(state, &ui_target).await?;
    let game = target.root().clone();
    let ledger = Ledger::load(&game).map_err(content_err)?;
    let only = content_id.and_then(|c| ledger_entry(&game, &c).map(|e| e.project_id));
    let mut found = Vec::new();
    if let Ok(index) = horizon_index(&env).await {
        found.extend(updates::horizon_updates(&ledger, &index, &env));
    }
    found.extend(
        updates::modrinth_updates(&ledger, &modrinth()?, &env)
            .await
            .map_err(content_err)?,
    );
    if let Some(p) = &only {
        found.retain(|u| &u.project_id == p);
    }
    let (task, _) = state.tasks.start(
        "content",
        format!("Update {} item(s)", found.len()),
        Some(instance_id.to_owned()),
        None,
    );
    let st = state.clone();
    let task2 = task.clone();
    let id = instance_id.to_owned();
    tauri::async_runtime::spawn(async move {
        let mut failed = Vec::new();
        for u in found {
            let pid = ui_id(u.source, &u.project_id);
            let vid = match u.source {
                SourceKind::Horizon => u.available.version.clone(),
                _ => u.available.id.clone(),
            };
            let target = InstallTarget::Instance {
                instance_id: id.clone(),
            };
            if let Err(e) = install_project(&st, &pid, Some(&vid), &target, true).await {
                failed.push(format!("{}: {}", u.project_id, e.message));
            }
        }
        if failed.is_empty() {
            st.tasks.finish(&task2);
        } else {
            st.tasks.fail(&task2, failed.join("; "));
        }
        crate::cmd::instances::instances_changed(&st);
    });
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_native_mod_that_fits_is_installable_and_says_it_builds() {
        let c = Compatibility {
            verdict: Verdict::NeedsBuild,
            badge: Badge::BuildsOnYourHorizon,
            reasons: vec!["needs hook points 1, your Horizon has 415".into()],
        };
        let v = compatibility(&c, false);
        assert_eq!(v["status"], "compatible");
        assert_eq!(v["needs_build"], true);
        assert!(v["detail"].as_str().unwrap().contains("hook points"));
        let refused = Compatibility {
            verdict: Verdict::Incompatible,
            badge: Badge::NeedsNewerHorizon,
            reasons: vec!["update Horizon".into()],
        };
        assert_eq!(compatibility(&refused, false)["reason"], "horizon_version");
    }

    fn rustic_project(kind: &str, price: i64) -> RusticProject {
        serde_json::from_value(json!({
            "source": "rustic", "id": "sky", "slug": "sky", "kind": kind, "name": "Sky",
            "summary": "s", "authors": ["lea"], "downloads": 3, "fair_play": { "status": "clean" },
            "verified": true, "price": { "amount_cents": price, "currency": "EUR" },
            "gallery": [{ "url": "https://cdn.rustic-launcher.com/a.png", "width": 1, "height": 1 }],
            "owner": { "id": "u", "username": "lea_b", "display_name": "Léa" },
            "updated_at": "2026-11-12T10:00:00Z", "size": 42, "latest_version": "1.0.0"
        }))
        .unwrap()
    }

    fn rustic_version(v: &str, api: &str) -> RusticVersion {
        serde_json::from_value(json!({
            "source": "rustic", "project_id": "sky", "id": format!("sky@{v}"), "name": "Sky",
            "version": v, "kind": "mod", "game_versions": [], "loaders": [], "files": [],
            "dependencies": [], "published": "2026-11-12T10:00:00Z",
            "horizon": { "api": api, "hooks": null, "side": null, "capabilities": [],
                         "broken_on": [], "key": null },
            "channel": "beta", "downloads": 7
        }))
        .unwrap()
    }

    #[test]
    fn marketplace_cards_carry_price_banner_and_author() {
        let p = rustic_project("mod", 499);
        let mut v = summary_json(&p.summary, json!({ "status": "unknown" }), vec![], "");
        rustic_extras(&mut v, &p);
        assert_eq!(v["id"], "rustic:sky");
        assert_eq!(v["author"], "Léa");
        assert_eq!(v["verified"], true);
        assert_eq!(
            v["price"],
            json!({ "amount_minor": 499, "currency": "EUR" })
        );
        assert_eq!(v["banner_url"], "https://cdn.rustic-launcher.com/a.png");
        assert_eq!(v["size_bytes"], 42);
        assert_eq!(v["updated_at"], "2026-11-12T10:00:00Z");
        assert_eq!(v["native"], false);
        assert_eq!(v["flagged"], false);
        let free = rustic_project("native_mod", 0);
        let mut v = summary_json(&free.summary, json!({ "status": "unknown" }), vec![], "");
        rustic_extras(&mut v, &free);
        assert_eq!(v["price"], Value::Null);
        assert_eq!(v["kind"], "mod");
        assert_eq!(v["native"], true);
    }

    #[test]
    fn the_newest_installable_version_is_preferred() {
        let env = HorizonEnv::default();
        let list = vec![
            rustic_version("2.0.0", "9.9"),
            rustic_version("1.0.0", "0.2"),
        ];
        assert_eq!(
            best_version(&list, None, &env).unwrap().version.version,
            "1.0.0"
        );
        let none = vec![rustic_version("2.0.0", "9.9")];
        assert_eq!(
            best_version(&none, None, &env).unwrap().version.version,
            "2.0.0"
        );
        assert!(best_version(&[], None, &env).is_none());
        let p = rustic_project("mod", 0);
        assert_eq!(
            rustic_compat(&p, Some(&list), None, &env)["status"],
            "compatible"
        );
        assert_eq!(
            rustic_compat(&p, Some(&none), None, &env)["status"],
            "incompatible"
        );
        assert_eq!(rustic_compat(&p, None, None, &env)["status"], "unknown");
    }

    #[test]
    fn fair_play_keeps_the_stricter_verdict() {
        let flagged = FairPlay::Flagged { reason: "r".into() };
        let blocked = FairPlay::Blocked { reason: "b".into() };
        assert_eq!(stricter(FairPlay::Clean, flagged.clone()), flagged);
        assert_eq!(stricter(flagged.clone(), FairPlay::Clean), flagged);
        assert_eq!(stricter(flagged, blocked.clone()), blocked);
        assert_eq!(stricter(blocked.clone(), FairPlay::Clean), blocked);
    }

    #[test]
    fn local_results_page_past_the_first() {
        // Regression: Horizon's index answered only the first page, so "Load more" stayed empty.
        assert_eq!(page_bounds(148, 0, 12), (0, 12));
        assert_eq!(page_bounds(148, 12, 12), (12, 24));
        assert_eq!(page_bounds(148, 144, 12), (144, 148));
        assert_eq!(page_bounds(148, 200, 12), (148, 148));
        assert_eq!(page_bounds(0, 0, 12), (0, 0));
    }
}
