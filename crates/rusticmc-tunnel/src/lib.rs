//! Rustic Relay tunnel client.
//!
//! A server running on a player's computer becomes reachable at a hostname
//! (`play.example.com`, a temporary `brave-otter-42.<temp-domain>`, ...) without port forwarding:
//! this crate keeps an outbound TLS tunnel to Rustic Relay open and forwards each player connection
//! the relay routes to it to the local server. Protocol and deployment: `launcher/docs/RELAY.md`.
//!
//! ```no_run
//! # async fn demo() -> Result<(), rusticmc_tunnel::TunnelError> {
//! use rusticmc_tunnel::{StaticToken, Tunnel, TunnelConfig, TunnelEvent};
//! use std::sync::Arc;
//! let config = TunnelConfig::new(
//!     "tunnel.relay.example:7443",
//!     "127.0.0.1:25565".parse().unwrap(),
//!     Arc::new(StaticToken("<token from Rustic Cloud>".into())),
//! );
//! let mut tunnel = Tunnel::connect(config)?;
//! let mut events = tunnel.take_events().unwrap();
//! while let Ok(event) = events.recv().await {
//!     if let TunnelEvent::Connected { routes, .. } = event {
//!         println!("reachable at {:?}", routes.hostnames);
//!         break;
//!     }
//! }
//! tunnel.shutdown().await;
//! # Ok(()) }
//! ```
pub mod client;
pub mod mux;
pub mod provider;
pub mod proxy;
pub mod splice;
pub mod tls;
pub mod wire;

pub use client::{
    Backoff, FnTokenSource, StaticToken, TokenSource, TokenSourceError, Tunnel, TunnelConfig,
    TunnelError, TunnelEvent, TunnelHandle, TunnelState, TunnelStats, TunnelStatus,
};
pub use provider::{RelayTunnelProvider, TunnelProvider, TunnelRequest};
pub use tls::TlsRoots;
pub use wire::{ErrorCode, Intent, Limits, NoticeCode, Routes, TempRoute};
