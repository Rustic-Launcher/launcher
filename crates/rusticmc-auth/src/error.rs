//! What can go wrong while signing in. Messages are written for the player and never contain a
//! token.

/// An account or sign-in error.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(
        "This build of the launcher has no Microsoft sign-in configured (no Azure client id). See TODO_HUMAN.md."
    )]
    NotConfigured,
    #[error("Could not reach {service}: {message}")]
    Network {
        service: &'static str,
        message: String,
    },
    #[error("Microsoft refused the sign-in: {0}")]
    Microsoft(String),
    #[error("The sign-in was declined in the browser.")]
    Declined,
    #[error("The sign-in code expired. Start again.")]
    Expired,
    #[error("The sign-in was cancelled.")]
    Cancelled,
    #[error("The sign-in took too long. Start again.")]
    Timeout,
    #[error("{0}")]
    Xbox(String),
    #[error("The Minecraft services refused this launcher: its app is not approved by Mojang yet.")]
    NotApproved,
    #[error("Too many sign-ins in a short time. Wait a minute and try again.")]
    RateLimited,
    #[error("The Minecraft services refused the sign-in.")]
    MinecraftRefused,
    #[error("This account doesn't own Minecraft: Java Edition.")]
    NotOwned,
    #[error(
        "This account owns the game but has no Minecraft profile yet: create one on minecraft.net first."
    )]
    NoProfile,
    #[error("This account needs to sign in again.")]
    ReauthRequired,
    #[error("You signed in as {got}, but this sign-in was for {expected}.")]
    WrongAccount { expected: String, got: String },
    #[error("No account {0} in this launcher.")]
    UnknownAccount(String),
    #[error(
        "Windows Credential Manager is full: remove entries you no longer use (Credential Manager > Windows Credentials) and sign in again."
    )]
    KeychainFull,
    #[error("The OS keychain refused the request: {0}")]
    Keychain(String),
    #[error("{0}")]
    OfflineNotAllowed(String),
    #[error("Rustic account: {0}")]
    Cloud(String),
    #[error("No sign-in in progress with id {0}.")]
    UnknownFlow(String),
    #[error("{0}")]
    Unexpected(String),
    #[error(transparent)]
    Core(#[from] rusticmc_launcher_core::CoreError),
}

impl AuthError {
    /// A stable machine-readable code (IPC errors carry it).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured => "auth.not_configured",
            Self::Network { .. } => "auth.network",
            Self::Microsoft(_) => "auth.microsoft",
            Self::Declined => "auth.declined",
            Self::Expired => "auth.expired",
            Self::Cancelled => "auth.cancelled",
            Self::Timeout => "auth.timeout",
            Self::Xbox(_) => "auth.xbox",
            Self::NotApproved => "auth.not_approved",
            Self::RateLimited => "auth.rate_limited",
            Self::MinecraftRefused => "auth.minecraft_refused",
            Self::NotOwned => "auth.not_owned",
            Self::NoProfile => "auth.no_profile",
            Self::ReauthRequired => "auth.reauth_required",
            Self::WrongAccount { .. } => "auth.wrong_account",
            Self::UnknownAccount(_) => "auth.unknown_account",
            Self::KeychainFull => "auth.keychain_full",
            Self::Keychain(_) => "auth.keychain",
            Self::OfflineNotAllowed(_) => "auth.offline_not_allowed",
            Self::Cloud(_) => "auth.cloud",
            Self::UnknownFlow(_) => "auth.unknown_flow",
            Self::Unexpected(_) => "auth.unexpected",
            Self::Core(_) => "auth.io",
        }
    }

    /// Whether the failure is the network's (the last ownership answer may then stand).
    pub fn is_network(&self) -> bool {
        matches!(self, Self::Network { .. })
    }

    pub(crate) fn network(service: &'static str, e: impl std::fmt::Display) -> Self {
        Self::Network {
            service,
            message: e.to_string(),
        }
    }
}

pub type Result<T, E = AuthError> = std::result::Result<T, E>;

impl From<AuthError> for rusticmc_launcher_core::IpcError {
    fn from(e: AuthError) -> Self {
        use rusticmc_launcher_core::IpcErrorCode as C;
        let code = match &e {
            AuthError::NotConfigured => C::Unavailable,
            AuthError::Network { .. } | AuthError::RateLimited => C::Network,
            AuthError::Cancelled => C::Cancelled,
            AuthError::NotOwned | AuthError::NoProfile | AuthError::OfflineNotAllowed(_) => {
                C::OwnershipRequired
            }
            AuthError::ReauthRequired | AuthError::Declined | AuthError::Expired => C::Unauthorized,
            AuthError::WrongAccount { .. } => C::Conflict,
            AuthError::UnknownAccount(_) | AuthError::UnknownFlow(_) => C::NotFound,
            AuthError::Cloud(_) => C::Unavailable,
            _ => C::Internal,
        };
        Self::with_reason(code, e.to_string(), e.code())
    }
}
