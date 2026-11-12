//! Rustic Launcher accounts.
//!
//! - [`api`]: the token chain (horizon/docs/microsoft-account.md): Microsoft OAuth (device code,
//!   or authorization code + PKCE with a loopback redirect, [`loopback`]) → Xbox Live → XSTS →
//!   Minecraft services → entitlements → profile.
//! - [`accounts`]: several accounts, one selected, silent renewal, the owner gate before every
//!   launch, offline profiles only while [`ownership`]'s window is open.
//! - [`store`]: secrets in the OS keychain only (`keyring`), split under Credential Manager's
//!   item limit.
//! - [`session`]: the session JSON handed to the game on stdin, and [`session::VerifiedOwner`].
//! - [`cloud`]: the Rustic account (Rustic Cloud), behind the [`cloud::CloudLink`] seam.

pub mod accounts;
pub mod api;
pub mod cloud;
pub mod config;
pub mod error;
pub mod loopback;
pub mod ownership;
pub mod session;
pub mod store;

pub use accounts::{
    AccountManager, AccountState, AccountView, AccountsSnapshot, ActiveIdentity, AuthEvent,
    AuthPhase, BrowserFlow, DeviceCodeFlow,
};
pub use api::AuthApi;
pub use cloud::{CloudLink, RusticAccount, RusticAccountStatus, UnavailableCloud};
pub use config::{AuthConfig, Endpoints, PollTiming};
pub use error::{AuthError, Result};
pub use ownership::{OfflineAccess, OwnershipPolicy, OwnershipRecord};
pub use session::{GameSession, LaunchAuth, VerifiedOwner};
pub use store::{KeyringStore, MemoryStore, SecretStore};

#[cfg(test)]
mod tests;
