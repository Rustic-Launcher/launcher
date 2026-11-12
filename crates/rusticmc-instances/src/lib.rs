//! Rustic Launcher instances: Horizon builds, preparation, launch.
//!
//! - [`builds`] + [`signing`]: list and install builds from GitHub releases or a channel index,
//!   checked against signed SHA256SUMS (Horizon's layout, horizon/docs/release-flow.md).
//! - [`launch_api`]: `--launch-api` of `horizon-client` and `horizon-prepare`.
//! - [`java`]: Java discovery for the preparer.
//! - [`prepare`]: `horizon-prepare --prepare ... --progress json`, `--check`, the owner-gated
//!   server jar.
//! - [`launch`]: `horizon-client` with the session on stdin, logs, exit codes, crash reports.
//! - [`model`]: instances on disk (settings, content, icon, playtime).
//! - [`manager`]: all of it together.

pub mod builds;
pub mod error;
pub mod game_files;
pub mod java;
pub mod launch;
pub mod launch_api;
pub mod manager;
pub mod model;
pub mod prepare;
pub mod process;
pub mod signing;

pub use builds::{BuildManager, BuildRelease, BuildSource, InstallStage, InstalledBuild};
pub use error::{InstanceError, Result};
pub use launch::{ExitOutcome, GameEvent, LaunchRequest, QuickPlay, RunningGame, WindowOptions};
pub use manager::{InstanceGameEvent, InstanceManager, LaunchIdentity, PrepareProgress};
pub use model::{
    BuildChoice, ContentEntry, Instance, InstanceIcon, InstancePatch, InstanceSettings,
    InstanceSource, InstanceStore, LastServer,
};
pub use prepare::{CheckResult, GameFiles, PrepareEvent};
