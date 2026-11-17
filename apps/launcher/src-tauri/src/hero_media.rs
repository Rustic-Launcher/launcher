//! The Launchpad's hero video (ADR 0179): the rendition the webview picked from
//! `GET /v1/media/hero` is downloaded once into `<cache>/hero-media/<sha256>.<ext>` (resumable,
//! checked against its SHA-256 and size), then played from there through the asset protocol,
//! whose scope is this one folder. Only the current file is kept: the others, and their partial
//! downloads, are removed once it is complete. Only files of the official CDN's hero folder are
//! fetched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusticmc_launcher_core::IpcError;
use rusticmc_launcher_core::download::{
    Checksum, DownloadManager, DownloadRequest, HashAlgo, hash_file,
};
use serde::Deserialize;
use tauri::Manager;

use crate::state::{Res, State};

/// Where the sets live (an immutable, content-addressed folder per set).
pub const CDN_PREFIX: &str = "https://cdn.rustic-launcher.com/media/hero/";
/// The cache folder's name under the launcher's cache.
pub const DIR: &str = "hero-media";
/// One rendition at most (the API refuses bigger ones too).
const MAX_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct HeroMediaArgs {
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    /// `false` (offline, data saver): only a file already cached is answered.
    #[serde(default = "yes")]
    pub download: bool,
}

fn yes() -> bool {
    true
}

/// A checked request: the file name it is kept under.
#[derive(Debug, PartialEq, Eq)]
pub struct Wanted {
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    /// `<sha256>.mp4` or `<sha256>.webm`.
    pub file: String,
}

/// Accepts only a hero rendition of the official CDN: `<prefix><set id>/<file>.mp4|webm`.
pub fn check(args: &HeroMediaArgs) -> Result<Wanted, String> {
    let rest = args
        .url
        .strip_prefix(CDN_PREFIX)
        .ok_or("not a hero video of Rustic's CDN")?;
    let (set, name) = rest.split_once('/').ok_or("not a hero video file")?;
    let set_ok = (8..=64).contains(&set.len())
        && set
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    let name_ok = !name.is_empty()
        && name.len() <= 96
        && name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'_')
        })
        && !name.contains("..");
    if !set_ok || !name_ok {
        return Err("not a hero video file".into());
    }
    let ext = match name.rsplit_once('.') {
        Some((_, "mp4")) => "mp4",
        Some((_, "webm")) => "webm",
        _ => return Err("a hero video is an .mp4 or .webm file".into()),
    };
    let sha = Checksum::sha256(&args.sha256).ok_or("sha256 is 64 hexadecimal characters")?;
    if args.bytes == 0 || args.bytes > MAX_BYTES {
        return Err("size out of range".into());
    }
    Ok(Wanted {
        url: args.url.clone(),
        file: format!("{}.{ext}", sha.hex),
        sha256: sha.hex,
        bytes: args.bytes,
    })
}

/// Removes every file of `dir` except `keep` and its partial download.
pub fn prune(dir: &Path, keep: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let part = format!("{keep}.part");
    for e in rd.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name == keep || name == part.as_str() {
            continue;
        }
        if e.file_type().is_ok_and(|t| t.is_file()) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// The hero cache: its own download queue (one at a time, out of the game downloads' progress)
/// and the files already checked in this run.
pub struct HeroMedia {
    dir: PathBuf,
    downloads: DownloadManager,
    verified: Mutex<HashSet<String>>,
    busy: tokio::sync::Mutex<()>,
}

impl HeroMedia {
    pub fn new(cache: &Path, http: rusticmc_launcher_core::reqwest::Client) -> Self {
        let dir = cache.join(DIR);
        Self {
            downloads: DownloadManager::with_client(http, dir.join(".unused"), 1, 3),
            dir,
            verified: Mutex::new(HashSet::new()),
            busy: tokio::sync::Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The local copy of `w`: checked when already there, downloaded (resuming) when not and
    /// `download` allows it (else `None`).
    pub async fn fetch(&self, w: &Wanted, download: bool) -> Result<Option<PathBuf>, String> {
        let _one = self.busy.lock().await;
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| e.to_string())?;
        let dest = self.dir.join(&w.file);
        let known = self.verified.lock().expect("hero lock").contains(&w.file);
        if !known && dest.is_file() {
            let (path, bytes, sha) = (dest.clone(), w.bytes, w.sha256.clone());
            let intact = tokio::task::spawn_blocking(move || {
                std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes)
                    && hash_file(&path, HashAlgo::Sha256).is_ok_and(|h| h == sha)
            })
            .await
            .unwrap_or(false);
            if !intact {
                let _ = std::fs::remove_file(&dest);
            }
        }
        if !dest.is_file() {
            if !download {
                return Ok(None);
            }
            let sha = Checksum::sha256(&w.sha256).ok_or("bad sha256")?;
            let req = DownloadRequest::new(&w.url, &dest)
                .checksum(sha)
                .size(w.bytes)
                .label("hero video")
                .no_cache();
            self.downloads
                .download(req)
                .await
                .map_err(|e| e.to_string())?;
        }
        self.verified
            .lock()
            .expect("hero lock")
            .insert(w.file.clone());
        let (dir, keep) = (self.dir.clone(), w.file.clone());
        let _ = tokio::task::spawn_blocking(move || prune(&dir, &keep)).await;
        Ok(Some(dest))
    }
}

/// `hero_media_fetch`: the local path of a hero rendition, downloaded when needed. The webview
/// plays it through the asset protocol (`convertFileSrc`).
#[tauri::command]
pub async fn hero_media_fetch(state: State<'_>, args: HeroMediaArgs) -> Res<Option<String>> {
    let wanted = check(&args).map_err(IpcError::invalid)?;
    let path = state
        .hero_media
        .fetch(&wanted, args.download)
        .await
        .map_err(|e| IpcError::unavailable(format!("hero video: {e}")))?;
    Ok(path.map(|p| p.display().to_string()))
}

/// Lets the asset protocol serve the hero folder, and nothing else.
pub fn allow_asset_scope(app: &tauri::AppHandle, dir: &Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        tracing::debug!(error = %e, "hero video: cache folder not created");
    }
    if let Err(e) = app.asset_protocol_scope().allow_directory(dir, false) {
        tracing::warn!(error = %e, "hero video: asset scope not set");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(url: &str) -> HeroMediaArgs {
        HeroMediaArgs {
            url: url.into(),
            sha256: "AB".repeat(32),
            bytes: 1000,
            download: true,
        }
    }

    #[test]
    fn only_hero_files_of_the_cdn() {
        let ok = check(&args(
            "https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero-1080p-av1.mp4",
        ))
        .unwrap();
        assert_eq!(ok.file, format!("{}.mp4", "ab".repeat(32)));
        assert!(
            check(&args(
                "https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero-1080p-vp9.webm"
            ))
            .is_ok()
        );
        for bad in [
            "https://evil.example/media/hero/0123456789ab/hero.mp4",
            "http://cdn.rustic-launcher.com/media/hero/0123456789ab/hero.mp4",
            "https://cdn.rustic-launcher.com/avatars/0123456789ab/hero.mp4",
            "https://cdn.rustic-launcher.com/media/hero/0123456789ab/../x.mp4",
            "https://cdn.rustic-launcher.com/media/hero/0123456789ab/a/b.mp4",
            "https://cdn.rustic-launcher.com/media/hero/xyz/hero.mp4",
            "https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero.exe",
            "https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero.mp4?x=1",
            "https://cdn.rustic-launcher.com.evil.example/media/hero/0123456789ab/hero.mp4",
        ] {
            assert!(check(&args(bad)).is_err(), "{bad}");
        }
        let mut a = args("https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero.mp4");
        a.sha256 = "zz".into();
        assert!(check(&a).is_err());
        a.sha256 = "ab".repeat(32);
        a.bytes = 0;
        assert!(check(&a).is_err());
        a.bytes = MAX_BYTES + 1;
        assert!(check(&a).is_err());
    }

    #[test]
    fn prune_keeps_the_current_file_and_its_partial_download() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["keep.mp4", "keep.mp4.part", "old.webm", "old.webm.part"] {
            std::fs::write(dir.path().join(f), b"x").unwrap();
        }
        std::fs::create_dir(dir.path().join(".unused")).unwrap();
        prune(dir.path(), "keep.mp4");
        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, [".unused", "keep.mp4", "keep.mp4.part"]);
    }

    #[tokio::test]
    async fn a_cached_file_is_checked_and_a_bad_one_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let hm = HeroMedia::new(dir.path(), rusticmc_launcher_core::http::client().unwrap());
        let body = b"hero video bytes".to_vec();
        let sha = rusticmc_launcher_core::download::hash_bytes(&body, HashAlgo::Sha256);
        let w = Wanted {
            url: "https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero.mp4".into(),
            file: format!("{sha}.mp4"),
            sha256: sha.clone(),
            bytes: body.len() as u64,
        };
        std::fs::create_dir_all(hm.dir()).unwrap();
        std::fs::write(hm.dir().join(&w.file), &body).unwrap();
        std::fs::write(hm.dir().join("stale.webm"), b"old").unwrap();
        // Already there and intact: no network, the stale file goes.
        let p = hm.fetch(&w, false).await.unwrap().unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), body);
        assert!(!hm.dir().join("stale.webm").exists());
        // A corrupted copy is dropped and fetched again (here the fetch fails: no CDN in tests).
        let hm2 = HeroMedia::new(dir.path(), rusticmc_launcher_core::http::client().unwrap());
        std::fs::write(hm2.dir().join(&w.file), b"corrupted bytes!").unwrap();
        let w_local = Wanted {
            url: "http://127.0.0.1:9/media/hero/0123456789ab/hero.mp4".into(),
            ..w
        };
        assert!(hm2.fetch(&w_local, true).await.is_err());
        assert!(!hm2.dir().join(&w_local.file).exists());
        // Not cached and no download allowed: nothing, and no network.
        assert_eq!(hm2.fetch(&w_local, false).await, Ok(None));
    }
}
