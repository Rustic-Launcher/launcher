//! The HTTP client every launcher crate uses: reqwest over rustls with the `ring` crypto provider
//! (ADR 0041: no aws-lc build tooling on the dev box), system root certificates through
//! `rustls-platform-verifier`, and the launcher's `User-Agent`.

use crate::error::{CoreError, Result};
use std::time::Duration;

/// Installs `ring` as the process's rustls provider (once; later calls do nothing).
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// A client with the launcher's defaults: 15 s to connect, no overall timeout (downloads can be
/// long; callers that want one set it per request).
pub fn client() -> Result<reqwest::Client> {
    builder()
        .build()
        .map_err(|e| CoreError::Http(e.to_string()))
}

/// The builder behind [`client`], for callers that add options.
pub fn builder() -> reqwest::ClientBuilder {
    install_crypto_provider();
    reqwest::Client::builder()
        .user_agent(crate::user_agent())
        .connect_timeout(Duration::from_secs(15))
}
