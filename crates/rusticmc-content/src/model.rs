//! The common content model every source maps into.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What a piece of content is. The Horizon kinds follow the registry's (`horizon_mods::index::
/// Kind`); `Plugin` is a Rustic Server plugin (`.rplugin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    /// A mod. From Horizon's registry: a content mod (`.hmod`). From Modrinth: a Java mod jar
    /// (Fabric/Forge/NeoForge/Quilt), which cannot run on Horizon (see `compat`).
    Mod,
    /// A Horizon native mod (`.hnmod`): Rust compiled into the game.
    NativeMod,
    /// A modpack: Horizon's `.hpack`, Modrinth's `.mrpack`, or a Rustic pack.
    Modpack,
    #[serde(rename = "resourcepack")]
    ResourcePack,
    /// A Horizon shader pack (`.hshader`, WGSL).
    Shader,
    /// A `shaders/`-folder pack (Iris/OptiFine format, `.zip`).
    #[serde(rename = "shaderpack")]
    ShaderPack,
    /// A data pack, installed into a world (or the packs new worlds start with).
    #[serde(rename = "datapack")]
    DataPack,
    /// A Rustic Server plugin (`.rplugin`), installed into a server's `plugins/`.
    Plugin,
}

impl ContentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ContentKind::Mod => "mod",
            ContentKind::NativeMod => "native_mod",
            ContentKind::Modpack => "modpack",
            ContentKind::ResourcePack => "resourcepack",
            ContentKind::Shader => "shader",
            ContentKind::ShaderPack => "shaderpack",
            ContentKind::DataPack => "datapack",
            ContentKind::Plugin => "plugin",
        }
    }
}

/// Where content comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Modrinth,
    /// Horizon's registry (`horizon-rust/horizon-mods`'s `index.json`).
    Horizon,
    /// Rustic's own registry (Rustic Cloud).
    Rustic,
    /// A file the player added by hand.
    Local,
}

/// Fair-play standing of a project (`fairplay`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FairPlay {
    #[default]
    Clean,
    /// Not promoted (never in featured or recommended lists); installable with a notice.
    Flagged {
        #[serde(default)]
        reason: String,
    },
    /// Never promoted and never installed through Rustic (X-ray, auto-clickers, hacks).
    Blocked {
        #[serde(default)]
        reason: String,
    },
}

/// A search result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub source: SourceKind,
    pub id: String,
    pub slug: String,
    pub kind: ContentKind,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    // Lists and counts default when a source leaves them out (Rustic's curated entries may).
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    /// Java loaders (Modrinth), when it is a Java mod.
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub fair_play: FairPlay,
}

/// Hashes a file is checked against; at least one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hashes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
}

impl Hashes {
    pub fn is_empty(&self) -> bool {
        self.sha1.is_none() && self.sha256.is_none() && self.sha512.is_none()
    }
}

/// One downloadable file of a version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentFile {
    pub url: String,
    pub filename: String,
    pub size: Option<u64>,
    pub hashes: Hashes,
    pub primary: bool,
    /// Horizon registry: the minisign signature text, checked against the project's key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Required,
    Optional,
    Incompatible,
    /// Shipped inside the file already: nothing to install.
    Embedded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    /// A version requirement (Horizon modpacks pin an exact version; Rustic plugins use semver).
    pub version_req: Option<String>,
    /// Horizon modpacks pin each mod's SHA-256.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub kind: DependencyKind,
}

/// What Horizon's registry says about a version beyond its files.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonFacts {
    /// A content mod's API line; a native mod's `native-1/<horizon>/<mixins>`; a modpack's format.
    pub api: String,
    /// A native mod's hook points version (`NATIVE_HOOKS_API` it needs).
    pub hooks: Option<u32>,
    pub side: Option<String>,
    pub capabilities: Vec<String>,
    /// Horizon commits horizon-mixins refused this native mod on.
    pub broken_on: Vec<String>,
    /// The project's minisign key, when it signs.
    pub key: Option<String>,
}

/// One version of a project, from any source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentVersion {
    pub source: SourceKind,
    pub project_id: String,
    /// The version's own id (Modrinth's), or `<project>@<version>` elsewhere.
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: ContentKind,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<ContentFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// RFC 3339 or a date.
    #[serde(default)]
    pub published: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<HorizonFacts>,
}

impl ContentVersion {
    /// The file to install: the primary one, else the first.
    pub fn primary_file(&self) -> Option<&ContentFile> {
        self.files
            .iter()
            .find(|f| f.primary)
            .or_else(|| self.files.first())
    }

    /// `<source>:<project>`: one installed copy per key.
    pub fn key(&self) -> String {
        format!("{:?}:{}", self.source, self.project_id).to_lowercase()
    }
}

/// Where content is installed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
    /// A Horizon instance's game folder (the one holding `options.txt`).
    Instance { game_dir: PathBuf },
    /// A Horizon server folder (`horizon-server --dir`): `plugins/` for Rustic Server plugins.
    Server { server_dir: PathBuf },
    /// One world of an instance: `saves/<world>/datapacks/`.
    World { game_dir: PathBuf, world: String },
    /// The packs every new world of an instance starts with (`new-world-datapacks/`, which the
    /// game copies into each world it creates).
    NewWorlds { game_dir: PathBuf },
}

impl Target {
    /// The folder the target's ledger lives in.
    pub fn root(&self) -> &PathBuf {
        match self {
            Target::Instance { game_dir }
            | Target::World { game_dir, .. }
            | Target::NewWorlds { game_dir } => game_dir,
            Target::Server { server_dir } => server_dir,
        }
    }

    /// How the ledger names it.
    pub fn label(&self) -> String {
        match self {
            Target::Instance { .. } => "instance".into(),
            Target::Server { .. } => "server".into(),
            Target::World { world, .. } => format!("world:{world}"),
            Target::NewWorlds { .. } => "new_worlds".into(),
        }
    }
}

/// How well something runs where it would go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Runs as is.
    Compatible,
    /// Runs in part (the reasons say which part).
    Partial,
    /// Runs only in a game built with it (native mods).
    NeedsBuild,
    /// Doesn't run here; never offered for install.
    Incompatible,
    /// Revoked by its registry or refused by the fair-play policy.
    Blocked,
}

/// The badge the UI shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Badge {
    Works,
    Partly,
    /// A native mod whose `hooks` (or pin) fits the instance's Horizon.
    BuildsOnYourHorizon,
    /// A native mod that needs a newer `NATIVE_HOOKS_API`.
    NeedsNewerHorizon,
    /// horizon-mixins refused it on this very commit (`broken_on`).
    DoesntBuildOnYourHorizon,
    /// A Java mod: its data and assets can be ported (horizon-port-pack), its code can't run.
    NeedsPort,
    DoesntRun,
    /// Not for this target (a plugin on a vanilla Horizon server, a data pack with no world...).
    WrongTarget,
    Revoked,
    FairPlayBlocked,
}

/// A verdict, its badge and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Compatibility {
    pub verdict: Verdict,
    pub badge: Badge,
    pub reasons: Vec<String>,
}

impl Compatibility {
    pub fn new(verdict: Verdict, badge: Badge, reason: impl Into<String>) -> Self {
        Self {
            verdict,
            badge,
            reasons: vec![reason.into()],
        }
    }

    /// Offered for install (possibly with a warning).
    pub fn installable(&self) -> bool {
        matches!(self.verdict, Verdict::Compatible | Verdict::Partial)
    }
}
