//! The installer: the version manifest, version files (following `inheritsFrom`), then the client
//! jar, this system's libraries, the logging setup, the asset index and its objects, and the Java
//! runtime, all from the official hosts into one game folder in the official launcher's layout
//! (shared with the Rustic edition's game files, ADR 0147). Every file is checked against the sha1
//! its manifest lists; finished files are kept, so an interrupted install resumes.

use crate::assets::AssetIndex;
use crate::download::{FileDownload, fetch_all, get_text, is_sha1};
use crate::libraries::{self, LibraryPlan};
use crate::manifest::VersionManifest;
use crate::platform::Platform;
use crate::rules::Features;
use crate::runtime::{self, RuntimeFiles, RuntimeIndex};
use crate::version::{VersionJson, safe_id};
use crate::{Hosts, JavaError, Result};
use rusticmc_launcher_core::download::hash_file;
use rusticmc_launcher_core::{CancellationToken, DownloadManager, HashAlgo, reqwest};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// What an install is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Metadata,
    Loader,
    Libraries,
    Assets,
    Java,
}

/// Progress of a step: bytes (or files) done of a total.
pub type StepProgress<'a> = &'a (dyn Fn(Step, u64, u64) + Send + Sync);

#[derive(Clone)]
pub struct Installer {
    /// The game folder (`versions/`, `libraries/`, `assets/`, `runtimes/`).
    pub root: PathBuf,
    pub http: reqwest::Client,
    /// A download manager of its own (its files are not tasks of their own in the UI).
    pub downloads: DownloadManager,
    pub hosts: Hosts,
    /// Extra allowed URL prefixes (tests' mock server).
    pub extra_hosts: Vec<String>,
    pub platform: Platform,
}

/// A version ready to start: its merged file and the paths the launch needs.
#[derive(Clone, Debug)]
pub struct Installed {
    pub version: VersionJson,
    pub libraries: LibraryPlan,
    /// `versions/<id>/<id>.jar`, relative to the root (none for versions without a client). A
    /// loader's version gets its own copy of the game's jar under its own name, as the official
    /// launcher does: Forge and NeoForge exclude `${version_name}.jar` from their module layer.
    pub client_jar: Option<PathBuf>,
    /// The game's jar the loader's copy is made from (relative to the root).
    pub client_source: Option<PathBuf>,
    pub asset_index: Option<String>,
    /// Relative to the root.
    pub logging_config: Option<PathBuf>,
}

impl Installer {
    pub fn version_file(&self, id: &str) -> PathBuf {
        self.root
            .join("versions")
            .join(id)
            .join(format!("{id}.json"))
    }

    fn manifest_cache(&self) -> PathBuf {
        self.root.join("versions").join("version_manifest_v2.json")
    }

    /// Mojang's version manifest; the last copy when the network fails.
    pub async fn manifest(&self) -> Result<VersionManifest> {
        match get_text(&self.http, &self.hosts.version_manifest, &self.extra_hosts).await {
            Ok(text) => {
                let m = VersionManifest::parse(&text)?;
                let cache = self.manifest_cache();
                if let Some(parent) = cache.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = rusticmc_launcher_core::fsutil::write_atomic(&cache, text.as_bytes());
                Ok(m)
            }
            Err(e) => match std::fs::read_to_string(self.manifest_cache()) {
                Ok(text) => {
                    tracing::info!(error = %e, "version manifest: using the last copy");
                    VersionManifest::parse(&text)
                }
                Err(_) => Err(e),
            },
        }
    }

    /// Reads an installed version file.
    pub fn read_version(&self, id: &str) -> Result<VersionJson> {
        if !safe_id(id) {
            return Err(JavaError::Invalid(format!("bad version id {id:?}")));
        }
        let path = self.version_file(id);
        let text = std::fs::read_to_string(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => JavaError::UnknownVersion(id.to_owned()),
            _ => JavaError::io(&path, e),
        })?;
        VersionJson::parse(&text)
    }

    /// Writes a version file (a loader's profile).
    pub fn write_version(&self, id: &str, text: &str) -> Result<()> {
        if !safe_id(id) {
            return Err(JavaError::Invalid(format!("bad version id {id:?}")));
        }
        VersionJson::parse(text)?;
        let path = self.version_file(id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| JavaError::io(parent, e))?;
        }
        rusticmc_launcher_core::fsutil::write_atomic(&path, text.as_bytes())?;
        Ok(())
    }

    /// Makes sure Mojang's version file of `id` is there and is the one the manifest lists.
    pub async fn ensure_vanilla(
        &self,
        id: &str,
        manifest: Option<&VersionManifest>,
        cancel: &CancellationToken,
    ) -> Result<()> {
        if !safe_id(id) {
            return Err(JavaError::Invalid(format!("bad version id {id:?}")));
        }
        let path = self.version_file(id);
        let owned;
        let manifest = match manifest {
            Some(m) => Some(m),
            None if path.is_file() => {
                // Offline-friendly: an existing file is kept when the manifest can't be read.
                match self.manifest().await {
                    Ok(m) => {
                        owned = m;
                        Some(&owned)
                    }
                    Err(_) => return Ok(()),
                }
            }
            None => {
                owned = self.manifest().await?;
                Some(&owned)
            }
        };
        let entry = manifest
            .and_then(|m| m.find(id))
            .ok_or_else(|| JavaError::UnknownVersion(id.to_owned()))?;
        if let Some(sha1) = entry.sha1.as_deref().filter(|s| is_sha1(s))
            && path.is_file()
            && hash_file(&path, HashAlgo::Sha1).is_ok_and(|h| h == sha1)
        {
            return Ok(());
        }
        if entry.sha1.is_none() && path.is_file() {
            return Ok(());
        }
        let file = FileDownload {
            url: entry.url.clone(),
            sha1: entry.sha1.clone(),
            size: None,
            path: path
                .strip_prefix(&self.root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.clone()),
        };
        // Republished version files change in place: fetch afresh.
        let _ = std::fs::remove_file(&path);
        fetch_all(
            &self.downloads,
            &self.root,
            vec![file],
            &self.extra_hosts,
            cancel,
            &|_, _| {},
        )
        .await
    }

    /// The merged version of `id`, fetching Mojang's files it inherits from when missing.
    pub async fn resolve(&self, id: &str, cancel: &CancellationToken) -> Result<VersionJson> {
        let mut files: HashMap<String, VersionJson> = HashMap::new();
        let mut next = Some(id.to_owned());
        let mut manifest: Option<VersionManifest> = None;
        while let Some(cur) = next.take() {
            if files.contains_key(&cur) || files.len() >= 8 {
                break;
            }
            let v = match self.read_version(&cur) {
                Ok(v) => v,
                Err(JavaError::UnknownVersion(_)) => {
                    if manifest.is_none() {
                        manifest = Some(self.manifest().await?);
                    }
                    self.ensure_vanilla(&cur, manifest.as_ref(), cancel).await?;
                    self.read_version(&cur)?
                }
                Err(e) => return Err(e),
            };
            next = v.inherits_from.clone();
            files.insert(cur, v);
        }
        crate::merge::resolve(id, |v| {
            files
                .remove(v)
                .ok_or_else(|| JavaError::UnknownVersion(v.to_owned()))
        })
    }

    /// The files `version` (merged) needs, besides asset objects and Java.
    pub fn plan(
        &self,
        version: &VersionJson,
        features: &Features,
    ) -> Result<(Installed, Vec<FileDownload>)> {
        let libraries = libraries::plan(version, &self.platform, features, &self.hosts.libraries)?;
        let mut files = libraries.downloads.clone();
        let jar_id = version.jar.clone().unwrap_or_else(|| version.id.clone());
        if !safe_id(&jar_id) {
            return Err(JavaError::bad(format!("bad jar id {jar_id:?}")));
        }
        let jar_rel = |id: &str| Path::new("versions").join(id).join(format!("{id}.jar"));
        let own = jar_rel(&version.id);
        let mut client_jar = None;
        let mut client_source = None;
        if let Some(client) = version.downloads.as_ref().and_then(|d| d.client.as_ref()) {
            let game = jar_rel(&jar_id);
            files.push(FileDownload {
                url: client.url.clone(),
                sha1: client.sha1.clone(),
                size: client.size,
                path: game.clone(),
            });
            if game != own {
                client_source = Some(game);
            }
            client_jar = Some(own);
        } else if self.root.join(&own).is_file() {
            client_jar = Some(own);
        } else if self.root.join(jar_rel(&jar_id)).is_file() {
            client_source = Some(jar_rel(&jar_id));
            client_jar = Some(own);
        }
        let mut logging_config = None;
        if let Some(log) = version.logging.as_ref().and_then(|l| l.client.as_ref()) {
            let name = crate::maven::safe_relative(&log.file.id)
                .ok_or_else(|| JavaError::bad("bad logging file id"))?;
            let rel = Path::new("assets/log_configs").join(name);
            files.push(FileDownload {
                url: log.file.url.clone(),
                sha1: Some(log.file.sha1.clone()),
                size: log.file.size,
                path: rel.clone(),
            });
            logging_config = Some(rel);
        }
        let mut asset_index = None;
        if let Some(index) = &version.asset_index {
            if !safe_id(&index.id) {
                return Err(JavaError::bad("bad asset index id"));
            }
            files.push(FileDownload {
                url: index.url.clone(),
                sha1: index.sha1.clone(),
                size: index.size,
                path: Path::new("assets/indexes").join(format!("{}.json", index.id)),
            });
            asset_index = Some(index.id.clone());
        }
        Ok((
            Installed {
                version: version.clone(),
                libraries,
                client_jar,
                client_source,
                asset_index,
                logging_config,
            },
            files,
        ))
    }

    /// Installs everything `id` needs but Java: its files, then its asset objects.
    pub async fn install(
        &self,
        id: &str,
        features: &Features,
        cancel: &CancellationToken,
        progress: StepProgress<'_>,
    ) -> Result<Installed> {
        progress(Step::Metadata, 0, 1);
        let version = self.resolve(id, cancel).await?;
        progress(Step::Metadata, 1, 1);
        let (installed, files) = self.plan(&version, features)?;
        fetch_all(
            &self.downloads,
            &self.root,
            files,
            &self.extra_hosts,
            cancel,
            &|d, t| progress(Step::Libraries, d, t),
        )
        .await?;
        if let (Some(src), Some(dst)) = (&installed.client_source, &installed.client_jar) {
            place_copy(&self.root.join(src), &self.root.join(dst))?;
        }
        if let Some(index_id) = &installed.asset_index {
            let path = self
                .root
                .join("assets/indexes")
                .join(format!("{index_id}.json"));
            let text = std::fs::read_to_string(&path).map_err(|e| JavaError::io(&path, e))?;
            let index = AssetIndex::parse(&text)?;
            fetch_all(
                &self.downloads,
                &self.root,
                index.downloads(&self.hosts.resources)?,
                &self.extra_hosts,
                cancel,
                &|d, t| progress(Step::Assets, d, t),
            )
            .await?;
            // Old "virtual" indexes read their copies under the game folder's assets.
            if index.is_virtual {
                let dir = self.root.join("assets/virtual").join(index_id);
                let (root, idx) = (self.root.clone(), index.clone());
                tokio::task::spawn_blocking(move || idx.place_copies(&root, &dir))
                    .await
                    .map_err(|e| JavaError::Process(e.to_string()))??;
            }
        }
        Ok(installed)
    }

    /// Mojang's Java runtime `component` for this system, installed if needed; its executable.
    pub async fn ensure_runtime(
        &self,
        component: &str,
        cancel: &CancellationToken,
        progress: StepProgress<'_>,
    ) -> Result<PathBuf> {
        if !safe_id(component) {
            return Err(JavaError::bad("bad runtime component"));
        }
        let dir = runtime::component_dir(&self.root, component);
        let cache = self.root.join("runtimes").join("all.json");
        let index =
            match get_text(&self.http, &self.hosts.runtime_manifest, &self.extra_hosts).await {
                Ok(text) => {
                    let idx = RuntimeIndex::parse(&text)?;
                    let _ = std::fs::create_dir_all(self.root.join("runtimes"));
                    let _ = rusticmc_launcher_core::fsutil::write_atomic(&cache, text.as_bytes());
                    idx
                }
                Err(e) => {
                    // Offline: an installed runtime still starts.
                    let java = self.platform.runtime_java(&dir);
                    if java.is_file() && dir.join(".rustic-runtime").is_file() {
                        return Ok(java);
                    }
                    match std::fs::read_to_string(&cache) {
                        Ok(t) => RuntimeIndex::parse(&t)?,
                        Err(_) => return Err(e),
                    }
                }
            };
        let entry = index.entry(&self.platform, component)?;
        if runtime::is_installed(&dir, &entry.manifest.sha1, &self.platform) {
            return Ok(self.platform.runtime_java(&dir));
        }
        progress(Step::Java, 0, 1);
        if !crate::url_allowed(&entry.manifest.url, &self.extra_hosts) {
            return Err(JavaError::HostNotAllowed(entry.manifest.url.clone()));
        }
        let text = get_text(&self.http, &entry.manifest.url, &self.extra_hosts).await?;
        if is_sha1(&entry.manifest.sha1) {
            let got = rusticmc_launcher_core::download::hash_bytes(text.as_bytes(), HashAlgo::Sha1);
            if got != entry.manifest.sha1 {
                return Err(JavaError::bad(
                    "the runtime file list does not match its sha1",
                ));
            }
        }
        let files = RuntimeFiles::parse(&text)?;
        runtime::install_files(
            &self.downloads,
            &dir,
            &entry.manifest.sha1,
            &files,
            &self.platform,
            &self.extra_hosts,
            cancel,
            &|d, t| progress(Step::Java, d, t),
        )
        .await
    }
}

/// Makes `to` a copy of `from` (a hard link when possible) unless it already has its size.
fn place_copy(from: &Path, to: &Path) -> Result<()> {
    let size = std::fs::metadata(from)
        .map_err(|e| JavaError::io(from, e))?
        .len();
    if std::fs::metadata(to).is_ok_and(|m| m.len() == size) {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| JavaError::io(parent, e))?;
    }
    let _ = std::fs::remove_file(to);
    if std::fs::hard_link(from, to).is_err() {
        std::fs::copy(from, to).map_err(|e| JavaError::io(to, e))?;
    }
    Ok(())
}
