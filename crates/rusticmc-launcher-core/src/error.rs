//! The shared error type of the core crate.

use std::path::PathBuf;

/// Errors from paths, configuration and file helpers.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("no home folder could be found for this user")]
    NoHomeDir,
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: invalid configuration: {message}")]
    ConfigInvalid { path: PathBuf, message: String },
    #[error(
        "{path}: written by a newer launcher (config version {found}, this one reads up to {supported})"
    )]
    ConfigTooNew {
        path: PathBuf,
        found: u32,
        supported: u32,
    },
    #[error("logging: {0}")]
    Logging(String),
    #[error("http client: {0}")]
    Http(String),
}

impl CoreError {
    /// An I/O error on `path`.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T, E = CoreError> = std::result::Result<T, E>;
