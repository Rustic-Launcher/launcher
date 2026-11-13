//! `.hnmod` files: Horizon's native mod package (horizon/docs/api/mod-index.md): a zip with
//! `horizon-native.json` and the bundle under `mixins/<id>/` (`horizon-mixin.toml`, `.rs` helpers,
//! `.toml`, data files under a `data/` folder). Read without running anything, with Horizon's
//! limits on what may be inside, then unpacked for horizon-mixins with the bundle re-pinned to the
//! build's Horizon commit (its `[compatibility] horizon_revision`, which horizon-mixins checks), as
//! Horizon's launcher does for mods that name hook points rather than a commit.
use crate::error::{IoContext, NativeError, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// The bundle's manifest inside a `.hnmod`.
pub const NATIVE_MANIFEST: &str = "horizon-native.json";
/// horizon-mixins' manifest inside a bundle folder.
pub const MIXIN_MANIFEST: &str = "horizon-mixin.toml";
/// The most a bundle may unpack to, and the most files it may hold.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_FILES: usize = 4096;

/// `horizon-native.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeManifest {
    pub schema: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub game: String,
    /// The Horizon commit it was made on.
    #[serde(default)]
    pub horizon: String,
    /// The horizon-mixins commit it was made with.
    #[serde(default)]
    pub mixins: String,
    /// The native hook points version it needs, when it uses only hook points.
    #[serde(default)]
    pub hooks: Option<u32>,
}

/// A checked `.hnmod`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    pub manifest: NativeManifest,
    /// SHA-256 of the file.
    pub sha256: String,
    /// The bundle's files under `mixins/<id>/`, relative to that folder.
    files: Vec<(String, Vec<u8>)>,
}

/// A mod id as Horizon allows them: lowercase letters, digits, `_` and `-`, 1 to 64.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// A relative path made only of plain parts.
fn plain(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}

impl Bundle {
    /// Reads and checks the `.hnmod` at `path`.
    pub fn read(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).at(path)?;
        Self::from_bytes(&bytes).map_err(|message| NativeError::Bundle {
            id: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            message,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, String> {
        use sha2::Digest;
        let sha256 = hex::encode(sha2::Sha256::digest(bytes));
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|e| format!("not a .hnmod (zip): {e}"))?;
        if zip.len() > MAX_FILES {
            return Err(format!("more than {MAX_FILES} files"));
        }
        let mut manifest: Option<NativeManifest> = None;
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        let mut total = 0u64;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_owned();
            if entry.is_dir() {
                continue;
            }
            if entry.is_symlink() {
                return Err(format!("{name}: links are not allowed"));
            }
            if !plain(&name) {
                return Err(format!("{name}: not a plain relative path"));
            }
            total += entry.size();
            if total > MAX_BYTES {
                return Err(format!("unpacks to more than {} MiB", MAX_BYTES >> 20));
            }
            let mut data = Vec::with_capacity(entry.size() as usize);
            entry
                .by_ref()
                .take(MAX_BYTES + 1)
                .read_to_end(&mut data)
                .map_err(|e| format!("{name}: {e}"))?;
            if name == NATIVE_MANIFEST {
                manifest = Some(
                    serde_json::from_slice(&data).map_err(|e| format!("{NATIVE_MANIFEST}: {e}"))?,
                );
            } else {
                entries.push((name, data));
            }
        }
        let manifest = manifest.ok_or_else(|| format!("no {NATIVE_MANIFEST}"))?;
        if !valid_id(&manifest.id) {
            return Err(format!("{:?} is not a valid mod id", manifest.id));
        }
        let prefix = format!("mixins/{}/", manifest.id);
        let mut files = Vec::new();
        for (name, data) in entries {
            let Some(rel) = name.strip_prefix(&prefix) else {
                if name.starts_with("mixins/") {
                    return Err(format!("{name}: outside mixins/{}/", manifest.id));
                }
                // A combined mod's content (`horizon-mod.toml`, assets): not part of the build.
                continue;
            };
            let in_data = rel.starts_with("data/") || rel.contains("/data/");
            let ext_ok = rel.ends_with(".rs") || rel.ends_with(".toml");
            if !(in_data || ext_ok) {
                return Err(format!(
                    "{name}: only Rust and TOML files, and data files under data/, may be in a bundle"
                ));
            }
            files.push((rel.to_owned(), data));
        }
        if !files.iter().any(|(n, _)| n == MIXIN_MANIFEST) {
            return Err(format!("no {prefix}{MIXIN_MANIFEST}"));
        }
        files.sort();
        Ok(Self {
            manifest,
            sha256,
            files,
        })
    }

    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    /// Unpacks the bundle into `<mods>/<id>/`, its `horizon_revision` set to `commit`.
    pub fn unpack_for(&self, mods: &Path, commit: &str) -> Result<PathBuf> {
        let dir = mods.join(self.id());
        let _ = std::fs::remove_dir_all(&dir);
        for (rel, data) in &self.files {
            let to = dir.join(rel);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).at(parent)?;
            }
            if rel == MIXIN_MANIFEST {
                let text = repin(&String::from_utf8_lossy(data), commit).map_err(|message| {
                    NativeError::Bundle {
                        id: self.id().to_owned(),
                        message,
                    }
                })?;
                std::fs::write(&to, text).at(&to)?;
            } else {
                std::fs::write(&to, data).at(&to)?;
            }
        }
        Ok(dir)
    }

    /// Whether the bundle brings its own crates or post-build programs (`[[crate]]`,
    /// `[[post_build]]`), which the Rustic edition can't build yet: its workspace's lockfile is
    /// fixed (`--locked`) and a mod crate would need entries in it.
    pub fn needs_unsupported(&self) -> Option<&'static str> {
        let manifest = self
            .files
            .iter()
            .find(|(n, _)| n == MIXIN_MANIFEST)
            .map(|(_, d)| String::from_utf8_lossy(d).into_owned())?;
        let value: toml::Table = manifest.parse().ok()?;
        if value.get("crate").is_some() {
            return Some("its own crate");
        }
        if value.get("post_build").is_some() {
            return Some("a post-build step");
        }
        if value.get("api").is_some() {
            return Some("a trait-mod API package");
        }
        None
    }
}

/// `horizon-mixin.toml` with `[compatibility] horizon_revision = commit`.
pub fn repin(text: &str, commit: &str) -> std::result::Result<String, String> {
    let mut value: toml::Table = text.parse().map_err(|e| format!("{MIXIN_MANIFEST}: {e}"))?;
    let compat = value
        .entry("compatibility")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let toml::Value::Table(compat) = compat else {
        return Err(format!("{MIXIN_MANIFEST}: [compatibility] is not a table"));
    };
    compat.insert("horizon_revision".into(), commit.into());
    toml::to_string(&value).map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    pub(crate) fn hnmod(id: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o: zip::write::SimpleFileOptions = Default::default();
            z.start_file(NATIVE_MANIFEST, o).unwrap();
            z.write_all(
                format!(
                    r#"{{"schema":1,"id":"{id}","version":"1.0.0","game":"26.3","horizon":"6f8c7a0e5d531d3834c4d0be4659a1db8e7e5c2a","mixins":"0d6b103c70633d8fd9443d8d0acd3aadaacedb06","hooks":1}}"#
                )
                .as_bytes(),
            )
            .unwrap();
            z.start_file(format!("mixins/{id}/{MIXIN_MANIFEST}"), o)
                .unwrap();
            z.write_all(
                format!(
                    "[mod]\nid = \"{id}\"\nversion = \"1.0.0\"\n\n[compatibility]\nhorizon_revision = \"6f8c7a0e5d531d3834c4d0be4659a1db8e7e5c2a\"\n"
                )
                .as_bytes(),
            )
            .unwrap();
            z.start_file(format!("mixins/{id}/hud.rs"), o).unwrap();
            z.write_all(b"fn hud() {}\n").unwrap();
            for (name, data) in extra {
                z.start_file(*name, o).unwrap();
                z.write_all(data).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn a_bundle_is_read_and_repinned() {
        let bytes = hnmod(
            "fps_overlay",
            &[("mixins/fps_overlay/data/a.bin", b"\0\x01")],
        );
        let b = Bundle::from_bytes(&bytes).unwrap();
        assert_eq!(b.id(), "fps_overlay");
        assert_eq!(b.manifest.hooks, Some(1));
        assert_eq!(b.sha256.len(), 64);
        assert!(b.needs_unsupported().is_none());
        let t = tempfile::tempdir().unwrap();
        let dir = b
            .unpack_for(t.path(), crate::source::PINNED_HORIZON)
            .unwrap();
        let toml = std::fs::read_to_string(dir.join(MIXIN_MANIFEST)).unwrap();
        assert!(toml.contains(crate::source::PINNED_HORIZON), "{toml}");
        assert!(dir.join("hud.rs").is_file());
        assert!(dir.join("data/a.bin").is_file());
    }

    #[test]
    fn unsafe_or_foreign_files_are_refused() {
        for (name, why) in [
            ("mixins/x/build.sh", "only Rust"),
            ("mixins/other/a.rs", "outside"),
            ("../evil.rs", "plain"),
            ("mixins/x/../../a.rs", "plain"),
        ] {
            let bytes = hnmod("x", &[(name, b"x")]);
            let err = Bundle::from_bytes(&bytes).unwrap_err();
            assert!(err.contains(why), "{name}: {err}");
        }
        // Content files of a combined mod are allowed and ignored.
        let bytes = hnmod(
            "x",
            &[("horizon-mod.toml", b"[mod]\n"), ("assets/a.png", b"p")],
        );
        assert!(Bundle::from_bytes(&bytes).is_ok());
        assert!(Bundle::from_bytes(b"not a zip").is_err());
    }

    #[test]
    fn mods_with_their_own_crates_are_named() {
        let mut bytes = hnmod("c", &[]);
        let b = Bundle::from_bytes(&bytes).unwrap();
        assert!(b.needs_unsupported().is_none());
        // Rebuild with a [[crate]] table.
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o: zip::write::SimpleFileOptions = Default::default();
            z.start_file(NATIVE_MANIFEST, o).unwrap();
            z.write_all(br#"{"schema":1,"id":"c","version":"1"}"#)
                .unwrap();
            z.start_file("mixins/c/horizon-mixin.toml", o).unwrap();
            z.write_all(b"[mod]\nid='c'\n[[crate]]\nid='e'\npath='e'\n")
                .unwrap();
            z.finish().unwrap();
        }
        bytes = buf.into_inner();
        let b = Bundle::from_bytes(&bytes).unwrap();
        assert_eq!(b.needs_unsupported(), Some("its own crate"));
    }

    #[test]
    fn ids_and_repins() {
        assert!(valid_id("fps_overlay"));
        assert!(!valid_id("Fps"));
        assert!(!valid_id("a/b"));
        let out = repin("[mod]\nid = \"a\"\n", "abc").unwrap();
        assert!(out.contains("horizon_revision = \"abc\""), "{out}");
        assert!(repin("compatibility = 3\n", "x").is_err());
    }
}
