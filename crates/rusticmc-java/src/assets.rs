//! The asset index (`assets/indexes/<id>.json`) and its objects (`assets/objects/xx/<hash>`).
//! Old indexes ask for a copy under readable names: `virtual` (`assets/virtual/<id>/<name>`,
//! 1.6 to 1.7) or `map_to_resources` (`<game dir>/resources/<name>`, before 1.6).

use crate::download::{FileDownload, is_sha1};
use crate::maven::safe_relative;
use crate::{JavaError, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct AssetIndex {
    #[serde(default)]
    pub objects: BTreeMap<String, AssetObject>,
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    #[serde(default)]
    pub size: Option<u64>,
}

impl AssetIndex {
    pub fn parse(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|e| JavaError::bad(format!("asset index: {e}")))
    }

    /// Every object once (several names can share one), served from `host`.
    pub fn downloads(&self, host: &str) -> Result<Vec<FileDownload>> {
        let mut out: Vec<FileDownload> = Vec::with_capacity(self.objects.len());
        for o in self.objects.values() {
            if !is_sha1(&o.hash) {
                return Err(JavaError::bad("an asset has no valid hash"));
            }
            let rel = format!("{}/{}", &o.hash[..2], o.hash);
            out.push(FileDownload {
                url: format!("{}/{rel}", host.trim_end_matches('/')),
                sha1: Some(o.hash.clone()),
                size: o.size,
                path: Path::new("assets/objects").join(rel),
            });
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out.dedup_by(|a, b| a.path == b.path);
        Ok(out)
    }

    /// Where the readable copies go, if this index wants them.
    pub fn copies_dir(&self, root: &Path, index_id: &str, game_dir: &Path) -> Option<PathBuf> {
        if self.map_to_resources {
            Some(game_dir.join("resources"))
        } else if self.is_virtual {
            Some(root.join("assets/virtual").join(index_id))
        } else {
            None
        }
    }

    /// Copies (hard-links when possible) every object to its readable name under `dir`, skipping
    /// the ones already there with the right size.
    pub fn place_copies(&self, root: &Path, dir: &Path) -> Result<()> {
        for (name, o) in &self.objects {
            let Some(rel) = safe_relative(name) else {
                continue;
            };
            if !is_sha1(&o.hash) {
                continue;
            }
            let to = dir.join(rel);
            let from = root.join("assets/objects").join(&o.hash[..2]).join(&o.hash);
            let same = std::fs::metadata(&to).is_ok_and(|m| o.size.is_none_or(|s| s == m.len()));
            if same {
                continue;
            }
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| JavaError::io(parent, e))?;
            }
            let _ = std::fs::remove_file(&to);
            if std::fs::hard_link(&from, &to).is_err() {
                std::fs::copy(&from, &to).map_err(|e| JavaError::io(&to, e))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn objects_once_each() {
        let idx = AssetIndex::parse(&format!(
            r#"{{"objects": {{"a.ogg": {{"hash": "{A}", "size": 1}}, "b.ogg": {{"hash": "{A}", "size": 1}}, "c.png": {{"hash": "{B}", "size": 2}}}}}}"#
        ))
        .unwrap();
        let d = idx
            .downloads("https://resources.download.minecraft.net")
            .unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(
            d[0].url,
            format!("https://resources.download.minecraft.net/aa/{A}")
        );
        assert_eq!(d[0].path, Path::new("assets/objects/aa").join(A));
        assert!(!idx.is_virtual && !idx.map_to_resources);
    }

    #[test]
    fn readable_copies_for_old_indexes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let obj = root.join("assets/objects/aa");
        std::fs::create_dir_all(&obj).unwrap();
        std::fs::write(obj.join(A), b"x").unwrap();
        let idx = AssetIndex::parse(&format!(
            r#"{{"map_to_resources": true, "objects": {{"sound/a.ogg": {{"hash": "{A}", "size": 1}}, "../evil": {{"hash": "{A}", "size": 1}}}}}}"#
        ))
        .unwrap();
        let game = root.join("game");
        let dir = idx.copies_dir(root, "pre-1.6", &game).unwrap();
        assert_eq!(dir, game.join("resources"));
        idx.place_copies(root, &dir).unwrap();
        assert_eq!(std::fs::read(dir.join("sound/a.ogg")).unwrap(), b"x");
        assert!(!root.join("evil").exists());
        let v = AssetIndex::parse(r#"{"virtual": true, "objects": {}}"#).unwrap();
        assert_eq!(
            v.copies_dir(root, "legacy", &game).unwrap(),
            root.join("assets/virtual/legacy")
        );
    }
}
