//! The edition's source: exactly what the prebuilt client was built from, so a native build
//! compiles the same game plus the mods.
//!
//! The source tree mirrors the folders the edition's workspace refers to by relative path
//! (`client-mod/rustic-companion/Cargo.toml` reaches Horizon at `../../../../Rustic-MC/horizon`):
//!
//! ```text
//! rustic-source.json                                    {"format": 1, "horizon_commit": "<sha>"}
//! Rustic-MC/horizon/                                    Horizon at the pin, Rustic's patches applied
//! Rustic-MC-launcher/launcher/client-mod/rustic-companion/   the companion and the edition crate
//! Rustic-MC-launcher/launcher/packages/cosmetics-format/     a crate the companion uses
//! Rustic-MC-launcher/launcher/apps/launcher/src-tauri/icons/icon.ico   the executable's icon
//! ```
//!
//! **Releases**: the edition bundle carries `rustic-source.json` naming an archive of that tree
//! (`.tar.gz`, built by the edition's CI with [`pack`] from the very checkout it compiled) and its
//! SHA-256; the launcher downloads it once, checks the hash, and unpacks it into
//! `native/source/<sha>/` (links and paths leaving the folder are refused).
//!
//! **Development**: a [`LocalSource`] (the launcher's config `native.local_source`, or the CLI's
//! `--local-horizon`/`--local-launcher`) copies the checkouts' files (no `target/`, `.git`, ...)
//! into `native/source/local/`, writing only what changed.
use crate::error::{IoContext, NativeError, Result};
use crate::paths::NativeRoot;
use crate::stage;
use rusticmc_launcher_core::{CancellationToken, Checksum, DownloadManager, DownloadRequest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The file in an edition bundle (and at the top of the source tree) that names the source.
pub const MANIFEST: &str = "rustic-source.json";
/// Where Horizon is in the tree.
pub const HORIZON: &str = "Rustic-MC/horizon";
/// Where the launcher checkout's parts are in the tree.
pub const LAUNCHER: &str = "Rustic-MC-launcher/launcher";
/// The companion workspace (its Cargo.toml is the build's manifest).
pub const COMPANION: &str = "Rustic-MC-launcher/launcher/client-mod/rustic-companion";
/// The parts of the launcher checkout the edition needs, relative to it.
pub const LAUNCHER_PARTS: &[&str] = &[
    "client-mod/rustic-companion",
    "packages/cosmetics-format",
    "apps/launcher/src-tauri/icons/icon.ico",
];
/// The Horizon commit the Rustic edition is built on (the `horizon` submodule's pin, before
/// Rustic's patches), for a local source that doesn't say.
pub const PINNED_HORIZON: &str = "a84f637dd6235199586b0c8b486c601e51e75b59";
/// Generated at the top of the launcher's part of the tree: its workspace root reduced to what the
/// edition's crates inherit (`packages/cosmetics-format` takes its package fields and lints from
/// it), with only that crate as a member, since the other members aren't in the tree.
pub const LAUNCHER_ROOT: &str = "Cargo.toml";
/// Folders never part of the source.
pub const SKIP_DIRS: &[&str] = &["target", ".git", ".local", "node_modules", "dist"];

/// `rustic-source.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub format: u32,
    /// The upstream Horizon commit (horizon-mixins checks mods against it).
    pub horizon_commit: String,
    /// The archive of the tree (in an edition bundle; absent in the tree itself).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRef {
    /// `https` only.
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
}

/// Local checkouts to build from (development).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSource {
    /// `Rustic-MC/horizon`, with Rustic's patches applied (`scripts/patches.py apply`).
    pub horizon: PathBuf,
    /// The launcher checkout (`Rustic-MC-launcher/launcher`).
    pub launcher: PathBuf,
    #[serde(default)]
    pub horizon_commit: Option<String>,
}

/// Where a build's source comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSpec {
    Archive {
        archive: ArchiveRef,
        horizon_commit: String,
    },
    Local(LocalSource),
}

impl SourceSpec {
    /// The source of the edition bundle in `build_dir` (its `rustic-source.json`), or `local`
    /// when given (development builds have no archive).
    pub fn for_build(build_dir: &Path, local: Option<&LocalSource>) -> Result<Self> {
        if let Some(local) = local {
            return Ok(Self::Local(local.clone()));
        }
        let path = build_dir.join(MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|_| {
            NativeError::Source(format!(
                "this Rustic edition build ({}) doesn't name its source ({MANIFEST}): native mods \
                 need a release build of the edition",
                build_dir.file_name().unwrap_or_default().to_string_lossy()
            ))
        })?;
        let m: SourceManifest = serde_json::from_str(&text)
            .map_err(|e| NativeError::Source(format!("{}: {e}", path.display())))?;
        if m.format != 1 {
            return Err(NativeError::Source(format!(
                "{MANIFEST} format {} (this launcher reads 1): update the launcher",
                m.format
            )));
        }
        let archive = m
            .archive
            .ok_or_else(|| NativeError::Source(format!("{MANIFEST} names no archive")))?;
        if !archive.url.starts_with("https://") {
            return Err(NativeError::Source(format!(
                "the source archive must be downloaded over https: {}",
                archive.url
            )));
        }
        if Checksum::sha256(&archive.sha256).is_none() {
            return Err(NativeError::Source(format!(
                "{MANIFEST}: {:?} is not a SHA-256",
                archive.sha256
            )));
        }
        check_commit(&m.horizon_commit)?;
        Ok(Self::Archive {
            archive,
            horizon_commit: m.horizon_commit,
        })
    }

    pub fn horizon_commit(&self) -> &str {
        match self {
            Self::Archive { horizon_commit, .. } => horizon_commit,
            Self::Local(l) => l.horizon_commit.as_deref().unwrap_or(PINNED_HORIZON),
        }
    }
}

fn check_commit(c: &str) -> Result<()> {
    if c.len() == 40 && c.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(NativeError::Source(format!("{c:?} is not a commit")))
    }
}

/// A source ready to build from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prepared {
    pub dir: PathBuf,
    /// Names the content: the archive's SHA-256, or a digest of the local files.
    pub id: String,
    pub horizon_commit: String,
}

impl Prepared {
    pub fn horizon(&self) -> PathBuf {
        self.dir.join(HORIZON)
    }
}

/// Prepares `spec` under `root` (see the module).
pub async fn prepare(
    root: &NativeRoot,
    spec: &SourceSpec,
    downloads: &DownloadManager,
    cancel: &CancellationToken,
) -> Result<Prepared> {
    match spec {
        SourceSpec::Archive {
            archive,
            horizon_commit,
        } => {
            let sha = archive.sha256.to_ascii_lowercase();
            let dir = root.sources().join(&sha[..16]);
            let marker = dir.join(".rustic-source-id");
            if std::fs::read_to_string(&marker).ok().as_deref() == Some(sha.as_str()) {
                return Ok(Prepared {
                    dir,
                    id: sha,
                    horizon_commit: horizon_commit.clone(),
                });
            }
            std::fs::create_dir_all(root.downloads()).at(&root.downloads())?;
            let file = root
                .downloads()
                .join(format!("rustic-source-{}.tar.gz", &sha[..16]));
            let mut req = DownloadRequest::new(archive.url.clone(), &file)
                .checksum(Checksum::sha256(&sha).expect("checked"))
                .label("Rustic edition source")
                .no_cache();
            if let Some(size) = archive.size {
                req = req.size(size);
            }
            let handle = downloads.enqueue(req);
            let token = handle.token();
            tokio::select! {
                r = handle.wait() => { r.map_err(|e| NativeError::Download(e.to_string()))?; }
                () = cancel.cancelled() => { token.cancel(); return Err(NativeError::Cancelled); }
            }
            let partial = root.sources().join(format!(".{}.partial", &sha[..16]));
            let (f, p) = (file.clone(), partial.clone());
            tokio::task::spawn_blocking(move || unpack(&f, &p, MAX_UNPACKED))
                .await
                .map_err(|e| NativeError::Other(e.to_string()))??;
            check_layout(&partial)?;
            std::fs::write(partial.join(".rustic-source-id"), &sha).at(&partial)?;
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::rename(&partial, &dir).at(&dir)?;
            let _ = std::fs::remove_file(&file);
            prune(root, &dir);
            Ok(Prepared {
                dir,
                id: sha,
                horizon_commit: horizon_commit.clone(),
            })
        }
        SourceSpec::Local(local) => {
            let dir = root.sources().join("local");
            let (l, d) = (local.clone(), dir.clone());
            let id = tokio::task::spawn_blocking(move || copy_local(&l, &d))
                .await
                .map_err(|e| NativeError::Other(e.to_string()))??;
            check_layout(&dir)?;
            Ok(Prepared {
                dir,
                id,
                horizon_commit: spec.horizon_commit().to_owned(),
            })
        }
    }
}

/// At most this much is unpacked from a source archive (Horizon's tree is about 130 MB).
pub const MAX_UNPACKED: u64 = 2 * 1024 * 1024 * 1024;

/// Removes the source folders other than `keep` (one edition's source at a time).
fn prune(root: &NativeRoot, keep: &Path) {
    let Ok(dir) = std::fs::read_dir(root.sources()) else {
        return;
    };
    for e in dir.flatten() {
        let p = e.path();
        if p != keep && p.file_name().is_some_and(|n| n != "local") {
            let _ = std::fs::remove_dir_all(p);
        }
    }
}

/// The tree has what the build needs.
pub fn check_layout(dir: &Path) -> Result<()> {
    for need in [
        format!("{HORIZON}/Cargo.toml"),
        format!("{HORIZON}/Cargo.lock"),
        format!("{COMPANION}/Cargo.toml"),
        format!("{COMPANION}/Cargo.lock"),
        format!("{COMPANION}/edition/Cargo.toml"),
        format!("{LAUNCHER}/{LAUNCHER_ROOT}"),
    ] {
        if !dir.join(&need).is_file() {
            return Err(NativeError::Source(format!("{need} is missing")));
        }
    }
    // The companion reaches Horizon where the tree has it.
    let manifest =
        std::fs::read_to_string(dir.join(COMPANION).join("Cargo.toml")).at(&dir.join(COMPANION))?;
    let value: toml::Table = manifest
        .parse()
        .map_err(|e| NativeError::Source(format!("{COMPANION}/Cargo.toml: {e}")))?;
    if let Some(path) = value
        .get("patch")
        .and_then(|p| p.get("crates-io"))
        .and_then(|p| p.get("wgpu-hal"))
        .and_then(|p| p.get("path"))
        .and_then(|p| p.as_str())
    {
        let resolved = normalize(&dir.join(COMPANION).join(path));
        let expected = normalize(&dir.join(HORIZON).join("third_party").join("wgpu-hal"));
        if resolved != expected {
            return Err(NativeError::Source(format!(
                "the companion reaches Horizon at {path}, not where the source tree has it ({HORIZON})"
            )));
        }
    }
    Ok(())
}

/// `path` with `.` and `..` resolved without touching the disk.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

/// Unpacks the `.tar.gz` at `file` into the fresh folder `dir`: files and folders only (links and
/// devices are left out), every path inside `dir`, at most `limit` bytes.
pub fn unpack(file: &Path, dir: &Path, limit: u64) -> Result<()> {
    use std::io::Read;
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).at(dir)?;
    let reader = flate2::read::GzDecoder::new(std::fs::File::open(file).at(file)?).take(limit);
    let mut tar = tar::Archive::new(reader);
    tar.set_preserve_permissions(false);
    tar.set_preserve_mtime(false);
    for entry in tar.entries().at(file)? {
        let mut entry = entry.map_err(|e| NativeError::Source(format!("the archive: {e}")))?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| NativeError::Source(e.to_string()))?
            .into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(NativeError::Source(format!(
                "the archive has a path outside its folder: {}",
                path.display()
            )));
        }
        let ok = entry
            .unpack_in(dir)
            .map_err(|e| NativeError::Source(format!("unpacking {}: {e}", path.display())))?;
        if !ok {
            return Err(NativeError::Source(format!(
                "the archive has a path outside its folder: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

/// The launcher workspace's root manifest reduced for the source tree ([`LAUNCHER_ROOT`]).
pub fn launcher_root_manifest(real: &str) -> Result<String> {
    let value: toml::Table = real
        .parse()
        .map_err(|e| NativeError::Source(format!("the launcher's Cargo.toml: {e}")))?;
    let ws = value
        .get("workspace")
        .and_then(|w| w.as_table())
        .ok_or_else(|| {
            NativeError::Source("the launcher's Cargo.toml has no [workspace]".into())
        })?;
    let mut out = toml::Table::new();
    if let Some(r) = ws.get("resolver") {
        out.insert("resolver".into(), r.clone());
    }
    out.insert(
        "members".into(),
        toml::Value::Array(vec!["packages/cosmetics-format".into()]),
    );
    out.insert(
        "exclude".into(),
        toml::Value::Array(vec!["client-mod/rustic-companion".into()]),
    );
    for key in ["package", "lints", "dependencies"] {
        if let Some(v) = ws.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    let mut root = toml::Table::new();
    root.insert("workspace".into(), toml::Value::Table(out));
    Ok(format!(
        "# Generated by rusticmc-native from the launcher's workspace root: only what the edition's\n# crates inherit.\n{}",
        toml::to_string(&root).map_err(|e| NativeError::Source(e.to_string()))?
    ))
}

/// The files of the source tree from local checkouts: `(path in the tree, file on disk)`.
pub fn local_files(local: &LocalSource) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    for (from, to) in std::iter::once((local.horizon.clone(), HORIZON.to_owned())).chain(
        LAUNCHER_PARTS
            .iter()
            .map(|p| (local.launcher.join(p), format!("{LAUNCHER}/{p}"))),
    ) {
        if from.is_file() {
            files.push((to, from));
            continue;
        }
        if !from.is_dir() {
            return Err(NativeError::Source(format!(
                "{} is missing",
                from.display()
            )));
        }
        for (rel, path) in stage::walk(&from, SKIP_DIRS)? {
            files.push((format!("{to}/{rel}"), path));
        }
    }
    files.sort();
    Ok(files)
}

/// Copies the local checkouts into `dir` (changed files only; files gone are removed) and returns
/// the tree's digest.
fn copy_local(local: &LocalSource, dir: &Path) -> Result<String> {
    let files = local_files(local)?;
    let mut digest = Sha256::new();
    let commit = local.horizon_commit.as_deref().unwrap_or(PINNED_HORIZON);
    check_commit(commit)?;
    digest.update(commit.as_bytes());
    let mut wanted = std::collections::BTreeSet::new();
    for (rel, path) in &files {
        let to = dir.join(rel);
        // A checkout changes a few files at a time: a copy no older than its file and of the same
        // size is current, without reading either (the copy gets the time it was written).
        let meta = std::fs::metadata(path).at(path)?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        digest.update(rel.as_bytes());
        digest.update(meta.len().to_le_bytes());
        digest.update(modified.to_le_bytes());
        let current = std::fs::metadata(&to).is_ok_and(|copy| {
            copy.len() == meta.len()
                && copy
                    .modified()
                    .ok()
                    .zip(meta.modified().ok())
                    .is_some_and(|(c, m)| c >= m)
        });
        if !current {
            let bytes = std::fs::read(path).at(path)?;
            stage::write_if_changed(&to, &bytes)?;
        }
        wanted.insert(rel.clone());
    }
    let manifest = serde_json::to_vec_pretty(&SourceManifest {
        format: 1,
        horizon_commit: commit.to_owned(),
        archive: None,
    })
    .expect("json");
    stage::write_if_changed(&dir.join(MANIFEST), &manifest)?;
    wanted.insert(MANIFEST.to_owned());
    let root_manifest = launcher_root_manifest(
        &std::fs::read_to_string(local.launcher.join("Cargo.toml"))
            .at(&local.launcher.join("Cargo.toml"))?,
    )?;
    digest.update(root_manifest.as_bytes());
    let rel = format!("{LAUNCHER}/{LAUNCHER_ROOT}");
    stage::write_if_changed(&dir.join(&rel), root_manifest.as_bytes())?;
    wanted.insert(rel);
    for (rel, path) in stage::walk(dir, &[])? {
        if !wanted.contains(&rel) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(hex::encode(digest.finalize()))
}

/// Packs the source tree of `local` into the `.tar.gz` at `out` (the edition's CI, with the same
/// checkout it compiled) and returns the archive's SHA-256: the value for `rustic-source.json`.
pub fn pack(local: &LocalSource, out: &Path) -> Result<String> {
    let commit = local.horizon_commit.as_deref().unwrap_or(PINNED_HORIZON);
    check_commit(commit)?;
    let files = local_files(local)?;
    let file = std::fs::File::create(out).at(out)?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    tar.mode(tar::HeaderMode::Deterministic);
    let manifest = serde_json::to_vec_pretty(&SourceManifest {
        format: 1,
        horizon_commit: commit.to_owned(),
        archive: None,
    })
    .expect("json");
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, MANIFEST, manifest.as_slice())
        .at(out)?;
    let root_manifest = launcher_root_manifest(
        &std::fs::read_to_string(local.launcher.join("Cargo.toml"))
            .at(&local.launcher.join("Cargo.toml"))?,
    )?;
    let mut header = tar::Header::new_gnu();
    header.set_size(root_manifest.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(
        &mut header,
        format!("{LAUNCHER}/{LAUNCHER_ROOT}"),
        root_manifest.as_bytes(),
    )
    .at(out)?;
    for (rel, path) in files {
        tar.append_path_with_name(&path, &rel).at(&path)?;
    }
    let gz = tar.into_inner().at(out)?;
    gz.finish().at(out)?;
    rusticmc_launcher_core::download::hash_file(out, rusticmc_launcher_core::HashAlgo::Sha256)
        .at(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal tree shaped like the edition's.
    pub(crate) fn fake_checkouts(base: &Path) -> LocalSource {
        let horizon = base.join("Rustic-MC/horizon");
        let launcher = base.join("Rustic-MC-launcher/launcher");
        let w = |p: PathBuf, s: &str| {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, s).unwrap();
        };
        w(horizon.join("Cargo.toml"), "[workspace]\n");
        w(horizon.join("Cargo.lock"), "");
        w(horizon.join("crates/a/src/lib.rs"), "pub fn a() {}\n");
        w(horizon.join("target/fast/huge.bin"), "skip me");
        w(horizon.join(".git"), "gitdir: ../.git/modules/horizon");
        let c = launcher.join("client-mod/rustic-companion");
        w(
            c.join("Cargo.toml"),
            "[workspace]\n[patch.crates-io]\nwgpu-hal = { path = \"../../../../Rustic-MC/horizon/third_party/wgpu-hal\" }\n",
        );
        w(c.join("Cargo.lock"), "");
        w(c.join("edition/Cargo.toml"), "[package]\n");
        w(c.join("target/x"), "skip");
        w(launcher.join("packages/cosmetics-format/Cargo.toml"), "");
        w(
            launcher.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/a\", \"services/api\"]\n\
             [workspace.package]\nedition = \"2024\"\n[workspace.lints.rust]\n\
             unsafe_code = \"deny\"\n[profile.release]\nlto = true\n",
        );
        w(
            launcher.join("apps/launcher/src-tauri/icons/icon.ico"),
            "ico",
        );
        w(launcher.join("apps/launcher/src/other.ts"), "not needed");
        LocalSource {
            horizon,
            launcher,
            horizon_commit: None,
        }
    }

    #[test]
    fn local_files_mirror_the_layout_without_build_outputs() {
        let t = tempfile::tempdir().unwrap();
        let local = fake_checkouts(t.path());
        let files: Vec<String> = local_files(&local)
            .unwrap()
            .into_iter()
            .map(|f| f.0)
            .collect();
        assert!(files.contains(&"Rustic-MC/horizon/crates/a/src/lib.rs".to_owned()));
        assert!(files.contains(&format!(
            "{LAUNCHER}/apps/launcher/src-tauri/icons/icon.ico"
        )));
        assert!(!files.iter().any(|f| f.contains("target/")), "{files:?}");
        assert!(!files.iter().any(|f| f.ends_with(".git")));
        assert!(!files.iter().any(|f| f.ends_with("other.ts")));
    }

    #[tokio::test]
    async fn a_local_source_is_copied_and_follows_changes() {
        let t = tempfile::tempdir().unwrap();
        let local = fake_checkouts(&t.path().join("co"));
        let root = NativeRoot::at(t.path().join("native"));
        let dl =
            rusticmc_launcher_core::DownloadManager::new(t.path().join("cache"), 1, 0).unwrap();
        let spec = SourceSpec::Local(local.clone());
        let a = prepare(&root, &spec, &dl, &CancellationToken::new())
            .await
            .unwrap();
        assert!(a.horizon().join("crates/a/src/lib.rs").is_file());
        assert_eq!(a.horizon_commit, PINNED_HORIZON);
        let again = prepare(&root, &spec, &dl, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(a.id, again.id);
        std::fs::write(local.horizon.join("crates/a/src/lib.rs"), "pub fn b() {}\n").unwrap();
        std::fs::remove_file(local.launcher.join("packages/cosmetics-format/Cargo.toml")).unwrap();
        let b = prepare(&root, &spec, &dl, &CancellationToken::new())
            .await
            .unwrap();
        assert_ne!(a.id, b.id);
        assert!(
            !b.dir
                .join(LAUNCHER)
                .join("packages/cosmetics-format/Cargo.toml")
                .exists()
        );
    }

    #[test]
    fn pack_and_unpack_round_trip() {
        let t = tempfile::tempdir().unwrap();
        let local = fake_checkouts(&t.path().join("co"));
        let out = t.path().join("src.tar.gz");
        let sha = pack(&local, &out).unwrap();
        assert_eq!(sha.len(), 64);
        // Deterministic: the same tree packs to the same bytes.
        let out2 = t.path().join("src2.tar.gz");
        assert_eq!(pack(&local, &out2).unwrap(), sha);
        let dir = t.path().join("un");
        unpack(&out, &dir, MAX_UNPACKED).unwrap();
        check_layout(&dir).unwrap();
        let m: SourceManifest =
            serde_json::from_slice(&std::fs::read(dir.join(MANIFEST)).unwrap()).unwrap();
        assert_eq!(m.horizon_commit, PINNED_HORIZON);
        // Over the limit it stops.
        assert!(
            unpack(&out, &t.path().join("small"), 10).is_err() || {
                // A tiny limit may still end cleanly on an entry boundary; then files are missing.
                check_layout(&t.path().join("small")).is_err()
            }
        );
    }

    #[test]
    fn the_launcher_root_keeps_only_what_is_inherited() {
        let real = "[workspace]\nresolver = \"2\"\nmembers = [\"crates/a\"]\n\
                    [workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n\
                    [workspace.lints.clippy]\nall = \"warn\"\n[profile.release]\nlto = true\n";
        let out = launcher_root_manifest(real).unwrap();
        let v: toml::Table = out.parse().unwrap();
        let ws = v["workspace"].as_table().unwrap();
        assert_eq!(ws["members"].as_array().unwrap().len(), 1);
        assert_eq!(ws["package"]["edition"].as_str(), Some("2024"));
        assert!(ws.get("lints").is_some());
        assert!(v.get("profile").is_none());
        assert!(launcher_root_manifest("[package]\n").is_err());
    }

    #[test]
    fn a_layout_whose_companion_points_elsewhere_is_refused() {
        let t = tempfile::tempdir().unwrap();
        let local = fake_checkouts(&t.path().join("co"));
        let out = t.path().join("src.tar.gz");
        pack(&local, &out).unwrap();
        let dir = t.path().join("un");
        unpack(&out, &dir, MAX_UNPACKED).unwrap();
        std::fs::write(
            dir.join(COMPANION).join("Cargo.toml"),
            "[patch.crates-io]\nwgpu-hal = { path = \"../horizon/third_party/wgpu-hal\" }\n",
        )
        .unwrap();
        assert!(check_layout(&dir).is_err());
    }

    #[test]
    fn a_bundle_names_its_source_or_says_it_cannot() {
        let t = tempfile::tempdir().unwrap();
        let err = SourceSpec::for_build(t.path(), None).unwrap_err();
        assert!(err.to_string().contains("rustic-source.json"), "{err}");
        let m = |url: &str, sha: &str| {
            format!(
                "{{\"format\":1,\"horizon_commit\":\"{PINNED_HORIZON}\",\"archive\":{{\"url\":\"{url}\",\"sha256\":\"{sha}\"}}}}"
            )
        };
        std::fs::write(t.path().join(MANIFEST), m("http://x/y", &"a".repeat(64))).unwrap();
        assert!(SourceSpec::for_build(t.path(), None).is_err());
        std::fs::write(t.path().join(MANIFEST), m("https://x/y", "zz")).unwrap();
        assert!(SourceSpec::for_build(t.path(), None).is_err());
        std::fs::write(t.path().join(MANIFEST), m("https://x/y", &"a".repeat(64))).unwrap();
        let spec = SourceSpec::for_build(t.path(), None).unwrap();
        assert_eq!(spec.horizon_commit(), PINNED_HORIZON);
        // A local source wins (development).
        let local = fake_checkouts(&t.path().join("co"));
        assert!(matches!(
            SourceSpec::for_build(t.path(), Some(&local)).unwrap(),
            SourceSpec::Local(_)
        ));
    }

    #[test]
    fn archives_with_paths_outside_are_refused() {
        let t = tempfile::tempdir().unwrap();
        let out = t.path().join("evil.tar.gz");
        {
            let gz = flate2::write::GzEncoder::new(
                std::fs::File::create(&out).unwrap(),
                flate2::Compression::default(),
            );
            let mut tar = tar::Builder::new(gz);
            let mut h = tar::Header::new_gnu();
            let data = b"x";
            h.set_size(1);
            h.set_mode(0o644);
            // Written raw: the builder itself refuses `..`.
            let name = b"../escape.txt";
            h.as_old_mut().name[..name.len()].copy_from_slice(name);
            h.set_cksum();
            tar.append(&h, &data[..]).unwrap();
            tar.into_inner().unwrap().finish().unwrap();
        }
        let dir = t.path().join("in");
        assert!(unpack(&out, &dir, MAX_UNPACKED).is_err());
        assert!(!t.path().join("escape.txt").exists());
    }
}
