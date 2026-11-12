//! Hand-written transport used by the generated methods.

use std::sync::{Arc, RwLock};

use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::types::ErrorBody;

/// Errors returned by [`Client`] methods.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    /// The API answered with a non-2xx status and the standard error envelope.
    #[error("api error {status}: {code}: {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("invalid response body: {0}")]
    Decode(String),
}

impl ClientError {
    /// Stable API error code (`unauthorized`, `rate_limited`...), if any.
    pub fn code(&self) -> Option<&str> {
        match self {
            ClientError::Api { code, .. } => Some(code),
            _ => None,
        }
    }
}

/// Rustic Cloud API client. Cheap to clone (shares the connection pool and the token).
#[derive(Clone)]
pub struct Client {
    base_url: String,
    http: reqwest::Client,
    access_token: Arc<RwLock<Option<String>>>,
}

impl Client {
    /// `base_url` like `https://api.rustic.example` (no trailing `/v1`).
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("rusticmc-api-client/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self::with_http(base_url, http)
    }

    pub fn with_http(base_url: impl Into<String>, http: reqwest::Client) -> Self {
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self {
            base_url,
            http,
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Sets (or clears) the bearer access token used by every call.
    pub fn set_access_token(&self, token: Option<String>) {
        if let Ok(mut t) = self.access_token.write() {
            *t = token;
        }
    }

    pub fn access_token(&self) -> Option<String> {
        self.access_token.read().ok().and_then(|t| t.clone())
    }

    pub(crate) async fn call<B: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<R, ClientError> {
        let resp = self.send(method, path, query, body).await?;
        let bytes = resp.bytes().await?;
        serde_json::from_slice(&bytes).map_err(|e| ClientError::Decode(e.to_string()))
    }

    pub(crate) async fn call_empty<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<(), ClientError> {
        self.send(method, path, query, body).await.map(|_| ())
    }

    async fn send<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<reqwest::Response, ClientError> {
        let mut req = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(token) = self.access_token() {
            req = req.bearer_auth(token);
        }
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let bytes = resp.bytes().await.unwrap_or_default();
        Err(match serde_json::from_slice::<ErrorBody>(&bytes) {
            Ok(e) => ClientError::Api {
                status: status.as_u16(),
                code: e.error.code,
                message: e.error.message,
            },
            Err(_) => ClientError::Api {
                status: status.as_u16(),
                code: default_code(status).to_owned(),
                message: String::from_utf8_lossy(&bytes).chars().take(200).collect(),
            },
        })
    }
}

fn default_code(status: StatusCode) -> &'static str {
    match status.as_u16() {
        401 => "unauthorized",
        403 => "forbidden",
        404 => "not_found",
        429 => "rate_limited",
        _ => "http_error",
    }
}

/// Percent-encodes one path segment.
pub(crate) fn seg(s: impl std::fmt::Display) -> String {
    let s = s.to_string();
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_encoding() {
        assert_eq!(seg("a b/c"), "a%20b%2Fc");
        assert_eq!(seg("play.example.com"), "play.example.com");
    }

    #[test]
    fn trailing_slash_trimmed() {
        let c = Client::new("http://localhost:8080//");
        assert_eq!(c.base_url(), "http://localhost:8080");
        c.set_access_token(Some("t".into()));
        assert_eq!(c.access_token().as_deref(), Some("t"));
    }
}
