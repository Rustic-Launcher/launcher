//! Horizon builds: listing what a source publishes, installing a build (signed checksums,
//! download, unpack into a folder of its own, kept whole: horizon/docs/api/launch-guide.md step
//! 1), and the installed builds.
//!
//! Sources (ADR 0045, `launcher.toml` `[builds]`):
//! - **GitHub** releases with Horizon's layout (`horizon-rust/horizon`): one archive per platform
//!   (`<prefix>-windows-x64.zip`, `-windows-arm64.zip`, `-linux-x86_64.tar.gz`, `-macos-arm64.zip`,
//!   `-macos-x64.zip`), `SHA256SUMS`, and `SHA256SUMS.v2.sig` (Nightlies, signed for
//!   `nightly-<date>-<commit>` from the title) or `SHA256SUMS.sig` (numbered releases, signed for
//!   the tag).
//! - **Channel**: a JSON index Rustic's CI publishes for the "Rustic edition" (Horizon with the
//!   rustic-companion trait mod), with the same SHA256SUMS + `SHA256SUMS.sig` per build, signed
//!   with Rustic's keys under its own namespace.

use crate::error::{InstanceError, Result};
use crate::signing::{self, PublicKey};
use rusticmc_launcher_core::config::{BuildSourceConfig, BuildSourceKind};
use rusticmc_launcher_core::{Checksum, DownloadManager, DownloadRequest, fsutil, reqwest, time};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The game program's name in a build.
pub fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// The archive suffix for this computer (`windows-x64.zip`), `None` where there is no build.
pub fn platform_suffix() -> Option<&'static str> {
    platform_suffix_for(std::env::consts::OS, std::env::consts::ARCH)
}

pub fn platform_suffix_for(os: &str, arch: &str) -> Option<&'static str> {
    Some(match (os, arch) {
        ("windows", "x86_64") => "windows-x64.zip",
        ("windows", "aarch64") => "windows-arm64.zip",
        ("linux", "x86_64") => "linux-x86_64.tar.gz",
        ("macos", "aarch64") => "macos-arm64.zip",
        ("macos", "x86_64") => "macos-x64.zip",
        _ => return None,
    })
}

/// A file attached to a release.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
}

/// A build a source publishes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildRelease {
    pub source: String,
    /// What it installs as (its folder name): `nightly-2026-10-28-0c02067`, `v2026.10.12`.
    pub version: String,
    pub title: String,
    pub published_at: String,
    pub prerelease: bool,
    pub notes: String,
    /// This computer's archive, if the build has one.
    pub asset: Option<ReleaseAsset>,
    pub sums_url: String,
    pub signature_url: String,
    /// What the signature names after the namespace.
    pub signed_for: String,
}

/// An installed build.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledBuild {
    pub version: String,
    pub source: String,
    pub dir: PathBuf,
    pub installed_at: u64,
    pub archive_sha256: String,
}

/// The Rustic edition's game program (ADRs 0006, 0070), beside Horizon's in its build folder.
pub const EDITION_CLIENT: &str = "rustic-client";

impl InstalledBuild {
    /// The game program instances of this build run: `rustic-client` in a Rustic edition build
    /// (Horizon's game with the companion linked in), else Horizon's `horizon-client`. Both take
    /// the same Launch API (`--launch-api`, the session on standard input).
    pub fn client(&self) -> PathBuf {
        let edition = self.dir.join(exe(EDITION_CLIENT));
        if self.source == "rustic-edition" && edition.is_file() {
            return edition;
        }
        self.dir.join(exe("horizon-client"))
    }
    pub fn preparer(&self) -> PathBuf {
        self.dir.join(exe("horizon-prepare"))
    }
    /// `horizon-server`, the supervisor every release folder holds (horizon/docs/hosting.md).
    pub fn server_supervisor(&self) -> PathBuf {
        self.dir.join(exe("horizon-server"))
    }
    /// `rustic-server`, the Rustic Server engine a Rustic edition bundle ships beside the
    /// supervisor, built from the same Horizon commit as the rest of the folder (ADR 0152).
    /// A plain Horizon build has none.
    pub fn rustic_server(&self) -> PathBuf {
        self.dir.join(exe(RUSTIC_SERVER))
    }
}

/// The Rustic Server engine's program name, without the platform's suffix.
pub const RUSTIC_SERVER: &str = "rustic-server";

/// A configured source with its keys.
#[derive(Clone, Debug)]
pub struct BuildSource {
    pub config: BuildSourceConfig,
    pub keys: Vec<PublicKey>,
    /// Override of the GitHub API base (tests).
    pub github_api: String,
}

impl BuildSource {
    pub fn new(config: BuildSourceConfig) -> Result<Self> {
        let mut keys = Vec::new();
        if config.builtin_horizon_keys {
            keys.extend(
                signing::parse_keys(signing::HORIZON_KEYS).map_err(InstanceError::Signature)?,
            );
        }
        if config.signature_namespace == signing::RUSTIC_NAMESPACE {
            keys.extend(
                signing::parse_keys(signing::RUSTIC_KEYS).map_err(InstanceError::Signature)?,
            );
        }
        for k in &config.public_keys {
            let key = signing::parse_key(k).map_err(InstanceError::Signature)?;
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        Ok(Self {
            config,
            keys,
            github_api: "https://api.github.com".into(),
        })
    }

    fn asset_name(&self) -> Option<String> {
        platform_suffix().map(|s| format!("{}-{s}", self.config.asset_prefix))
    }
}

/// The channel index format (ADR 0045).
#[derive(Debug, Deserialize)]
struct Channel {
    format: u32,
    builds: Vec<ChannelBuild>,
}

#[derive(Debug, Deserialize)]
struct ChannelBuild {
    version: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    notes: String,
    /// Assets, SHA256SUMS and SHA256SUMS.sig are under this URL (ending in `/`).
    base_url: String,
    assets: Vec<ChannelAsset>,
}

#[derive(Debug, Deserialize)]
struct ChannelAsset {
    name: String,
    #[serde(default)]
    size: Option<u64>,
}

/// What an install reports.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum InstallStage {
    Verifying,
    Downloading,
    Unpacking,
    Done { version: String },
}

/// Lists, installs and removes builds under `<data>/builds`.
#[derive(Clone)]
pub struct BuildManager {
    root: PathBuf,
    http: reqwest::Client,
    downloads: DownloadManager,
}

const BUILD_FILE: &str = "rustic-build.json";
/// Where an installed bundle's own `rustic-build.json` (what CI built it from) is kept.
pub const BUNDLE_FILE: &str = "rustic-bundle.json";

impl BuildManager {
    pub fn new(root: PathBuf, http: reqwest::Client, downloads: DownloadManager) -> Self {
        Self {
            root,
            http,
            downloads,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let r = self
            .http
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| InstanceError::Http(e.to_string()))?;
        if !r.status().is_success() {
            return Err(InstanceError::Http(format!("{url}: HTTP {}", r.status())));
        }
        r.json()
            .await
            .map_err(|e| InstanceError::Http(format!("{url}: {e}")))
    }

    async fn get_small(&self, url: &str) -> Result<Option<Vec<u8>>> {
        let r = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| InstanceError::Http(e.to_string()))?;
        if r.status().as_u16() == 404 {
            return Ok(None);
        }
        if !r.status().is_success() {
            return Err(InstanceError::Http(format!("{url}: HTTP {}", r.status())));
        }
        let bytes = r
            .bytes()
            .await
            .map_err(|e| InstanceError::Http(e.to_string()))?;
        if bytes.len() > 1 << 20 {
            return Err(InstanceError::Signature("checksum file too large".into()));
        }
        Ok(Some(bytes.to_vec()))
    }

    /// The newest release of `source` this computer can install: the first non-prerelease with
    /// an archive for this platform (releases come newest first).
    pub async fn latest(&self, source: &BuildSource) -> Result<Option<BuildRelease>> {
        Ok(self
            .releases(source)
            .await?
            .into_iter()
            .find(|r| !r.prerelease && r.asset.is_some()))
    }

    /// What `source` publishes, newest first.
    pub async fn releases(&self, source: &BuildSource) -> Result<Vec<BuildRelease>> {
        match source.config.kind {
            BuildSourceKind::Github => {
                let repo = source
                    .config
                    .repo
                    .as_deref()
                    .ok_or_else(|| InstanceError::UnknownSource(source.config.id.clone()))?;
                let list = self
                    .get_json(&format!(
                        "{}/repos/{repo}/releases?per_page=100",
                        source.github_api.trim_end_matches('/')
                    ))
                    .await?;
                Ok(parse_github(source, repo, &list))
            }
            BuildSourceKind::Channel => {
                let url = source
                    .config
                    .channel_url
                    .as_deref()
                    .ok_or_else(|| InstanceError::UnknownSource(source.config.id.clone()))?;
                let v = self.get_json(url).await?;
                parse_channel(source, &v)
            }
        }
    }

    /// Installs `release` (verify the signed checksum, download, unpack). Re-installing a
    /// version replaces it.
    pub async fn install(
        &self,
        source: &BuildSource,
        release: &BuildRelease,
        progress: impl Fn(InstallStage) + Send + Sync,
    ) -> Result<InstalledBuild> {
        let asset = release.asset.clone().ok_or_else(|| {
            InstanceError::NoAssetForPlatform(
                release.version.clone(),
                format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
            )
        })?;
        if release.version.contains(['/', '\\']) || release.version.starts_with('.') {
            return Err(InstanceError::BadArchive("bad version name".into()));
        }
        progress(InstallStage::Verifying);
        let sums = self.get_small(&release.sums_url).await?.ok_or_else(|| {
            InstanceError::Signature(format!(
                "{} is not signed (no SHA256SUMS): not installed.",
                release.version
            ))
        })?;
        let signature = self
            .get_small(&release.signature_url)
            .await?
            .ok_or_else(|| {
                InstanceError::Signature(format!(
                    "{} has no signature file: not installed.",
                    release.version
                ))
            })?;
        let sha256 = signing::signed_digest(
            &source.config.signature_namespace,
            &release.signed_for,
            &asset.name,
            &sums,
            &String::from_utf8_lossy(&signature),
            &source.keys,
        )
        .map_err(InstanceError::Signature)?;
        progress(InstallStage::Downloading);
        let partial = self.root.join(format!(".{}.partial", release.version));
        let _ = tokio::fs::remove_dir_all(&partial).await;
        tokio::fs::create_dir_all(&partial)
            .await
            .map_err(|e| InstanceError::io(&partial, e))?;
        let archive = partial.join(&asset.name);
        let mut req = DownloadRequest::new(&asset.url, &archive)
            .checksum(
                Checksum::sha256(&sha256)
                    .ok_or_else(|| InstanceError::Signature("bad checksum".into()))?,
            )
            .label(format!("Horizon {}", release.version))
            .no_cache();
        if let Some(size) = asset.size {
            req = req.size(size);
        }
        self.downloads.download(req).await?;
        progress(InstallStage::Unpacking);
        let unpacked = partial.join("unpacked");
        let archive2 = archive.clone();
        let unpacked2 = unpacked.clone();
        tokio::task::spawn_blocking(move || unpack(&archive2, &unpacked2))
            .await
            .map_err(|e| InstanceError::Process(e.to_string()))??;
        let top = single_top(&unpacked)?;
        if !top.join(exe("horizon-client")).is_file() {
            return Err(InstanceError::BadArchive("it has no horizon-client".into()));
        }
        #[cfg(unix)]
        for name in ["horizon-client", "horizon-prepare", "horizon-server"] {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(top.join(name), std::fs::Permissions::from_mode(0o755));
        }
        // The bundle's own build record (Horizon commit, companion, target: ADR 0006) keeps its
        // content under another name: `rustic-build.json` is the launcher's install record.
        if top.join(BUILD_FILE).is_file() {
            let _ = std::fs::rename(top.join(BUILD_FILE), top.join(BUNDLE_FILE));
        }
        let target = self.root.join(&release.version);
        let _ = tokio::fs::remove_dir_all(&target).await;
        tokio::fs::rename(&top, &target)
            .await
            .map_err(|e| InstanceError::io(&target, e))?;
        let build = InstalledBuild {
            version: release.version.clone(),
            source: source.config.id.clone(),
            dir: target.clone(),
            installed_at: time::now(),
            archive_sha256: sha256,
        };
        fsutil::write_json(&target.join(BUILD_FILE), &build)?;
        let _ = tokio::fs::remove_dir_all(&partial).await;
        progress(InstallStage::Done {
            version: release.version.clone(),
        });
        Ok(build)
    }

    /// The installed builds, newest install first.
    pub fn installed(&self) -> Vec<InstalledBuild> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut out: Vec<InstalledBuild> = entries
            .flatten()
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|e| {
                let mut b: InstalledBuild = fsutil::read_json(&e.path().join(BUILD_FILE))
                    .ok()
                    .flatten()?;
                b.dir = e.path();
                Some(b)
            })
            .collect();
        out.sort_by_key(|b| std::cmp::Reverse(b.installed_at));
        out
    }

    pub fn get(&self, version: &str) -> Result<InstalledBuild> {
        self.installed()
            .into_iter()
            .find(|b| b.version == version)
            .ok_or_else(|| InstanceError::BuildNotInstalled(version.into()))
    }

    /// Removes an installed build (its prepared resources stay until removed apart).
    pub fn remove(&self, version: &str) -> Result<()> {
        let build = self.get(version)?;
        std::fs::remove_dir_all(&build.dir).map_err(|e| InstanceError::io(&build.dir, e))
    }

    /// Registers a build folder unpacked elsewhere (development builds).
    pub fn register_local(&self, version: &str, dir: &Path) -> Result<InstalledBuild> {
        if !dir.join(exe("horizon-client")).is_file() {
            return Err(InstanceError::BadArchive(format!(
                "{} has no horizon-client",
                dir.display()
            )));
        }
        let target = self.root.join(version);
        if target != dir {
            fsutil::copy_dir(dir, &target).map_err(|e| InstanceError::io(&target, e))?;
        }
        let build = InstalledBuild {
            version: version.into(),
            source: "local".into(),
            dir: target.clone(),
            installed_at: time::now(),
            archive_sha256: String::new(),
        };
        fsutil::write_json(&target.join(BUILD_FILE), &build)?;
        Ok(build)
    }
}

/// Builds in a GitHub releases list, newest first (Horizon's `parse_releases` / `game_releases`).
pub fn parse_github(
    source: &BuildSource,
    repo: &str,
    list: &serde_json::Value,
) -> Vec<BuildRelease> {
    let asset_name = source.asset_name();
    let download =
        |tag: &str, name: &str| format!("https://github.com/{repo}/releases/download/{tag}/{name}");
    let mut out = Vec::new();
    for r in list.as_array().into_iter().flatten() {
        if r["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let tag = r["tag_name"].as_str().unwrap_or_default();
        let title = r["name"].as_str().unwrap_or_default();
        let nightly = tag == "nightly" || tag.starts_with("nightly-");
        let numbered = tag
            .strip_prefix('v')
            .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()));
        let (version, signed_for, sig) = if nightly {
            // Verified against the version its title names; a title without one can't be.
            let Some(version) = signing::nightly_title_version(title) else {
                continue;
            };
            (version.clone(), version, "SHA256SUMS.v2.sig")
        } else if numbered {
            (tag.to_owned(), tag.to_owned(), "SHA256SUMS.sig")
        } else {
            continue; // launcher@..., anything else
        };
        let asset = asset_name.as_ref().and_then(|wanted| {
            r["assets"].as_array().into_iter().flatten().find_map(|a| {
                (a["name"].as_str()? == wanted).then(|| ReleaseAsset {
                    name: wanted.clone(),
                    url: a["browser_download_url"]
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| download(tag, wanted)),
                    size: a["size"].as_u64(),
                })
            })
        });
        let asset_url = |name: &str| {
            r["assets"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|a| a["name"].as_str() == Some(name))
                .and_then(|a| a["browser_download_url"].as_str().map(str::to_owned))
                .unwrap_or_else(|| download(tag, name))
        };
        out.push(BuildRelease {
            source: source.config.id.clone(),
            version,
            title: title.to_owned(),
            published_at: r["published_at"].as_str().unwrap_or_default().to_owned(),
            prerelease: r["prerelease"].as_bool().unwrap_or(false) && tag != "nightly",
            notes: r["body"].as_str().unwrap_or_default().replace("\r\n", "\n"),
            asset,
            sums_url: asset_url("SHA256SUMS"),
            signature_url: asset_url(sig),
            signed_for,
        });
    }
    // The current `nightly` and its dated copy name the same version: keep one.
    let mut seen = std::collections::HashSet::new();
    out.retain(|r| seen.insert(r.version.clone()));
    out.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    out
}

fn parse_channel(source: &BuildSource, v: &serde_json::Value) -> Result<Vec<BuildRelease>> {
    let channel: Channel = serde_json::from_value(v.clone())
        .map_err(|e| InstanceError::Http(format!("channel index: {e}")))?;
    if channel.format != 1 {
        return Err(InstanceError::Http(format!(
            "channel index format {} is newer than this launcher reads (1)",
            channel.format
        )));
    }
    let wanted = source.asset_name();
    let mut out: Vec<BuildRelease> = channel
        .builds
        .into_iter()
        .map(|b| {
            let base = if b.base_url.ends_with('/') {
                b.base_url.clone()
            } else {
                format!("{}/", b.base_url)
            };
            let asset = wanted.as_ref().and_then(|w| {
                b.assets
                    .iter()
                    .find(|a| &a.name == w)
                    .map(|a| ReleaseAsset {
                        name: a.name.clone(),
                        url: format!("{base}{}", a.name),
                        size: a.size,
                    })
            });
            BuildRelease {
                source: source.config.id.clone(),
                title: b.title.unwrap_or_else(|| b.version.clone()),
                signed_for: b.version.clone(),
                version: b.version,
                published_at: b.published_at,
                prerelease: b.prerelease,
                notes: b.notes,
                asset,
                sums_url: format!("{base}SHA256SUMS"),
                signature_url: format!("{base}SHA256SUMS.sig"),
            }
        })
        .collect();
    out.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    Ok(out)
}

/// Unpacks a `.zip` or `.tar.gz`, refusing paths that leave the folder.
fn unpack(archive: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(|e| InstanceError::io(to, e))?;
    let file = std::fs::File::open(archive).map_err(|e| InstanceError::io(archive, e))?;
    let name = archive.to_string_lossy();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        tar.set_preserve_permissions(true);
        for entry in tar
            .entries()
            .map_err(|e| InstanceError::BadArchive(e.to_string()))?
        {
            let mut entry = entry.map_err(|e| InstanceError::BadArchive(e.to_string()))?;
            // `unpack_in` refuses `..` and absolute paths.
            entry
                .unpack_in(to)
                .map_err(|e| InstanceError::BadArchive(e.to_string()))?;
        }
        return Ok(());
    }
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| InstanceError::BadArchive(e.to_string()))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| InstanceError::BadArchive(e.to_string()))?;
        let Some(rel) = entry.enclosed_name() else {
            return Err(InstanceError::BadArchive(format!(
                "{} leaves the archive's folder",
                entry.name()
            )));
        };
        let out = to.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| InstanceError::io(&out, e))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| InstanceError::io(parent, e))?;
        }
        let mut f = std::fs::File::create(&out).map_err(|e| InstanceError::io(&out, e))?;
        std::io::copy(&mut entry, &mut f).map_err(|e| InstanceError::io(&out, e))?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode));
        }
    }
    Ok(())
}

/// Archives hold one top folder (`horizon-linux-x86_64/...`): the build is its contents.
fn single_top(dir: &Path) -> Result<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| InstanceError::io(dir, e))?
        .flatten()
        .collect();
    Ok(match entries.as_slice() {
        [only] if only.path().is_dir() => only.path(),
        _ => dir.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn horizon_source() -> BuildSource {
        BuildSource::new(
            rusticmc_launcher_core::LauncherConfig::default()
                .build_source("horizon")
                .unwrap()
                .clone(),
        )
        .unwrap()
    }

    #[test]
    fn github_list_follows_horizons_tags() {
        let source = horizon_source();
        let asset = source.asset_name().unwrap();
        let list = serde_json::json!([
            {"tag_name": "nightly", "name": "Nightly 2026-10-29 · 4baa571", "published_at": "2026-10-29T10:00:00Z", "prerelease": true, "draft": false,
             "assets": [{"name": asset, "browser_download_url": "https://dl/nightly/a", "size": 10}, {"name": "SHA256SUMS", "browser_download_url": "https://dl/nightly/sums"}, {"name": "SHA256SUMS.v2.sig", "browser_download_url": "https://dl/nightly/sig2"}]},
            {"tag_name": "nightly-2026-10-29", "name": "Nightly 2026-10-29 · 4baa571", "published_at": "2026-10-29T09:00:00Z", "prerelease": true, "assets": []},
            {"tag_name": "nightly-2026-10-28", "name": "Nightly 2026-10-28", "published_at": "2026-10-28T09:00:00Z", "prerelease": true, "assets": []},
            {"tag_name": "launcher@0.3.1", "name": "Launcher", "published_at": "2026-10-30T09:00:00Z", "assets": []},
            {"tag_name": "v2026.10.12", "name": "v2026.10.12", "published_at": "2026-10-12T09:00:00Z", "assets": []},
            {"tag_name": "v2026.10.13", "name": "draft", "draft": true, "assets": []}
        ]);
        let releases = parse_github(&source, "horizon-rust/horizon", &list);
        let versions: Vec<_> = releases.iter().map(|r| r.version.as_str()).collect();
        assert_eq!(versions, ["nightly-2026-10-29-4baa571", "v2026.10.12"]);
        let n = &releases[0];
        assert_eq!(n.signed_for, "nightly-2026-10-29-4baa571");
        assert_eq!(n.signature_url, "https://dl/nightly/sig2");
        assert!(!n.prerelease, "the current Nightly is the update");
        assert_eq!(n.asset.as_ref().unwrap().size, Some(10));
        let v = &releases[1];
        assert_eq!(v.signed_for, "v2026.10.12");
        assert!(v.signature_url.ends_with("/v2026.10.12/SHA256SUMS.sig"));
        assert!(v.asset.is_none());
    }

    #[test]
    fn a_rustic_edition_build_runs_rustic_client() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(exe("horizon-client")), b"").unwrap();
        let mut build = InstalledBuild {
            version: "rustic-edition-test".into(),
            source: "rustic-edition".into(),
            dir: dir.path().to_owned(),
            installed_at: 0,
            archive_sha256: String::new(),
        };
        // Not there (yet): Horizon's game.
        assert_eq!(build.client(), dir.path().join(exe("horizon-client")));
        std::fs::write(dir.path().join(exe(EDITION_CLIENT)), b"").unwrap();
        assert_eq!(build.client(), dir.path().join(exe(EDITION_CLIENT)));
        // A plain Horizon build never runs another program it may hold.
        build.source = "horizon".into();
        assert_eq!(build.client(), dir.path().join(exe("horizon-client")));
    }

    #[test]
    fn platforms() {
        assert_eq!(
            platform_suffix_for("windows", "x86_64"),
            Some("windows-x64.zip")
        );
        assert_eq!(
            platform_suffix_for("linux", "x86_64"),
            Some("linux-x86_64.tar.gz")
        );
        assert_eq!(platform_suffix_for("linux", "aarch64"), None);
    }
}
