//! Fetching many files at once through the core download manager (parallel, resumable `.part`
//! files, sha1 and size checked, retries), with one aggregated progress instead of one task per
//! file: a version has thousands of asset objects.

use crate::{JavaError, Result};
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use rusticmc_launcher_core::{
    CancellationToken, Checksum, DownloadError, DownloadManager, DownloadRequest,
};
use std::path::{Path, PathBuf};

/// One file to fetch, relative to a root folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDownload {
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    /// Relative to the root folder.
    pub path: PathBuf,
}

/// Bytes done and the total of a batch (files without a size count as 0).
pub type Progress<'a> = &'a (dyn Fn(u64, u64) + Send + Sync);

/// Whether `s` is a sha1 in hex.
pub fn is_sha1(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Downloads every file of `files` under `root` that is not already there and valid. Files are
/// checked against their sha1 (and size); a file without a sha1 that exists with the right size
/// (or any size, without one) is kept. Every URL must be on an allowed host (`extra_hosts`:
/// tests). Stops at the first failure or on `cancel`; what is done stays.
pub async fn fetch_all(
    downloads: &DownloadManager,
    root: &Path,
    files: Vec<FileDownload>,
    extra_hosts: &[String],
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> Result<()> {
    for f in &files {
        if !crate::url_allowed(&f.url, extra_hosts) {
            return Err(JavaError::HostNotAllowed(f.url.clone()));
        }
    }
    let total: u64 = files.iter().filter_map(|f| f.size).sum();
    let mut done = 0u64;
    let mut reqs = Vec::with_capacity(files.len());
    for f in files {
        let dest = root.join(&f.path);
        let sha1 = f.sha1.filter(|s| is_sha1(s));
        if sha1.is_none() {
            let present = std::fs::metadata(&dest)
                .is_ok_and(|m| m.is_file() && f.size.is_none_or(|s| s == m.len()));
            if present {
                done += f.size.unwrap_or(0);
                continue;
            }
        }
        let mut req = DownloadRequest::new(&f.url, dest).no_cache();
        if let Some(h) = &sha1 {
            req = req.checksum(Checksum::sha1(h).ok_or_else(|| JavaError::bad("bad sha1"))?);
        }
        if let Some(s) = f.size {
            req = req.size(s);
        }
        reqs.push((req, f.size.unwrap_or(0)));
    }
    progress(done, total);
    if reqs.is_empty() {
        return Ok(());
    }
    let handles: Vec<_> = reqs
        .into_iter()
        .map(|(r, size)| (downloads.enqueue(r), size))
        .collect();
    let tokens: Vec<CancellationToken> = handles.iter().map(|(h, _)| h.token()).collect();
    let stop = || tokens.iter().for_each(CancellationToken::cancel);
    let mut pending: FuturesUnordered<_> = handles
        .into_iter()
        .map(|(h, size)| async move { (h.wait().await, size) })
        .collect();
    let mut first_error: Option<DownloadError> = None;
    loop {
        let next = tokio::select! {
            n = pending.next() => n,
            _ = cancel.cancelled() => {
                stop();
                return Err(JavaError::Cancelled);
            }
        };
        let Some((result, size)) = next else { break };
        match result {
            Ok(_) => {
                done += size;
                progress(done, total);
            }
            Err(e) if first_error.is_none() => {
                stop();
                first_error = Some(e);
            }
            Err(_) => {}
        }
    }
    match first_error {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

/// GETs `url` as text (manifests and metadata), on an allowed host.
pub async fn get_text(
    http: &rusticmc_launcher_core::reqwest::Client,
    url: &str,
    extra_hosts: &[String],
) -> Result<String> {
    if !crate::url_allowed(url, extra_hosts) {
        return Err(JavaError::HostNotAllowed(url.to_owned()));
    }
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| JavaError::Network(format!("{url}: {e}")))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(JavaError::Http {
            url: url.to_owned(),
            status,
        });
    }
    resp.text()
        .await
        .map_err(|e| JavaError::Network(format!("{url}: {e}")))
}
