//! The Rustic Cloud seam of the desktop app: [`ApiCloud`] implements `rusticmc_auth::CloudLink`
//! over the cloud's HTTP API (`services/api`, docs/API.md), with the launcher's own reqwest client
//! and the wire types of `rusticmc-api-client` (types only: no second HTTP stack). Also hosts the
//! account calls that are not part of `CloudLink` (sign-up, resending the verification mail).
//!
//! Base URL: `[cloud] api_base` in `launcher.toml` (default `https://api.rustic-launcher.com`), overridable
//! with `RUSTIC_CLOUD_API` for local QA against `services/api` (`http://localhost:8080`).

use async_trait::async_trait;
use rusticmc_api_client::types::{
    ClientKind, EmailRequest, ErrorBody, HandoffExchangeRequest, LoginRequest, LoginResponse,
    LoginStatus, Me, MfaLoginRequest, RefreshRequest, SignupRequest, SignupResponse, TokenPair,
};
use rusticmc_auth::cloud::{CloudLink, CloudLogin, CloudTokens, CloudUser, LinkedMinecraft};
use rusticmc_auth::error::{AuthError, Result};
use rusticmc_launcher_core::{Secret, reqwest};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Environment override of the cloud API base (local QA).
pub const CLOUD_API_ENV: &str = "RUSTIC_CLOUD_API";

/// Privacy policy version the launcher shows and the user accepts at sign-up
/// (`PRIVACY_POLICY_VERSION` of the cloud; docs/LEGAL.md).
pub const PRIVACY_POLICY_VERSION: &str = "2026-11-01";

/// Where the browser login comes back to (registered in the cloud's `OAUTH_CLIENT_REDIRECTS`).
/// The website (apps/web): "Continue with Microsoft" runs there and hands back
/// `rustic://auth?code=` (ADR 0090). `RUSTIC_WEB_URL` overrides; otherwise derived from the API.
pub const WEB_URL_ENV: &str = "RUSTIC_WEB_URL";

pub fn web_base(api: &str) -> String {
    if let Ok(v) = std::env::var(WEB_URL_ENV)
        && !v.trim().is_empty()
    {
        return v.trim_end_matches('/').to_owned();
    }
    if api.contains("localhost:8080") || api.contains("127.0.0.1:8080") {
        return "http://localhost:5173".into();
    }
    match api.strip_prefix("https://api.") {
        Some(rest) => format!("https://{}", rest.trim_end_matches('/')),
        None => "https://rustic-launcher.com".into(),
    }
}

pub fn api_base(configured: &str) -> String {
    std::env::var(CLOUD_API_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| configured.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

#[derive(Clone)]
pub struct ApiCloud {
    base: String,
    http: reqwest::Client,
}

/// A cloud error as the UI shows it: the API's own message when it sent one.
fn cloud_err(status: Option<u16>, code: &str, message: &str) -> AuthError {
    let text = if message.is_empty() {
        code.to_owned()
    } else {
        message.to_owned()
    };
    match (status, code) {
        (Some(429), _) => AuthError::RateLimited,
        // A dead session (revoked, expired refresh token): the user must sign in again. Wrong
        // passwords and expired MFA steps have their own codes and keep their message.
        (Some(401), "unauthorized") => AuthError::ReauthRequired,
        _ => AuthError::Cloud(text),
    }
}

impl ApiCloud {
    pub fn new(base: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            http,
        }
    }

    async fn send<B: Serialize + ?Sized>(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<&Secret>,
        query: &[(&str, &str)],
        body: Option<&B>,
    ) -> Result<reqwest::Response> {
        let mut req = self.http.request(method, format!("{}{}", self.base, path));
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(t) = token {
            req = req.bearer_auth(t.expose());
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| AuthError::Network {
            service: "Rustic Cloud",
            message: e.without_url().to_string(),
        })?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let bytes = resp.bytes().await.unwrap_or_default();
        Err(match serde_json::from_slice::<ErrorBody>(&bytes) {
            Ok(e) => cloud_err(Some(status.as_u16()), &e.error.code, &e.error.message),
            Err(_) => cloud_err(Some(status.as_u16()), status.as_str(), ""),
        })
    }

    async fn json<B: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<&Secret>,
        query: &[(&str, &str)],
        body: Option<&B>,
    ) -> Result<R> {
        let resp = self.send(method, path, token, query, body).await?;
        resp.json::<R>()
            .await
            .map_err(|e| AuthError::Unexpected(format!("Rustic Cloud answer: {e}")))
    }

    /// Creates a Rustic account (the verification mail is sent by the cloud).
    pub async fn signup(&self, req: &SignupRequest) -> Result<SignupResponse> {
        self.json(
            reqwest::Method::POST,
            "/v1/auth/signup",
            None,
            &[],
            Some(req),
        )
        .await
    }

    pub async fn resend_verification(&self, email: &str) -> Result<()> {
        let body = EmailRequest {
            email: email.to_owned(),
        };
        self.send(
            reqwest::Method::POST,
            "/v1/auth/verify-email/resend",
            None,
            &[],
            Some(&body),
        )
        .await
        .map(|_| ())
    }
}

pub fn tokens(pair: TokenPair) -> CloudTokens {
    CloudTokens {
        access_token: Secret::new(pair.access_token),
        access_expires_at: u64::try_from(pair.access_expires_at.timestamp()).unwrap_or(0),
        refresh_token: pair.refresh_token.map(Secret::new),
        session_id: pair.session_id.to_string(),
    }
}

fn login_answer(resp: LoginResponse) -> Result<CloudLogin> {
    match resp.status {
        LoginStatus::Ok => resp
            .tokens
            .map(|t| CloudLogin::Done(tokens(t)))
            .ok_or_else(|| AuthError::Unexpected("login answered without tokens".into())),
        LoginStatus::MfaRequired => resp
            .mfa_token
            .map(|m| CloudLogin::MfaRequired {
                mfa_token: Secret::new(m),
            })
            .ok_or_else(|| AuthError::Unexpected("MFA required without a token".into())),
        LoginStatus::SignupRequired => Err(AuthError::Cloud(
            "finish creating your account on the website first".into(),
        )),
    }
}

#[async_trait]
impl CloudLink for ApiCloud {
    async fn login(&self, email: &str, password: &Secret, device: &str) -> Result<CloudLogin> {
        let body = LoginRequest {
            email: email.to_owned(),
            password: password.expose().to_owned(),
            device_name: Some(device.to_owned()),
            client: ClientKind::Launcher,
        };
        let resp: LoginResponse = self
            .json(
                reqwest::Method::POST,
                "/v1/auth/login",
                None,
                &[],
                Some(&body),
            )
            .await?;
        login_answer(resp)
    }

    async fn login_mfa(&self, mfa_token: &Secret, code: &str, device: &str) -> Result<CloudTokens> {
        let body = MfaLoginRequest {
            mfa_token: mfa_token.expose().to_owned(),
            code: Some(code.to_owned()),
            recovery_code: None,
            device_name: Some(device.to_owned()),
            client: ClientKind::Launcher,
        };
        let resp: LoginResponse = self
            .json(
                reqwest::Method::POST,
                "/v1/auth/login/mfa",
                None,
                &[],
                Some(&body),
            )
            .await?;
        match login_answer(resp)? {
            CloudLogin::Done(t) => Ok(t),
            CloudLogin::MfaRequired { .. } => Err(AuthError::Cloud("MFA still required".into())),
        }
    }

    /// "Continue with Microsoft" happens on the website (OIDC + PKCE there); the launcher marker
    /// makes it hand a single-use code back through the `rustic://auth` deep link.
    async fn oauth_start(&self, _provider: &str) -> Result<String> {
        Ok(format!("{}/login?launcher=1", web_base(&self.base)))
    }

    async fn oauth_exchange(&self, ticket: &str, device: &str) -> Result<CloudTokens> {
        let body = HandoffExchangeRequest {
            code: ticket.to_owned(),
            device_name: Some(device.to_owned()),
            client: ClientKind::Launcher,
        };
        let resp: LoginResponse = self
            .json(
                reqwest::Method::POST,
                "/v1/auth/handoff/exchange",
                None,
                &[],
                Some(&body),
            )
            .await?;
        match login_answer(resp)? {
            CloudLogin::Done(t) => Ok(t),
            CloudLogin::MfaRequired { .. } => Err(AuthError::Cloud(
                "This account uses two-factor authentication: sign in with email and password."
                    .into(),
            )),
        }
    }

    async fn refresh(&self, refresh_token: &Secret) -> Result<CloudTokens> {
        let body = RefreshRequest {
            refresh_token: Some(refresh_token.expose().to_owned()),
            client: ClientKind::Launcher,
        };
        let pair: TokenPair = self
            .json(
                reqwest::Method::POST,
                "/v1/auth/refresh",
                None,
                &[],
                Some(&body),
            )
            .await?;
        Ok(tokens(pair))
    }

    async fn logout(&self, access_token: &Secret) -> Result<()> {
        self.send::<()>(
            reqwest::Method::POST,
            "/v1/auth/logout",
            Some(access_token),
            &[],
            None,
        )
        .await
        .map(|_| ())
    }

    async fn me(&self, access_token: &Secret) -> Result<CloudUser> {
        let me: Me = self
            .json::<(), _>(
                reqwest::Method::GET,
                "/v1/me",
                Some(access_token),
                &[],
                None,
            )
            .await?;
        Ok(CloudUser {
            id: me.id.to_string(),
            display_name: me.display_name,
        })
    }

    async fn link_minecraft(&self, _: &Secret, _: &Secret) -> Result<LinkedMinecraft> {
        // The cloud links through a join/hasJoined challenge (`/v1/minecraft/link/start|finish`),
        // which needs a signed-in Microsoft game session: gated until the Azure app exists.
        Err(AuthError::Cloud(
            "Linking a Minecraft account needs Microsoft sign-in (not configured in this build)."
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusticmc_api_client::types::TokenPair;

    #[test]
    fn env_overrides_the_configured_base() {
        // Not set in the test environment: the configured value wins, trailing slash dropped.
        if std::env::var(CLOUD_API_ENV).is_err() {
            assert_eq!(api_base("https://api.example/"), "https://api.example");
        }
    }

    #[test]
    fn maps_token_pairs() {
        let pair: TokenPair = serde_json::from_value(serde_json::json!({
            "token_type": "Bearer",
            "access_token": "a",
            "access_expires_at": "2030-01-01T00:00:00Z",
            "refresh_token": "r",
            "session_id": "7d3f9b2e-41c2-4f7a-9a51-2b8f0c6e1d44"
        }))
        .unwrap();
        let t = tokens(pair);
        assert_eq!(t.access_token.expose(), "a");
        assert_eq!(t.access_expires_at, 1_893_456_000);
        assert_eq!(t.refresh_token.unwrap().expose(), "r");
    }

    #[test]
    fn maps_login_answers() {
        let mfa: LoginResponse = serde_json::from_value(serde_json::json!({
            "status": "mfa_required", "mfa_token": "m"
        }))
        .unwrap();
        assert!(matches!(
            login_answer(mfa).unwrap(),
            CloudLogin::MfaRequired { .. }
        ));
        let bad: LoginResponse =
            serde_json::from_value(serde_json::json!({ "status": "ok" })).unwrap();
        assert!(login_answer(bad).is_err());
    }

    #[test]
    fn rate_limit_is_typed() {
        assert!(matches!(
            cloud_err(Some(429), "x", ""),
            AuthError::RateLimited
        ));
        assert!(
            matches!(cloud_err(Some(401), "invalid_credentials", "Wrong email"), AuthError::Cloud(m) if m == "Wrong email")
        );
        // Regression (QA): a revoked session failed every call instead of asking to sign in.
        assert!(matches!(
            cloud_err(Some(401), "unauthorized", "authentication required"),
            AuthError::ReauthRequired
        ));
    }
}

#[cfg(test)]
mod web_tests {
    use super::web_base;

    #[test]
    fn website_follows_the_api() {
        assert_eq!(
            web_base("https://api.rustic-launcher.com"),
            "https://rustic-launcher.com"
        );
        assert_eq!(web_base("http://localhost:8080"), "http://localhost:5173");
        assert_eq!(
            web_base("https://cloud.example.invalid"),
            "https://rustic-launcher.com"
        );
    }
}
