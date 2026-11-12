//! Rustic Launcher core: the pieces every other launcher crate builds on.
//!
//! - [`paths`]: per-OS data, config, cache and log folders (`directories`), overridable for tests
//!   and portable installs with `RUSTIC_LAUNCHER_HOME`.
//! - [`config`]: the launcher's versioned TOML configuration, migrated on load.
//! - [`error`]: the shared error type.
//! - [`logging`]: tracing to the console and to daily rotated files.
//! - [`download`]: a concurrent, resumable, verifying download queue with a content-addressed cache,
//!   global progress events and cancellation.
//! - [`http`]: the one way to build an HTTP client (rustls with the ring provider).
//! - [`secret`]: [`secret::Secret`], a string that never prints.
//! - [`fsutil`], [`time`]: small helpers (atomic writes, unix time).

pub mod config;
pub mod download;
pub mod error;
pub mod fsutil;
pub mod http;
pub mod ipc;
pub mod logging;
pub mod paths;
pub mod secret;
pub mod time;

pub use config::LauncherConfig;
pub use download::{
    Checksum, DownloadError, DownloadEvent, DownloadManager, DownloadRequest, GlobalProgress,
    HashAlgo,
};
pub use error::{CoreError, Result};
pub use ipc::{IpcError, IpcErrorCode};
pub use paths::LauncherPaths;
/// The HTTP crate every launcher crate shares (one version, one TLS setup).
pub use reqwest;
pub use secret::Secret;
pub use tokio_util::sync::CancellationToken;

/// The product name shown to users (never "Minecraft").
pub const PRODUCT_NAME: &str = "Rustic Launcher";
/// The application identifier (Tauri bundle id, keychain service prefix). Placeholder until the
/// domain name is bought (launcher/docs/TODO_HUMAN.md).
pub const APP_ID: &str = "gg.rustic.launcher";
/// This launcher's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `User-Agent` sent with every request.
pub fn user_agent() -> String {
    format!("RusticLauncher/{VERSION} (+https://github.com/Rustic-Launcher/launcher)")
}
