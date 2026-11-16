//! Mojang's version manifest (`version_manifest_v2.json` on piston-meta): every Java Edition
//! version with its type and the URL and sha1 of its version file.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    /// `release`, `snapshot`, `old_beta`, `old_alpha`.
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub release_time: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

/// Which versions a list shows: releases always, snapshots and old betas/alphas when asked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VersionFilter {
    pub snapshots: bool,
    pub old: bool,
}

impl VersionFilter {
    pub fn accepts(&self, kind: &str) -> bool {
        match kind {
            "release" => true,
            "snapshot" => self.snapshots,
            "old_beta" | "old_alpha" => self.old,
            _ => false,
        }
    }
}

impl VersionManifest {
    pub fn parse(text: &str) -> crate::Result<Self> {
        serde_json::from_str(text)
            .map_err(|e| crate::JavaError::bad(format!("version manifest: {e}")))
    }

    /// The versions `filter` accepts, newest first (the manifest's order).
    pub fn filtered(&self, filter: VersionFilter) -> Vec<&ManifestEntry> {
        self.versions
            .iter()
            .filter(|v| filter.accepts(&v.kind))
            .collect()
    }

    pub fn find(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|v| v.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest in Mojang's shape, written for this test (no Mojang file is committed).
    pub(crate) const FIXTURE: &str = r#"{
      "latest": {"release": "9.2", "snapshot": "9.3-snapshot-1"},
      "versions": [
        {"id": "9.3-snapshot-1", "type": "snapshot", "url": "https://piston-meta.mojang.com/v1/packages/aaaa/9.3-snapshot-1.json", "time": "2026-01-02T00:00:00+00:00", "releaseTime": "2026-01-02T00:00:00+00:00", "sha1": "aaaa", "complianceLevel": 1},
        {"id": "9.2", "type": "release", "url": "https://piston-meta.mojang.com/v1/packages/bbbb/9.2.json", "time": "2026-01-01T00:00:00+00:00", "releaseTime": "2026-01-01T00:00:00+00:00", "sha1": "bbbb", "complianceLevel": 1},
        {"id": "b9.9", "type": "old_beta", "url": "https://piston-meta.mojang.com/v1/packages/cccc/b9.9.json", "time": "2010-01-01T00:00:00+00:00", "releaseTime": "2010-01-01T00:00:00+00:00", "sha1": "cccc"},
        {"id": "a9.9", "type": "old_alpha", "url": "https://piston-meta.mojang.com/v1/packages/dddd/a9.9.json", "time": "2009-01-01T00:00:00+00:00", "releaseTime": "2009-01-01T00:00:00+00:00"}
      ]
    }"#;

    #[test]
    fn filters_by_type() {
        let m = VersionManifest::parse(FIXTURE).unwrap();
        assert_eq!(m.latest.release, "9.2");
        let ids = |f| {
            m.filtered(f)
                .iter()
                .map(|v| v.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(VersionFilter::default()), vec!["9.2"]);
        assert_eq!(
            ids(VersionFilter {
                snapshots: true,
                old: false
            }),
            vec!["9.3-snapshot-1", "9.2"]
        );
        assert_eq!(
            ids(VersionFilter {
                snapshots: false,
                old: true
            }),
            vec!["9.2", "b9.9", "a9.9"]
        );
        assert_eq!(m.find("b9.9").unwrap().sha1.as_deref(), Some("cccc"));
        assert!(m.find("a9.9").unwrap().sha1.is_none());
    }
}
