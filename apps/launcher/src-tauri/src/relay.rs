//! Rustic Relay: a free temporary address for a server on this PC, no port to open. The relay
//! accepts a tunnel only with a token Rustic Cloud signs for the server, so turning it on:
//!
//! 1. registers the local server in the cloud once (`POST /v1/servers`, its id kept in the
//!    server's profile as `cloud_id`);
//! 2. reserves the temporary hostname (`POST /v1/servers/{id}/temp-hostname`), which also hands
//!    the relay's address and a first token;
//! 3. before every tunnel connection, asks a fresh token (`POST /v1/servers/{id}/tunnel-token`),
//!    or renews the hostname's lease once it nears its end (`…/temp-hostname/renew`), which
//!    hands a token too.
//!
//! The player must be signed in to a Rustic account (the server belongs to it).

use crate::cloud_link::ApiCloud;
use crate::state::{AppState, Res};
use rusticmc_launcher_core::{IpcError, IpcErrorCode, Secret};
use rusticmc_tunnel::{FnTokenSource, TokenSource, TokenSourceError};
use std::sync::{Arc, Mutex};

/// What a relay session needs: where the relay listens and where tokens come from.
pub struct RelaySession {
    pub relay_addr: String,
    pub tokens: Arc<dyn TokenSource>,
}

fn sign_in_first() -> IpcError {
    IpcError::with_reason(
        IpcErrorCode::Unauthorized,
        "Sign in to your Rustic account (Settings > Accounts) to get a temporary address.",
        "relay.rustic_required",
    )
}

fn cloud_error(e: rusticmc_auth::AuthError) -> IpcError {
    match e {
        rusticmc_auth::AuthError::Cloud(m) => {
            IpcError::with_reason(IpcErrorCode::Unavailable, m, "relay.cloud")
        }
        other => other.into(),
    }
}

/// A display name the cloud accepts (3 to 32 characters).
pub fn cloud_name(name: &str) -> String {
    let n: String = name.trim().chars().take(32).collect();
    if n.chars().count() < 3 {
        "Rustic server".into()
    } else {
        n
    }
}

/// Registers the server if needed, reserves its temporary hostname and builds the token source.
pub async fn session(state: &Arc<AppState>, id: &str) -> Res<RelaySession> {
    let access = state
        .rustic
        .access_token()
        .await
        .map_err(|_| sign_in_first())?;
    let profile = state.servers.profile(id)?;
    let cloud_id = match profile.cloud_id.clone() {
        Some(c) => c,
        None => {
            let created = state
                .cloud
                .create_server(&access, &cloud_name(&profile.name))
                .await
                .map_err(cloud_error)?;
            let c = created.id.to_string();
            state
                .servers
                .update_profile(id, |p| p.cloud_id = Some(c.clone()))?;
            c
        }
    };
    let lease = state
        .cloud
        .temp_hostname(&access, &cloud_id)
        .await
        .map_err(cloud_error)?;
    let renew_before = Arc::new(Mutex::new(lease.renew_before));
    let st = state.clone();
    let tokens = FnTokenSource(move || {
        let st = st.clone();
        let cloud_id = cloud_id.clone();
        let renew_before = renew_before.clone();
        async move { next_token(&st.rustic, &st.cloud, &cloud_id, &renew_before).await }
    });
    Ok(RelaySession {
        relay_addr: lease.relay_token.relay_addr,
        tokens: Arc::new(tokens),
    })
}

async fn next_token(
    rustic: &rusticmc_auth::cloud::RusticAccount,
    cloud: &ApiCloud,
    cloud_id: &str,
    renew_before: &Mutex<chrono::DateTime<chrono::Utc>>,
) -> Result<String, TokenSourceError> {
    let access: Secret = rustic
        .access_token()
        .await
        .map_err(|e| TokenSourceError(e.to_string()))?;
    let due = *renew_before.lock().expect("lease") <= chrono::Utc::now();
    if due {
        let lease = cloud
            .temp_hostname_renew(&access, cloud_id)
            .await
            .map_err(|e| TokenSourceError(e.to_string()))?;
        *renew_before.lock().expect("lease") = lease.renew_before;
        return Ok(lease.relay_token.token);
    }
    cloud
        .tunnel_token(&access, cloud_id)
        .await
        .map(|t| t.token)
        .map_err(|e| TokenSourceError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::cloud_name;

    #[test]
    fn names_fit_the_cloud() {
        assert_eq!(cloud_name("Les Copains SMP"), "Les Copains SMP");
        assert_eq!(cloud_name("  a "), "Rustic server");
        assert_eq!(cloud_name(&"x".repeat(40)).chars().count(), 32);
    }
}
