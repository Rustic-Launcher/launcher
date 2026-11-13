//! What can stop a native build, each with a stable `code()` the UI maps to a message.
use std::path::{Path, PathBuf};

pub type Result<T, E = NativeError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    /// The player has not accepted the trust notice for this instance.
    #[error("native mods run unsandboxed: accept the trust notice for this instance first")]
    ConsentRequired,
    /// No usable Rust: the launcher can install it (after the player agreed), or what is missing
    /// must be installed by the player (a linker, system libraries).
    #[error("Rust {version} is needed to build native mods: {reason}")]
    ToolchainMissing { version: String, reason: String },
    #[error("installing Rust failed: {0}")]
    ToolchainInstall(String),
    #[error(
        "not enough free disk space to build native mods: {free_gib:.1} GiB free on the drive of {}, {needed_gib:.1} GiB needed",
        path.display()
    )]
    DiskSpace {
        path: PathBuf,
        free_gib: f64,
        needed_gib: f64,
    },
    /// The edition's source could not be prepared (download, checksum, layout).
    #[error("the Rustic edition's source: {0}")]
    Source(String),
    /// A `.hnmod` file failed its checks.
    #[error("{id}: {message}")]
    Bundle { id: String, message: String },
    /// horizon-mixins could not apply a mod (and it could not be left out).
    #[error("{}{message}", mod_id.as_deref().map(|m| format!("{m}: ")).unwrap_or_default())]
    Mixins {
        mod_id: Option<String>,
        message: String,
    },
    /// The compiler failed; `mods` are the mods the errors were traced to.
    #[error("the build failed{}: {excerpt}", if mods.is_empty() { String::new() } else { format!(" in {}", mods.join(", ")) })]
    Compile { mods: Vec<String>, excerpt: String },
    /// Every native mod of the instance was left out.
    #[error("no native mod could be built: {0}")]
    NothingToBuild(String),
    #[error("no native mod is turned on in this instance")]
    NoMods,
    #[error("a native build is already running")]
    Busy,
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Download(String),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Other(String),
}

impl NativeError {
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_owned(),
            source,
        }
    }

    /// A stable machine-readable code (`native.<code>` in the UI).
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConsentRequired => "consent_required",
            Self::ToolchainMissing { .. } => "toolchain_missing",
            Self::ToolchainInstall(_) => "toolchain_install",
            Self::DiskSpace { .. } => "disk_space",
            Self::Source(_) => "source",
            Self::Bundle { .. } => "bundle",
            Self::Mixins { .. } => "mixins",
            Self::Compile { .. } => "compile",
            Self::NothingToBuild(_) => "nothing_to_build",
            Self::NoMods => "no_mods",
            Self::Busy => "busy",
            Self::Cancelled => "cancelled",
            Self::Download(_) => "download",
            Self::Io { .. } => "io",
            Self::Other(_) => "other",
        }
    }
}

/// `io::Result` with the path in the error.
pub(crate) trait IoContext<T> {
    fn at(self, path: &Path) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn at(self, path: &Path) -> Result<T> {
        self.map_err(|e| NativeError::io(path, e))
    }
}
