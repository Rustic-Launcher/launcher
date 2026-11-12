//! What a downloaded file says about itself (its manifest), read without running anything:
//! `horizon-mod.toml` (content mod), `horizon-native.json` (native mod), `horizon-modpack.toml`
//! (`.hpack`), `horizon-shaders.toml` (`.hshader`), `pack.mcmeta` (resource and data packs),
//! `shaders/` (Iris/OptiFine packs) (horizon/docs/api/mod-files.md).
use crate::error::{ContentError, Result};
use crate::model::{ContentKind, ContentVersion};
use std::io::Read;
use std::path::Path;

/// Reads one entry of a zip (at most 1 MiB: manifests are small).
pub fn read_entry(path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let found = match zip.by_name(name) {
        Ok(mut e) => {
            if e.size() > 1 << 20 {
                return Err(ContentError::Refused(format!("{name} is over 1 MiB")));
            }
            let mut b = Vec::new();
            e.read_to_end(&mut b)?;
            Some(b)
        }
        Err(zip::result::ZipError::FileNotFound) => None,
        Err(e) => return Err(e.into()),
    };
    Ok(found)
}

/// Every entry name of a zip.
pub fn entry_names(path: &Path) -> Result<Vec<String>> {
    let zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    Ok(zip.file_names().map(str::to_owned).collect())
}

/// A content mod's manifest (the fields Rustic reads).
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ModManifest {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub side: Option<String>,
}

/// A native mod's `horizon-native.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct NativeManifest {
    pub schema: u32,
    pub id: String,
    pub version: String,
    pub game: String,
    pub horizon: String,
    pub mixins: String,
    #[serde(default)]
    pub hooks: Option<u32>,
}

pub fn mod_manifest(path: &Path) -> Result<ModManifest> {
    let bytes = read_entry(path, "horizon-mod.toml")?
        .ok_or_else(|| ContentError::Refused("no horizon-mod.toml".into()))?;
    toml::from_str(&String::from_utf8_lossy(&bytes))
        .map_err(|e| ContentError::Refused(format!("horizon-mod.toml: {e}")))
}

pub fn native_manifest(path: &Path) -> Result<NativeManifest> {
    let bytes = read_entry(path, "horizon-native.json")?
        .ok_or_else(|| ContentError::Refused("no horizon-native.json".into()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| ContentError::Refused(format!("horizon-native.json: {e}")))
}

/// `pack.mcmeta`'s supported formats: `(lowest, highest)` major versions, from
/// `min_format`/`max_format`, the older `supported_formats`, or `pack_format`.
pub fn pack_formats(mcmeta: &[u8]) -> Option<(i64, i64)> {
    let v: serde_json::Value = serde_json::from_slice(mcmeta).ok()?;
    let pack = &v["pack"];
    let major = |x: &serde_json::Value| -> Option<i64> {
        x.as_i64().or_else(|| {
            x.as_array()
                .and_then(|a| a.first())
                .and_then(|m| m.as_i64())
        })
    };
    if let (Some(lo), Some(hi)) = (major(&pack["min_format"]), major(&pack["max_format"])) {
        return Some((lo, hi));
    }
    match &pack["supported_formats"] {
        serde_json::Value::Array(a) if a.len() == 2 => {
            return Some((a[0].as_i64()?, a[1].as_i64()?));
        }
        serde_json::Value::Object(o) => {
            return Some((
                o.get("min_inclusive")?.as_i64()?,
                o.get("max_inclusive")?.as_i64()?,
            ));
        }
        serde_json::Value::Number(n) => {
            let f = n.as_i64()?;
            return Some((f, f));
        }
        _ => {}
    }
    let f = pack["pack_format"].as_i64()?;
    Some((f, f))
}

/// The kind a file is, from what it holds.
pub fn detect_kind(path: &Path) -> Result<Option<ContentKind>> {
    let names = entry_names(path)?;
    let has = |n: &str| names.iter().any(|e| e == n);
    let under = |p: &str| names.iter().any(|e| e.starts_with(p));
    Ok(if has("rustic.plugin.toml") {
        Some(ContentKind::Plugin)
    } else if has("horizon-native.json") {
        Some(ContentKind::NativeMod)
    } else if has("horizon-mod.toml") {
        Some(ContentKind::Mod)
    } else if has("horizon-modpack.toml") || has("modrinth.index.json") || has("rustic-pack.json") {
        Some(ContentKind::Modpack)
    } else if has("horizon-shaders.toml") {
        Some(ContentKind::Shader)
    } else if under("shaders/") {
        Some(ContentKind::ShaderPack)
    } else if has("pack.mcmeta") && under("data/") && !under("assets/") {
        Some(ContentKind::DataPack)
    } else if has("pack.mcmeta") {
        Some(ContentKind::ResourcePack)
    } else if names.iter().any(|n| n.ends_with(".class")) || has("fabric.mod.json") {
        // A Java mod jar: kind Mod, judged by `compat` as Java.
        Some(ContentKind::Mod)
    } else {
        None
    })
}

/// The file's own manifest matches the registry entry `v` (id, version, API), as
/// `horizon_mods::index::check_file` requires.
pub fn check_matches(v: &ContentVersion, path: &Path) -> Result<()> {
    let refuse = |what: String| Err(ContentError::Refused(what));
    match v.kind {
        ContentKind::Mod => {
            let m = mod_manifest(path)?;
            let api = v.horizon.as_ref().map(|h| h.api.as_str()).unwrap_or("");
            if m.id != v.project_id || m.version != v.version || (!api.is_empty() && m.api != api) {
                return refuse(format!(
                    "horizon-mod.toml says {} {} (API {}), the index {} {} (API {api})",
                    m.id, m.version, m.api, v.project_id, v.version
                ));
            }
        }
        ContentKind::NativeMod => {
            let m = native_manifest(path)?;
            if m.id != v.project_id || m.version != v.version {
                return refuse(format!(
                    "horizon-native.json says {} {}, the index {} {}",
                    m.id, m.version, v.project_id, v.version
                ));
            }
        }
        ContentKind::Modpack => {
            let bytes = read_entry(path, "horizon-modpack.toml")?
                .ok_or_else(|| ContentError::Refused("no horizon-modpack.toml".into()))?;
            let t: toml::Table = toml::from_str(&String::from_utf8_lossy(&bytes))
                .map_err(|e| ContentError::Refused(format!("horizon-modpack.toml: {e}")))?;
            let id = t.get("id").and_then(|x| x.as_str()).unwrap_or("");
            let version = t.get("version").and_then(|x| x.as_str()).unwrap_or("");
            if id != v.project_id || version != v.version {
                return refuse(format!(
                    "horizon-modpack.toml says {id} {version}, the index {} {}",
                    v.project_id, v.version
                ));
            }
        }
        ContentKind::ResourcePack => {
            if read_entry(path, "pack.mcmeta")?.is_none() {
                return refuse("a resource pack without pack.mcmeta".into());
            }
        }
        ContentKind::DataPack => {
            let names = entry_names(path)?;
            if !names.iter().any(|n| n == "pack.mcmeta")
                || !names.iter().any(|n| n.starts_with("data/"))
            {
                return refuse("a data pack holds pack.mcmeta and data/".into());
            }
        }
        ContentKind::Shader => {
            let bytes = read_entry(path, "horizon-shaders.toml")?
                .ok_or_else(|| ContentError::Refused("no horizon-shaders.toml".into()))?;
            let t: toml::Table = toml::from_str(&String::from_utf8_lossy(&bytes))
                .map_err(|e| ContentError::Refused(format!("horizon-shaders.toml: {e}")))?;
            let id = t.get("id").and_then(|x| x.as_str()).unwrap_or("");
            let version = t.get("version").and_then(|x| x.as_str()).unwrap_or("");
            if id != v.project_id || version != v.version {
                return refuse(format!(
                    "horizon-shaders.toml says {id} {version}, the index {} {}",
                    v.project_id, v.version
                ));
            }
        }
        ContentKind::ShaderPack => {
            if !entry_names(path)?.iter().any(|n| n.starts_with("shaders/")) {
                return refuse("a shader pack holds a shaders/ folder".into());
            }
        }
        ContentKind::Plugin => {
            crate::plugins::read_rplugin(path)?;
        }
    }
    Ok(())
}
