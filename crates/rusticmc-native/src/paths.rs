//! Where native builds live, under the launcher's data folder: `<data>/native/`.
//!
//! ```text
//! native/
//!   rustup/  cargo/       the launcher's own Rust (RUSTUP_HOME, CARGO_HOME), when it installed it;
//!                         cargo/ is also the Cargo home of every build (crates downloaded once)
//!   mingw64/              Windows: the C compiler and linker of the launcher's Rust (WinLibs)
//!   downloads/            rustup-init, the source archive, while they download
//!   tools/<rev>/bin/      horizon-mixins, built once per commit
//!   source/<id>/          the edition's pristine source (Horizon + patches, companion)
//!   work/                 one build's bundles and horizon-mixins' output
//!   stage/                what Cargo compiles (only changed files are written)
//!   target/               Cargo's outputs: the compile cache, cleared above the size limit
//!   build.log             the last build's full log
//! ```
//!
//! Nothing here is an instance's: the built client and its record go into the instance's own
//! folder ([`crate::instance`]).
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct NativeRoot {
    root: PathBuf,
}

impl NativeRoot {
    /// `<data>/native`.
    pub fn new(data: &Path) -> Self {
        Self {
            root: data.join("native"),
        }
    }

    /// A root at exactly `dir` (tests, the CLI).
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { root: dir.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn rustup_home(&self) -> PathBuf {
        self.root.join("rustup")
    }
    pub fn cargo_home(&self) -> PathBuf {
        self.root.join("cargo")
    }
    pub fn mingw(&self) -> PathBuf {
        self.root.join("mingw64")
    }
    pub fn downloads(&self) -> PathBuf {
        self.root.join("downloads")
    }
    pub fn tools(&self, rev: &str) -> PathBuf {
        self.root.join("tools").join(&rev[..rev.len().min(12)])
    }
    pub fn sources(&self) -> PathBuf {
        self.root.join("source")
    }
    pub fn work(&self) -> PathBuf {
        self.root.join("work")
    }
    pub fn stage(&self) -> PathBuf {
        self.root.join("stage")
    }
    pub fn target(&self) -> PathBuf {
        self.root.join("target")
    }
    pub fn log(&self) -> PathBuf {
        self.root.join("build.log")
    }
    /// The folder build tools get as TMP/TEMP/TMPDIR.
    pub fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }
}
