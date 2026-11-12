//! Rustic's own registry (Rustic Cloud). The cloud owner defines its API
//! (`launcher/services/api/openapi.json`); until it exists, this is a trait with an HTTP
//! implementation over a **proposed** shape (launcher/docs/HANDOFF.md) and a disabled one when
//! no endpoint is configured:
//!
//! - `GET {base}/v1/content/search?q=&kind=&offset=&limit=` -> `{ "hits": [ProjectSummary] }`
//! - `GET {base}/v1/content/projects/{id}/versions` -> `[ContentVersion]`
use crate::error::{ContentError, Result};
use crate::model::{ContentKind, ContentVersion, ProjectSummary, SourceKind};
use rusticmc_launcher_core::reqwest;

#[async_trait::async_trait]
pub trait RusticRegistry: Send + Sync {
    async fn search(
        &self,
        query: &str,
        kind: Option<ContentKind>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ProjectSummary>>;
    async fn versions(&self, project: &str) -> Result<Vec<ContentVersion>>;
}

/// No endpoint configured: searches find nothing, version lookups say so.
pub struct DisabledRusticRegistry;

#[async_trait::async_trait]
impl RusticRegistry for DisabledRusticRegistry {
    async fn search(
        &self,
        _query: &str,
        _kind: Option<ContentKind>,
        _offset: u32,
        _limit: u32,
    ) -> Result<Vec<ProjectSummary>> {
        Ok(Vec::new())
    }

    async fn versions(&self, project: &str) -> Result<Vec<ContentVersion>> {
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

#[derive(serde::Deserialize)]
struct Hits {
    hits: Vec<ProjectSummary>,
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
            .map_err(|e| ContentError::Json(e.to_string()))
    }
}

#[async_trait::async_trait]
impl RusticRegistry for HttpRusticRegistry {
    async fn search(
        &self,
        query: &str,
        kind: Option<ContentKind>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ProjectSummary>> {
        let mut q = vec![
            ("q", query.to_owned()),
            ("offset", offset.to_string()),
            ("limit", limit.clamp(1, 100).to_string()),
        ];
        if let Some(k) = kind {
            q.push(("kind", k.as_str().to_owned()));
        }
        let hits: Hits = self.get("/v1/content/search", &q).await?;
        Ok(hits
            .hits
            .into_iter()
            .map(|mut h| {
                h.source = SourceKind::Rustic;
                h
            })
            .collect())
    }

    async fn versions(&self, project: &str) -> Result<Vec<ContentVersion>> {
        let mut v: Vec<ContentVersion> = self
            .get(&format!("/v1/content/projects/{project}/versions"), &[])
            .await?;
        for x in &mut v {
            x.source = SourceKind::Rustic;
        }
        Ok(v)
    }
}
