//! Rustic Launcher servers.
//!
//! - [`supervisor`]: `horizon-server`'s command line (init, config set, start/stop).
//! - [`control`]: its control API (`/api/v1`: status, metrics, logs, events, command, players and
//!   moderation, backups, restore, config, crash reports).
//! - [`console`]: command completion from the prepared `commands.json`.
//! - [`plugins`]: Rustic Server `.rplugin` management.
//! - [`network`], [`invite`]: LAN addresses, UPnP/NAT-PMP, reachability, invite codes;
//!   relay through `rusticmc-tunnel`.
//! - [`dns`], [`porkbun`]: domains (required records, verification, Porkbun API, purchase).
//! - [`provider`]: `rusticmc_hosting::HostingProvider` for this computer's servers.
//! - [`manager`]: local servers.
//! - [`engine`]: which `rustic-server` program a Rustic Server runs.

pub mod console;
pub mod control;
pub mod dns;
pub mod engine;
pub mod error;
pub mod hosting;
pub mod invite;
pub mod manager;
pub mod network;
pub mod plugins;
pub mod porkbun;
pub mod provider;
pub mod supervisor;

pub use error::{Result, ServerError};
pub use hosting::{CreateServer, ServerKind};
pub use manager::{ServerManager, ServerProfile};
pub use provider::LocalServers;
