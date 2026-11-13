//! The Rustic account (Rustic Cloud): a platform identity for friends, cosmetics, purchases and
//! servers. It never replaces the game licence: playing still needs a Microsoft account that owns
//! the game.
//!
//! [`CloudLink`] is the seam to the cloud API; the launcher implements it over
//! `rusticmc-api-client` once that client is generated (its wire types already exist:
//! `LoginRequest`, `TokenPair`, `OAuthExchangeRequest`, `MinecraftLinkRequest`). Until then
//! [`UnavailableCloud`] answers every call with "not available yet". The session (access and
//! refresh tokens) is kept in the OS keychain under `rustic-cloud`.

use crate::error::{AuthError, Result};
use crate::store::SecretStore;
use async_trait::async_trait;
use rusticmc_launcher_core::{Secret, time};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The cloud's tokens (`TokenPair`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudTokens {
    pub access_token: Secret,
    /// Unix seconds.
    pub access_expires_at: u64,
    pub refresh_token: Option<Secret>,
    pub session_id: String,
}

/// A login's answer.
#[derive(Clone, Debug)]
pub enum CloudLogin {
    Done(CloudTokens),
    /// Send a TOTP or recovery code with this token.
    MfaRequired {
        mfa_token: Secret,
    },
}

/// The signed-in Rustic user (public data).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudUser {
    pub id: String,
    pub display_name: String,
}

/// A Minecraft account linked to the Rustic account.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkedMinecraft {
    pub uuid: String,
    pub username: String,
    pub last_verified_at: u64,
}

/// What the launcher asks Rustic Cloud for its account.
#[async_trait]
pub trait CloudLink: Send + Sync {
    async fn login(&self, email: &str, password: &Secret, device: &str) -> Result<CloudLogin>;
    async fn login_mfa(&self, mfa_token: &Secret, code: &str, device: &str) -> Result<CloudTokens>;
    /// A browser login: the URL to open; the cloud redirects to `rustic://auth?ticket=...`.
    async fn oauth_start(&self, provider: &str) -> Result<String>;
    async fn oauth_exchange(&self, ticket: &str, device: &str) -> Result<CloudTokens>;
    async fn refresh(&self, refresh_token: &Secret) -> Result<CloudTokens>;
    async fn logout(&self, access_token: &Secret) -> Result<()>;
    async fn me(&self, access_token: &Secret) -> Result<CloudUser>;
    /// Links the game profile of `minecraft` to the Rustic account: the cloud hands a challenge,
    /// the game session "joins" it at Mojang (as a client joining a server), the cloud checks
    /// `hasJoined`. The game token goes to Mojang only, never to the cloud.
    async fn link_minecraft(
        &self,
        access_token: &Secret,
        minecraft: &crate::session::GameSession,
    ) -> Result<LinkedMinecraft>;
}

/// The cloud before its client exists.
pub struct UnavailableCloud;

fn unavailable<T>() -> Result<T> {
    Err(AuthError::Cloud(
        "Rustic accounts are not available in this build yet.".into(),
    ))
}

#[async_trait]
impl CloudLink for UnavailableCloud {
    async fn login(&self, _: &str, _: &Secret, _: &str) -> Result<CloudLogin> {
        unavailable()
    }
    async fn login_mfa(&self, _: &Secret, _: &str, _: &str) -> Result<CloudTokens> {
        unavailable()
    }
    async fn oauth_start(&self, _: &str) -> Result<String> {
        unavailable()
    }
    async fn oauth_exchange(&self, _: &str, _: &str) -> Result<CloudTokens> {
        unavailable()
    }
    async fn refresh(&self, _: &Secret) -> Result<CloudTokens> {
        unavailable()
    }
    async fn logout(&self, _: &Secret) -> Result<()> {
        unavailable()
    }
    async fn me(&self, _: &Secret) -> Result<CloudUser> {
        unavailable()
    }
    async fn link_minecraft(
        &self,
        _: &Secret,
        _: &crate::session::GameSession,
    ) -> Result<LinkedMinecraft> {
        unavailable()
    }
}

const CLOUD_KEY: &str = "rustic-cloud";

/// The Rustic account of this launcher.
pub struct RusticAccount {
    store: Arc<dyn SecretStore>,
    link: Arc<dyn CloudLink>,
    device: String,
    /// One refresh at a time: the cloud rotates refresh tokens and treats a second use of the
    /// same one as theft (it revokes the whole session), so concurrent callers (two windows
    /// asking for a token at start-up) must wait for the first refresh and reuse its result.
    refreshing: tokio::sync::Mutex<()>,
}

/// The Rustic account's status for the UI.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RusticAccountStatus {
    SignedOut,
    SignedIn {
        user: CloudUser,
    },
    /// Signed in, but the cloud couldn't be reached to say who.
    Unreachable,
}

impl RusticAccount {
    pub fn new(store: Arc<dyn SecretStore>, link: Arc<dyn CloudLink>) -> Self {
        Self {
            store,
            link,
            device: format!("Rustic Launcher ({})", std::env::consts::OS),
            refreshing: tokio::sync::Mutex::new(()),
        }
    }

    pub fn link(&self) -> &Arc<dyn CloudLink> {
        &self.link
    }

    fn tokens(&self) -> Result<Option<CloudTokens>> {
        Ok(self
            .store
            .get(CLOUD_KEY)?
            .and_then(|t| serde_json::from_str(&t).ok()))
    }

    fn save(&self, tokens: &CloudTokens) -> Result<()> {
        let text =
            serde_json::to_string(tokens).map_err(|e| AuthError::Unexpected(e.to_string()))?;
        self.store.set(CLOUD_KEY, &text)
    }

    /// Email and password; MFA answers `MfaRequired`.
    pub async fn login(&self, email: &str, password: &Secret) -> Result<Option<Secret>> {
        match self.link.login(email, password, &self.device).await? {
            CloudLogin::Done(tokens) => {
                self.save(&tokens)?;
                Ok(None)
            }
            CloudLogin::MfaRequired { mfa_token } => Ok(Some(mfa_token)),
        }
    }

    pub async fn login_mfa(&self, mfa_token: &Secret, code: &str) -> Result<()> {
        let tokens = self.link.login_mfa(mfa_token, code, &self.device).await?;
        self.save(&tokens)
    }

    /// Finishes a browser login from the `rustic://auth?ticket=` deep link.
    pub async fn complete_oauth(&self, ticket: &str) -> Result<()> {
        let tokens = self.link.oauth_exchange(ticket, &self.device).await?;
        self.save(&tokens)
    }

    /// A valid access token, refreshed when it expires within a minute.
    pub async fn access_token(&self) -> Result<Secret> {
        Ok(self.access_token_with_expiry().await?.0)
    }

    /// [`Self::access_token`] with its expiry (unix seconds), for the webview's cloud client.
    pub async fn access_token_with_expiry(&self) -> Result<(Secret, u64)> {
        let tokens = self
            .tokens()?
            .ok_or_else(|| AuthError::Cloud("Not signed in to a Rustic account.".into()))?;
        if tokens.access_expires_at > time::now() + 60 {
            return Ok((tokens.access_token, tokens.access_expires_at));
        }
        let _one_at_a_time = self.refreshing.lock().await;
        // Another caller may have refreshed while this one waited: use its tokens.
        let tokens = self
            .tokens()?
            .ok_or_else(|| AuthError::Cloud("Not signed in to a Rustic account.".into()))?;
        if tokens.access_expires_at > time::now() + 60 {
            return Ok((tokens.access_token, tokens.access_expires_at));
        }
        let refresh = tokens
            .refresh_token
            .ok_or_else(|| AuthError::Cloud("The Rustic session expired: sign in again.".into()))?;
        let renewed = match self.link.refresh(&refresh).await {
            Ok(t) => t,
            Err(AuthError::ReauthRequired) => {
                // The cloud ended the session: forget it, so the launcher shows "signed out"
                // (and offers to sign in) instead of failing every call.
                self.store.delete(CLOUD_KEY)?;
                return Err(AuthError::ReauthRequired);
            }
            Err(e) => return Err(e),
        };
        self.save(&renewed)?;
        Ok((renewed.access_token, renewed.access_expires_at))
    }

    pub async fn status(&self) -> Result<RusticAccountStatus> {
        if self.tokens()?.is_none() {
            return Ok(RusticAccountStatus::SignedOut);
        }
        let token = match self.access_token().await {
            Ok(t) => t,
            Err(_) => return Ok(RusticAccountStatus::Unreachable),
        };
        match self.link.me(&token).await {
            Ok(user) => Ok(RusticAccountStatus::SignedIn { user }),
            // Revoked elsewhere (sign-out everywhere, password change, refresh reuse).
            Err(AuthError::ReauthRequired) => {
                self.store.delete(CLOUD_KEY)?;
                Ok(RusticAccountStatus::SignedOut)
            }
            Err(_) => Ok(RusticAccountStatus::Unreachable),
        }
    }

    /// Signs out (the cloud session is revoked when reachable; the keychain item always goes).
    pub async fn logout(&self) -> Result<()> {
        if let Some(tokens) = self.tokens()? {
            let _ = self.link.logout(&tokens.access_token).await;
        }
        self.store.delete(CLOUD_KEY)
    }

    /// Links the Minecraft account whose session is `minecraft`.
    pub async fn link_minecraft(
        &self,
        minecraft: &crate::session::GameSession,
    ) -> Result<LinkedMinecraft> {
        let token = self.access_token().await?;
        self.link.link_minecraft(&token, minecraft).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;
    use std::sync::Mutex;

    struct FakeCloud {
        refreshed: Mutex<u32>,
        /// The cloud revoked the session: refreshing answers 401.
        dead: bool,
    }

    fn tokens(expires: u64) -> CloudTokens {
        CloudTokens {
            access_token: Secret::new(format!("access-{expires}")),
            access_expires_at: expires,
            refresh_token: Some(Secret::new("r")),
            session_id: "s".into(),
        }
    }

    #[async_trait]
    impl CloudLink for FakeCloud {
        async fn login(&self, email: &str, _: &Secret, _: &str) -> Result<CloudLogin> {
            if email == "mfa@x" {
                return Ok(CloudLogin::MfaRequired {
                    mfa_token: Secret::new("m"),
                });
            }
            Ok(CloudLogin::Done(tokens(0)))
        }
        async fn login_mfa(&self, _: &Secret, _: &str, _: &str) -> Result<CloudTokens> {
            Ok(tokens(time::now() + 900))
        }
        async fn oauth_start(&self, _: &str) -> Result<String> {
            Ok("https://x".into())
        }
        async fn oauth_exchange(&self, _: &str, _: &str) -> Result<CloudTokens> {
            Ok(tokens(time::now() + 900))
        }
        async fn refresh(&self, _: &Secret) -> Result<CloudTokens> {
            *self.refreshed.lock().unwrap() += 1;
            if self.dead {
                return Err(AuthError::ReauthRequired);
            }
            // A network round trip: lets concurrent callers overlap.
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            Ok(tokens(time::now() + 900))
        }
        async fn logout(&self, _: &Secret) -> Result<()> {
            Ok(())
        }
        async fn me(&self, _: &Secret) -> Result<CloudUser> {
            if self.dead {
                return Err(AuthError::ReauthRequired);
            }
            Ok(CloudUser {
                id: "u1".into(),
                display_name: "Sam".into(),
            })
        }
        async fn link_minecraft(
            &self,
            _: &Secret,
            _: &crate::session::GameSession,
        ) -> Result<LinkedMinecraft> {
            unavailable()
        }
    }

    #[tokio::test]
    async fn session_lives_in_the_keychain_and_refreshes() {
        let store = Arc::new(MemoryStore::new());
        let cloud = Arc::new(FakeCloud {
            refreshed: Mutex::new(0),
            dead: false,
        });
        let account = RusticAccount::new(store.clone(), cloud.clone());
        assert_eq!(
            account.status().await.unwrap(),
            RusticAccountStatus::SignedOut
        );
        // Logged in with an already-expired access token: the first use refreshes.
        assert!(
            account
                .login("a@x", &Secret::new("pw"))
                .await
                .unwrap()
                .is_none()
        );
        assert!(store.raw_items().contains_key("rustic-cloud"));
        let _ = account.access_token().await.unwrap();
        assert_eq!(*cloud.refreshed.lock().unwrap(), 1);
        assert!(matches!(
            account.status().await.unwrap(),
            RusticAccountStatus::SignedIn { .. }
        ));
        account.logout().await.unwrap();
        assert!(store.raw_items().is_empty());
    }

    #[tokio::test]
    async fn concurrent_callers_share_one_refresh() {
        // Regression (QA): the main and Social windows asked for a token together at start-up;
        // two refreshes presented the same refresh token and the cloud revoked the session.
        let store = Arc::new(MemoryStore::new());
        let cloud = Arc::new(FakeCloud {
            refreshed: Mutex::new(0),
            dead: false,
        });
        let account = Arc::new(RusticAccount::new(store, cloud.clone()));
        account.login("a@x", &Secret::new("pw")).await.unwrap(); // expired access token
        let calls = (0..4).map(|_| {
            let a = account.clone();
            tokio::spawn(async move { a.access_token_with_expiry().await.unwrap().1 })
        });
        let mut expiries = Vec::new();
        for c in calls {
            expiries.push(c.await.unwrap());
        }
        assert_eq!(*cloud.refreshed.lock().unwrap(), 1);
        assert!(expiries.windows(2).all(|w| w[0] == w[1]));
    }

    #[tokio::test]
    async fn a_revoked_session_signs_out() {
        let store = Arc::new(MemoryStore::new());
        let cloud = Arc::new(FakeCloud {
            refreshed: Mutex::new(0),
            dead: true,
        });
        let account = RusticAccount::new(store.clone(), cloud);
        account.login("a@x", &Secret::new("pw")).await.unwrap(); // expired access token
        assert!(matches!(
            account.access_token().await,
            Err(AuthError::ReauthRequired)
        ));
        assert!(store.raw_items().is_empty());
        assert_eq!(
            account.status().await.unwrap(),
            RusticAccountStatus::SignedOut
        );
        // A still-valid access token whose session the cloud revoked: forgotten as well.
        let mfa = account.login("mfa@x", &Secret::new("pw")).await.unwrap();
        account.login_mfa(&mfa.unwrap(), "123456").await.unwrap();
        assert!(!store.raw_items().is_empty());
        assert_eq!(
            account.status().await.unwrap(),
            RusticAccountStatus::SignedOut
        );
        assert!(store.raw_items().is_empty());
    }

    #[tokio::test]
    async fn mfa_is_a_second_step() {
        let store = Arc::new(MemoryStore::new());
        let account = RusticAccount::new(
            store.clone(),
            Arc::new(FakeCloud {
                refreshed: Mutex::new(0),
                dead: false,
            }),
        );
        let mfa = account.login("mfa@x", &Secret::new("pw")).await.unwrap();
        assert!(mfa.is_some() && store.raw_items().is_empty());
        account.login_mfa(&mfa.unwrap(), "123456").await.unwrap();
        assert!(store.raw_items().contains_key("rustic-cloud"));
    }

    #[tokio::test]
    async fn the_stub_says_not_available() {
        let account = RusticAccount::new(Arc::new(MemoryStore::new()), Arc::new(UnavailableCloud));
        let err = account.login("a@x", &Secret::new("pw")).await.unwrap_err();
        assert_eq!(err.code(), "auth.cloud");
    }
}
