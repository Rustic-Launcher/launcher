//! Rustic Launcher hosting: one [`HostingProvider`] abstraction over three ways to host a Horizon
//! (or Rustic Server) server.
//!
//! | Provider | Status | What it does |
//! |---|---|---|
//! | [`LocalProvider`] | adapter | Forwards to the desktop's local-server backend (`rusticmc-servers`). |
//! | [`RemoteSshProvider`] | real | Installs and manages a server on the user's Linux VPS over SSH/SFTP. |
//! | [`RusticCloudProvider`] | stub | "Coming soon": typed `NotAvailableYet` errors, API shape documented. |
//!
//! Design and security model: `launcher/docs/HOSTING.md`, ADRs 0120-0125; IPC commands:
//! `launcher/docs/IPC.md` ("Hosting"). Horizon facts used here come from `horizon/docs/hosting.md`,
//! `docs/api/server-reference.md` and `crates/horizon-server` (nothing invented: the supervisor CLI,
//! its `--json` outputs, `.horizon/api.json` and `/api/v1`).

pub mod access;
pub mod cloud;
pub mod error;
pub mod local;
pub mod model;
pub mod plan;
pub mod platform;
pub mod provider;
pub mod registry;
pub mod remote;
pub mod rplugin;
pub mod secret;
pub mod shell;
pub mod slp;
pub mod ssh;
pub mod systemd;

pub use cloud::RusticCloudProvider;
pub use error::{ErrorPayload, HostingError, Result};
pub use local::LocalProvider;
pub use model::*;
pub use provider::{
    Capabilities, ConsoleSubscription, HostingProvider, ProviderDescriptor, descriptors,
};
pub use remote::{AuthMethod, RemoteInstallOptions, RemoteProfile, RemoteSshProvider};
pub use secret::{KeyringStore, MemoryStore, Secret, SecretStore};
pub use ssh::hostkey::{HostKey, KnownHosts};
pub use ssh::{SshAuth, SshTarget};
