//! Horizon's registry: one `index.json` (schema 1) built by horizon-rust/horizon-mods' CI
//! (horizon/docs/api/mod-index.md; types `horizon_mods::index`). Read as the reference launcher
//! reads it: unknown fields ignored, unknown kinds kept and not offered, a newer schema refused.
//!
//! Before a file is installed (`check_file`, docs/api/content.md "Before installing a file"):
//! its address has one of the two allowed shapes, its size and SHA-256 are the entry's, it is not
//! revoked, its minisign signature verifies with the project's key when it has one, and what the
//! file says (its manifest) matches the entry.
use crate::error::{ContentError, Result};
use crate::horizon_env::HorizonEnv;
use crate::inspect;
use crate::model::*;
use rusticmc_launcher_core::reqwest;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_INDEX: &str = "https://horizon-rust.github.io/horizon-mods/index.json";
pub const SCHEMA: u32 = 1;
pub const MAX_FILE: u64 = 64 << 20;
pub const FILE_HOST: &str = "https://github.com/horizon-rust/horizon-mods/releases/download/";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub schema: u32,
    #[serde(default)]
    pub generated: String,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub revoked: Vec<Revoked>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    /// `mod`, `native-mod`, `modpack`, `resourcepack`, `shader`, `shaderpack`, `datapack`, or a
    /// kind this reader doesn't know (kept, not offered).
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub versions: Vec<Version>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub stars: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub version: String,
    #[serde(default)]
    pub published: String,
    #[serde(default)]
    pub changelog: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub hooks: Option<u32>,
    #[serde(default)]
    pub game: String,
    #[serde(default)]
    pub broken_on: Vec<BrokenOn>,
    #[serde(default)]
    pub requires: Vec<Requirement>,
    pub file: File,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BrokenOn {
    pub horizon: String,
    pub code: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Revoked {
    pub id: String,
    #[serde(default)]
    pub sha256: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub date: String,
}

/// The content kind of a registry kind; None for kinds this launcher doesn't offer.
pub fn kind_of(kind: &str) -> Option<ContentKind> {
    Some(match kind {
        "mod" => ContentKind::Mod,
        "native-mod" => ContentKind::NativeMod,
        "modpack" => ContentKind::Modpack,
        "resourcepack" => ContentKind::ResourcePack,
        "shader" => ContentKind::Shader,
        "shaderpack" => ContentKind::ShaderPack,
        "datapack" => ContentKind::DataPack,
        _ => return None,
    })
}

fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
}

/// The two address shapes the reference launcher accepts: a registry release asset, or a
/// release asset of the project's own GitHub repository, ending in the file's name.
pub fn check_url(url: &str, name: &str) -> Result<()> {
    if !plain_name(name) {
        return Err(ContentError::Refused(format!(
            "{name:?} is not a plain file name"
        )));
    }
    if url.chars().any(char::is_whitespace) || url.contains("..") {
        return Err(ContentError::Refused(format!(
            "{url:?}: not a clean address"
        )));
    }
    if let Some(rest) = url.strip_prefix(FILE_HOST) {
        if rest.split('/').count() == 2 && rest.ends_with(&format!("/{name}")) {
            return Ok(());
        }
    } else if let Some(rest) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 6
            && parts[2] == "releases"
            && parts[3] == "download"
            && parts.iter().all(|p| !p.is_empty())
            && parts[5] == name
        {
            return Ok(());
        }
    }
    Err(ContentError::Refused(format!(
        "{url}: not a registry release or a release asset of the project's repository"
    )))
}

impl Index {
    /// Parses an index; a newer schema is refused ("update").
    pub fn parse(bytes: &[u8], env: &HorizonEnv) -> Result<Index> {
        let index: Index = serde_json::from_slice(bytes)?;
        let max = env.index_schema.max(SCHEMA);
        if index.schema > max {
            return Err(ContentError::Incompatible(format!(
                "Horizon's registry index is schema {}, this launcher reads {max}: update Rustic Launcher",
                index.schema
            )));
        }
        Ok(index)
    }

    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// Why `sha256` of project `id` is revoked, if it is.
    pub fn revoked(&self, id: &str, sha256: &str) -> Option<&Revoked> {
        self.revoked.iter().find(|r| {
            r.id == id
                && r.sha256
                    .as_deref()
                    .is_none_or(|s| s.eq_ignore_ascii_case(sha256))
        })
    }

    /// Projects of `kind` matching `query` (name, summary, id, tags), with every category and
    /// tag in `filters`.
    pub fn search(
        &self,
        query: &str,
        kind: Option<ContentKind>,
        filters: &[String],
    ) -> Vec<ProjectSummary> {
        let q = query.to_lowercase();
        self.projects
            .iter()
            .filter(|p| kind_of(&p.kind).is_some())
            .filter(|p| kind.is_none_or(|k| kind_of(&p.kind) == Some(k)))
            .filter(|p| {
                q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.summary.to_lowercase().contains(&q)
                    || p.id.contains(&q)
                    || p.tags.iter().any(|t| t.contains(&q))
            })
            .filter(|p| {
                filters
                    .iter()
                    .all(|f| p.categories.contains(f) || p.tags.contains(f))
            })
            .map(|p| ProjectSummary {
                source: SourceKind::Horizon,
                id: p.id.clone(),
                slug: p.id.clone(),
                kind: kind_of(&p.kind).unwrap_or(ContentKind::Mod),
                name: p.name.clone(),
                summary: p.summary.clone(),
                authors: p.authors.clone(),
                icon_url: p.icon.clone(),
                downloads: p.downloads,
                categories: p.categories.clone(),
                tags: p.tags.clone(),
                license: Some(p.license.clone()).filter(|l| !l.is_empty()),
                loaders: Vec::new(),
                game_versions: p.versions.iter().map(|v| v.game.clone()).collect(),
                fair_play: FairPlay::Clean,
            })
            .collect()
    }

    /// The project's versions as content versions (newest first, as listed).
    pub fn versions(&self, id: &str) -> Vec<ContentVersion> {
        let Some(p) = self.project(id) else {
            return Vec::new();
        };
        let Some(kind) = kind_of(&p.kind) else {
            return Vec::new();
        };
        p.versions
            .iter()
            .map(|v| ContentVersion {
                source: SourceKind::Horizon,
                project_id: p.id.clone(),
                id: format!("{}@{}", p.id, v.version),
                name: p.name.clone(),
                version: v.version.clone(),
                kind,
                game_versions: vec![v.game.clone()],
                loaders: Vec::new(),
                files: vec![ContentFile {
                    url: v.file.url.clone(),
                    filename: v.file.name.clone(),
                    size: Some(v.file.size),
                    hashes: Hashes {
                        sha256: Some(v.file.sha256.clone()),
                        ..Hashes::default()
                    },
                    primary: true,
                    signature: v.file.signature.clone(),
                }],
                dependencies: v
                    .requires
                    .iter()
                    .map(|r| Dependency {
                        project_id: Some(r.id.clone()),
                        version_id: Some(format!("{}@{}", r.id, r.version)),
                        version_req: Some(format!("={}", r.version)),
                        sha256: Some(r.sha256.clone()),
                        kind: DependencyKind::Required,
                    })
                    .collect(),
                published: v.published.clone(),
                horizon: Some(HorizonFacts {
                    api: v.api.clone(),
                    hooks: v.hooks,
                    side: v.side.clone(),
                    capabilities: v.capabilities.clone(),
                    broken_on: v.broken_on.iter().map(|b| b.horizon.clone()).collect(),
                    key: p.key.clone(),
                }),
            })
            .collect()
    }

    /// The newest version of `id` this Horizon can install (its game, and its API for a content
    /// mod); `Index::installable` in the reference launcher.
    pub fn installable(&self, id: &str, env: &HorizonEnv) -> Option<ContentVersion> {
        self.versions(id).into_iter().find(|v| {
            let c = crate::compat::check(v, None, env);
            v.game_versions.iter().any(|g| g == &env.game)
                && matches!(
                    c.verdict,
                    Verdict::Compatible | Verdict::Partial | Verdict::NeedsBuild
                )
        })
    }

    /// The checks a downloaded registry file passes before it reaches the game folder.
    pub fn check_file(&self, v: &ContentVersion, path: &Path) -> Result<()> {
        let file = v
            .primary_file()
            .ok_or_else(|| ContentError::Refused("the version lists no file".into()))?;
        check_url(&file.url, &file.filename)?;
        let size = std::fs::metadata(path)?.len();
        if size > MAX_FILE || Some(size) != file.size {
            return Err(ContentError::Refused(format!(
                "{}: {size} bytes, the index lists {:?} (at most {MAX_FILE})",
                file.filename, file.size
            )));
        }
        crate::download::verify_file(path, file.size, &file.hashes)?;
        let sha = file.hashes.sha256.clone().unwrap_or_default();
        if let Some(r) = self.revoked(&v.project_id, &sha) {
            return Err(ContentError::Refused(format!(
                "taken down by Horizon's registry: {}",
                r.reason
            )));
        }
        let key = v.horizon.as_ref().and_then(|h| h.key.clone());
        match (key, &file.signature) {
            (Some(key), Some(sig)) => verify_signature(&key, sig, &std::fs::read(path)?)?,
            (Some(_), None) => {
                return Err(ContentError::Refused(
                    "the project signs its files but this version has no signature".into(),
                ));
            }
            (None, _) => tracing::info!("{}: unsigned project", v.project_id),
        }
        inspect::check_matches(v, path)
    }
}

/// Checks a minisign signature (prehashed or legacy) of `bytes` against `key` (base64).
pub fn verify_signature(key: &str, signature: &str, bytes: &[u8]) -> Result<()> {
    let key = minisign_verify::PublicKey::from_base64(key.trim())
        .map_err(|e| ContentError::Refused(format!("signing key: {e}")))?;
    let sig = minisign_verify::Signature::decode(signature)
        .map_err(|e| ContentError::Refused(format!("signature: {e}")))?;
    key.verify(bytes, &sig, false)
        .map_err(|e| ContentError::Refused(format!("signature does not verify: {e}")))
}

/// Horizon's registry over HTTP.
pub struct HorizonRegistry {
    http: reqwest::Client,
    pub index_url: String,
}

impl HorizonRegistry {
    pub fn new(index_url: Option<String>) -> Result<Self> {
        Ok(Self {
            http: rusticmc_launcher_core::http::client()
                .map_err(|e| ContentError::Network(e.to_string()))?,
            index_url: index_url.unwrap_or_else(|| DEFAULT_INDEX.into()),
        })
    }

    /// Downloads and parses the index.
    pub async fn fetch(&self, env: &HorizonEnv) -> Result<Index> {
        let resp = self
            .http
            .get(&self.index_url)
            .send()
            .await
            .map_err(|e| ContentError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(ContentError::Http {
                url: self.index_url.clone(),
                status,
                message: String::new(),
            });
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ContentError::Network(e.to_string()))?;
        Index::parse(&bytes, env)
    }
}

/// The index as a dependency source (a modpack's `requires`).
pub struct IndexSource<'a> {
    pub index: &'a Index,
    pub env: &'a HorizonEnv,
}

#[async_trait::async_trait]
impl crate::resolve::DependencySource for IndexSource<'_> {
    async fn by_version_id(&self, id: &str) -> Result<ContentVersion> {
        let (project, version) = id
            .split_once('@')
            .ok_or_else(|| ContentError::NotFound(id.into()))?;
        self.index
            .versions(project)
            .into_iter()
            .find(|v| v.version == version)
            .ok_or_else(|| ContentError::NotFound(id.into()))
    }

    async fn best_for_project(
        &self,
        project: &str,
        _ctx: &crate::resolve::ResolveContext,
    ) -> Result<Option<ContentVersion>> {
        Ok(self.index.installable(project, self.env))
    }
}
