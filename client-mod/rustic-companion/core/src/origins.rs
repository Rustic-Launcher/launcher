//! Where each installed mod comes from, for the tags on the settings' Mods page (ADR 0165), read
//! from what is on disk in the game folder: the launcher's content ledger (`rustic-content.json`,
//! launcher crate `rusticmc-content`: each entry's `source` — `rustic`, `horizon`, `modrinth`,
//! `local` — and the id the game knows it by), the game's own browser record (`mod-browser.json`:
//! Horizon's registry), then the mod's file (`horizon-mods/`, `horizon-native-mods/`): a file no
//! record names was dropped in by the player. Native mods (built into the game) carry "Native" too.
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// A mod's origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    Horizon,
    Rustic,
    Modrinth,
    Local,
}

impl Origin {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "horizon" => Some(Origin::Horizon),
            "rustic" => Some(Origin::Rustic),
            "modrinth" => Some(Origin::Modrinth),
            "local" => Some(Origin::Local),
            _ => None,
        }
    }
    /// Its tag's colour (`0xRRGGBB`), as the launcher's source badges: Rustic in the accent, the
    /// others neutral.
    pub fn rgb(self) -> u32 {
        match self {
            Origin::Rustic => 0xFF_8A3D,
            Origin::Horizon => 0xB8_B4AE,
            Origin::Modrinth => 0xB8_B4AE,
            Origin::Local => 0x8E_8A85,
        }
    }
}

/// The "Native" tag's colour (the launcher's info tone).
pub const NATIVE_RGB: u32 = 0x5A_A9E6;

/// What the game folder says about its mods.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Origins {
    /// By the id the game knows a mod by (and by project id).
    pub by_id: HashMap<String, Origin>,
}

impl Origins {
    /// Reads `rustic-content.json` and `mod-browser.json` in `game_dir`; missing or unreadable
    /// files give nothing.
    pub fn read(game_dir: &Path) -> Self {
        let json = |name: &str| -> Option<Value> {
            let bytes = std::fs::read(game_dir.join(name)).ok()?;
            serde_json::from_slice(&bytes).ok()
        };
        let mut by_id = HashMap::new();
        // The game's own browser first: the launcher's ledger, which knows more, wins.
        if let Some(v) = json("mod-browser.json") {
            for e in v["entries"].as_array().into_iter().flatten() {
                let source = e["source"].as_str().and_then(Origin::parse);
                if let (Some(source), Some(id)) = (source, e["project"].as_str()) {
                    by_id.insert(id.to_owned(), source);
                }
            }
        }
        if let Some(v) = json("rustic-content.json") {
            for e in v["entries"].as_array().into_iter().flatten() {
                let Some(source) = e["source"].as_str().and_then(Origin::parse) else {
                    continue;
                };
                for key in ["game_id", "project_id"] {
                    if let Some(id) = e[key].as_str().filter(|s| !s.is_empty()) {
                        by_id.insert(id.to_owned(), source);
                    }
                }
            }
        }
        Origins { by_id }
    }

    /// Mod `id`'s origin: a record's, else Local when its file is in the game folder (`on_disk`),
    /// else none (built into the game).
    pub fn of(&self, id: &str, on_disk: bool) -> Option<Origin> {
        self.by_id
            .get(id)
            .copied()
            .or(on_disk.then_some(Origin::Local))
    }
}

/// Whether mod `id` has a file of its own in `game_dir` (`horizon-mods/<id>*.hmod`,
/// `horizon-native-mods/<id>*.hnmod`).
pub fn on_disk(game_dir: &Path, id: &str) -> bool {
    ["horizon-mods", "horizon-native-mods"].iter().any(|dir| {
        std::fs::read_dir(game_dir.join(dir))
            .map(|entries| {
                entries.filter_map(Result::ok).any(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    let stem = name.split('.').next().unwrap_or_default();
                    stem == id.to_lowercase()
                        || stem.starts_with(&format!("{}-", id.to_lowercase()))
                })
            })
            .unwrap_or(false)
    })
}

/// The tags of a mod: its origin (named by `label`), then "Native" for one built into the game.
/// `label` gives each tag's text in the game's language.
pub fn tags(
    origin: Option<Origin>,
    native: bool,
    label: impl Fn(Option<Origin>) -> String,
) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    if let Some(o) = origin {
        out.push((label(Some(o)), o.rgb()));
    }
    if native {
        out.push((label(None), NATIVE_RGB));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_come_from_the_ledger_the_browser_and_the_files() {
        let dir = std::env::temp_dir().join(format!("rustic-origins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("horizon-mods")).unwrap();
        std::fs::write(dir.join("horizon-mods/dropped-1.0.hmod"), b"x").unwrap();
        std::fs::write(
            dir.join("mod-browser.json"),
            r#"{"entries":[{"source":"horizon","project":"waystones"},{"source":"horizon","project":"both"}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("rustic-content.json"),
            r#"{"format":1,"entries":[
                {"source":"rustic","project_id":"p1","game_id":"fancy"},
                {"source":"modrinth","project_id":"both"}]}"#,
        )
        .unwrap();
        let o = Origins::read(&dir);
        assert_eq!(o.of("fancy", false), Some(Origin::Rustic));
        assert_eq!(o.of("waystones", true), Some(Origin::Horizon));
        assert_eq!(
            o.of("both", false),
            Some(Origin::Modrinth),
            "the ledger wins"
        );
        assert!(on_disk(&dir, "dropped"));
        assert!(!on_disk(&dir, "drop"));
        assert_eq!(
            o.of("dropped", on_disk(&dir, "dropped")),
            Some(Origin::Local)
        );
        assert_eq!(o.of("builtin", false), None);
        let t = tags(Some(Origin::Rustic), true, |o| format!("{o:?}"));
        assert_eq!(
            t,
            [
                ("Some(Rustic)".to_owned(), 0xFF_8A3D),
                ("None".to_owned(), NATIVE_RGB)
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert!(Origins::read(&dir).by_id.is_empty());
    }
}
