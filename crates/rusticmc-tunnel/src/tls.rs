//! TLS for the tunnel connection (rustls with the `ring` provider, chosen explicitly so the
//! process-wide default provider does not matter).
use rustls::{ClientConfig, RootCertStore, pki_types::CertificateDer};
use std::sync::Arc;

/// Which certificate authorities the client trusts for the relay.
#[derive(Clone, Debug, Default)]
pub enum TlsRoots {
    /// The operating system's store (`rustls-native-certs`): the production setting.
    #[default]
    Native,
    /// These certificates only (a self-hosted relay with its own CA, tests).
    Certs(Vec<CertificateDer<'static>>),
}

#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("no usable root certificate")]
    NoRoots,
    #[error("TLS setup: {0}")]
    Rustls(#[from] rustls::Error),
}

/// The client configuration: TLS 1.2 and 1.3, ALPN `rustic-relay/1`.
pub fn client_config(roots: &TlsRoots) -> Result<Arc<ClientConfig>, TlsError> {
    let mut store = RootCertStore::empty();
    match roots {
        TlsRoots::Native => {
            let found = rustls_native_certs::load_native_certs();
            for e in &found.errors {
                tracing::debug!(error = %e, "skipping an OS root certificate");
            }
            let (added, _) = store.add_parsable_certificates(found.certs);
            if added == 0 {
                return Err(TlsError::NoRoots);
            }
        }
        TlsRoots::Certs(certs) => {
            let (added, _) = store.add_parsable_certificates(certs.iter().cloned());
            if added == 0 {
                return Err(TlsError::NoRoots);
            }
        }
    }
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(store)
            .with_no_client_auth();
    config.alpn_protocols = vec![crate::wire::ALPN.to_vec()];
    Ok(Arc::new(config))
}
