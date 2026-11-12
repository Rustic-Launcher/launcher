//! Rustic Relay tunnel tokens. The relay accepts a tunnel only with a token Rustic Cloud signs for
//! the server (HANDOFF.md relay → cloud: `POST /v1/servers/{id}/tunnel-token`). The cloud client
//! isn't wired into the app yet, so turning the relay on answers "not available yet" instead of
//! starting a tunnel that can only fail.

use crate::state::{AppState, Res};
use crate::util::not_available_yet;
use std::sync::Arc;

pub fn token_source(_state: &AppState) -> Res<Arc<dyn rusticmc_tunnel::TokenSource>> {
    Err(not_available_yet(
        "The Rustic Relay (it needs the Rustic Cloud tunnel-token client)",
    ))
}
