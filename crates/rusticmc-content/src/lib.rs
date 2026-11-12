//! Rustic Launcher's content: where mods, packs, shaders, data packs and plugins come from,
//! whether they run on the player's Horizon, and putting them where Horizon reads them
//! (launcher/docs/CONTENT.md).
//!
//! - [`model`]: the common model (kinds, sources, versions, files, dependencies, targets,
//!   verdicts and badges).
//! - [`sources`]: Modrinth API v2, Horizon's registry index, Rustic's registry.
//! - [`horizon_env`]: what the instance's Horizon loads (`horizon-client --launch-api`).
//! - [`compat`]: the verdicts, with reasons, per kind.
//! - [`resolve`]: dependency auto-resolution.
//! - [`install`]: installs into an instance, a world, new worlds or a server; enable/disable;
//!   uninstall. [`ledger`] remembers what was installed; [`switches`] are Horizon's on/off files.
//! - [`updates`]: update checks.
//! - [`mrpack`]: `.mrpack` import (the Horizon-compatible part, with a report).
//! - [`rustic_pack`]: Rustic packs (export/import).
//! - [`plugins`]: Rustic Server `.rplugin` validation, dependency resolution, install.
//! - [`fairplay`]: X-ray, auto-clickers and hacks are never promoted nor installed.
//! - [`download`]: the [`download::Downloader`] trait over the launcher's download manager.
#![forbid(unsafe_code)]

pub mod compat;
pub mod download;
pub mod error;
pub mod fairplay;
pub mod horizon_env;
pub mod inspect;
pub mod install;
pub mod ledger;
pub mod model;
pub mod mrpack;
pub mod plugins;
pub mod resolve;
pub mod rustic_pack;
pub mod sources;
pub mod switches;
pub mod updates;

pub use error::{ContentError, Result};
pub use horizon_env::HorizonEnv;
pub use model::*;
