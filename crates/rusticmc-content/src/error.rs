//! The crate's error type.

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("{url}: HTTP {status}: {message}")]
    Http {
        url: String,
        status: u16,
        message: String,
    },
    #[error("network: {0}")]
    Network(String),
    #[error("rate limited by {0}: gave up after retrying")]
    RateLimited(String),
    #[error("unexpected answer: {0}")]
    Json(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("download: {0}")]
    Download(String),
    /// A check failed: the file is not installed.
    #[error("refused: {0}")]
    Refused(String),
    #[error("not compatible: {0}")]
    Incompatible(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("archive: {0}")]
    Archive(String),
    #[error("plugin: {0}")]
    Plugin(String),
    #[error("dependencies: {0}")]
    Dependency(String),
    #[error("this source is not configured: {0}")]
    NotConfigured(String),
}

pub type Result<T> = std::result::Result<T, ContentError>;

impl From<zip::result::ZipError> for ContentError {
    fn from(e: zip::result::ZipError) -> Self {
        ContentError::Archive(e.to_string())
    }
}

impl From<serde_json::Error> for ContentError {
    fn from(e: serde_json::Error) -> Self {
        ContentError::Json(e.to_string())
    }
}
