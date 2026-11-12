//! Downloads through a small trait, so installs run against the launcher's download manager in
//! the app and against an in-memory fake in tests (no network in tests).
//!
//! [`CoreDownloader`] adapts `rusticmc_launcher_core::DownloadManager` (the desktop owner's
//! queue: resumable, verifying, cached): it hands the manager the strongest hash, then checks
//! every other hash the source listed before the file counts as downloaded.
use crate::error::{ContentError, Result};
use crate::model::Hashes;
use rusticmc_launcher_core::{Checksum, DownloadManager, DownloadRequest, HashAlgo};
use std::path::{Path, PathBuf};

/// One file to fetch.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRequest {
    pub url: String,
    pub dest: PathBuf,
    pub size: Option<u64>,
    pub hashes: Hashes,
}

/// Fetches files to their destination, checked against their size and every hash given. A file
/// that fails a check is removed and never reaches `dest`.
#[async_trait::async_trait]
pub trait Downloader: Send + Sync {
    async fn fetch(&self, req: &FileRequest) -> Result<PathBuf>;
}

/// The hex digests of `bytes`.
pub fn digest(bytes: &[u8]) -> Hashes {
    use sha1::Digest as _;
    Hashes {
        sha1: Some(hex::encode(sha1::Sha1::digest(bytes))),
        sha256: Some(hex::encode(sha2::Sha256::digest(bytes))),
        sha512: Some(hex::encode(sha2::Sha512::digest(bytes))),
    }
}

/// Every hash `expected` lists equals the file's; and its size when given.
pub fn verify_file(path: &Path, size: Option<u64>, expected: &Hashes) -> Result<()> {
    let bytes = std::fs::read(path)?;
    verify_bytes(&bytes, size, expected).inspect_err(|_| {
        let _ = std::fs::remove_file(path);
    })
}

pub fn verify_bytes(bytes: &[u8], size: Option<u64>, expected: &Hashes) -> Result<()> {
    if expected.is_empty() {
        return Err(ContentError::Refused(
            "the source lists no checksum for this file".into(),
        ));
    }
    if let Some(s) = size
        && bytes.len() as u64 != s
    {
        return Err(ContentError::Refused(format!(
            "size {} differs from the listed {s}",
            bytes.len()
        )));
    }
    let got = digest(bytes);
    let pairs = [
        ("sha1", &expected.sha1, &got.sha1),
        ("sha256", &expected.sha256, &got.sha256),
        ("sha512", &expected.sha512, &got.sha512),
    ];
    for (name, want, have) in pairs {
        if let (Some(w), Some(h)) = (want, have)
            && !w.eq_ignore_ascii_case(h)
        {
            return Err(ContentError::Refused(format!(
                "{name} mismatch (expected {w}, got {h})"
            )));
        }
    }
    Ok(())
}

/// The launcher's download manager as a [`Downloader`].
#[derive(Clone)]
pub struct CoreDownloader {
    pub manager: DownloadManager,
}

#[async_trait::async_trait]
impl Downloader for CoreDownloader {
    async fn fetch(&self, req: &FileRequest) -> Result<PathBuf> {
        let checksum = req
            .hashes
            .sha512
            .as_deref()
            .and_then(|h| Checksum::new(HashAlgo::Sha512, h))
            .or_else(|| {
                req.hashes
                    .sha256
                    .as_deref()
                    .and_then(|h| Checksum::new(HashAlgo::Sha256, h))
            })
            .or_else(|| {
                req.hashes
                    .sha1
                    .as_deref()
                    .and_then(|h| Checksum::new(HashAlgo::Sha1, h))
            })
            .ok_or_else(|| ContentError::Refused("no usable checksum".into()))?;
        let mut dr = DownloadRequest::new(req.url.clone(), req.dest.clone()).checksum(checksum);
        if let Some(s) = req.size {
            dr = dr.size(s);
        }
        let path = self
            .manager
            .download(dr)
            .await
            .map_err(|e| ContentError::Download(e.to_string()))?;
        verify_file(&path, req.size, &req.hashes)?;
        Ok(path)
    }
}

/// An in-memory downloader keyed by URL (tests, and imports from files already on disk).
#[derive(Clone, Debug, Default)]
pub struct MemoryDownloader {
    pub files: std::collections::HashMap<String, Vec<u8>>,
}

impl MemoryDownloader {
    pub fn with(mut self, url: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.files.insert(url.into(), bytes);
        self
    }
}

#[async_trait::async_trait]
impl Downloader for MemoryDownloader {
    async fn fetch(&self, req: &FileRequest) -> Result<PathBuf> {
        let bytes = self
            .files
            .get(&req.url)
            .ok_or_else(|| ContentError::Download(format!("{}: 404", req.url)))?;
        verify_bytes(bytes, req.size, &req.hashes)?;
        if let Some(dir) = req.dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&req.dest, bytes)?;
        Ok(req.dest.clone())
    }
}
