//! Server errors.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("No server {0}.")]
    UnknownServer(String),
    #[error(
        "Running a server needs your agreement to the Minecraft EULA (https://aka.ms/MinecraftEULA)."
    )]
    EulaNotAccepted,
    #[error("The server is not running.")]
    NotRunning,
    #[error("horizon-server: {0}")]
    Supervisor(String),
    #[error("The server's control API answered {status}: {message}")]
    Api { status: u16, message: String },
    #[error("Could not reach the server's control API: {0}")]
    ApiUnreachable(String),
    #[error(
        "The rustic-server program was not found. Choose it in Settings > Game and instances > \
         Rustic Server, or install a Rustic edition build that ships it."
    )]
    NoRusticServer,
    #[error("{path} is not a rustic-server program: {why}")]
    NotRusticServer { path: PathBuf, why: String },
    #[error("Plugin: {0}")]
    Plugin(String),
    #[error("Network: {0}")]
    Network(String),
    #[error("DNS: {0}")]
    Dns(String),
    #[error("Porkbun: {0}")]
    Porkbun(String),
    #[error("The purchase was not confirmed: {0}")]
    PurchaseNotConfirmed(String),
    #[error("Relay: {0}")]
    Relay(String),
    #[error("Hosting: {0}")]
    Hosting(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Core(#[from] rusticmc_launcher_core::CoreError),
    #[error(transparent)]
    Instance(#[from] rusticmc_instances::InstanceError),
    #[error(transparent)]
    Auth(#[from] rusticmc_auth::AuthError),
}

impl ServerError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownServer(_) => "server.unknown",
            Self::EulaNotAccepted => "server.eula",
            Self::NotRunning => "server.not_running",
            Self::Supervisor(_) => "server.supervisor",
            Self::Api { .. } => "server.api",
            Self::ApiUnreachable(_) => "server.api_unreachable",
            Self::NoRusticServer => "server.no_rustic_server",
            Self::NotRusticServer { .. } => "server.not_rustic_server",
            Self::Plugin(_) => "plugin.invalid",
            Self::Network(_) => "network",
            Self::Dns(_) => "dns",
            Self::Porkbun(_) => "dns.porkbun",
            Self::PurchaseNotConfirmed(_) => "dns.purchase_not_confirmed",
            Self::Relay(_) => "relay",
            Self::Hosting(_) => "hosting",
            Self::Invalid(_) => "invalid",
            Self::Io { .. } => "io",
            Self::Core(_) => "io",
            Self::Instance(e) => e.code(),
            Self::Auth(e) => e.code(),
        }
    }
}

pub type Result<T, E = ServerError> = std::result::Result<T, E>;

impl From<ServerError> for rusticmc_launcher_core::IpcError {
    fn from(e: ServerError) -> Self {
        use rusticmc_launcher_core::IpcErrorCode as C;
        match e {
            ServerError::Instance(i) => return i.into(),
            ServerError::Auth(a) => return a.into(),
            _ => {}
        }
        let code = match &e {
            ServerError::UnknownServer(_) => C::NotFound,
            ServerError::EulaNotAccepted
            | ServerError::Invalid(_)
            | ServerError::NotRusticServer { .. }
            | ServerError::PurchaseNotConfirmed(_) => C::InvalidArgument,
            ServerError::NotRunning => C::Conflict,
            ServerError::Api { status: 409, .. } => C::Conflict,
            ServerError::Api {
                status: 401 | 403, ..
            } => C::Unauthorized,
            ServerError::Api { status: 404, .. } => C::NotFound,
            ServerError::ApiUnreachable(_) | ServerError::Network(_) | ServerError::Porkbun(_) => {
                C::Network
            }
            ServerError::Hosting(_) | ServerError::NoRusticServer => C::Unavailable,
            _ => C::Internal,
        };
        Self::with_reason(code, e.to_string(), e.code())
    }
}
