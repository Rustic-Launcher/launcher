//! Where the sign-in talks to, and how fast it polls. Every URL is configurable so tests run the
//! whole chain against a local mock (as Horizon's launcher does, docs/microsoft-account.md).

use std::time::Duration;

/// The services of the token chain (horizon/docs/microsoft-account.md, "Flow").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoints {
    /// `https://login.microsoftonline.com/consumers/oauth2/v2.0` (`/devicecode`, `/token`,
    /// `/authorize`).
    pub microsoft: String,
    /// `https://user.auth.xboxlive.com/user/authenticate`.
    pub xbox_user: String,
    /// `https://xsts.auth.xboxlive.com/xsts/authorize`.
    pub xsts: String,
    /// `https://api.minecraftservices.com`.
    pub minecraft: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            microsoft: "https://login.microsoftonline.com/consumers/oauth2/v2.0".into(),
            xbox_user: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            minecraft: "https://api.minecraftservices.com".into(),
        }
    }
}

impl Endpoints {
    /// Every service under one base URL (tests): `<base>/ms`, `<base>/xbl`, `<base>/xsts`,
    /// `<base>/mc`.
    pub fn under(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Self {
            microsoft: format!("{base}/ms"),
            xbox_user: format!("{base}/xbl"),
            xsts: format!("{base}/xsts"),
            minecraft: format!("{base}/mc"),
        }
    }
}

/// How the device-code flow polls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollTiming {
    /// The shortest wait between polls, whatever the server says.
    pub min_interval: Duration,
    /// Added on `slow_down` (RFC 8628 says 5 s).
    pub slow_down_step: Duration,
}

impl Default for PollTiming {
    fn default() -> Self {
        Self {
            min_interval: Duration::from_secs(1),
            slow_down_step: Duration::from_secs(5),
        }
    }
}

/// Everything the sign-in needs.
#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// The Azure application (client) id; `None`: sign-in unavailable.
    pub client_id: Option<String>,
    pub endpoints: Endpoints,
    /// `XboxLive.signin offline_access`: `offline_access` gives the refresh token.
    pub scope: String,
    pub poll: PollTiming,
    /// Offline profiles stay usable this long after the last successful ownership check.
    pub revalidation_window: Duration,
    /// The Minecraft token is renewed when it has less than this left (Horizon: one hour).
    pub refresh_margin: Duration,
    /// The keychain service name.
    pub keychain_service: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            client_id: None,
            endpoints: Endpoints::default(),
            scope: "XboxLive.signin offline_access".into(),
            poll: PollTiming::default(),
            revalidation_window: Duration::from_secs(30 * 86_400),
            refresh_margin: Duration::from_secs(3600),
            keychain_service: rusticmc_launcher_core::APP_ID.into(),
        }
    }
}

impl AuthConfig {
    /// From the launcher's configuration.
    pub fn from_launcher(config: &rusticmc_launcher_core::LauncherConfig) -> Self {
        Self {
            client_id: config.microsoft.effective_client_id(),
            revalidation_window: Duration::from_secs(config.ownership.window_seconds()),
            ..Self::default()
        }
    }
}
