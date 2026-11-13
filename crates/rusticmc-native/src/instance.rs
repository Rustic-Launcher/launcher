//! An instance's side of native mods, all in `<instance>/native/` (beside its `game/` folder):
//!
//! - `consent.json`: the player accepted the trust notice for this instance (when, which notice);
//! - `build.json`: the [`BuildRecord`] of the client built for it;
//! - `bin/`: that client (`rustic-client`), with what it reads beside itself (`native-mods.json`,
//!   Horizon's `mods/` folder of the edition bundle, the mods' `native-data/`).
//!
//! The native mods of an instance are the `.hnmod` files in `game/horizon-native-mods/` that
//! `game/horizon-mods.txt` doesn't turn off (`mod.<id>:false`). A build is up to date when its key
//! ([`key`]: the edition build, the profile and each mod's id, version and SHA-256) is the one the
//! instance's mods give now; else the next launch rebuilds (only what changed compiles). With no
//! native mod turned on, the instance plays the prebuilt client.
use crate::bundle::{Bundle, NativeManifest};
use crate::error::{IoContext, NativeError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Where the instance's native files are.
pub fn dir(instance_dir: &Path) -> PathBuf {
    instance_dir.join("native")
}
/// Where native mods are installed in a game folder (as Horizon's launcher).
pub const MODS_DIR: &str = "horizon-native-mods";
/// What the game reads beside itself to know it was built with native mods (Horizon's
/// `BUILD_INFO`, `horizon-client/src/game/mods.rs`).
pub const BUILD_INFO: &str = "native-mods.json";
/// Bumped when the trust notice's text changes in substance: an older consent asks again.
pub const NOTICE_VERSION: u32 = 1;

/// A native mod installed in an instance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeModFile {
    pub id: String,
    pub version: String,
    pub sha256: String,
    pub file: PathBuf,
    #[serde(default)]
    pub hooks: Option<u32>,
    pub enabled: bool,
}

/// `mod.<id>` in `horizon-mods.txt`: on unless `false`.
fn switch_on(game_dir: &Path, id: &str) -> bool {
    let text = std::fs::read_to_string(game_dir.join("horizon-mods.txt")).unwrap_or_default();
    let key = format!("mod.{id}");
    !text
        .lines()
        .filter_map(|l| l.split_once(':'))
        .any(|(k, v)| k.trim() == key && v.trim() == "false")
}

/// The native mods installed in `game_dir`, turned on or not, sorted by id. A file that fails its
/// checks is an error naming it (it would not build).
pub fn installed(game_dir: &Path) -> Result<Vec<(NativeModFile, Bundle)>> {
    let dir = game_dir.join(MODS_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut out: Vec<(NativeModFile, Bundle)> = Vec::new();
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("hnmod"))
        })
        .collect();
    paths.sort();
    for path in paths {
        let bundle = Bundle::read(&path)?;
        let NativeManifest {
            id, version, hooks, ..
        } = bundle.manifest.clone();
        if out.iter().any(|(m, _)| m.id == id) {
            return Err(NativeError::Bundle {
                id,
                message: "installed twice".into(),
            });
        }
        out.push((
            NativeModFile {
                enabled: switch_on(game_dir, &id),
                id,
                version,
                sha256: bundle.sha256.clone(),
                file: path,
                hooks,
            },
            bundle,
        ));
    }
    out.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    Ok(out)
}

/// The mods turned on.
pub fn enabled(game_dir: &Path) -> Result<Vec<(NativeModFile, Bundle)>> {
    Ok(installed(game_dir)?
        .into_iter()
        .filter(|(m, _)| m.enabled)
        .collect())
}

/// A mod as a build record keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModRef {
    pub id: String,
    pub version: String,
    pub sha256: String,
}

impl From<&NativeModFile> for ModRef {
    fn from(m: &NativeModFile) -> Self {
        Self {
            id: m.id.clone(),
            version: m.version.clone(),
            sha256: m.sha256.clone(),
        }
    }
}

/// A mod a build left out, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOut {
    pub id: String,
    pub version: String,
    /// `other_version` (doesn't apply to or compile on this Horizon), `unsupported` (needs what
    /// the Rustic edition can't build yet).
    pub why: String,
    /// The tool's or the compiler's words.
    pub detail: String,
}

/// The key of a build of `mods` on the edition build `base_build`.
pub fn key(base_build: &str, mods: &[ModRef]) -> String {
    let mut sorted: Vec<&ModRef> = mods.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let mut h = Sha256::new();
    h.update(b"rustic-native-1\0");
    h.update(base_build.as_bytes());
    h.update(b"\0");
    h.update(crate::PROFILE.as_bytes());
    for m in sorted {
        h.update(b"\0");
        h.update(m.id.as_bytes());
        h.update(b"\0");
        h.update(m.version.as_bytes());
        h.update(b"\0");
        h.update(m.sha256.as_bytes());
    }
    hex::encode(h.finalize())
}

/// `build.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildRecord {
    pub format: u32,
    /// [`key`] of `requested` on `base_build`.
    pub key: String,
    /// The edition build it started from (`rustic-edition-…`).
    pub base_build: String,
    pub source_id: String,
    pub horizon_commit: String,
    pub mixins_rev: String,
    pub rust_version: String,
    pub profile: String,
    /// The mods turned on when it was built.
    pub requested: Vec<ModRef>,
    /// The mods it was built with.
    pub built: Vec<ModRef>,
    pub left_out: Vec<LeftOut>,
    /// Unix seconds.
    pub built_at: u64,
    pub seconds: f64,
    /// Crates compiled (0: nothing changed).
    pub compiled: u32,
    /// The client, relative to `<instance>/native/`.
    pub client: String,
    pub client_bytes: u64,
}

impl BuildRecord {
    pub fn path(instance_dir: &Path) -> PathBuf {
        dir(instance_dir).join("build.json")
    }

    pub fn load(instance_dir: &Path) -> Option<Self> {
        let bytes = std::fs::read(Self::path(instance_dir)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self, instance_dir: &Path) -> Result<()> {
        write_atomic(
            &Self::path(instance_dir),
            &serde_json::to_vec_pretty(self).expect("json"),
        )
    }

    /// The client program, when it is there.
    pub fn client_path(&self, instance_dir: &Path) -> Option<PathBuf> {
        let p = dir(instance_dir).join(&self.client);
        p.is_file().then_some(p)
    }
}

/// The player's acceptance of the trust notice for an instance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consent {
    pub notice: u32,
    /// Unix seconds.
    pub accepted_at: u64,
}

impl Consent {
    fn path(instance_dir: &Path) -> PathBuf {
        dir(instance_dir).join("consent.json")
    }

    /// The consent in force (the current notice's), if any.
    pub fn load(instance_dir: &Path) -> Option<Self> {
        let bytes = std::fs::read(Self::path(instance_dir)).ok()?;
        let c: Self = serde_json::from_slice(&bytes).ok()?;
        (c.notice == NOTICE_VERSION).then_some(c)
    }

    pub fn accept(instance_dir: &Path) -> Result<Self> {
        let c = Self {
            notice: NOTICE_VERSION,
            accepted_at: now(),
        };
        write_atomic(
            &Self::path(instance_dir),
            &serde_json::to_vec_pretty(&c).expect("json"),
        )?;
        Ok(c)
    }

    pub fn revoke(instance_dir: &Path) -> Result<()> {
        let p = Self::path(instance_dir);
        match std::fs::remove_file(&p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(NativeError::io(&p, e)),
            _ => Ok(()),
        }
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).at(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).at(&tmp)?;
    std::fs::rename(&tmp, path).at(path)
}

/// Where an instance stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// No native mod turned on: the prebuilt client.
    NotNeeded,
    NotBuilt,
    NeedsRebuild,
    UpToDate,
}

/// An instance's native mods and build.
#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub state: State,
    pub mods: Vec<NativeModFile>,
    pub consent: Option<Consent>,
    pub record: Option<BuildRecord>,
    /// The client to launch when up to date.
    pub client: Option<PathBuf>,
    /// The key the mods turned on give now.
    pub key: Option<String>,
}

/// The status of the instance at `instance_dir` (its game folder `game_dir`) on `base_build`.
pub fn status(instance_dir: &Path, game_dir: &Path, base_build: &str) -> Result<Status> {
    let all = installed(game_dir)?;
    let mods: Vec<NativeModFile> = all.into_iter().map(|(m, _)| m).collect();
    let on: Vec<ModRef> = mods
        .iter()
        .filter(|m| m.enabled)
        .map(ModRef::from)
        .collect();
    let record = BuildRecord::load(instance_dir);
    let consent = Consent::load(instance_dir);
    if on.is_empty() {
        return Ok(Status {
            state: State::NotNeeded,
            mods,
            consent,
            record,
            client: None,
            key: None,
        });
    }
    let want = key(base_build, &on);
    let client = record.as_ref().and_then(|r| r.client_path(instance_dir));
    let state = match &record {
        Some(r) if r.key == want && client.is_some() => State::UpToDate,
        Some(_) if client.is_some() => State::NeedsRebuild,
        _ => State::NotBuilt,
    };
    Ok(Status {
        state,
        mods,
        consent,
        client: (state == State::UpToDate).then_some(client).flatten(),
        record,
        key: Some(want),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::tests::hnmod;

    fn setup(t: &Path) -> (PathBuf, PathBuf) {
        let inst = t.join("instances/one");
        let game = inst.join("game");
        std::fs::create_dir_all(game.join(MODS_DIR)).unwrap();
        (inst, game)
    }

    #[test]
    fn mods_are_found_and_switched_off_by_the_games_file() {
        let t = tempfile::tempdir().unwrap();
        let (_inst, game) = setup(t.path());
        std::fs::write(
            game.join(MODS_DIR).join("zoom-2.0.0.hnmod"),
            hnmod("zoom", &[]),
        )
        .unwrap();
        std::fs::write(
            game.join(MODS_DIR).join("fps.hnmod"),
            hnmod("fps_overlay", &[]),
        )
        .unwrap();
        std::fs::write(game.join(MODS_DIR).join("readme.txt"), "x").unwrap();
        let all = installed(&game).unwrap();
        assert_eq!(
            all.iter().map(|m| m.0.id.as_str()).collect::<Vec<_>>(),
            ["fps_overlay", "zoom"]
        );
        std::fs::write(
            game.join("horizon-mods.txt"),
            "mod.zoom:false\nmod.fps_overlay:true\n",
        )
        .unwrap();
        let on = enabled(&game).unwrap();
        assert_eq!(on.len(), 1);
        assert_eq!(on[0].0.id, "fps_overlay");
        // A broken file is named.
        std::fs::write(game.join(MODS_DIR).join("bad.hnmod"), "nope").unwrap();
        assert!(installed(&game).unwrap_err().to_string().contains("bad"));
    }

    #[test]
    fn the_state_follows_the_mods_turned_on() {
        let t = tempfile::tempdir().unwrap();
        let (inst, game) = setup(t.path());
        let s = status(&inst, &game, "rustic-edition-1").unwrap();
        assert_eq!(s.state, State::NotNeeded);
        std::fs::write(game.join(MODS_DIR).join("zoom.hnmod"), hnmod("zoom", &[])).unwrap();
        let s = status(&inst, &game, "rustic-edition-1").unwrap();
        assert_eq!(s.state, State::NotBuilt);
        // A build of exactly these mods.
        let client = dir(&inst).join("bin").join(crate::exe("rustic-client"));
        std::fs::create_dir_all(client.parent().unwrap()).unwrap();
        std::fs::write(&client, "exe").unwrap();
        let on: Vec<ModRef> = s.mods.iter().map(ModRef::from).collect();
        let record = BuildRecord {
            format: 1,
            key: s.key.clone().unwrap(),
            base_build: "rustic-edition-1".into(),
            source_id: "x".into(),
            horizon_commit: crate::source::PINNED_HORIZON.into(),
            mixins_rev: crate::mixins::REV.into(),
            rust_version: crate::toolchain::RUST_VERSION.into(),
            profile: crate::PROFILE.into(),
            requested: on.clone(),
            built: on,
            left_out: vec![],
            built_at: now(),
            seconds: 1.0,
            compiled: 3,
            client: format!("bin/{}", crate::exe("rustic-client")),
            client_bytes: 3,
        };
        record.save(&inst).unwrap();
        let s = status(&inst, &game, "rustic-edition-1").unwrap();
        assert_eq!(s.state, State::UpToDate);
        assert_eq!(s.client.as_deref(), Some(client.as_path()));
        // Another edition build, or the mod off: rebuild / prebuilt.
        assert_eq!(
            status(&inst, &game, "rustic-edition-2").unwrap().state,
            State::NeedsRebuild
        );
        std::fs::write(game.join("horizon-mods.txt"), "mod.zoom:false\n").unwrap();
        assert_eq!(
            status(&inst, &game, "rustic-edition-1").unwrap().state,
            State::NotNeeded
        );
        std::fs::write(game.join("horizon-mods.txt"), "").unwrap();
        // A new version of the mod: rebuild.
        std::fs::write(
            game.join(MODS_DIR).join("zoom.hnmod"),
            hnmod("zoom", &[("mixins/zoom/extra.rs", b"fn x() {}")]),
        )
        .unwrap();
        assert_eq!(
            status(&inst, &game, "rustic-edition-1").unwrap().state,
            State::NeedsRebuild
        );
    }

    #[test]
    fn consent_is_per_instance_and_per_notice() {
        let t = tempfile::tempdir().unwrap();
        let (inst, _) = setup(t.path());
        assert!(Consent::load(&inst).is_none());
        Consent::accept(&inst).unwrap();
        assert!(Consent::load(&inst).is_some());
        std::fs::write(
            dir(&inst).join("consent.json"),
            r#"{"notice":0,"accepted_at":1}"#,
        )
        .unwrap();
        assert!(Consent::load(&inst).is_none(), "an older notice asks again");
        Consent::revoke(&inst).unwrap();
        Consent::revoke(&inst).unwrap();
        assert!(Consent::load(&inst).is_none());
    }

    #[test]
    fn keys_ignore_order_and_see_every_part() {
        let a = ModRef {
            id: "a".into(),
            version: "1".into(),
            sha256: "x".into(),
        };
        let b = ModRef {
            id: "b".into(),
            version: "1".into(),
            sha256: "y".into(),
        };
        assert_eq!(
            key("e", &[a.clone(), b.clone()]),
            key("e", &[b.clone(), a.clone()])
        );
        assert_ne!(
            key("e", std::slice::from_ref(&a)),
            key("f", std::slice::from_ref(&a))
        );
        let mut a2 = a.clone();
        a2.sha256 = "z".into();
        assert_ne!(key("e", &[a]), key("e", &[a2]));
    }
}
