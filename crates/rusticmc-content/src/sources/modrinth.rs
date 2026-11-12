//! Modrinth's API v2 (https://docs.modrinth.com/api/): search with facets, projects, versions,
//! files with their hashes, dependencies.
//!
//! - Every request carries an identifying `User-Agent`: `rustic-launcher/<version> (<contact>)`,
//!   the contact coming from the launcher's config (Modrinth asks for one; TODO_HUMAN).
//! - Rate limits: Modrinth answers `X-Ratelimit-Limit`, `X-Ratelimit-Remaining` and
//!   `X-Ratelimit-Reset` (seconds until the window resets). When `Remaining` reaches 0 the next
//!   request waits for the reset; a 429 is retried after the reset (or an exponential backoff),
//!   at most `max_retries` times; 5xx and network errors back off the same way.
use crate::error::{ContentError, Result};
use crate::model::*;
use rusticmc_launcher_core::reqwest;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub const DEFAULT_BASE: &str = "https://api.modrinth.com/v2";

/// Where files from Modrinth may come from (its documented `.mrpack` download hosts).
pub const ALLOWED_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

#[derive(Clone, Debug)]
pub struct ModrinthConfig {
    pub base_url: String,
    /// Contact shown in the User-Agent (an e-mail or a URL), from the launcher's config.
    pub contact: String,
    pub max_retries: u32,
    /// First backoff; doubled at each retry.
    pub base_delay: Duration,
    /// No wait is longer than this (a reset far away is retried later instead).
    pub max_delay: Duration,
}

impl Default for ModrinthConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE.into(),
            contact: "contact unset".into(),
            max_retries: 4,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
        }
    }
}

/// The identifying User-Agent Modrinth requires.
pub fn user_agent(contact: &str) -> String {
    format!(
        "rustic-launcher/{} ({contact})",
        rusticmc_launcher_core::VERSION
    )
}

#[derive(Default)]
struct RateState {
    /// Don't send before this (the window ran out).
    not_before: Option<Instant>,
}

/// A Modrinth client.
pub struct Modrinth {
    http: reqwest::Client,
    cfg: ModrinthConfig,
    rate: Mutex<RateState>,
}

/// A search.
#[derive(Clone, Debug, Default)]
pub struct SearchQuery {
    pub query: String,
    pub kind: Option<ContentKind>,
    /// Extra facet groups, ANDed; each group's facets ORed (`["categories:adventure"]`).
    pub facets: Vec<Vec<String>>,
    /// Restrict to a game version (`26.3`).
    pub game_version: Option<String>,
    pub offset: u32,
    pub limit: u32,
    /// `relevance`, `downloads`, `follows`, `newest`, `updated`.
    pub index: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchPage {
    pub hits: Vec<ProjectSummary>,
    pub offset: u32,
    pub limit: u32,
    pub total: u64,
}

#[derive(Deserialize)]
struct RawSearch {
    hits: Vec<RawHit>,
    #[serde(default)]
    offset: u32,
    #[serde(default)]
    limit: u32,
    #[serde(default)]
    total_hits: u64,
}

#[derive(Deserialize)]
struct RawHit {
    project_id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    display_categories: Vec<String>,
    #[serde(default)]
    project_type: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    author: String,
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    license: Option<String>,
}

/// A project (`GET /project/{id}`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RawProject {
    pub id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub license: Option<RawLicense>,
    #[serde(default)]
    pub followers: u64,
    #[serde(default)]
    pub updated: String,
    /// `required`, `optional`, `unsupported` or `unknown`.
    #[serde(default)]
    pub client_side: String,
    #[serde(default)]
    pub server_side: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub gallery: Vec<RawGalleryImage>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RawLicense {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RawGalleryImage {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Deserialize)]
struct RawVersion {
    id: String,
    project_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    files: Vec<RawFile>,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
    #[serde(default)]
    date_published: String,
    #[serde(default)]
    project_type: Option<String>,
}

#[derive(Deserialize)]
struct RawFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    hashes: RawHashes,
}

#[derive(Deserialize, Default)]
struct RawHashes {
    sha1: Option<String>,
    sha512: Option<String>,
}

#[derive(Deserialize)]
struct RawDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String,
}

/// Java loaders on Modrinth (their code can't run on Horizon).
pub const JAVA_LOADERS: &[&str] = &["fabric", "forge", "neoforge", "quilt", "liteloader", "rift"];
/// Server plugin platforms on Modrinth (Bukkit-family plugins: not Rustic Server plugins).
pub const PLUGIN_LOADERS: &[&str] = &[
    "bukkit",
    "spigot",
    "paper",
    "purpur",
    "folia",
    "sponge",
    "velocity",
    "bungeecord",
    "waterfall",
];

/// The content kind of a Modrinth project type and its loaders.
pub fn kind_of(project_type: &str, loaders: &[String]) -> ContentKind {
    let only_datapack = !loaders.is_empty() && loaders.iter().all(|l| l == "datapack");
    match project_type {
        "modpack" => ContentKind::Modpack,
        "resourcepack" => ContentKind::ResourcePack,
        "shader" => ContentKind::ShaderPack,
        "datapack" => ContentKind::DataPack,
        "plugin" => ContentKind::Plugin,
        _ if only_datapack => ContentKind::DataPack,
        // `/version` objects carry no project type: Modrinth tags resource packs with the
        // `minecraft` loader and shader packs with their shader loader (QA: a Faithful pack was
        // refused as "a Java mod (minecraft)").
        _ if !loaders.is_empty() && loaders.iter().all(|l| l == "minecraft") => {
            ContentKind::ResourcePack
        }
        _ if !loaders.is_empty()
            && loaders.iter().all(|l| SHADER_LOADERS.contains(&l.as_str())) =>
        {
            ContentKind::ShaderPack
        }
        _ => ContentKind::Mod,
    }
}

/// Modrinth's shader-pack loaders.
pub const SHADER_LOADERS: &[&str] = &["iris", "optifine", "canvas", "vanilla"];

fn facet_type(kind: ContentKind) -> Option<&'static str> {
    Some(match kind {
        ContentKind::Mod => "project_type:mod",
        ContentKind::Modpack => "project_type:modpack",
        ContentKind::ResourcePack => "project_type:resourcepack",
        ContentKind::ShaderPack | ContentKind::Shader => "project_type:shader",
        ContentKind::DataPack => "project_type:datapack",
        ContentKind::NativeMod | ContentKind::Plugin => return None,
    })
}

fn header_u64(resp: &reqwest::Response, name: &str) -> Option<u64> {
    resp.headers().get(name)?.to_str().ok()?.trim().parse().ok()
}

impl Modrinth {
    /// A client on the launcher's shared HTTP stack with Modrinth's User-Agent.
    pub fn new(cfg: ModrinthConfig) -> Result<Self> {
        let http = rusticmc_launcher_core::http::builder()
            .user_agent(user_agent(&cfg.contact))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ContentError::Network(e.to_string()))?;
        Ok(Self {
            http,
            cfg,
            rate: Mutex::new(RateState::default()),
        })
    }

    pub fn config(&self) -> &ModrinthConfig {
        &self.cfg
    }

    fn backoff(&self, attempt: u32) -> Duration {
        let d = self.cfg.base_delay.saturating_mul(1u32 << attempt.min(16));
        d.min(self.cfg.max_delay)
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{path}", self.cfg.base_url.trim_end_matches('/'));
        let mut attempt = 0;
        loop {
            let wait = {
                let rate = self.rate.lock().await;
                rate.not_before
                    .map(|t| t.saturating_duration_since(Instant::now()))
            };
            if let Some(w) = wait.filter(|w| !w.is_zero()) {
                tokio::time::sleep(w.min(self.cfg.max_delay)).await;
            }
            let sent = self.http.get(&url).query(query).send().await;
            let resp = match sent {
                Ok(r) => r,
                Err(e) if attempt < self.cfg.max_retries => {
                    tracing::debug!("modrinth: {e}; retrying");
                    tokio::time::sleep(self.backoff(attempt)).await;
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(ContentError::Network(e.to_string())),
            };
            let remaining = header_u64(&resp, "x-ratelimit-remaining");
            let reset = header_u64(&resp, "x-ratelimit-reset");
            {
                let mut rate = self.rate.lock().await;
                rate.not_before = match (remaining, reset) {
                    (Some(0), Some(secs)) => {
                        Some(Instant::now() + Duration::from_secs(secs).min(self.cfg.max_delay))
                    }
                    _ => None,
                };
            }
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                if attempt >= self.cfg.max_retries {
                    return Err(if status == 429 {
                        ContentError::RateLimited("Modrinth".into())
                    } else {
                        ContentError::Http {
                            url,
                            status,
                            message: "server error".into(),
                        }
                    });
                }
                let wait = match reset {
                    Some(secs) if status == 429 => {
                        Duration::from_secs(secs).min(self.cfg.max_delay)
                    }
                    _ => self.backoff(attempt),
                };
                tokio::time::sleep(wait.max(Duration::from_millis(1))).await;
                attempt += 1;
                continue;
            }
            if status == 404 {
                return Err(ContentError::NotFound(url));
            }
            if !(200..300).contains(&status) {
                let message = resp.text().await.unwrap_or_default();
                return Err(ContentError::Http {
                    url,
                    status,
                    message,
                });
            }
            return resp
                .json::<T>()
                .await
                .map_err(|e| ContentError::Json(e.to_string()));
        }
    }

    /// `GET /search` with facets: the kind, the game version and any extra groups.
    pub async fn search(&self, q: &SearchQuery) -> Result<SearchPage> {
        let mut facets: Vec<Vec<String>> = Vec::new();
        if let Some(f) = q.kind.and_then(facet_type) {
            facets.push(vec![f.to_owned()]);
        }
        if let Some(v) = &q.game_version {
            facets.push(vec![format!("versions:{v}")]);
        }
        facets.extend(q.facets.iter().cloned());
        let mut query = vec![
            ("query", q.query.clone()),
            (
                "limit",
                (if q.limit == 0 { 20 } else { q.limit.min(100) }).to_string(),
            ),
            ("offset", q.offset.to_string()),
        ];
        if !facets.is_empty() {
            query.push(("facets", serde_json::to_string(&facets)?));
        }
        if let Some(i) = &q.index {
            query.push(("index", i.clone()));
        }
        let raw: RawSearch = self.get("/search", &query).await?;
        Ok(SearchPage {
            hits: raw
                .hits
                .into_iter()
                .map(|h| {
                    let loaders: Vec<String> = h
                        .categories
                        .iter()
                        .filter(|c| {
                            JAVA_LOADERS.contains(&c.as_str())
                                || PLUGIN_LOADERS.contains(&c.as_str())
                                || c.as_str() == "datapack"
                        })
                        .cloned()
                        .collect();
                    let tags = h
                        .display_categories
                        .iter()
                        .filter(|c| !h.categories.contains(c))
                        .cloned()
                        .collect();
                    ProjectSummary {
                        source: SourceKind::Modrinth,
                        kind: kind_of(&h.project_type, &loaders),
                        id: h.project_id,
                        slug: h.slug,
                        name: h.title,
                        summary: h.description,
                        authors: vec![h.author],
                        icon_url: h.icon_url,
                        downloads: h.downloads,
                        categories: h.categories,
                        tags,
                        license: h.license,
                        loaders,
                        game_versions: h.versions,
                        fair_play: FairPlay::Clean,
                    }
                })
                .collect(),
            offset: raw.offset,
            limit: raw.limit,
            total: raw.total_hits,
        })
    }

    /// `GET /project/{id|slug}`.
    pub async fn project(&self, id: &str) -> Result<RawProject> {
        self.get(&format!("/project/{id}"), &[]).await
    }

    /// `GET /projects?ids=[...]`: several projects at once (a version's dependencies' names).
    pub async fn projects(&self, ids: &[&str]) -> Result<Vec<RawProject>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        self.get("/projects", &[("ids", serde_json::to_string(ids)?)])
            .await
    }

    fn convert(&self, v: RawVersion, project_type: Option<&str>) -> ContentVersion {
        let pt = v
            .project_type
            .as_deref()
            .or(project_type)
            .unwrap_or("mod")
            .to_owned();
        ContentVersion {
            source: SourceKind::Modrinth,
            kind: kind_of(&pt, &v.loaders),
            project_id: v.project_id,
            id: v.id,
            name: v.name,
            version: v.version_number,
            game_versions: v.game_versions,
            loaders: v.loaders,
            files: v
                .files
                .into_iter()
                .map(|f| ContentFile {
                    url: f.url,
                    filename: f.filename,
                    size: f.size,
                    hashes: Hashes {
                        sha1: f.hashes.sha1,
                        sha256: None,
                        sha512: f.hashes.sha512,
                    },
                    primary: f.primary,
                    signature: None,
                })
                .collect(),
            dependencies: v
                .dependencies
                .into_iter()
                .map(|d| Dependency {
                    project_id: d.project_id,
                    version_id: d.version_id,
                    version_req: None,
                    sha256: None,
                    kind: match d.dependency_type.as_str() {
                        "required" => DependencyKind::Required,
                        "incompatible" => DependencyKind::Incompatible,
                        "embedded" => DependencyKind::Embedded,
                        _ => DependencyKind::Optional,
                    },
                })
                .collect(),
            published: v.date_published,
            horizon: None,
        }
    }

    /// `GET /project/{id}/version`, filtered by loaders and game versions, newest first.
    pub async fn versions(
        &self,
        project: &str,
        loaders: &[&str],
        game_versions: &[&str],
        project_type: Option<&str>,
    ) -> Result<Vec<ContentVersion>> {
        let mut query = Vec::new();
        if !loaders.is_empty() {
            query.push(("loaders", serde_json::to_string(loaders)?));
        }
        if !game_versions.is_empty() {
            query.push(("game_versions", serde_json::to_string(game_versions)?));
        }
        let raw: Vec<RawVersion> = self
            .get(&format!("/project/{project}/version"), &query)
            .await?;
        let mut out: Vec<ContentVersion> = raw
            .into_iter()
            .map(|v| self.convert(v, project_type))
            .collect();
        out.sort_by(|a, b| b.published.cmp(&a.published));
        Ok(out)
    }

    /// `GET /version/{id}`.
    pub async fn version(&self, id: &str) -> Result<ContentVersion> {
        let raw: RawVersion = self.get(&format!("/version/{id}"), &[]).await?;
        Ok(self.convert(raw, None))
    }
}

#[async_trait::async_trait]
impl crate::resolve::DependencySource for Modrinth {
    async fn by_version_id(&self, id: &str) -> Result<ContentVersion> {
        self.version(id).await
    }

    async fn best_for_project(
        &self,
        project: &str,
        ctx: &crate::resolve::ResolveContext,
    ) -> Result<Option<ContentVersion>> {
        let loaders: Vec<&str> = ctx.loaders.iter().map(String::as_str).collect();
        let games: Vec<&str> = ctx.game_versions.iter().map(String::as_str).collect();
        Ok(self
            .versions(project, &loaders, &games, None)
            .await?
            .into_iter()
            .next())
    }
}
