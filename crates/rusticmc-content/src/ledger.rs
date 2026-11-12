//! What Rustic installed where: `rustic-content.json` in a game or server folder. It is what
//! uninstall, enable/disable and update checks read; the game itself never reads it.
use crate::error::Result;
use crate::model::{ContentKind, Hashes, SourceKind};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FILE: &str = "rustic-content.json";
pub const FORMAT: u32 = 1;

/// One installed file, relative to the folder the ledger is in, `/`-separated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    pub path: String,
    pub hashes: Hashes,
}

/// One installed project version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub source: SourceKind,
    pub project_id: String,
    pub version_id: String,
    pub version: String,
    pub name: String,
    pub kind: ContentKind,
    /// `instance`, `server`, `world:<name>`, `new_worlds`.
    pub target: String,
    pub files: Vec<InstalledFile>,
    pub enabled: bool,
    /// The id the game knows it by (a mod's manifest `id`, a pack's `file/<name>`), for the
    /// switch files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_id: Option<String>,
    /// Installed because these projects needed it (empty: the player asked for it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needed_by: Vec<String>,
    /// The download URL, so a Rustic pack can name it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ledger {
    pub format: u32,
    pub entries: Vec<Entry>,
}

impl Ledger {
    pub fn load(dir: &Path) -> Result<Ledger> {
        let path = dir.join(FILE);
        if !path.exists() {
            return Ok(Ledger {
                format: FORMAT,
                entries: Vec::new(),
            });
        }
        let ledger: Ledger = serde_json::from_slice(&std::fs::read(path)?)?;
        if ledger.format > FORMAT {
            return Err(crate::ContentError::Json(format!(
                "{FILE} format {} is newer than this launcher's",
                ledger.format
            )));
        }
        Ok(ledger)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let tmp = dir.join(format!("{FILE}.part"));
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, dir.join(FILE))?;
        Ok(())
    }

    pub fn find_project(&self, source: SourceKind, project: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.source == source && e.project_id == project)
    }

    pub fn find_mut(
        &mut self,
        source: SourceKind,
        project: &str,
        target: &str,
    ) -> Option<&mut Entry> {
        self.entries
            .iter_mut()
            .find(|e| e.source == source && e.project_id == project && e.target == target)
    }

    /// Adds `entry`, replacing the same project on the same target.
    pub fn upsert(&mut self, entry: Entry) {
        self.entries.retain(|e| {
            !(e.source == entry.source
                && e.project_id == entry.project_id
                && e.target == entry.target)
        });
        self.entries.push(entry);
    }
}
