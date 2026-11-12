//! Game accounts (Microsoft; offline play is the owner under its official name, ADR 0146) and the
//! Rustic account.

use crate::dto::*;
use crate::state::{AppState, LoginOutcome, Res, State};
use crate::util::{iso, not_available_yet};
use rusticmc_auth::{AccountState, AuthError};
use rusticmc_launcher_core::{IpcError, IpcErrorCode, time};
use std::sync::Arc;
use tauri::Emitter;
use tauri_plugin_opener::OpenerExt;

/// Every Microsoft account, as the UI lists them.
pub fn accounts(state: &AppState) -> Res<Vec<Account>> {
    let snap = state.accounts.snapshot()?;
    let window = state.accounts.policy().window_seconds;
    let now = time::now();
    // Who plays, as the launch sees it (a stale selection falls back to the first account).
    let active = match state.accounts.active()? {
        rusticmc_auth::ActiveIdentity::Microsoft(uuid) => Some(uuid),
        rusticmc_auth::ActiveIdentity::None => None,
    };
    let out: Vec<Account> = snap
        .accounts
        .iter()
        .map(|a| {
            let checked = a.last_ownership_check;
            let owns = a.state != AccountState::NotOwned && checked.is_some();
            let status = match (checked, a.state) {
                (None, _) => "never_checked",
                (_, AccountState::NotOwned) => "failed",
                (Some(at), _) if now.saturating_sub(at) > window => "expired",
                _ => "valid",
            };
            Account {
                id: a.uuid.clone(),
                kind: "microsoft",
                username: a.name.clone(),
                uuid: a.uuid.clone(),
                active: active.as_deref() == Some(a.uuid.as_str()),
                ownership: Some(Ownership {
                    owns_game: owns,
                    status,
                    checked_at: checked.map(iso),
                    next_check_at: checked.map(|c| iso(c + window)),
                }),
                skin: Skin::active(&a.skins),
            }
        })
        .collect();
    Ok(out)
}

fn changed(state: &AppState) {
    if let Ok(list) = accounts(state) {
        let _ = state.app.emit("accounts-changed", list);
    }
}

#[tauri::command]
pub fn auth_accounts_list(state: State<'_>) -> Res<Vec<Account>> {
    accounts(&state)
}

/// Runs a started flow to its end in the background; `auth_microsoft_login_poll` reads it.
fn spawn_completion(state: Arc<AppState>, flow_id: String) {
    state
        .logins
        .lock()
        .expect("logins")
        .insert(flow_id.clone(), LoginOutcome::Pending);
    tauri::async_runtime::spawn(async move {
        let result = state.accounts.complete(&flow_id).await;
        let ok = result.is_ok();
        match &result {
            Ok(_) => tracing::info!("Microsoft sign-in completed"),
            // Messages never carry a token (rusticmc_auth::AuthError).
            Err(e) => tracing::warn!(reason = e.code(), "Microsoft sign-in failed: {e}"),
        }
        state
            .logins
            .lock()
            .expect("logins")
            .insert(flow_id, LoginOutcome::Done(result));
        if ok {
            changed(&state);
        }
    });
}

#[tauri::command]
pub async fn auth_microsoft_login_start(state: State<'_>) -> Res<MicrosoftLoginStart> {
    let flow = state.accounts.start_device_code(None).await?;
    spawn_completion(state.inner().clone(), flow.flow_id.clone());
    Ok(MicrosoftLoginStart {
        flow_id: flow.flow_id,
        verification_uri: flow.verification_uri,
        user_code: flow.user_code,
        expires_in_s: flow.expires_at.saturating_sub(time::now()),
    })
}

/// Browser sign-in (authorization code + PKCE, loopback redirect): opens the system browser and
/// is polled like the device code (IPC.md addition, ADR 0010's primary flow).
#[tauri::command]
pub async fn auth_microsoft_browser_login_start(
    state: State<'_>,
) -> Res<MicrosoftBrowserLoginStart> {
    let flow = state.accounts.start_browser(None).await?;
    state
        .app
        .opener()
        .open_url(flow.authorize_url.clone(), None::<&str>)
        .map_err(|e| IpcError::internal(e.to_string()))?;
    spawn_completion(state.inner().clone(), flow.flow_id.clone());
    Ok(MicrosoftBrowserLoginStart {
        flow_id: flow.flow_id,
        authorize_url: flow.authorize_url,
    })
}

#[tauri::command]
pub fn auth_microsoft_login_poll(state: State<'_>, args: FlowArgs) -> Res<MicrosoftLoginPoll> {
    let outcome = {
        let mut logins = state.logins.lock().expect("logins");
        match logins.get(&args.flow_id) {
            None => return Err(IpcError::not_found("no sign-in with that id")),
            Some(LoginOutcome::Pending) => {
                return Ok(MicrosoftLoginPoll {
                    status: "pending",
                    account: None,
                    error: None,
                });
            }
            Some(LoginOutcome::Done(_)) => logins.remove(&args.flow_id),
        }
    };
    let Some(LoginOutcome::Done(result)) = outcome else {
        unreachable!("checked above")
    };
    let poll = |status, error: Option<String>| MicrosoftLoginPoll {
        status,
        account: None,
        error,
    };
    match result {
        Ok(view) => {
            let account = accounts(&state)?.into_iter().find(|a| a.id == view.uuid);
            Ok(MicrosoftLoginPoll {
                status: "success",
                account,
                error: None,
            })
        }
        Err(e @ (AuthError::Expired | AuthError::Timeout)) => {
            Ok(poll("expired", Some(e.to_string())))
        }
        Err(e @ (AuthError::Declined | AuthError::Cancelled | AuthError::WrongAccount { .. })) => {
            Ok(poll("denied", Some(e.to_string())))
        }
        Err(e @ (AuthError::NotOwned | AuthError::NoProfile)) => {
            Ok(poll("no_ownership", Some(e.to_string())))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn auth_ownership_check(state: State<'_>, args: AccountArgs) -> Res<Account> {
    let result = state.accounts.verified_owner(Some(&args.account_id)).await;
    changed(&state);
    result?;
    accounts(&state)?
        .into_iter()
        .find(|a| a.id == args.account_id)
        .ok_or_else(|| IpcError::not_found("no such account"))
}

#[tauri::command]
pub fn auth_set_active(state: State<'_>, args: AccountArgs) -> Res<Vec<Account>> {
    state.accounts.select(&args.account_id)?;
    changed(&state);
    accounts(&state)
}

#[tauri::command]
pub async fn auth_logout(state: State<'_>, args: AccountArgs) -> Res<Vec<Account>> {
    state.accounts.remove(&args.account_id).await?;
    changed(&state);
    accounts(&state)
}

async fn session(state: &AppState) -> Res<RusticSession> {
    Ok(match state.rustic.status().await? {
        rusticmc_auth::RusticAccountStatus::SignedIn { user } => RusticSession {
            signed_in: true,
            user_id: Some(user.id),
            username: Some(user.display_name),
            email: None,
        },
        _ => RusticSession {
            signed_in: false,
            user_id: None,
            username: None,
            email: None,
        },
    })
}

#[tauri::command]
pub async fn auth_rustic_session(state: State<'_>) -> Res<RusticSession> {
    session(&state).await
}

/// Browser login to Rustic Cloud; the cloud redirects to `rustic://auth?ticket=...`, which the
/// deep-link handler completes. Answers the current session (signed out until then).
#[tauri::command]
pub async fn auth_rustic_login(state: State<'_>) -> Res<RusticSession> {
    let url = state.rustic.link().oauth_start("rustic").await?;
    state
        .app
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|e| IpcError::internal(e.to_string()))?;
    session(&state).await
}

#[tauri::command]
pub async fn auth_rustic_logout(state: State<'_>) -> Res<RusticSession> {
    state.rustic.logout().await?;
    session(&state).await
}

#[derive(serde::Serialize)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_at: String,
}

#[tauri::command]
pub async fn auth_rustic_access_token(state: State<'_>) -> Res<AccessToken> {
    let (token, expires) = state
        .rustic
        .access_token_with_expiry()
        .await
        .map_err(|e| match e {
            AuthError::Cloud(m) if m.contains("Not signed in") => {
                IpcError::with_reason(IpcErrorCode::Unauthorized, m, "auth.cloud")
            }
            AuthError::Cloud(_) => not_available_yet("Rustic accounts"),
            other => other.into(),
        })?;
    Ok(AccessToken {
        // The one place the token leaves Rust: the webview's own cloud client needs it.
        access_token: token.expose().to_owned(),
        expires_at: iso(expires),
    })
}

#[derive(serde::Deserialize)]
pub struct RusticSignupArgs {
    pub email: String,
    pub password: String,
    pub username: String,
    /// `YYYY-MM-DD`.
    pub date_of_birth: String,
    /// ISO 3166-1 alpha-2.
    pub country: String,
    pub locale: Option<String>,
    /// The user ticked "I accept the privacy policy" (required).
    pub accept_privacy: bool,
}

#[derive(serde::Serialize)]
pub struct RusticSignupResult {
    pub email_verification_required: bool,
    pub guardian_required: bool,
}

/// Creates a Rustic account; the cloud mails a verification link. Sign in afterwards.
#[tauri::command]
pub async fn auth_rustic_signup(
    state: State<'_>,
    args: RusticSignupArgs,
) -> Res<RusticSignupResult> {
    if !args.accept_privacy {
        return Err(IpcError::with_reason(
            IpcErrorCode::InvalidArgument,
            "The privacy policy must be accepted.",
            "auth.privacy",
        ));
    }
    let date_of_birth = chrono::NaiveDate::parse_from_str(&args.date_of_birth, "%Y-%m-%d")
        .map_err(|_| {
            IpcError::with_reason(
                IpcErrorCode::InvalidArgument,
                "Date of birth must be YYYY-MM-DD.",
                "auth.date_of_birth",
            )
        })?;
    let req = rusticmc_api_client::types::SignupRequest {
        email: args.email.trim().to_owned(),
        password: args.password,
        username: args.username.trim().to_owned(),
        date_of_birth,
        country: args.country.trim().to_uppercase(),
        privacy_policy_version: crate::cloud_link::PRIVACY_POLICY_VERSION.to_owned(),
        locale: args.locale,
    };
    let resp = state.cloud.signup(&req).await.map_err(cloud_ipc)?;
    Ok(RusticSignupResult {
        email_verification_required: resp.email_verification_required,
        guardian_required: resp.guardian_required,
    })
}

#[derive(serde::Deserialize)]
pub struct RusticLoginArgs {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct RusticLoginResult {
    pub mfa_required: bool,
    pub session: RusticSession,
}

/// Email + password sign-in; tokens go to the OS keychain. With TOTP on, answers
/// `mfa_required` and waits for `auth_rustic_login_mfa`.
#[tauri::command]
pub async fn auth_rustic_login_password(
    state: State<'_>,
    args: RusticLoginArgs,
) -> Res<RusticLoginResult> {
    let password = rusticmc_launcher_core::Secret::new(args.password);
    let mfa = state
        .rustic
        .login(args.email.trim(), &password)
        .await
        .map_err(cloud_ipc)?;
    let mfa_required = mfa.is_some();
    *state.rustic_mfa.lock().expect("mfa") = mfa;
    Ok(RusticLoginResult {
        mfa_required,
        session: session(&state).await?,
    })
}

#[derive(serde::Deserialize)]
pub struct RusticMfaArgs {
    pub code: String,
}

#[tauri::command]
pub async fn auth_rustic_login_mfa(state: State<'_>, args: RusticMfaArgs) -> Res<RusticSession> {
    let token = state
        .rustic_mfa
        .lock()
        .expect("mfa")
        .take()
        .ok_or_else(|| {
            IpcError::with_reason(
                IpcErrorCode::NotFound,
                "No sign-in is waiting for a code.",
                "auth.mfa",
            )
        })?;
    state
        .rustic
        .login_mfa(&token, args.code.trim())
        .await
        .map_err(cloud_ipc)?;
    session(&state).await
}

#[derive(serde::Deserialize)]
pub struct RusticEmailArgs {
    pub email: String,
}

#[tauri::command]
pub async fn auth_rustic_resend_verification(state: State<'_>, args: RusticEmailArgs) -> Res<()> {
    state
        .cloud
        .resend_verification(args.email.trim())
        .await
        .map_err(cloud_ipc)
}

/// Cloud answers keep their message (wrong password, unverified email…) for the UI.
fn cloud_ipc(e: AuthError) -> IpcError {
    match e {
        AuthError::Cloud(m) => {
            IpcError::with_reason(IpcErrorCode::InvalidArgument, m, "auth.cloud")
        }
        other => other.into(),
    }
}
