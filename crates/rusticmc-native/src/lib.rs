//! Native mods for the Rustic edition (ADR 0157): Horizon's native mods are horizon-mixins bundles
//! that change the game's Rust source before it is compiled, so an instance that has any plays a
//! client compiled for it, on the player's computer, as Horizon's own launcher does
//! (horizon/docs/native-mods.md, "What the launcher does"). Instances without native mods keep the
//! prebuilt, signed client.
//!
//! The pipeline ([`pipeline::Builder::build`]):
//!
//! 1. **Consent** ([`instance::Consent`]): native code is not sandboxed; nothing is built before the
//!    player accepted the trust notice for that instance.
//! 2. **Rust** ([`toolchain`]): the Rust release Horizon's launcher pins ([`toolchain::RUST_VERSION`]),
//!    either already on the computer (rustup or a plain install of exactly that release, with a
//!    linker) or installed by the launcher into its own folder with rustup-init (checked against
//!    the SHA-256 static.rust-lang.org publishes beside it), after the player agreed.
//! 3. **Source** ([`source`]): exactly what the prebuilt client was built from: Horizon at the
//!    pinned commit with Rustic's patches applied, the Rustic companion and the edition crate, as
//!    one archive named by the edition bundle (`rustic-source.json`, SHA-256 checked), or, for
//!    development, the local checkouts.
//! 4. **Mixins** ([`mixins`]): Horizon's own tool (horizon-mixins, MIT), built once at the commit
//!    Horizon's launcher pins for this Horizon, applies the instance's bundles to the source. A mod
//!    the tool refuses is left out and the others are tried again ([`pipeline`]).
//! 5. **Stage** ([`stage`]): only the files whose content changed are written, so Cargo (which goes
//!    by file times) recompiles only what the mods touched.
//! 6. **Compile** ([`compile`]): `cargo build --locked --profile fast -p rustic-edition`, progress
//!    read from Cargo, errors traced back to the mod whose helper fails ([`diagnostics`]).
//! 7. **Install**: the client goes into the instance's own folder with a record
//!    ([`instance::BuildRecord`]) whose key says when a rebuild is needed.
//! 8. **Cache** ([`cache`]): compiler outputs are cleared after a build when they exceed the limit
//!    (30 GiB by default, like Horizon).
//!
//! Some parts follow Horizon's launcher (`horizon/launcher/src/native_mods*.rs`,
//! `rust_toolchain.rs`, MIT, Horizon contributors), rewritten for Rustic: the bundle re-pin,
//! the staged-file rule, the environment Cargo runs in, the WinLibs pin, the compile-error owner
//! rule. See docs/THIRD_PARTY.md.

pub mod bundle;
pub mod cache;
pub mod compile;
pub mod diagnostics;
pub mod disk;
pub mod error;
pub mod instance;
pub mod mixins;
pub mod paths;
pub mod pipeline;
pub mod process;
pub mod progress;
pub mod source;
pub mod stage;
pub mod toolchain;

pub use error::{NativeError, Result};
pub use instance::{BuildRecord, Consent, NativeModFile};
pub use paths::NativeRoot;
pub use pipeline::{BuildRequest, Builder};
pub use progress::{Phase, Progress};
pub use source::{LocalSource, SourceSpec};
pub use toolchain::{Toolchain, ToolchainStatus};

/// The Cargo profile native builds compile with: Horizon's `fast` (incremental, 16 codegen units,
/// no cross-crate LTO), which Horizon's launcher uses for players ("performance") and the edition
/// workspace declares too. A mod change recompiles only the crates it touched.
pub const PROFILE: &str = "fast";

/// The edition's package and program (client-mod/rustic-companion/edition).
pub const EDITION_PACKAGE: &str = "rustic-edition";
pub const EDITION_BIN: &str = "rustic-client";

/// `name` with the platform's executable suffix.
pub fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}
