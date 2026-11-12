//! Reading a `.rplugin` before it is uploaded: a zip with `rustic.plugin.toml` at its root
//! (`[plugin] id, name, version, api, ...`, launcher/AGENTS.md "Rustic Server plugins"). The file is
//! installed as `<server>/plugins/<id>.rplugin`, so the id must be a safe file name.
//!
//! Disabling: Rustic Server's discovery loads only `*.rplugin` files and directories with a
//! manifest (`rusticmc-core/src/discovery.rs`), so a disabled plugin is renamed
//! `<id>.rplugin.disabled` and is ignored until renamed back. Changes apply at the next start.

use crate::error::{HostingError, Result};
use std::io::Read;
use std::path::Path;

pub const MANIFEST_FILE: &str = "rustic.plugin.toml";
pub const EXTENSION: &str = "rplugin";
pub const DISABLED_SUFFIX: &str = ".rplugin.disabled";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
}

/// Plugin ids are what Rustic's core accepts (`[a-z0-9_-]{1,64}`, docs/PLUGIN_FORMAT.md).
/// Checked here again because the id becomes a remote file name.
pub fn validate_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'));
    if ok {
        Ok(())
    } else {
        Err(HostingError::Invalid(format!(
            "`{id}` is not a valid plugin id ([a-z0-9_-], 1 to 64 characters)"
        )))
    }
}

/// Read and check the manifest of a `.rplugin` file.
pub fn read_manifest(path: &Path) -> Result<PluginManifest> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        HostingError::Invalid(format!("{} is not a .rplugin (zip): {e}", path.display()))
    })?;
    let mut text = String::new();
    {
        let mut entry = zip.by_name(MANIFEST_FILE).map_err(|_| {
            HostingError::Invalid(format!(
                "{} has no {MANIFEST_FILE} at its root",
                path.display()
            ))
        })?;
        if entry.size() > 64 * 1024 {
            return Err(HostingError::Invalid(format!(
                "{MANIFEST_FILE} is too large"
            )));
        }
        entry
            .by_ref()
            .take(64 * 1024)
            .read_to_string(&mut text)
            .map_err(|e| HostingError::Invalid(format!("{MANIFEST_FILE}: {e}")))?;
    }
    parse_manifest(&text)
}

pub fn parse_manifest(text: &str) -> Result<PluginManifest> {
    let v: toml::Value =
        toml::from_str(text).map_err(|e| HostingError::Invalid(format!("{MANIFEST_FILE}: {e}")))?;
    let plugin = v
        .get("plugin")
        .ok_or_else(|| HostingError::Invalid(format!("{MANIFEST_FILE} has no [plugin] table")))?;
    let field = |k: &str| -> Result<String> {
        plugin
            .get(k)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                HostingError::Invalid(format!("{MANIFEST_FILE}: [plugin] {k} is missing"))
            })
    };
    let id = field("id")?;
    validate_id(&id)?;
    Ok(PluginManifest {
        name: field("name").unwrap_or_else(|_| id.clone()),
        version: field("version")?,
        id,
    })
}

/// The id a file in `plugins/` stands for, and whether it is enabled.
pub fn classify(file: &str) -> Option<(String, bool)> {
    if let Some(id) = file.strip_suffix(DISABLED_SUFFIX) {
        return Some((id.to_owned(), false));
    }
    file.strip_suffix(".rplugin")
        .map(|id| (id.to_owned(), true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    pub(crate) fn make_rplugin(dir: &Path, manifest: &str) -> std::path::PathBuf {
        let path = dir.join("p.rplugin");
        let f = std::fs::File::create(&path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        z.start_file(MANIFEST_FILE, o).unwrap();
        z.write_all(manifest.as_bytes()).unwrap();
        z.start_file("plugin.wasm", o).unwrap();
        z.write_all(b"\0asm").unwrap();
        z.finish().unwrap();
        path
    }

    #[test]
    fn reads_a_manifest() {
        let d = tempfile::tempdir().unwrap();
        let p = make_rplugin(
            d.path(),
            "[plugin]\nid = \"homes\"\nname = \"Homes\"\nversion = \"1.2.0\"\napi = \"0.1\"\nentry = \"plugin.wasm\"\n",
        );
        let m = read_manifest(&p).unwrap();
        assert_eq!(m.id, "homes");
        assert_eq!(m.version, "1.2.0");
    }

    #[test]
    fn refuses_bad_ids_and_files() {
        let d = tempfile::tempdir().unwrap();
        let p = make_rplugin(d.path(), "[plugin]\nid = \"../../etc\"\nversion = \"1\"\n");
        assert!(read_manifest(&p).is_err());
        let p = make_rplugin(d.path(), "[nope]\n");
        assert!(read_manifest(&p).is_err());
        let not_zip = d.path().join("x.rplugin");
        std::fs::write(&not_zip, b"hello").unwrap();
        assert!(read_manifest(&not_zip).is_err());
        assert!(validate_id("Homes").is_err());
        assert!(validate_id("a b").is_err());
        assert!(validate_id("a..b").is_err());
        assert!(validate_id("chat-format_2").is_ok());
        assert!(validate_id("2fa").is_ok());
        assert!(validate_id("a.b").is_err());
    }

    #[test]
    fn classifies_files() {
        assert_eq!(classify("homes.rplugin"), Some(("homes".into(), true)));
        assert_eq!(
            classify("homes.rplugin.disabled"),
            Some(("homes".into(), false))
        );
        assert_eq!(classify("config.toml"), None);
    }
}
