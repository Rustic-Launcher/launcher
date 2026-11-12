//! The token chain, one HTTP call per step (horizon/docs/microsoft-account.md, "Flow"):
//! Microsoft OAuth (device code, or authorization code with PKCE) → Xbox Live → XSTS → Minecraft
//! services (`launcher/login`, falling back to `authentication/login_with_xbox`, as Horizon's
//! launcher does) → entitlements → profile. Requests are logged by endpoint name only.

use crate::config::AuthConfig;
use crate::error::{AuthError, Result};
use rusticmc_launcher_core::Secret;
use rusticmc_launcher_core::reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Microsoft's tokens.
#[derive(Clone, Debug)]
pub struct MicrosoftTokens {
    pub access_token: Secret,
    pub refresh_token: Secret,
    pub expires_in: u64,
}

/// What the device-code flow shows the player.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    /// Seconds until the code expires.
    pub expires_in: u64,
    /// Seconds between polls.
    pub interval: u64,
    /// Microsoft's own sentence ("To sign in, use a web browser to open...").
    pub message: Option<String>,
    #[serde(skip)]
    pub(crate) device_code: Secret,
}

/// One answer to a device-code poll.
#[derive(Debug)]
pub enum PollAnswer {
    Pending,
    SlowDown,
    Done(MicrosoftTokens),
}

/// The Xbox Live part: what Minecraft's login needs.
#[derive(Clone, Debug)]
pub struct XboxTokens {
    pub xsts_token: Secret,
    pub user_hash: String,
    pub xuid: Option<String>,
}

/// The Minecraft access token.
#[derive(Clone, Debug)]
pub struct MinecraftToken {
    pub access_token: Secret,
    pub expires_in: u64,
}

/// A skin or a cape of a profile.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Texture {
    pub id: String,
    pub url: String,
    pub state: String,
    /// Skins: `CLASSIC` or `SLIM`.
    #[serde(default)]
    pub variant: Option<String>,
    /// Capes: the cape's name.
    #[serde(default)]
    pub alias: Option<String>,
}

/// The public Minecraft profile.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    /// Undashed UUID.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Texture>,
    #[serde(default)]
    pub capes: Vec<Texture>,
}

/// The HTTP side of the sign-in.
#[derive(Clone)]
pub struct AuthApi {
    http: reqwest::Client,
    config: AuthConfig,
}

const MICROSOFT: &str = "Microsoft";
const XBOX: &str = "Xbox Live";
const MINECRAFT: &str = "the Minecraft services";

impl AuthApi {
    pub fn new(http: reqwest::Client, config: AuthConfig) -> Self {
        Self { http, config }
    }

    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    fn client_id(&self) -> Result<&str> {
        self.config
            .client_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .ok_or(AuthError::NotConfigured)
    }

    async fn form(&self, url: &str, form: &[(&str, &str)]) -> Result<(u16, Value)> {
        tracing::debug!(endpoint = %endpoint_name(url), "POST (form)");
        let response = self
            .http
            .post(url)
            .header("Accept", "application/json")
            .form(form)
            .send()
            .await
            .map_err(|e| AuthError::network(MICROSOFT, e))?;
        let status = response.status().as_u16();
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    async fn post_json(
        &self,
        url: &str,
        body: &Value,
        service: &'static str,
    ) -> Result<(u16, Value)> {
        tracing::debug!(endpoint = %endpoint_name(url), "POST");
        let response = self
            .http
            .post(url)
            .header("Accept", "application/json")
            .header("x-xbl-contract-version", "1")
            .json(body)
            .send()
            .await
            .map_err(|e| AuthError::network(service, e))?;
        let status = response.status().as_u16();
        Ok((
            status,
            response.json::<Value>().await.unwrap_or(Value::Null),
        ))
    }

    async fn get_bearer(&self, url: &str, token: &Secret) -> Result<(u16, Value)> {
        tracing::debug!(endpoint = %endpoint_name(url), "GET");
        let response = self
            .http
            .get(url)
            .header("Accept", "application/json")
            .bearer_auth(token.expose())
            .send()
            .await
            .map_err(|e| AuthError::network(MINECRAFT, e))?;
        let status = response.status().as_u16();
        Ok((
            status,
            response.json::<Value>().await.unwrap_or(Value::Null),
        ))
    }

    // ---- Microsoft --------------------------------------------------------------------------

    /// Starts the device-code flow.
    pub async fn device_code(&self) -> Result<DeviceCode> {
        let client_id = self.client_id()?;
        let url = format!("{}/devicecode", self.config.endpoints.microsoft);
        let (status, v) = self
            .form(
                &url,
                &[("client_id", client_id), ("scope", &self.config.scope)],
            )
            .await?;
        if status != 200 {
            return Err(oauth_error(&v));
        }
        let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
        Ok(DeviceCode {
            user_code: text("user_code").ok_or_else(|| unexpected("devicecode", "user_code"))?,
            verification_uri: text("verification_uri")
                .unwrap_or_else(|| "https://www.microsoft.com/link".into()),
            expires_in: v.get("expires_in").and_then(Value::as_u64).unwrap_or(900),
            interval: v.get("interval").and_then(Value::as_u64).unwrap_or(5),
            message: text("message"),
            device_code: Secret::new(
                text("device_code").ok_or_else(|| unexpected("devicecode", "device_code"))?,
            ),
        })
    }

    /// One poll of the token endpoint for a device code.
    pub async fn poll_device_code(&self, code: &DeviceCode) -> Result<PollAnswer> {
        let client_id = self.client_id()?;
        let url = format!("{}/token", self.config.endpoints.microsoft);
        let (status, v) = self
            .form(
                &url,
                &[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("client_id", client_id),
                    ("device_code", code.device_code.expose()),
                ],
            )
            .await?;
        if status == 200 {
            return tokens(&v).map(PollAnswer::Done);
        }
        match v.get("error").and_then(Value::as_str) {
            Some("authorization_pending") => Ok(PollAnswer::Pending),
            Some("slow_down") => Ok(PollAnswer::SlowDown),
            Some("authorization_declined") => Err(AuthError::Declined),
            Some("expired_token") => Err(AuthError::Expired),
            Some("bad_verification_code") => Err(AuthError::Expired),
            _ => Err(oauth_error(&v)),
        }
    }

    /// The browser sign-in page for the authorization-code flow with PKCE (RFC 7636).
    pub fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
        challenge: &str,
    ) -> Result<String> {
        let client_id = self.client_id()?;
        let mut url = url::Url::parse(&format!("{}/authorize", self.config.endpoints.microsoft))
            .map_err(|e| AuthError::Unexpected(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_mode", "query")
            .append_pair("scope", &self.config.scope)
            .append_pair("state", state)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("prompt", "select_account");
        Ok(url.into())
    }

    /// Exchanges an authorization code (with its PKCE verifier) for tokens.
    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        verifier: &Secret,
    ) -> Result<MicrosoftTokens> {
        let client_id = self.client_id()?;
        let url = format!("{}/token", self.config.endpoints.microsoft);
        let (status, v) = self
            .form(
                &url,
                &[
                    ("grant_type", "authorization_code"),
                    ("client_id", client_id),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                    ("code_verifier", verifier.expose()),
                    ("scope", &self.config.scope),
                ],
            )
            .await?;
        if status != 200 {
            return Err(oauth_error(&v));
        }
        tokens(&v)
    }

    /// Renews Microsoft's tokens. `invalid_grant` → [`AuthError::ReauthRequired`].
    pub async fn refresh(&self, refresh_token: &Secret) -> Result<MicrosoftTokens> {
        let client_id = self.client_id()?;
        let url = format!("{}/token", self.config.endpoints.microsoft);
        let (status, v) = self
            .form(
                &url,
                &[
                    ("grant_type", "refresh_token"),
                    ("client_id", client_id),
                    ("refresh_token", refresh_token.expose()),
                    ("scope", &self.config.scope),
                ],
            )
            .await?;
        if status == 200 {
            return tokens(&v);
        }
        match v.get("error").and_then(Value::as_str) {
            Some("invalid_grant") => Err(AuthError::ReauthRequired),
            _ => Err(oauth_error(&v)),
        }
    }

    // ---- Xbox Live --------------------------------------------------------------------------

    /// Xbox Live then XSTS for the Minecraft services.
    pub async fn xbox(&self, ms_access: &Secret) -> Result<XboxTokens> {
        let (status, xbl) = self
            .post_json(
                &self.config.endpoints.xbox_user,
                &json!({
                    "Properties": {
                        "AuthMethod": "RPS",
                        "SiteName": "user.auth.xboxlive.com",
                        "RpsTicket": format!("d={}", ms_access.expose()),
                    },
                    "RelyingParty": "http://auth.xboxlive.com",
                    "TokenType": "JWT",
                }),
                XBOX,
            )
            .await?;
        let xbl_token = str_field(&xbl, "Token")
            .filter(|_| status == 200)
            .ok_or_else(|| AuthError::Xbox("Xbox Live refused the sign-in.".into()))?;
        let (status, xsts) = self
            .post_json(
                &self.config.endpoints.xsts,
                &json!({
                    "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
                    "RelyingParty": "rp://api.minecraftservices.com/",
                    "TokenType": "JWT",
                }),
                XBOX,
            )
            .await?;
        if let Some(code) = xsts.get("XErr").and_then(Value::as_u64) {
            return Err(AuthError::Xbox(xsts_error(code)));
        }
        let claims = xsts.pointer("/DisplayClaims/xui/0");
        let (Some(token), Some(uhs)) = (
            str_field(&xsts, "Token").filter(|_| status == 200),
            claims.and_then(|c| str_field(c, "uhs")),
        ) else {
            return Err(AuthError::Xbox("Xbox Live refused the sign-in.".into()));
        };
        Ok(XboxTokens {
            xsts_token: Secret::new(token),
            user_hash: uhs,
            xuid: claims.and_then(|c| str_field(c, "xid")),
        })
    }

    // ---- Minecraft services -----------------------------------------------------------------

    /// The Minecraft token from the XSTS one.
    pub async fn minecraft_login(&self, xbox: &XboxTokens) -> Result<MinecraftToken> {
        let xtoken = format!("XBL3.0 x={};{}", xbox.user_hash, xbox.xsts_token.expose());
        let lifetime = |v: &Value| {
            v.get("expires_in")
                .and_then(Value::as_u64)
                .unwrap_or(86_400)
        };
        let base = &self.config.endpoints.minecraft;
        // `launcher/login` first: a new account's token from login_with_xbox names no profile
        // (Horizon launcher/src/account.rs, minecraft_token).
        if let Ok((200, v)) = self
            .post_json(
                &format!("{base}/launcher/login"),
                &json!({ "xtoken": xtoken, "platform": "PC_LAUNCHER" }),
                MINECRAFT,
            )
            .await
            && let Some(token) = str_field(&v, "access_token")
        {
            return Ok(MinecraftToken {
                access_token: Secret::new(token),
                expires_in: lifetime(&v),
            });
        }
        let (status, v) = self
            .post_json(
                &format!("{base}/authentication/login_with_xbox"),
                &json!({ "identityToken": xtoken }),
                MINECRAFT,
            )
            .await?;
        match (status, str_field(&v, "access_token")) {
            (200, Some(token)) => Ok(MinecraftToken {
                access_token: Secret::new(token),
                expires_in: lifetime(&v),
            }),
            (403, _) => Err(AuthError::NotApproved),
            (429, _) => Err(AuthError::RateLimited),
            _ => Err(AuthError::MinecraftRefused),
        }
    }

    /// Whether the account owns Minecraft: Java Edition (Game Pass included):
    /// `game_minecraft` or `product_minecraft` in `entitlements/mcstore`.
    pub async fn owns_game(&self, token: &Secret) -> Result<bool> {
        let url = format!("{}/entitlements/mcstore", self.config.endpoints.minecraft);
        let (status, v) = self.get_bearer(&url, token).await?;
        match status {
            200 => Ok(v
                .get("items")
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items.iter().any(|i| {
                        matches!(
                            i.get("name").and_then(Value::as_str),
                            Some("game_minecraft" | "product_minecraft")
                        )
                    })
                })),
            401 => Err(AuthError::ReauthRequired),
            429 => Err(AuthError::RateLimited),
            _ => Err(AuthError::Network {
                service: MINECRAFT,
                message: format!("entitlements answered HTTP {status}"),
            }),
        }
    }

    /// The profile; `None` (404) when the account owns the game but has no profile yet.
    pub async fn profile(&self, token: &Secret) -> Result<Option<Profile>> {
        let url = format!("{}/minecraft/profile", self.config.endpoints.minecraft);
        let (status, v) = self.get_bearer(&url, token).await?;
        match status {
            200 => serde_json::from_value(v)
                .map(Some)
                .map_err(|e| AuthError::Unexpected(format!("profile: {e}"))),
            404 => Ok(None),
            401 => Err(AuthError::ReauthRequired),
            429 => Err(AuthError::RateLimited),
            _ => Err(AuthError::Network {
                service: MINECRAFT,
                message: format!("profile answered HTTP {status}"),
            }),
        }
    }
}

/// The XSTS `XErr` codes, as Horizon's launcher (and the official one) explains them.
pub fn xsts_error(code: u64) -> String {
    match code {
        2148916233 => "This Microsoft account has no Xbox profile yet. Sign in once at xbox.com to create one, then try again.".into(),
        2148916235 => "Xbox Live is not available in your country or region.".into(),
        2148916236 | 2148916237 => "This account needs adult verification on xbox.com before it can play.".into(),
        2148916238 => "This is a child account: an adult has to add it to a Microsoft family first.".into(),
        other => format!("Xbox Live refused the sign-in (error {other})."),
    }
}

fn tokens(v: &Value) -> Result<MicrosoftTokens> {
    Ok(MicrosoftTokens {
        access_token: Secret::new(
            str_field(v, "access_token").ok_or_else(|| unexpected("token", "access_token"))?,
        ),
        refresh_token: Secret::new(
            str_field(v, "refresh_token").ok_or_else(|| unexpected("token", "refresh_token"))?,
        ),
        expires_in: v.get("expires_in").and_then(Value::as_u64).unwrap_or(3600),
    })
}

fn oauth_error(v: &Value) -> AuthError {
    let code = v.get("error").and_then(Value::as_str).unwrap_or("unknown");
    // error_description can quote request data; keep its first line only.
    let description = v
        .get("error_description")
        .and_then(Value::as_str)
        .and_then(|d| d.lines().next())
        .unwrap_or("");
    AuthError::Microsoft(format!("{code} {description}").trim().to_owned())
}

fn unexpected(endpoint: &str, field: &str) -> AuthError {
    AuthError::Unexpected(format!("{endpoint}: the answer has no {field}"))
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// The last path segment of a URL, for logs (never a query string, which may hold a code).
fn endpoint_name(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    path.rsplit('/').next().unwrap_or(path).to_owned()
}
