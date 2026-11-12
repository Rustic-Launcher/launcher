//! Instance, build, preparation and launch errors.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error("No instance {0}.")]
    UnknownInstance(String),
    #[error("Build {0} is not installed.")]
    BuildNotInstalled(String),
    #[error("No build source {0}.")]
    UnknownSource(String),
    #[error("{0} has no build for this computer ({1}).")]
    NoAssetForPlatform(String, String),
    #[error("{0}")]
    Signature(String),
    #[error("The build archive is not usable: {0}")]
    BadArchive(String),
    #[error("{program} answered something unexpected to --launch-api: {message}")]
    LaunchApi { program: String, message: String },
    #[error("This build speaks Launch API {found}; this launcher speaks 0.1.x.")]
    LaunchApiVersion { found: String },
    #[error("This build does not accept {0}.")]
    OptionNotSupported(String),
    #[error("Preparing the game files failed: {0}")]
    Prepare(String),
    #[error("The game files for this build are not prepared yet.")]
    NotPrepared,
    #[error(
        "No Java {0} found. Install one (for example Eclipse Temurin {0}) or choose its folder in Settings."
    )]
    NoJava(u32),
    #[error("Could not find your Minecraft game folder; choose it in Settings.")]
    NoMinecraftDir,
    #[error("{0}")]
    Process(String),
    #[error("The instance is already running.")]
    AlreadyRunning,
    #[error("The instance is not running.")]
    NotRunning,
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Http(String),
    #[error(transparent)]
    Download(#[from] rusticmc_launcher_core::DownloadError),
    #[error(transparent)]
    Core(#[from] rusticmc_launcher_core::CoreError),
    #[error(transparent)]
    Auth(#[from] rusticmc_auth::AuthError),
}

impl InstanceError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// A stable machine-readable code (IPC errors carry it).
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownInstance(_) => "instance.unknown",
            Self::BuildNotInstalled(_) => "build.not_installed",
            Self::UnknownSource(_) => "build.unknown_source",
            Self::NoAssetForPlatform(..) => "build.no_asset",
            Self::Signature(_) => "build.signature",
            Self::BadArchive(_) => "build.bad_archive",
            Self::LaunchApi { .. } => "launch.api",
            Self::LaunchApiVersion { .. } => "launch.api_version",
            Self::OptionNotSupported(_) => "launch.option_not_supported",
            Self::Prepare(_) => "prepare.failed",
            Self::NotPrepared => "prepare.needed",
            Self::NoJava(_) => "prepare.no_java",
            Self::NoMinecraftDir => "prepare.no_minecraft_dir",
            Self::Process(_) => "launch.process",
            Self::AlreadyRunning => "launch.already_running",
            Self::NotRunning => "launch.not_running",
            Self::Io { .. } => "io",
            Self::Http(_) => "network",
            Self::Download(_) => "download",
            Self::Core(_) => "io",
            Self::Auth(e) => e.code(),
        }
    }
}

pub type Result<T, E = InstanceError> = std::result::Result<T, E>;

impl From<InstanceError> for rusticmc_launcher_core::IpcError {
    fn from(e: InstanceError) -> Self {
        use rusticmc_launcher_core::IpcErrorCode as C;
        if let InstanceError::Auth(a) = e {
            return a.into();
        }
        let code = match &e {
            InstanceError::UnknownInstance(_)
            | InstanceError::BuildNotInstalled(_)
            | InstanceError::UnknownSource(_) => C::NotFound,
            InstanceError::AlreadyRunning | InstanceError::NotRunning => C::Conflict,
            InstanceError::Http(_) => C::Network,
            InstanceError::Download(rusticmc_launcher_core::DownloadError::Cancelled) => {
                C::Cancelled
            }
            InstanceError::Download(_) => C::Network,
            InstanceError::NoAssetForPlatform(..)
            | InstanceError::NotPrepared
            | InstanceError::NoJava(_) => C::Unavailable,
            _ => C::Internal,
        };
        Self::with_reason(code, e.to_string(), e.code())
    }
}
