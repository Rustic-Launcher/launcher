//! Typed client for the Rustic Cloud API.
//!
//! - [`types`]: the JSON wire types, shared with the server (which derives `openapi.json` from
//!   them, so schema names equal type names).
//! - [`realtime`]: WebSocket frames of the realtime service.
//! - [`Client`] (feature `client`, default): one async method per OpenAPI operation, generated
//!   from `services/api/openapi.json` into `src/generated.rs` by
//!   `cargo run -p rusticmc-cloud-api --bin gen-openapi` (see ADR 0082). Do not edit
//!   `generated.rs` by hand: a test in the API crate fails when it is stale.

pub mod realtime;
pub mod types;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
#[rustfmt::skip]
mod generated;

#[cfg(feature = "client")]
pub use client::{Client, ClientError};
#[cfg(feature = "client")]
pub use generated::OPERATIONS;
