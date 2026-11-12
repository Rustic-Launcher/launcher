//! Signed-in accounts: several Microsoft accounts (one selected), silent renewal, and the session
//! a launch hands to the game. Offline play is the owner's own account under its official name and
//! UUID (no free-name profiles, as Horizon decided on 2026-11-09; ADR 0146).
//!
//! Public data (UUID, name, skins, capes, which is selected) is in
//! `accounts.json`; every token and the ownership answers are in the OS keychain, one item per
//! account (`minecraft:<uuid>`), never in a file.

use crate::api::{AuthApi, DeviceCode, MicrosoftTokens, PollAnswer, Profile, Texture};
use crate::error::{AuthError, Result};
use crate::loopback::{Loopback, Pkce, random_state};
use crate::ownership::{OfflineAccess, OwnershipPolicy, OwnershipRecord};
use crate::session::{GameSession, LaunchAuth, VerifiedOwner};
use crate::store::SecretStore;
use rusticmc_launcher_core::{CancellationToken, Secret, fsutil, time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

/// What the keychain keeps per account.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccountSecrets {
    v: u32,
    ms_refresh: Secret,
    mc_access: Secret,
    mc_expires_at: u64,
    #[serde(default)]
    xuid: Option<String>,
    #[serde(default)]
    ownership: Option<OwnershipRecord>,
}

/// An account's state, for the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountState {
    /// Token fresh and last ownership answer "owns".
    Ready,
    /// The Minecraft token has an hour or less left: renewed before the next launch.
    NeedsRefresh,
    /// Microsoft refused the refresh token, or the keychain lost the secrets.
    SignInAgain,
    /// The last answer was "doesn't own".
    NotOwned,
}

/// A Microsoft account as `accounts.json` keeps it (public data only).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountRecord {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Texture>,
    #[serde(default)]
    pub capes: Vec<Texture>,
    pub added_at: u64,
    /// A copy of the last check for display; the gate reads the keychain's.
    #[serde(default)]
    pub last_check: Option<OwnershipRecord>,
    #[serde(default)]
    pub needs_sign_in: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct AccountsFile {
    #[serde(default = "one")]
    version: u32,
    #[serde(default)]
    selected: Option<String>,
    #[serde(default)]
    accounts: Vec<AccountRecord>,
    // Free-name offline profiles of older versions (`offline_profiles`) are ignored on load and
    // dropped at the next save.
}

fn one() -> u32 {
    1
}

/// One account for the UI.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountView {
    pub uuid: String,
    pub name: String,
    pub skins: Vec<Texture>,
    pub capes: Vec<Texture>,
    pub selected: bool,
    pub state: AccountState,
    pub last_ownership_check: Option<u64>,
    pub token_expires_at: Option<u64>,
}

/// Everything the accounts page shows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountsSnapshot {
    pub accounts: Vec<AccountView>,
    pub selected: Option<String>,
    /// Until when the owner may play without network (the last "owns" answer plus the window).
    pub offline_access: OfflineAccess,
    /// False when secrets are only in memory (no keychain).
    pub keychain_persistent: bool,
    pub sign_in_available: bool,
}

/// A step of a sign-in or renewal, for progress UI (no secrets).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthPhase {
    WaitingForBrowser,
    WaitingForCode,
    Microsoft,
    Xbox,
    Minecraft,
    Ownership,
    Profile,
    Saving,
    Done,
}

/// Progress of one flow.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthEvent {
    pub flow_id: String,
    pub phase: AuthPhase,
}

/// Who plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveIdentity {
    /// A Microsoft account's UUID.
    Microsoft(String),
    None,
}

/// A started device-code sign-in.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeviceCodeFlow {
    pub flow_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: u64,
    pub message: Option<String>,
}

/// A started browser sign-in.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BrowserFlow {
    pub flow_id: String,
    /// Open this in the system browser.
    pub authorize_url: String,
}

enum Pending {
    Device {
        code: DeviceCode,
        target: Option<String>,
        cancel: CancellationToken,
    },
    Browser {
        loopback: Loopback,
        state: String,
        pkce: Pkce,
        target: Option<String>,
        cancel: CancellationToken,
    },
}

/// The accounts of this launcher.
pub struct AccountManager {
    file: PathBuf,
    store: Arc<dyn SecretStore>,
    api: AuthApi,
    policy: OwnershipPolicy,
    /// Serialises credential changes (refresh, sign-in, sign-out).
    gate: tokio::sync::Mutex<()>,
    pending: Mutex<HashMap<String, Pending>>,
    cancels: Mutex<HashMap<String, CancellationToken>>,
    events: broadcast::Sender<AuthEvent>,
    /// How long a browser sign-in waits.
    pub browser_timeout: Duration,
}

fn secret_key(uuid: &str) -> String {
    format!("minecraft:{uuid}")
}

impl AccountManager {
    pub fn new(file: PathBuf, store: Arc<dyn SecretStore>, api: AuthApi) -> Self {
        let policy = OwnershipPolicy::new(api.config().revalidation_window);
        let (events, _) = broadcast::channel(64);
        Self {
            file,
            store,
            api,
            policy,
            gate: tokio::sync::Mutex::new(()),
            pending: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
            events,
            browser_timeout: Duration::from_secs(600),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AuthEvent> {
        self.events.subscribe()
    }

    pub fn policy(&self) -> OwnershipPolicy {
        self.policy
    }

    fn phase(&self, flow_id: &str, phase: AuthPhase) {
        let _ = self.events.send(AuthEvent {
            flow_id: flow_id.to_owned(),
            phase,
        });
    }

    fn load(&self) -> Result<AccountsFile> {
        Ok(fsutil::read_json(&self.file)?.unwrap_or_default())
    }

    fn save(&self, file: &AccountsFile) -> Result<()> {
        fsutil::write_json(&self.file, file)?;
        Ok(())
    }

    fn secrets(&self, uuid: &str) -> Result<Option<AccountSecrets>> {
        let Some(text) = self.store.get(&secret_key(uuid))? else {
            return Ok(None);
        };
        // Never overwrite secrets that don't parse; treat them as missing for this run.
        Ok(serde_json::from_str(&text).ok())
    }

    fn put_secrets(&self, uuid: &str, secrets: &AccountSecrets) -> Result<()> {
        let text =
            serde_json::to_string(secrets).map_err(|e| AuthError::Unexpected(e.to_string()))?;
        self.store.set(&secret_key(uuid), &text)
    }

    /// Every account's ownership answer, from the keychain.
    fn ownership_records(&self, file: &AccountsFile) -> Vec<OwnershipRecord> {
        file.accounts
            .iter()
            .filter_map(|a| self.secrets(&a.uuid).ok().flatten())
            .filter_map(|s| s.ownership)
            .collect()
    }

    /// Whether the owner may play without network now.
    pub fn offline_access(&self) -> Result<OfflineAccess> {
        let file = self.load()?;
        Ok(self
            .policy
            .evaluate(&self.ownership_records(&file), time::now()))
    }

    /// The accounts page.
    pub fn snapshot(&self) -> Result<AccountsSnapshot> {
        let file = self.load()?;
        let now = time::now();
        let margin = self.api.config().refresh_margin.as_secs();
        let accounts = file
            .accounts
            .iter()
            .map(|a| {
                let secrets = self.secrets(&a.uuid).ok().flatten();
                let state = match &secrets {
                    _ if a.needs_sign_in => AccountState::SignInAgain,
                    None => AccountState::SignInAgain,
                    Some(s) if s.ownership.is_some_and(|o| !o.owns) => AccountState::NotOwned,
                    Some(s) if s.mc_expires_at <= now + margin => AccountState::NeedsRefresh,
                    Some(_) => AccountState::Ready,
                };
                AccountView {
                    uuid: a.uuid.clone(),
                    name: a.name.clone(),
                    skins: a.skins.clone(),
                    capes: a.capes.clone(),
                    selected: file.selected.as_deref() == Some(a.uuid.as_str()),
                    state,
                    last_ownership_check: secrets
                        .as_ref()
                        .and_then(|s| s.ownership)
                        .map(|o| o.checked_at),
                    token_expires_at: secrets.map(|s| s.mc_expires_at),
                }
            })
            .collect();
        Ok(AccountsSnapshot {
            accounts,
            selected: file.selected.clone(),
            offline_access: self.policy.evaluate(&self.ownership_records(&file), now),
            keychain_persistent: self.store.persistent(),
            sign_in_available: self.api.config().client_id.is_some(),
        })
    }

    /// Selects which Microsoft account plays.
    pub fn select(&self, id: &str) -> Result<()> {
        let mut file = self.load()?;
        if !file.accounts.iter().any(|a| a.uuid == id) {
            return Err(AuthError::UnknownAccount(id.into()));
        }
        file.selected = Some(id.into());
        self.save(&file)
    }

    /// Who plays now.
    pub fn active(&self) -> Result<ActiveIdentity> {
        let file = self.load()?;
        Ok(file
            .selected
            .filter(|s| file.accounts.iter().any(|a| &a.uuid == s))
            .or_else(|| file.accounts.first().map(|a| a.uuid.clone()))
            .map_or(ActiveIdentity::None, ActiveIdentity::Microsoft))
    }

    /// Signs an account out: its keychain secrets go, then its public record.
    pub async fn remove(&self, uuid: &str) -> Result<()> {
        let _gate = self.gate.lock().await;
        let mut file = self.load()?;
        if !file.accounts.iter().any(|a| a.uuid == uuid) {
            return Err(AuthError::UnknownAccount(uuid.into()));
        }
        self.store.delete(&secret_key(uuid))?;
        file.accounts.retain(|a| a.uuid != uuid);
        if file.selected.as_deref() == Some(uuid) {
            file.selected = file.accounts.first().map(|a| a.uuid.clone());
        }
        self.save(&file)
    }

    // ---- sign-in flows ----------------------------------------------------------------------

    /// Starts a device-code sign-in. `target`: the UUID a re-sign-in must come back as.
    pub async fn start_device_code(&self, target: Option<String>) -> Result<DeviceCodeFlow> {
        let code = self.api.device_code().await?;
        let flow_id = uuid::Uuid::new_v4().to_string();
        let flow = DeviceCodeFlow {
            flow_id: flow_id.clone(),
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
            expires_at: time::now() + code.expires_in,
            message: code.message.clone(),
        };
        let cancel = CancellationToken::new();
        self.cancels
            .lock()
            .expect("cancels")
            .insert(flow_id.clone(), cancel.clone());
        self.pending.lock().expect("pending").insert(
            flow_id,
            Pending::Device {
                code,
                target,
                cancel,
            },
        );
        Ok(flow)
    }

    /// Starts a browser sign-in (authorization code + PKCE, loopback redirect).
    pub async fn start_browser(&self, target: Option<String>) -> Result<BrowserFlow> {
        let loopback = Loopback::bind(0).await?;
        let pkce = Pkce::generate();
        let state = random_state();
        let authorize_url =
            self.api
                .authorize_url(loopback.redirect_uri(), &state, &pkce.challenge)?;
        let flow_id = uuid::Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        self.cancels
            .lock()
            .expect("cancels")
            .insert(flow_id.clone(), cancel.clone());
        self.pending.lock().expect("pending").insert(
            flow_id.clone(),
            Pending::Browser {
                loopback,
                state,
                pkce,
                target,
                cancel,
            },
        );
        Ok(BrowserFlow {
            flow_id,
            authorize_url,
        })
    }

    /// Cancels a started flow.
    pub fn cancel_flow(&self, flow_id: &str) {
        if let Some(cancel) = self.cancels.lock().expect("cancels").remove(flow_id) {
            cancel.cancel();
        }
        self.pending.lock().expect("pending").remove(flow_id);
    }

    /// Waits for a started flow to finish and adds (or renews) the account.
    pub async fn complete(&self, flow_id: &str) -> Result<AccountView> {
        let pending = self
            .pending
            .lock()
            .expect("pending")
            .remove(flow_id)
            .ok_or_else(|| AuthError::UnknownFlow(flow_id.into()))?;
        let result = match pending {
            Pending::Device {
                code,
                target,
                cancel,
            } => {
                self.phase(flow_id, AuthPhase::WaitingForCode);
                let tokens = tokio::select! {
                    _ = cancel.cancelled() => Err(AuthError::Cancelled),
                    t = self.poll_device(&code) => t,
                }?;
                self.finish(flow_id, tokens, target.as_deref()).await
            }
            Pending::Browser {
                loopback,
                state,
                pkce,
                target,
                cancel,
            } => {
                self.phase(flow_id, AuthPhase::WaitingForBrowser);
                let redirect_uri = loopback.redirect_uri().to_owned();
                let timeout = self.browser_timeout;
                let redirect = tokio::select! {
                    _ = cancel.cancelled() => Err(AuthError::Cancelled),
                    r = loopback.wait(&state, timeout) => r,
                }?;
                self.phase(flow_id, AuthPhase::Microsoft);
                let tokens = self
                    .api
                    .exchange_code(&redirect.code, &redirect_uri, &pkce.verifier)
                    .await?;
                self.finish(flow_id, tokens, target.as_deref()).await
            }
        };
        self.cancels.lock().expect("cancels").remove(flow_id);
        result
    }

    async fn poll_device(&self, code: &DeviceCode) -> Result<MicrosoftTokens> {
        let timing = self.api.config().poll;
        let mut interval = Duration::from_secs(code.interval).max(timing.min_interval);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(code.expires_in);
        loop {
            tokio::time::sleep(interval).await;
            if tokio::time::Instant::now() > deadline {
                return Err(AuthError::Expired);
            }
            match self.api.poll_device_code(code).await? {
                PollAnswer::Pending => {}
                PollAnswer::SlowDown => interval += timing.slow_down_step,
                PollAnswer::Done(tokens) => return Ok(tokens),
            }
        }
    }

    /// Microsoft tokens → Xbox → Minecraft → ownership → profile → keychain + accounts.json.
    async fn finish(
        &self,
        flow_id: &str,
        ms: MicrosoftTokens,
        target: Option<&str>,
    ) -> Result<AccountView> {
        self.phase(flow_id, AuthPhase::Xbox);
        let xbox = self.api.xbox(&ms.access_token).await?;
        self.phase(flow_id, AuthPhase::Minecraft);
        let mc = self.api.minecraft_login(&xbox).await?;
        self.phase(flow_id, AuthPhase::Ownership);
        if !self.api.owns_game(&mc.access_token).await? {
            return Err(AuthError::NotOwned);
        }
        self.phase(flow_id, AuthPhase::Profile);
        let profile = self
            .api
            .profile(&mc.access_token)
            .await?
            .ok_or(AuthError::NoProfile)?;
        if let Some(target) = target
            && !same_uuid(target, &profile.id)
        {
            // Refused before anything is stored.
            return Err(AuthError::WrongAccount {
                expected: target.into(),
                got: profile.name,
            });
        }
        self.phase(flow_id, AuthPhase::Saving);
        let now = time::now();
        let _gate = self.gate.lock().await;
        let uuid = profile.id.clone();
        self.put_secrets(
            &uuid,
            &AccountSecrets {
                v: 1,
                ms_refresh: ms.refresh_token,
                mc_access: mc.access_token,
                mc_expires_at: now + mc.expires_in,
                xuid: xbox.xuid,
                ownership: Some(OwnershipRecord {
                    owns: true,
                    checked_at: now,
                }),
            },
        )?;
        let mut file = self.load()?;
        upsert(&mut file, &profile, now);
        if file.selected.is_none() || target.is_none() && file.accounts.len() == 1 {
            file.selected = Some(uuid.clone());
        }
        self.save(&file)?;
        drop(_gate);
        self.phase(flow_id, AuthPhase::Done);
        self.view(&uuid)
    }

    fn view(&self, uuid: &str) -> Result<AccountView> {
        self.snapshot()?
            .accounts
            .into_iter()
            .find(|a| a.uuid == uuid)
            .ok_or_else(|| AuthError::UnknownAccount(uuid.into()))
    }

    // ---- renewal and launch -----------------------------------------------------------------

    /// Renews an account's tokens (when due, or always with `force`) and checks ownership and
    /// the profile again.
    pub async fn refresh(&self, uuid: &str, force: bool) -> Result<AccountView> {
        let _gate = self.gate.lock().await;
        self.refresh_locked(uuid, force).await?;
        drop(_gate);
        self.view(uuid)
    }

    async fn refresh_locked(&self, uuid: &str, force: bool) -> Result<AccountSecrets> {
        let mut secrets = match self.secrets(uuid)? {
            Some(s) => s,
            None => {
                self.mark_sign_in(uuid, true)?;
                return Err(AuthError::ReauthRequired);
            }
        };
        let now = time::now();
        let margin = self.api.config().refresh_margin.as_secs();
        if force || secrets.mc_expires_at <= now + margin {
            let ms = match self.api.refresh(&secrets.ms_refresh).await {
                Ok(ms) => ms,
                Err(AuthError::ReauthRequired) => {
                    self.mark_sign_in(uuid, true)?;
                    return Err(AuthError::ReauthRequired);
                }
                Err(e) => return Err(e),
            };
            // A rotated refresh token is saved at once, so a failure further down can recover.
            secrets.ms_refresh = ms.refresh_token.clone();
            self.put_secrets(uuid, &secrets)?;
            let xbox = self.api.xbox(&ms.access_token).await?;
            let mc = self.api.minecraft_login(&xbox).await?;
            secrets.mc_access = mc.access_token;
            secrets.mc_expires_at = time::now() + mc.expires_in;
            secrets.xuid = xbox.xuid.or(secrets.xuid);
            self.put_secrets(uuid, &secrets)?;
        }
        Ok(secrets)
    }

    fn mark_sign_in(&self, uuid: &str, needed: bool) -> Result<()> {
        let mut file = self.load()?;
        if let Some(a) = file.accounts.iter_mut().find(|a| a.uuid == uuid) {
            a.needs_sign_in = needed;
            self.save(&file)?;
        }
        Ok(())
    }

    /// The owner gate before a launch: renews the token if due, asks entitlements and the profile
    /// again, and returns the session for the game. `uuid`: the selected account when `None`.
    ///
    /// With no network, an account whose own last answer (within the window) was "owns" starts
    /// offline ([`LaunchAuth::Offline`]); a "doesn't own" answer blocks at once.
    pub async fn launch_session(&self, uuid: Option<&str>) -> Result<LaunchAuth> {
        let file = self.load()?;
        let uuid = match uuid {
            Some(u) => u.to_owned(),
            None => file
                .selected
                .clone()
                .or_else(|| file.accounts.first().map(|a| a.uuid.clone()))
                .ok_or_else(|| {
                    AuthError::OfflineNotAllowed(
                        "Sign in with a Microsoft account that owns Minecraft: Java Edition to play."
                            .into(),
                    )
                })?,
        };
        let record = file
            .accounts
            .iter()
            .find(|a| a.uuid == uuid)
            .cloned()
            .ok_or_else(|| AuthError::UnknownAccount(uuid.clone()))?;
        let _gate = self.gate.lock().await;
        let secrets = match self.refresh_locked(&uuid, false).await {
            Ok(s) => s,
            Err(e) if e.is_network() => return self.offline_fallback(&uuid, &record.name, e),
            Err(e) => return Err(e),
        };
        match self.check_owner(&uuid, secrets).await {
            Ok((secrets, profile)) => Ok(LaunchAuth::Online(GameSession {
                name: profile.name,
                uuid: profile.id,
                access_token: secrets.mc_access,
                xuid: secrets.xuid,
            })),
            Err(e) if e.is_network() => self.offline_fallback(&uuid, &record.name, e),
            Err(e) => Err(e),
        }
    }

    /// Entitlements and profile with the current token; records the answer.
    async fn check_owner(
        &self,
        uuid: &str,
        mut secrets: AccountSecrets,
    ) -> Result<(AccountSecrets, Profile)> {
        let owns = self.api.owns_game(&secrets.mc_access).await?;
        let now = time::now();
        secrets.ownership = Some(OwnershipRecord {
            owns,
            checked_at: now,
        });
        self.put_secrets(uuid, &secrets)?;
        if !owns {
            self.update_record(uuid, None, secrets.ownership)?;
            return Err(AuthError::NotOwned);
        }
        let profile = self
            .api
            .profile(&secrets.mc_access)
            .await?
            .ok_or(AuthError::NoProfile)?;
        self.update_record(uuid, Some(&profile), secrets.ownership)?;
        Ok((secrets, profile))
    }

    fn update_record(
        &self,
        uuid: &str,
        profile: Option<&Profile>,
        check: Option<OwnershipRecord>,
    ) -> Result<()> {
        let mut file = self.load()?;
        if let Some(a) = file.accounts.iter_mut().find(|a| a.uuid == uuid) {
            if let Some(p) = profile {
                a.name = p.name.clone();
                a.skins = p.skins.clone();
                a.capes = p.capes.clone();
            }
            a.last_check = check;
            a.needs_sign_in = false;
        }
        self.save(&file)
    }

    fn offline_fallback(&self, uuid: &str, name: &str, cause: AuthError) -> Result<LaunchAuth> {
        let record = self.secrets(uuid)?.and_then(|s| s.ownership);
        match record {
            Some(r) if self.policy.account_may_start_offline(&r, time::now()) => {
                // The game still plays as the owner (official name and UUID, as on LAN servers),
                // with the last token: the game falls back to offline when it can't be used.
                let secrets = self.secrets(uuid)?;
                let session = GameSession {
                    name: name.into(),
                    uuid: uuid.into(),
                    access_token: secrets
                        .as_ref()
                        .map(|s| s.mc_access.clone())
                        .unwrap_or_else(|| Secret::new("offline")),
                    xuid: secrets.and_then(|s| s.xuid),
                };
                Ok(LaunchAuth::Offline {
                    uuid: uuid.into(),
                    name: name.into(),
                    reason: cause.to_string(),
                    session,
                })
            }
            _ => Err(cause),
        }
    }

    /// Checks ownership now and returns the proof owner-only actions need.
    pub async fn verified_owner(&self, uuid: Option<&str>) -> Result<VerifiedOwner> {
        match self.launch_session(uuid).await? {
            LaunchAuth::Online(session) => {
                Ok(VerifiedOwner::new(session.uuid, session.name, time::now()))
            }
            LaunchAuth::Offline { reason, .. } => Err(AuthError::Network {
                service: "the Minecraft services",
                message: reason,
            }),
        }
    }

    /// Renews every account whose token is due (launcher start), in the background.
    pub async fn refresh_due(&self) -> Vec<(String, Result<()>)> {
        let Ok(file) = self.load() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for a in file.accounts {
            let r = self.refresh(&a.uuid, false).await.map(|_| ());
            out.push((a.uuid, r));
        }
        out
    }
}

fn same_uuid(a: &str, b: &str) -> bool {
    a.replace('-', "").eq_ignore_ascii_case(&b.replace('-', ""))
}

fn upsert(file: &mut AccountsFile, profile: &Profile, now: u64) {
    let check = Some(OwnershipRecord {
        owns: true,
        checked_at: now,
    });
    match file.accounts.iter_mut().find(|a| a.uuid == profile.id) {
        Some(a) => {
            a.name = profile.name.clone();
            a.skins = profile.skins.clone();
            a.capes = profile.capes.clone();
            a.last_check = check;
            a.needs_sign_in = false;
        }
        None => file.accounts.push(AccountRecord {
            uuid: profile.id.clone(),
            name: profile.name.clone(),
            skins: profile.skins.clone(),
            capes: profile.capes.clone(),
            added_at: now,
            last_check: check,
            needs_sign_in: false,
        }),
    }
}

/// Test helpers: seed an account's state directly.
#[cfg(test)]
impl AccountManager {
    pub(crate) fn seed(
        &self,
        uuid: &str,
        name: &str,
        expires_at: u64,
        ownership: Option<OwnershipRecord>,
    ) {
        self.put_secrets(
            uuid,
            &AccountSecrets {
                v: 1,
                ms_refresh: Secret::new("refresh-old"),
                mc_access: Secret::new("mc-old"),
                mc_expires_at: expires_at,
                xuid: None,
                ownership,
            },
        )
        .unwrap();
        let mut file = self.load().unwrap();
        upsert(
            &mut file,
            &Profile {
                id: uuid.into(),
                name: name.into(),
                skins: vec![],
                capes: vec![],
            },
            0,
        );
        file.selected.get_or_insert_with(|| uuid.into());
        self.save(&file).unwrap();
    }
}
