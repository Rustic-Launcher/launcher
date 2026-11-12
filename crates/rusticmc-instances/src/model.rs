//! Instances (profiles) on disk: `<data>/instances/<id>/instance.toml`, the game folder
//! `<id>/game/` (`--game-dir` and `--home`: saves, options, packs, mods, logs, crash reports) and
//! an optional `icon.png`.

use crate::error::{InstanceError, Result};
use crate::launch::{QuickPlay, WindowOptions};
use rusticmc_launcher_core::{fsutil, time};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Which build an instance plays.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildChoice {
    /// The build source id (`rustic-edition`, `horizon`).
    pub source: String,
    /// An installed version, `None`: the newest installed build of the source.
    #[serde(default)]
    pub version: Option<String>,
}

/// A content item the instance holds (filled by the content crate; the launcher shows it).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentEntry {
    /// `mod`, `resourcepack`, `shader`, `datapack`, `native_mod`.
    pub kind: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    /// File relative to the game folder.
    pub file: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Where it came from (`modrinth`, `local`...).
    #[serde(default)]
    pub source: Option<String>,
}

fn yes() -> bool {
    true
}

/// Per-instance settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct InstanceSettings {
    pub window: WindowOptions,
    /// Memory the player wants for this instance, in MiB. Horizon is a native program: its Launch
    /// API 0.1 has no memory option, so this is shown and kept but not passed (RISKS.md).
    pub memory_mb: Option<u32>,
    /// A Java home for preparing this instance's build.
    pub java_home: Option<PathBuf>,
    /// Open a world or server at start.
    pub quick_play: Option<QuickPlay>,
}

/// An instance's icon: a preset with a colour, or an image (the instance's `icon.png`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstanceIcon {
    Preset { preset: String, color: String },
    Image { url: String },
}

impl Default for InstanceIcon {
    fn default() -> Self {
        Self::Preset {
            preset: "grass".into(),
            color: "#d9822b".into(),
        }
    }
}

/// The server an instance last joined.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LastServer {
    pub name: String,
    pub address: String,
}

/// Where an instance came from (a modpack import).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstanceSource {
    pub kind: String,
    pub project_id: String,
    pub name: String,
    pub version: String,
}

/// One instance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub build: BuildChoice,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub icon: InstanceIcon,
    #[serde(default)]
    pub has_icon: bool,
    #[serde(default)]
    pub pinned: bool,
    /// Extra game options; only options the build's `--launch-api` lists are accepted at launch.
    #[serde(default)]
    pub game_args: String,
    #[serde(default)]
    pub last_server: Option<LastServer>,
    #[serde(default)]
    pub source: Option<InstanceSource>,
    pub created_at: u64,
    #[serde(default)]
    pub last_played_at: Option<u64>,
    #[serde(default)]
    pub playtime_seconds: u64,
    #[serde(default)]
    pub settings: InstanceSettings,
    #[serde(default)]
    pub content: Vec<ContentEntry>,
}

/// What a create or update may change.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct InstancePatch {
    pub name: Option<String>,
    pub icon: Option<InstanceIcon>,
    pub pinned: Option<bool>,
    pub game_args: Option<String>,
    pub build: Option<BuildChoice>,
    pub group: Option<Option<String>>,
    pub settings: Option<InstanceSettings>,
}

const FILE: &str = "instance.toml";

/// Instance folders.
#[derive(Clone)]
pub struct InstanceStore {
    root: PathBuf,
}

impl InstanceStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    /// `--game-dir` and `--home`.
    pub fn game_dir(&self, id: &str) -> PathBuf {
        self.dir(id).join("game")
    }

    pub fn icon_path(&self, id: &str) -> PathBuf {
        self.dir(id).join("icon.png")
    }

    fn check_id(id: &str) -> Result<()> {
        if id.is_empty() || id.contains(['/', '\\', '.']) {
            return Err(InstanceError::UnknownInstance(id.into()));
        }
        Ok(())
    }

    pub fn list(&self) -> Vec<Instance> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut out: Vec<Instance> = entries
            .flatten()
            .filter_map(|e| fsutil::read_toml(&e.path().join(FILE)).ok().flatten())
            .collect();
        out.sort_by(|a, b| {
            b.last_played_at
                .cmp(&a.last_played_at)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        out
    }

    pub fn get(&self, id: &str) -> Result<Instance> {
        Self::check_id(id)?;
        fsutil::read_toml(&self.dir(id).join(FILE))?
            .ok_or_else(|| InstanceError::UnknownInstance(id.into()))
    }

    fn save(&self, instance: &Instance) -> Result<()> {
        fsutil::write_toml(&self.dir(&instance.id).join(FILE), instance)?;
        Ok(())
    }

    fn unique_id(&self, name: &str) -> String {
        let base = fsutil::slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.dir(&id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    pub fn create(&self, name: &str, build: BuildChoice) -> Result<Instance> {
        let name = name.trim();
        if name.is_empty() {
            return Err(InstanceError::Process("An instance needs a name.".into()));
        }
        let instance = Instance {
            id: self.unique_id(name),
            name: name.into(),
            build,
            group: None,
            icon: InstanceIcon::default(),
            has_icon: false,
            pinned: false,
            game_args: String::new(),
            last_server: None,
            source: None,
            created_at: time::now(),
            last_played_at: None,
            playtime_seconds: 0,
            settings: InstanceSettings::default(),
            content: Vec::new(),
        };
        std::fs::create_dir_all(self.game_dir(&instance.id))
            .map_err(|e| InstanceError::io(self.game_dir(&instance.id), e))?;
        self.save(&instance)?;
        Ok(instance)
    }

    pub fn update(&self, id: &str, patch: InstancePatch) -> Result<Instance> {
        let mut i = self.get(id)?;
        if let Some(name) = patch.name.filter(|n| !n.trim().is_empty()) {
            i.name = name.trim().into();
        }
        if let Some(icon) = patch.icon {
            i.icon = icon;
        }
        if let Some(pinned) = patch.pinned {
            i.pinned = pinned;
        }
        if let Some(args) = patch.game_args {
            i.game_args = args;
        }
        if let Some(build) = patch.build {
            i.build = build;
        }
        if let Some(group) = patch.group {
            i.group = group;
        }
        if let Some(settings) = patch.settings {
            i.settings = settings;
        }
        self.save(&i)?;
        Ok(i)
    }

    /// Replaces the content list (the content crate's installs call this).
    pub fn set_content(&self, id: &str, content: Vec<ContentEntry>) -> Result<Instance> {
        let mut i = self.get(id)?;
        i.content = content;
        self.save(&i)?;
        Ok(i)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        Self::check_id(id)?;
        let dir = self.dir(id);
        if !dir.join(FILE).is_file() {
            return Err(InstanceError::UnknownInstance(id.into()));
        }
        std::fs::remove_dir_all(&dir).map_err(|e| InstanceError::io(&dir, e))
    }

    /// Copies an instance (worlds and all) under a new name; playtime starts at zero.
    pub fn duplicate(&self, id: &str, name: &str) -> Result<Instance> {
        let source = self.get(id)?;
        let new_id = self.unique_id(name);
        fsutil::copy_dir(&self.dir(id), &self.dir(&new_id))
            .map_err(|e| InstanceError::io(self.dir(&new_id), e))?;
        let copy = Instance {
            id: new_id,
            name: name.trim().into(),
            created_at: time::now(),
            last_played_at: None,
            playtime_seconds: 0,
            ..source
        };
        self.save(&copy)?;
        Ok(copy)
    }

    /// Sets the icon from PNG bytes (at most 1 MiB).
    pub fn set_icon(&self, id: &str, png: &[u8]) -> Result<Instance> {
        if png.len() > 1 << 20 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(InstanceError::Process(
                "An icon is a PNG of at most 1 MiB.".into(),
            ));
        }
        let mut i = self.get(id)?;
        fsutil::write_atomic(&self.icon_path(id), png)?;
        i.has_icon = true;
        self.save(&i)?;
        Ok(i)
    }

    /// Remembers the server an instance joined.
    pub fn set_last_server(&self, id: &str, server: Option<LastServer>) -> Result<Instance> {
        let mut i = self.get(id)?;
        i.last_server = server;
        self.save(&i)?;
        Ok(i)
    }

    /// Records where an imported instance came from.
    pub fn set_source(&self, id: &str, source: Option<InstanceSource>) -> Result<Instance> {
        let mut i = self.get(id)?;
        i.source = source;
        self.save(&i)?;
        Ok(i)
    }

    /// Adds a play session's time.
    pub fn record_play(&self, id: &str, seconds: u64, ended_at: u64) -> Result<Instance> {
        let mut i = self.get(id)?;
        i.playtime_seconds += seconds;
        i.last_played_at = Some(ended_at);
        self.save(&i)?;
        Ok(i)
    }

    /// The crash reports of an instance, newest first.
    pub fn crash_reports(&self, id: &str) -> Vec<PathBuf> {
        list_newest(&self.game_dir(id).join("crash-reports"))
    }

    /// `logs/latest.log`.
    pub fn latest_log(&self, id: &str) -> PathBuf {
        self.game_dir(id).join("logs").join("latest.log")
    }
}

fn list_newest(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .map(|e| {
            e.flatten()
                .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    files.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, InstanceStore) {
        let t = tempfile::tempdir().unwrap();
        let s = InstanceStore::new(t.path().join("instances"));
        (t, s)
    }

    fn build() -> BuildChoice {
        BuildChoice {
            source: "rustic-edition".into(),
            version: None,
        }
    }

    #[test]
    fn create_update_duplicate_delete() {
        let (_t, s) = store();
        let a = s.create("My World", build()).unwrap();
        assert_eq!(a.id, "my-world");
        let b = s.create("My World", build()).unwrap();
        assert_eq!(b.id, "my-world-2");
        assert!(s.game_dir(&a.id).is_dir());
        let a = s
            .update(
                &a.id,
                InstancePatch {
                    name: Some("Renamed".into()),
                    settings: Some(InstanceSettings {
                        memory_mb: Some(4096),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(a.name, "Renamed");
        assert_eq!(s.get(&a.id).unwrap().settings.memory_mb, Some(4096));
        std::fs::write(s.game_dir(&a.id).join("options.txt"), "x").unwrap();
        let c = s.duplicate(&a.id, "Copy").unwrap();
        assert!(s.game_dir(&c.id).join("options.txt").is_file());
        assert_eq!(s.list().len(), 3);
        s.delete(&b.id).unwrap();
        assert!(s.get(&b.id).is_err());
        assert!(s.get("../etc").is_err());
    }

    #[test]
    fn playtime_adds_up() {
        let (_t, s) = store();
        let a = s.create("A", build()).unwrap();
        s.record_play(&a.id, 60, 1000).unwrap();
        let a = s.record_play(&a.id, 30, 2000).unwrap();
        assert_eq!(a.playtime_seconds, 90);
        assert_eq!(a.last_played_at, Some(2000));
    }

    #[test]
    fn icons_must_be_png() {
        let (_t, s) = store();
        let a = s.create("A", build()).unwrap();
        assert!(s.set_icon(&a.id, b"GIF89a").is_err());
        let a = s.set_icon(&a.id, b"\x89PNG\r\n\x1a\nrest").unwrap();
        assert!(a.has_icon && s.icon_path(&a.id).is_file());
    }
}
