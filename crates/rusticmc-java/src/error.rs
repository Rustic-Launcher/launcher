//! Errors of the Java Edition launcher.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum JavaError {
    #[error("No Java Edition version {0}.")]
    UnknownVersion(String),
    #[error("No Java profile {0}.")]
    UnknownProfile(String),
    #[error("The version file is not usable: {0}")]
    BadVersion(String),
    #[error("{0}")]
    Network(String),
    #[error("{url}: the server answered HTTP {status}")]
    Http { url: String, status: u16 },
    #[error("Refused to download from {0}: not an official host.")]
    HostNotAllowed(String),
    #[error("No Java runtime {component} for this system ({platform}).")]
    NoRuntime { component: String, platform: String },
    #[error("The Java executable {0} does not exist.")]
    NoJavaExecutable(PathBuf),
    #[error("{loader} has no version for Minecraft {game}.")]
    NoLoaderVersion { loader: String, game: String },
    #[error("The {loader} installer failed: {message}")]
    Installer { loader: String, message: String },
    #[error("A library is missing: {0}. Install the profile again.")]
    MissingFile(PathBuf),
    #[error("{0}")]
    Process(String),
    #[error("The profile is already running.")]
    AlreadyRunning,
    #[error("The profile is not running.")]
    NotRunning,
    #[error("{0}")]
    Invalid(String),
    #[error("Bedrock Edition: {0}")]
    Bedrock(String),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("download cancelled")]
    Cancelled,
    #[error(transparent)]
    Download(#[from] rusticmc_launcher_core::DownloadError),
    #[error(transparent)]
    Core(#[from] rusticmc_launcher_core::CoreError),
}

impl JavaError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn bad(message: impl Into<String>) -> Self {
        Self::BadVersion(message.into())
    }

    /// A stable code for the UI (`details.reason`).
    pub fn reason(&self) -> &'static str {
        match self {
            Self::UnknownVersion(_) => "java.unknown_version",
            Self::UnknownProfile(_) => "java.unknown_profile",
            Self::BadVersion(_) => "java.bad_version",
            Self::Network(_) | Self::Http { .. } => "java.network",
            Self::HostNotAllowed(_) => "java.host_not_allowed",
            Self::NoRuntime { .. } => "java.no_runtime",
            Self::NoJavaExecutable(_) => "java.no_java",
            Self::NoLoaderVersion { .. } => "java.no_loader_version",
            Self::Installer { .. } => "java.installer",
            Self::MissingFile(_) => "java.missing_file",
            Self::Process(_) => "java.process",
            Self::AlreadyRunning => "java.already_running",
            Self::NotRunning => "java.not_running",
            Self::Invalid(_) => "java.invalid",
            Self::Bedrock(_) => "bedrock.failed",
            Self::Io { .. } | Self::Core(_) => "java.io",
            Self::Cancelled => "java.cancelled",
            Self::Download(rusticmc_launcher_core::DownloadError::Cancelled) => "java.cancelled",
            Self::Download(_) => "java.download",
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Download(rusticmc_launcher_core::DownloadError::Cancelled)
        )
    }
}

impl From<reqwest_error::Error> for JavaError {
    fn from(e: reqwest_error::Error) -> Self {
        Self::Network(e.to_string())
    }
}

/// `reqwest` as re-exported by the core crate.
mod reqwest_error {
    pub use rusticmc_launcher_core::reqwest::Error;
}

pub type Result<T, E = JavaError> = std::result::Result<T, E>;
