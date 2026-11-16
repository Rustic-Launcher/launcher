//! Forge and NeoForge: versions from their Maven metadata, installs through their official
//! installer jars run headless (`--installClient <game folder>`) with Mojang's Java.

use super::{LoaderKind, LoaderVersion, maven_versions};
use crate::download::{FileDownload, fetch_all, get_text, is_sha1};
use crate::install::Installer;
use crate::maven::Coordinate;
use crate::platform::compare_versions;
use crate::process::no_window;
use crate::version::safe_id;
use crate::{JavaError, Result};
use rusticmc_launcher_core::CancellationToken;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The game version a NeoForge version is for: `21.1.266` → `1.21.1`, `21.0.5` → `1.21`, and
/// from the year-numbered game versions on, `26.3.0.103` → `26.3`, `26.1.2.126` → `26.1.2`.
pub fn neoforge_game_version(v: &str) -> Option<String> {
    let core = v.split('-').next()?;
    let parts: Vec<&str> = core.split('.').collect();
    let major: u32 = parts.first()?.parse().ok()?;
    if major >= 25 {
        // year.drop.hotfix.build
        let (drop, hotfix) = (parts.get(1)?, parts.get(2)?);
        parts.get(3)?;
        return Some(if *hotfix == "0" {
            format!("{major}.{drop}")
        } else {
            format!("{major}.{drop}.{hotfix}")
        });
    }
    let minor = parts.get(1)?;
    parts.get(2)?;
    Some(if *minor == "0" {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    })
}

/// NeoForge versions for `game`, newest first.
pub fn neoforge_for(all: &[String], game: &str) -> Vec<LoaderVersion> {
    let mut out: Vec<LoaderVersion> = all
        .iter()
        .filter(|v| neoforge_game_version(v).as_deref() == Some(game))
        .map(|v| LoaderVersion {
            version: v.clone(),
            stable: !v.contains("beta") && !v.contains("alpha"),
        })
        .collect();
    out.sort_by(|a, b| compare_versions(&b.version, &a.version));
    out
}

/// Forge versions for `game` (metadata entries `<game>-<forge>`), newest first; the recommended
/// one (promotions) marked stable when there is one.
pub fn forge_for(all: &[String], game: &str, recommended: Option<&str>) -> Vec<LoaderVersion> {
    let prefix = format!("{game}-");
    let mut out: Vec<LoaderVersion> = all
        .iter()
        .filter_map(|v| v.strip_prefix(&prefix))
        // `1.7.10-10.13.4.1614-1.7.10`: keep the Forge part only.
        .map(|rest| rest.split('-').next().unwrap_or(rest).to_owned())
        .map(|forge| LoaderVersion {
            stable: recommended.is_none_or(|r| r == forge),
            version: forge,
        })
        .collect();
    out.sort_by(|a, b| compare_versions(&b.version, &a.version));
    out.dedup_by(|a, b| a.version == b.version);
    out
}

/// The full Maven version of a Forge build (`1.20.1-47.3.0`, old ones `1.7.10-10.13.4.1614-1.7.10`).
fn forge_maven_version(all: &[String], game: &str, forge: &str) -> String {
    let exact = format!("{game}-{forge}");
    all.iter()
        .find(|v| **v == exact || v.starts_with(&format!("{exact}-")))
        .cloned()
        .unwrap_or(exact)
}

async fn metadata(installer: &Installer, kind: LoaderKind) -> Result<Vec<String>> {
    let url = match kind {
        LoaderKind::Forge => format!(
            "{}/net/minecraftforge/forge/maven-metadata.xml",
            installer.hosts.forge_maven.trim_end_matches('/')
        ),
        LoaderKind::NeoForge => format!(
            "{}/net/neoforged/neoforge/maven-metadata.xml",
            installer.hosts.neoforge_maven.trim_end_matches('/')
        ),
        _ => return Err(JavaError::Invalid("not an installer loader".into())),
    };
    Ok(maven_versions(
        &get_text(&installer.http, &url, &installer.extra_hosts).await?,
    ))
}

/// Forge's recommended build for `game` (`promotions_slim.json`), if any.
async fn forge_recommended(installer: &Installer, game: &str) -> Option<String> {
    let url = format!(
        "{}/net/minecraftforge/forge/promotions_slim.json",
        installer.hosts.forge_maven.trim_end_matches('/')
    );
    let text = get_text(&installer.http, &url, &installer.extra_hosts)
        .await
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v["promos"][format!("{game}-recommended")]
        .as_str()
        .map(str::to_owned)
}

pub async fn versions(
    installer: &Installer,
    kind: LoaderKind,
    game: &str,
) -> Result<Vec<LoaderVersion>> {
    let all = metadata(installer, kind).await?;
    Ok(match kind {
        LoaderKind::NeoForge => neoforge_for(&all, game),
        _ => {
            let rec = forge_recommended(installer, game).await;
            forge_for(&all, game, rec.as_deref())
        }
    })
}

/// The id of the version file an installer writes, from its `install_profile.json`.
pub fn installer_version_id(profile: &serde_json::Value) -> Option<String> {
    profile["version"]
        .as_str()
        .or_else(|| profile["install"]["target"].as_str())
        .or_else(|| profile["versionInfo"]["id"].as_str())
        .filter(|id| safe_id(id))
        .map(str::to_owned)
}

fn read_zip_entry(jar: &Path, name: &str) -> Result<Option<Vec<u8>>> {
    let f = std::fs::File::open(jar).map_err(|e| JavaError::io(jar, e))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| JavaError::Installer {
        loader: "installer".into(),
        message: format!("not a jar: {e}"),
    })?;
    let Ok(mut entry) = zip.by_name(name) else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|e| JavaError::io(jar, e))?;
    Ok(Some(bytes))
}

/// Installs Forge or NeoForge `loader` for `game` with `java`; returns the version id. The game's
/// own files must be installed first (the installer reads its client jar).
pub async fn install(
    installer: &Installer,
    kind: LoaderKind,
    game: &str,
    loader: &str,
    java: &Path,
    cancel: &CancellationToken,
    on_line: &(dyn Fn(&str) + Send + Sync),
) -> Result<String> {
    let label = kind.label().to_owned();
    if !safe_id(loader) || !safe_id(game) {
        return Err(JavaError::Invalid("bad loader version".into()));
    }
    let (url, file) = match kind {
        LoaderKind::Forge => {
            let all = metadata(installer, kind).await?;
            let full = forge_maven_version(&all, game, loader);
            (
                format!(
                    "{}/net/minecraftforge/forge/{full}/forge-{full}-installer.jar",
                    installer.hosts.forge_maven.trim_end_matches('/')
                ),
                format!("forge-{full}-installer.jar"),
            )
        }
        LoaderKind::NeoForge => (
            format!(
                "{}/net/neoforged/neoforge/{loader}/neoforge-{loader}-installer.jar",
                installer.hosts.neoforge_maven.trim_end_matches('/')
            ),
            format!("neoforge-{loader}-installer.jar"),
        ),
        _ => return Err(JavaError::Invalid("not an installer loader".into())),
    };
    let sha1 = get_text(
        &installer.http,
        &format!("{url}.sha1"),
        &installer.extra_hosts,
    )
    .await
    .ok()
    .map(|s| s.trim().chars().take(40).collect::<String>())
    .filter(|s| is_sha1(s));
    let rel = PathBuf::from("installers").join(&file);
    fetch_all(
        &installer.downloads,
        &installer.root,
        vec![FileDownload {
            url: url.clone(),
            sha1,
            size: None,
            path: rel.clone(),
        }],
        &installer.extra_hosts,
        cancel,
        &|_, _| {},
    )
    .await
    .map_err(|e| match e {
        JavaError::Download(rusticmc_launcher_core::DownloadError::Http {
            status: 404, ..
        }) => JavaError::NoLoaderVersion {
            loader: label.clone(),
            game: game.into(),
        },
        other => other,
    })?;
    let jar = installer.root.join(&rel);
    let profile_bytes =
        read_zip_entry(&jar, "install_profile.json")?.ok_or_else(|| JavaError::Installer {
            loader: label.clone(),
            message: "no install_profile.json in the installer".into(),
        })?;
    let profile: serde_json::Value =
        serde_json::from_slice(&profile_bytes).map_err(|e| JavaError::Installer {
            loader: label.clone(),
            message: format!("install_profile.json: {e}"),
        })?;
    let id = installer_version_id(&profile).ok_or_else(|| JavaError::Installer {
        loader: label.clone(),
        message: "the installer names no version".into(),
    })?;
    if profile.get("versionInfo").is_some() {
        install_legacy(installer, &jar, &profile, &id)?;
        return Ok(id);
    }
    // The installers want the official launcher's profile file in the folder.
    let profiles = installer.root.join("launcher_profiles.json");
    if !profiles.is_file() {
        std::fs::write(&profiles, br#"{"profiles":{}}"#)
            .map_err(|e| JavaError::io(&profiles, e))?;
    }
    run_installer(java, &jar, &installer.root, &label, cancel, on_line).await?;
    if !installer.version_file(&id).is_file() {
        return Err(JavaError::Installer {
            loader: label,
            message: format!("it did not write the version {id}"),
        });
    }
    Ok(id)
}

/// Runs `java -jar <installer> --installClient <root>`, its output to `on_line`.
async fn run_installer(
    java: &Path,
    jar: &Path,
    root: &Path,
    label: &str,
    cancel: &CancellationToken,
    on_line: &(dyn Fn(&str) + Send + Sync),
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-Djava.awt.headless=true")
        .arg("-jar")
        .arg(jar)
        .arg("--installClient")
        .arg(root)
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| JavaError::Installer {
        loader: label.into(),
        message: format!("could not start Java: {e}"),
    })?;
    let mut out = BufReader::new(child.stdout.take().expect("piped")).lines();
    let mut err = BufReader::new(child.stderr.take().expect("piped")).lines();
    let mut tail: Vec<String> = Vec::new();
    let push = |l: String, tail: &mut Vec<String>| {
        on_line(&l);
        tail.push(l);
        if tail.len() > 20 {
            tail.remove(0);
        }
    };
    let deadline = tokio::time::sleep(Duration::from_secs(30 * 60));
    tokio::pin!(deadline);
    let (mut out_done, mut err_done) = (false, false);
    while !(out_done && err_done) {
        tokio::select! {
            l = out.next_line(), if !out_done => match l {
                Ok(Some(l)) => push(l, &mut tail),
                _ => out_done = true,
            },
            l = err.next_line(), if !err_done => match l {
                Ok(Some(l)) => push(l, &mut tail),
                _ => err_done = true,
            },
            _ = cancel.cancelled() => {
                let _ = child.start_kill();
                return Err(JavaError::Cancelled);
            }
            _ = &mut deadline => {
                let _ = child.start_kill();
                return Err(JavaError::Installer { loader: label.into(), message: "timed out".into() });
            }
        }
    }
    let status = child.wait().await.map_err(|e| JavaError::Installer {
        loader: label.into(),
        message: e.to_string(),
    })?;
    if !status.success() {
        let last = tail
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_default();
        return Err(JavaError::Installer {
            loader: label.into(),
            message: format!("exit {:?}: {last}", status.code()),
        });
    }
    Ok(())
}

/// Old Forge installers (before 1.13): the version file is `versionInfo`, and the universal jar
/// inside the installer goes to the library path `install.path` names.
fn install_legacy(
    installer: &Installer,
    jar: &Path,
    profile: &serde_json::Value,
    id: &str,
) -> Result<()> {
    let bad = |m: &str| JavaError::Installer {
        loader: "Forge".into(),
        message: m.into(),
    };
    let info = profile["versionInfo"].clone();
    let text = serde_json::to_string_pretty(&info).map_err(|e| bad(&e.to_string()))?;
    let lib = profile["install"]["path"]
        .as_str()
        .and_then(Coordinate::parse)
        .ok_or_else(|| bad("no install.path"))?;
    let inner = profile["install"]["filePath"]
        .as_str()
        .ok_or_else(|| bad("no install.filePath"))?;
    let bytes = read_zip_entry(jar, inner)?.ok_or_else(|| bad("no universal jar"))?;
    let dest = installer.root.join("libraries").join(lib.path());
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| JavaError::io(parent, e))?;
    }
    std::fs::write(&dest, bytes).map_err(|e| JavaError::io(&dest, e))?;
    installer.write_version(id, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_game_versions() {
        assert_eq!(neoforge_game_version("21.1.266").as_deref(), Some("1.21.1"));
        assert_eq!(
            neoforge_game_version("21.0.5-beta").as_deref(),
            Some("1.21")
        );
        assert_eq!(neoforge_game_version("20.4.237").as_deref(), Some("1.20.4"));
        assert_eq!(neoforge_game_version("26.3.0.103").as_deref(), Some("26.3"));
        assert_eq!(
            neoforge_game_version("26.1.2.126").as_deref(),
            Some("26.1.2")
        );
        assert_eq!(neoforge_game_version("x"), None);
        let all: Vec<String> = [
            "21.1.1",
            "26.3.0.99",
            "21.1.266",
            "26.3.0.103",
            "21.1.10-beta",
        ]
        .map(String::from)
        .to_vec();
        let v = neoforge_for(&all, "1.21.1");
        let ids: Vec<_> = v.iter().map(|l| l.version.as_str()).collect();
        assert_eq!(ids, vec!["21.1.266", "21.1.10-beta", "21.1.1"]);
        assert!(!v[1].stable);
        assert_eq!(neoforge_for(&all, "26.3")[0].version, "26.3.0.103");
    }

    #[test]
    fn forge_versions() {
        let all: Vec<String> = [
            "1.21-51.0.33",
            "1.20.1-47.3.0",
            "1.20.1-47.10.1",
            "1.7.10-10.13.4.1614-1.7.10",
            "1.20.10-1.0",
        ]
        .map(String::from)
        .to_vec();
        let v = forge_for(&all, "1.20.1", Some("47.3.0"));
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].version, "47.10.1");
        assert!(!v[0].stable && v[1].stable);
        assert_eq!(forge_for(&all, "1.7.10", None)[0].version, "10.13.4.1614");
        assert_eq!(
            forge_maven_version(&all, "1.7.10", "10.13.4.1614"),
            "1.7.10-10.13.4.1614-1.7.10"
        );
        assert_eq!(
            forge_maven_version(&all, "1.20.1", "47.3.0"),
            "1.20.1-47.3.0"
        );
    }

    #[test]
    fn installer_profiles() {
        let modern =
            serde_json::json!({"spec": 1, "version": "neoforge-21.1.266", "minecraft": "1.21.1"});
        assert_eq!(
            installer_version_id(&modern).as_deref(),
            Some("neoforge-21.1.266")
        );
        let legacy = serde_json::json!({"install": {"target": "1.12.2-forge-14.23.5.2860"}, "versionInfo": {"id": "1.12.2-forge-14.23.5.2860"}});
        assert_eq!(
            installer_version_id(&legacy).as_deref(),
            Some("1.12.2-forge-14.23.5.2860")
        );
        assert!(installer_version_id(&serde_json::json!({"version": "../x"})).is_none());
    }
}
