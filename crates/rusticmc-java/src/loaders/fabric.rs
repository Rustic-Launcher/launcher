//! Fabric and Quilt: two meta APIs of the same shape (`/versions/loader/<game>` and
//! `/versions/loader/<game>/<loader>/profile/json`; Fabric under `/v2`, Quilt under `/v3`).

use super::{LoaderKind, LoaderVersion};
use crate::download::get_text;
use crate::install::Installer;
use crate::version::{VersionJson, safe_id};
use crate::{JavaError, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    /// Fabric says; Quilt doesn't (its betas say so in the version).
    #[serde(default)]
    stable: Option<bool>,
}

fn base(installer: &Installer, kind: LoaderKind) -> Result<String> {
    Ok(match kind {
        LoaderKind::Fabric => format!("{}/v2", installer.hosts.fabric_meta.trim_end_matches('/')),
        LoaderKind::Quilt => format!("{}/v3", installer.hosts.quilt_meta.trim_end_matches('/')),
        _ => return Err(JavaError::Invalid("not a meta API loader".into())),
    })
}

fn path_part(s: &str) -> Result<&str> {
    if safe_id(s) && !s.contains(' ') {
        Ok(s)
    } else {
        Err(JavaError::Invalid(format!("bad version {s:?}")))
    }
}

/// Parses a loader list (newest first, as both APIs answer).
pub fn parse_versions(text: &str) -> Result<Vec<LoaderVersion>> {
    let entries: Vec<Entry> =
        serde_json::from_str(text).map_err(|e| JavaError::bad(format!("loader list: {e}")))?;
    let mut out: Vec<LoaderVersion> = Vec::new();
    for e in entries {
        if out.iter().any(|v| v.version == e.loader.version) {
            continue;
        }
        let stable = e.loader.stable.unwrap_or_else(|| {
            let v = e.loader.version.to_ascii_lowercase();
            !(v.contains("beta") || v.contains("alpha") || v.contains("pre") || v.contains("rc"))
        });
        out.push(LoaderVersion {
            version: e.loader.version,
            stable,
        });
    }
    Ok(out)
}

/// The loader versions for `game`.
pub async fn versions(
    installer: &Installer,
    kind: LoaderKind,
    game: &str,
) -> Result<Vec<LoaderVersion>> {
    let url = format!(
        "{}/versions/loader/{}",
        base(installer, kind)?,
        path_part(game)?
    );
    match get_text(&installer.http, &url, &installer.extra_hosts).await {
        Ok(text) => parse_versions(&text),
        // An unknown game version is a 400 on Fabric's API.
        Err(JavaError::Http {
            status: 400 | 404, ..
        }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Writes the loader's profile as a version file; returns its id.
pub async fn install(
    installer: &Installer,
    kind: LoaderKind,
    game: &str,
    loader: &str,
) -> Result<String> {
    let url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        base(installer, kind)?,
        path_part(game)?,
        path_part(loader)?
    );
    let text = match get_text(&installer.http, &url, &installer.extra_hosts).await {
        Err(JavaError::Http {
            status: 400 | 404, ..
        }) => {
            return Err(JavaError::NoLoaderVersion {
                loader: kind.label().into(),
                game: game.into(),
            });
        }
        other => other?,
    };
    let profile = VersionJson::parse(&text)?;
    if profile.inherits_from.as_deref() != Some(game) {
        return Err(JavaError::bad(format!(
            "{} profile inherits from {:?}, not {game}",
            kind.label(),
            profile.inherits_from
        )));
    }
    installer.write_version(&profile.id, &text)?;
    Ok(profile.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_lists() {
        // Shapes of the two APIs' answers, written for this test.
        let fabric = r#"[{"loader": {"separator": ".", "build": 2, "maven": "net.fabricmc:fabric-loader:0.18.0", "version": "0.18.0", "stable": false}, "intermediary": {}},
                         {"loader": {"version": "0.17.2", "stable": true}, "intermediary": {}}]"#;
        let v = parse_versions(fabric).unwrap();
        assert_eq!(v.len(), 2);
        assert!(!v[0].stable && v[1].stable);
        let quilt =
            r#"[{"loader": {"version": "0.30.0-beta.1"}}, {"loader": {"version": "0.29.2"}}]"#;
        let q = parse_versions(quilt).unwrap();
        assert!(!q[0].stable && q[1].stable);
    }
}
