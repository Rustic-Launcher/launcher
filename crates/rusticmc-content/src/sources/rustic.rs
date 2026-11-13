//! Rustic's own registry (Rustic Cloud's catalogue and creators' marketplace, ADR 0155), read over
//! the API's public content routes (`services/api/src/routes/content.rs`, `openapi.json`):
//!
//! - `GET {base}/v1/content/search?q=&kind=&category=&tag=&sort=&price=&verified=&offset=&limit=`
//!   -> `{ "hits": [ContentProject], "has_more": bool, "total": int }` (`has_more`/`total` are
//!   missing on an API older than the marketplace: then a full page means "maybe more").
//! - `GET {base}/v1/content/projects/{id}` -> `ContentProject` (with `description`, `links`).
//! - `GET {base}/v1/content/projects/{id}/versions` -> `[ContentVersion]` (with `horizon` facts,
//!   `changelog`, `channel`, `downloads`). Files point at `/v1/content/files/<version>/<file>`,
//!   which redirects to the CDN; the downloader checks size and hashes as for any source.
//!
//! A project's JSON is `rusticmc_content::model::ProjectSummary` plus optional marketplace fields
//! (`RusticProject`); a version's is `ContentVersion` plus optional ones (`RusticVersion`). Private,
//! pending or taken-down projects answer 404 (the launcher sends no token): `ContentError::NotFound`.
use crate::error::{ContentError, Result};
use crate::model::{ContentKind, ContentVersion, ProjectSummary, SourceKind};
use rusticmc_launcher_core::reqwest;
use serde::{Deserialize, Serialize};

/// `GET /v1/content/search`'s `sort`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RusticSort {
    #[default]
    Relevance,
    Downloads,
    Newest,
    Updated,
}

impl RusticSort {
    pub fn as_str(self) -> &'static str {
        match self {
            RusticSort::Relevance => "relevance",
            RusticSort::Downloads => "downloads",
            RusticSort::Newest => "newest",
            RusticSort::Updated => "updated",
        }
    }

    /// The UI's sort names (`relevance`, `downloads`, `newest`, `updated`); anything else is
    /// relevance.
    pub fn parse(s: &str) -> Self {
        match s {
            "downloads" => RusticSort::Downloads,
            "newest" => RusticSort::Newest,
            "updated" => RusticSort::Updated,
            _ => RusticSort::Relevance,
        }
    }
}

/// `GET /v1/content/search`'s `price`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriceFilter {
    Free,
    Paid,
}

impl PriceFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            PriceFilter::Free => "free",
            PriceFilter::Paid => "paid",
        }
    }
}

/// One search of the Rustic catalogue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RusticSearch {
    pub query: String,
    pub kind: Option<ContentKind>,
    /// One of the kind's categories (`GET /v1/content/catalog`).
    pub category: Option<String>,
    pub tag: Option<String>,
    pub sort: RusticSort,
    pub price: Option<PriceFilter>,
    /// Only projects a Rustic admin verified.
    pub verified: bool,
    pub offset: u32,
    /// 1 to 100 (clamped).
    pub limit: u32,
}

/// One page of search results.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RusticPage {
    pub hits: Vec<RusticProject>,
    /// More results after `offset + hits.len()`.
    pub has_more: bool,
    /// Matching projects in all.
    pub total: u64,
}

/// A screenshot (re-encoded on upload, served from Rustic's CDN).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RusticImage {
    pub url: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

/// Links shown on a project's page (`https://` only, checked by the API).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RusticLinks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wiki: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

/// The Rustic account publishing a project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RusticOwner {
    pub id: String,
    pub username: String,
    pub display_name: String,
}

/// A price; `amount_cents` 0 is free. Creator sales are off: a paid project is shown, never sold
/// through the launcher (ADR 0155).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RusticPrice {
    pub amount_cents: i64,
    pub currency: String,
}

impl RusticPrice {
    pub fn is_paid(&self) -> bool {
        self.amount_cents > 0
    }
}

/// A project as the API sends it: the `ProjectSummary` fields, then the marketplace's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RusticProject {
    #[serde(flatten)]
    pub summary: ProjectSummary,
    /// Markdown; `GET /v1/content/projects/{id}` only.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub gallery: Vec<RusticImage>,
    #[serde(default)]
    pub links: RusticLinks,
    #[serde(default)]
    pub owner: Option<RusticOwner>,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub price: Option<RusticPrice>,
    /// RFC 3339.
    #[serde(default)]
    pub created_at: Option<String>,
    /// RFC 3339.
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub latest_version: Option<String>,
    /// Bytes of the newest version's primary file.
    #[serde(default)]
    pub size: Option<u64>,
    /// Uploaded by its creator (rather than curated by Rustic).
    #[serde(default)]
    pub marketplace: bool,
}

impl RusticProject {
    pub fn is_paid(&self) -> bool {
        self.price.as_ref().is_some_and(RusticPrice::is_paid)
    }

    /// Who to credit: the owner's display name, else the first author.
    pub fn author(&self) -> String {
        self.owner
            .as_ref()
            .map(|o| o.display_name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| self.summary.authors.first().cloned())
            .unwrap_or_default()
    }
}

/// A version's release channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    #[default]
    Release,
    Beta,
    Alpha,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Release => "release",
            Channel::Beta => "beta",
            Channel::Alpha => "alpha",
        }
    }
}

/// A version as the API sends it: the `ContentVersion` fields, then the marketplace's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RusticVersion {
    #[serde(flatten)]
    pub version: ContentVersion,
    #[serde(default)]
    pub changelog: String,
    #[serde(default)]
    pub channel: Channel,
    #[serde(default)]
    pub downloads: u64,
    /// Never listed to the launcher (the API keeps yanked versions for their owner); skipped if
    /// one shows up anyway.
    #[serde(default)]
    pub yanked: bool,
}

/// A project id or slug the API can hold: `[a-z0-9_-]{1,64}` (also what `rustic://content/...`
/// links carry). Anything else is refused before a request is made.
pub fn valid_project_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

#[async_trait::async_trait]
pub trait RusticRegistry: Send + Sync {
    async fn search(&self, q: &RusticSearch) -> Result<RusticPage>;
    async fn project(&self, id: &str) -> Result<RusticProject>;
    /// Newest first, yanked ones left out.
    async fn versions(&self, project: &str) -> Result<Vec<RusticVersion>>;
}

/// No endpoint configured: searches find nothing, lookups say so.
pub struct DisabledRusticRegistry;

#[async_trait::async_trait]
impl RusticRegistry for DisabledRusticRegistry {
    async fn search(&self, _q: &RusticSearch) -> Result<RusticPage> {
        Ok(RusticPage::default())
    }

    async fn project(&self, id: &str) -> Result<RusticProject> {
        Err(ContentError::NotConfigured(format!(
            "Rustic registry (asked for {id})"
        )))
    }

    async fn versions(&self, project: &str) -> Result<Vec<RusticVersion>> {
        Err(ContentError::NotConfigured(format!(
            "Rustic registry (asked for {project})"
        )))
    }
}

/// The registry over HTTP, at the endpoint from the launcher's config.
pub struct HttpRusticRegistry {
    http: reqwest::Client,
    base: String,
}

#[derive(Deserialize)]
struct Hits {
    hits: Vec<RusticProject>,
    /// Absent before the marketplace API.
    #[serde(default)]
    has_more: Option<bool>,
    #[serde(default)]
    total: Option<i64>,
}

impl HttpRusticRegistry {
    pub fn new(base: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: rusticmc_launcher_core::http::client()
                .map_err(|e| ContentError::Network(e.to_string()))?,
            base: base.into().trim_end_matches('/').to_owned(),
        })
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .get(&url)
            .query(query)
            .send()
            .await
            .map_err(|e| ContentError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status == 404 {
            return Err(ContentError::NotFound(url));
        }
        if !(200..300).contains(&status) {
            return Err(ContentError::Http {
                url,
                status,
                message: resp.text().await.unwrap_or_default(),
            });
        }
        resp.json()
            .await
            .map_err(|e| ContentError::Json(format!("{url}: {e}")))
    }
}

fn checked_id(id: &str) -> Result<&str> {
    if valid_project_id(id) {
        Ok(id)
    } else {
        Err(ContentError::NotFound(format!(
            "{id:?} is not a Rustic project id"
        )))
    }
}

#[async_trait::async_trait]
impl RusticRegistry for HttpRusticRegistry {
    async fn search(&self, s: &RusticSearch) -> Result<RusticPage> {
        let limit = s.limit.clamp(1, 100);
        let mut q = vec![
            ("q", s.query.clone()),
            ("offset", s.offset.to_string()),
            ("limit", limit.to_string()),
        ];
        if let Some(k) = s.kind {
            q.push(("kind", k.as_str().to_owned()));
        }
        if let Some(c) = s.category.as_deref().filter(|c| !c.is_empty()) {
            q.push(("category", c.to_owned()));
        }
        if let Some(t) = s.tag.as_deref().filter(|t| !t.is_empty()) {
            q.push(("tag", t.to_owned()));
        }
        if s.sort != RusticSort::Relevance {
            q.push(("sort", s.sort.as_str().to_owned()));
        }
        if let Some(p) = s.price {
            q.push(("price", p.as_str().to_owned()));
        }
        if s.verified {
            q.push(("verified", "true".to_owned()));
        }
        let page: Hits = self.get("/v1/content/search", &q).await?;
        let shown = page.hits.len() as u64;
        let has_more = page.has_more.unwrap_or(shown >= u64::from(limit));
        let total = page
            .total
            .and_then(|t| u64::try_from(t).ok())
            .unwrap_or(u64::from(s.offset) + shown + u64::from(has_more));
        Ok(RusticPage {
            hits: page
                .hits
                .into_iter()
                .map(|mut h| {
                    h.summary.source = SourceKind::Rustic;
                    h
                })
                .collect(),
            has_more,
            total,
        })
    }

    async fn project(&self, id: &str) -> Result<RusticProject> {
        let id = checked_id(id)?;
        let mut p: RusticProject = self.get(&format!("/v1/content/projects/{id}"), &[]).await?;
        p.summary.source = SourceKind::Rustic;
        Ok(p)
    }

    async fn versions(&self, project: &str) -> Result<Vec<RusticVersion>> {
        let project = checked_id(project)?;
        let mut v: Vec<RusticVersion> = self
            .get(&format!("/v1/content/projects/{project}/versions"), &[])
            .await?;
        v.retain(|x| !x.yanked);
        for x in &mut v {
            x.version.source = SourceKind::Rustic;
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_ids_are_strict() {
        for ok in [
            "a",
            "skyblock",
            "cozy_pack",
            "x-ray-blocker",
            &"a".repeat(64),
        ] {
            assert!(valid_project_id(ok), "{ok}");
        }
        for bad in [
            "",
            "Skyblock",
            "../etc",
            "a/b",
            "a b",
            "a%2Fb",
            "é",
            "a.b",
            &"a".repeat(65),
        ] {
            assert!(!valid_project_id(bad), "{bad}");
        }
    }

    #[test]
    fn sorts_round_trip() {
        for s in ["relevance", "downloads", "newest", "updated"] {
            assert_eq!(RusticSort::parse(s).as_str(), s);
        }
        assert_eq!(RusticSort::parse("anything"), RusticSort::Relevance);
    }
}
