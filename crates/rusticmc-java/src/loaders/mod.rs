//! Mod loaders, each through its own official public metadata:
//!
//! - **Fabric** (`meta.fabricmc.net/v2`) and **Quilt** (`meta.quiltmc.org/v3`): the list of loader
//!   versions for a game version, then the loader's profile JSON, written as a version file that
//!   inherits from the game's ([`crate::merge`]).
//! - **Forge** (`maven.minecraftforge.net`) and **NeoForge** (`maven.neoforged.net`): the versions
//!   in their Maven metadata, then their official installer jar (sha1 from the repository), run
//!   headless with the downloaded Java into the game folder (`--installClient`); the installer
//!   writes the version file and runs its processors. Very old Forge installers (no client
//!   install option) are read directly: their `install_profile.json` holds the version file and
//!   the universal jar.

pub mod fabric;
pub mod forge;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl LoaderKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
        }
    }

    /// Modrinth's loader name (its `categories` facet).
    pub fn modrinth(self) -> Option<&'static str> {
        match self {
            Self::Vanilla => None,
            Self::Fabric => Some("fabric"),
            Self::Quilt => Some("quilt"),
            Self::Forge => Some("forge"),
            Self::NeoForge => Some("neoforge"),
        }
    }
}

/// One loader version for a game version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

/// The newest stable version of a list (newest first), else the newest.
pub fn pick_default(list: &[LoaderVersion]) -> Option<&LoaderVersion> {
    list.iter().find(|v| v.stable).or_else(|| list.first())
}

/// The `<version>` values of a Maven `maven-metadata.xml`, in file order.
pub fn maven_versions(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<version>") {
        let after = &rest[start + "<version>".len()..];
        let Some(end) = after.find("</version>") else {
            break;
        };
        let v = after[..end].trim();
        if !v.is_empty() {
            out.push(v.to_owned());
        }
        rest = &after[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_metadata() {
        let xml = "<metadata><versioning><latest>2</latest><versions><version>1</version>\n<version> 2 </version></versions></versioning></metadata>";
        assert_eq!(maven_versions(xml), vec!["1", "2"]);
        let l = vec![
            LoaderVersion {
                version: "0.2-beta".into(),
                stable: false,
            },
            LoaderVersion {
                version: "0.1".into(),
                stable: true,
            },
        ];
        assert_eq!(pick_default(&l).unwrap().version, "0.1");
        assert!(pick_default(&[]).is_none());
    }
}
