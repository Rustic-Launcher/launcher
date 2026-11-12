//! The download manager: a queue of HTTP downloads run a few at a time, each resumable (HTTP range
//! requests on a `.part` file kept across retries and launcher restarts), verified (size and
//! SHA-1/SHA-256/SHA-512), and shared through a content-addressed cache
//! (`<cache>/<algo>/<xx>/<hex>`), so a file with a known hash is downloaded once. Global progress
//! goes out as [`DownloadEvent`]s on a broadcast channel; any download, or all of them, can be
//! cancelled.

use crate::http;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Semaphore, broadcast};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// A hash algorithm a download can be checked with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgo {
    Sha1,
    Sha256,
    Sha512,
}

impl HashAlgo {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    fn hex_len(self) -> usize {
        match self {
            Self::Sha1 => 40,
            Self::Sha256 => 64,
            Self::Sha512 => 128,
        }
    }
}

/// An expected hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Checksum {
    pub algo: HashAlgo,
    /// Lowercase hex.
    pub hex: String,
}

impl Checksum {
    /// `None` when `hex` isn't a hash of that algorithm.
    pub fn new(algo: HashAlgo, hex: &str) -> Option<Self> {
        let hex = hex.trim().to_ascii_lowercase();
        (hex.len() == algo.hex_len() && hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .then_some(Self { algo, hex })
    }
    pub fn sha1(hex: &str) -> Option<Self> {
        Self::new(HashAlgo::Sha1, hex)
    }
    pub fn sha256(hex: &str) -> Option<Self> {
        Self::new(HashAlgo::Sha256, hex)
    }
    pub fn sha512(hex: &str) -> Option<Self> {
        Self::new(HashAlgo::Sha512, hex)
    }
}

/// An incremental hasher of any [`HashAlgo`].
pub enum Hasher {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha512(sha2::Sha512),
}

impl Hasher {
    pub fn new(algo: HashAlgo) -> Self {
        match algo {
            HashAlgo::Sha1 => Self::Sha1(sha1::Sha1::new()),
            HashAlgo::Sha256 => Self::Sha256(sha2::Sha256::new()),
            HashAlgo::Sha512 => Self::Sha512(sha2::Sha512::new()),
        }
    }
    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1(h) => h.update(bytes),
            Self::Sha256(h) => h.update(bytes),
            Self::Sha512(h) => h.update(bytes),
        }
    }
    pub fn finish_hex(self) -> String {
        match self {
            Self::Sha1(h) => hex::encode(h.finalize()),
            Self::Sha256(h) => hex::encode(h.finalize()),
            Self::Sha512(h) => hex::encode(h.finalize()),
        }
    }
}

/// The hex hash of a file.
pub fn hash_file(path: &Path, algo: HashAlgo) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Hasher::new(algo);
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish_hex())
}

/// The hex hash of bytes.
pub fn hash_bytes(bytes: &[u8], algo: HashAlgo) -> String {
    let mut hasher = Hasher::new(algo);
    hasher.update(bytes);
    hasher.finish_hex()
}

/// One file to download.
#[derive(Clone, Debug)]
pub struct DownloadRequest {
    pub url: String,
    /// Where the file ends up.
    pub dest: PathBuf,
    pub checksum: Option<Checksum>,
    /// The expected size, when known (progress and a size check).
    pub size: Option<u64>,
    /// Shown in progress events.
    pub label: Option<String>,
    /// Keep a copy in the content-addressed cache (only with a checksum). Off for big one-off
    /// archives, to save disk.
    pub cache: bool,
}

impl DownloadRequest {
    pub fn new(url: impl Into<String>, dest: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            dest: dest.into(),
            checksum: None,
            size: None,
            label: None,
            cache: true,
        }
    }
    pub fn checksum(mut self, checksum: Checksum) -> Self {
        self.checksum = Some(checksum);
        self
    }
    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn no_cache(mut self) -> Self {
        self.cache = false;
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("{url}: the server answered HTTP {status}")]
    Http { url: String, status: u16 },
    #[error("{url}: {message}")]
    Network { url: String, message: String },
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{url}: {algo} mismatch (expected {expected}, got {actual}): the file was discarded")]
    Checksum {
        url: String,
        algo: &'static str,
        expected: String,
        actual: String,
    },
    #[error("{url}: expected {expected} bytes, got {actual}")]
    Size {
        url: String,
        expected: u64,
        actual: u64,
    },
    #[error("download cancelled")]
    Cancelled,
    #[error("download task failed: {0}")]
    Task(String),
}

impl DownloadError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Whether trying again may help.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network { .. } => true,
            Self::Http { status, .. } => *status >= 500 || *status == 429 || *status == 408,
            _ => false,
        }
    }
}

/// Totals over every download this manager has seen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub queued: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

/// What the manager reports.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DownloadEvent {
    Queued {
        id: u64,
        url: String,
        label: Option<String>,
        size: Option<u64>,
        global: GlobalProgress,
    },
    Started {
        id: u64,
        global: GlobalProgress,
    },
    Progress {
        id: u64,
        downloaded: u64,
        total: Option<u64>,
        global: GlobalProgress,
    },
    Finished {
        id: u64,
        path: PathBuf,
        from_cache: bool,
        global: GlobalProgress,
    },
    Failed {
        id: u64,
        error: String,
        global: GlobalProgress,
    },
    Cancelled {
        id: u64,
        global: GlobalProgress,
    },
}

/// A queued download.
pub struct DownloadHandle {
    pub id: u64,
    cancel: CancellationToken,
    task: JoinHandle<Result<PathBuf, DownloadError>>,
}

impl DownloadHandle {
    /// Cancels this download (its `.part` file stays, to resume later).
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// The cancellation token of this download.
    pub fn token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Waits for the file.
    pub async fn wait(self) -> Result<PathBuf, DownloadError> {
        self.task
            .await
            .map_err(|e| DownloadError::Task(e.to_string()))?
    }
}

/// The download queue. Cheap to clone; clones share the queue.
#[derive(Clone)]
pub struct DownloadManager {
    inner: Arc<Inner>,
}

struct Inner {
    client: reqwest::Client,
    cache: PathBuf,
    permits: Arc<Semaphore>,
    retries: u32,
    events: broadcast::Sender<DownloadEvent>,
    totals: Mutex<GlobalProgress>,
    shutdown: CancellationToken,
    next_id: AtomicU64,
    running: Mutex<std::collections::HashMap<u64, CancellationToken>>,
}

/// Bytes between two progress events of one download.
const PROGRESS_STEP: u64 = 64 * 1024;

impl DownloadManager {
    /// A manager caching under `cache_dir`, `concurrency` downloads at once, `retries` extra
    /// attempts per file.
    pub fn new(
        cache_dir: impl Into<PathBuf>,
        concurrency: usize,
        retries: u32,
    ) -> crate::Result<Self> {
        Ok(Self::with_client(
            http::client()?,
            cache_dir,
            concurrency,
            retries,
        ))
    }

    pub fn with_client(
        client: reqwest::Client,
        cache_dir: impl Into<PathBuf>,
        concurrency: usize,
        retries: u32,
    ) -> Self {
        let (events, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(Inner {
                client,
                cache: cache_dir.into(),
                permits: Arc::new(Semaphore::new(concurrency.max(1))),
                retries,
                events,
                totals: Mutex::new(GlobalProgress::default()),
                shutdown: CancellationToken::new(),
                next_id: AtomicU64::new(1),
                running: Mutex::new(std::collections::HashMap::new()),
            }),
        }
    }

    /// Progress events of every download.
    pub fn subscribe(&self) -> broadcast::Receiver<DownloadEvent> {
        self.inner.events.subscribe()
    }

    /// The totals now.
    pub fn progress(&self) -> GlobalProgress {
        *self.inner.totals.lock().expect("totals lock")
    }

    /// The HTTP client (for small requests that don't need the queue).
    pub fn client(&self) -> &reqwest::Client {
        &self.inner.client
    }

    /// Cancels one download by its id; false when it isn't queued or running.
    pub fn cancel(&self, id: u64) -> bool {
        match self.inner.running.lock().expect("running lock").get(&id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// Cancels every queued and running download.
    pub fn cancel_all(&self) {
        self.inner.shutdown.cancel();
    }

    /// Where a request's partial data is kept while it downloads.
    pub fn partial_path(&self, req: &DownloadRequest) -> PathBuf {
        match (&req.checksum, req.cache) {
            (Some(ck), true) => {
                self.inner
                    .cache
                    .join("partial")
                    .join(format!("{}-{}.part", ck.algo.name(), ck.hex))
            }
            _ => {
                let mut name = req.dest.file_name().unwrap_or_default().to_os_string();
                name.push(".part");
                req.dest.with_file_name(name)
            }
        }
    }

    /// Where the cache keeps a file with this checksum.
    pub fn cache_path(&self, checksum: &Checksum) -> PathBuf {
        self.inner
            .cache
            .join(checksum.algo.name())
            .join(&checksum.hex[..2])
            .join(&checksum.hex)
    }

    /// Queues a download.
    pub fn enqueue(&self, req: DownloadRequest) -> DownloadHandle {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let cancel = self.inner.shutdown.child_token();
        self.inner
            .running
            .lock()
            .expect("running lock")
            .insert(id, cancel.clone());
        let global = self.update(|t| {
            t.queued += 1;
            t.total_bytes += req.size.unwrap_or(0);
        });
        self.emit(DownloadEvent::Queued {
            id,
            url: req.url.clone(),
            label: req.label.clone(),
            size: req.size,
            global,
        });
        let this = self.clone();
        let token = cancel.clone();
        let task = tokio::spawn(async move { this.run(id, req, token).await });
        DownloadHandle { id, cancel, task }
    }

    /// Downloads one file.
    pub async fn download(&self, req: DownloadRequest) -> Result<PathBuf, DownloadError> {
        self.enqueue(req).wait().await
    }

    /// Downloads every file; on the first failure the others are cancelled and the error returned.
    pub async fn download_all(
        &self,
        reqs: Vec<DownloadRequest>,
    ) -> Result<Vec<PathBuf>, DownloadError> {
        let handles: Vec<DownloadHandle> = reqs.into_iter().map(|r| self.enqueue(r)).collect();
        let tokens: Vec<CancellationToken> = handles.iter().map(DownloadHandle::token).collect();
        let mut out = Vec::with_capacity(handles.len());
        let mut first_error = None;
        for handle in handles {
            match handle.wait().await {
                Ok(path) => out.push(path),
                Err(e) => {
                    if first_error.is_none() {
                        tokens.iter().for_each(CancellationToken::cancel);
                        first_error = Some(e);
                    }
                }
            }
        }
        match first_error {
            Some(e) => Err(e),
            None => Ok(out),
        }
    }

    /// The size of the content-addressed cache.
    pub fn cache_size(&self) -> u64 {
        crate::fsutil::dir_size(&self.inner.cache)
    }

    /// Empties the cache (partial files included).
    pub fn clear_cache(&self) -> std::io::Result<()> {
        match std::fs::remove_dir_all(&self.inner.cache) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    fn emit(&self, event: DownloadEvent) {
        let _ = self.inner.events.send(event);
    }

    fn update(&self, f: impl FnOnce(&mut GlobalProgress)) -> GlobalProgress {
        let mut totals = self.inner.totals.lock().expect("totals lock");
        f(&mut totals);
        *totals
    }

    async fn run(
        &self,
        id: u64,
        req: DownloadRequest,
        cancel: CancellationToken,
    ) -> Result<PathBuf, DownloadError> {
        let mut credited = 0u64;
        let result = self.run_inner(id, &req, &cancel, &mut credited).await;
        self.inner.running.lock().expect("running lock").remove(&id);
        let global = self.update(|t| match &result {
            Ok(_) => t.completed += 1,
            Err(DownloadError::Cancelled) => t.cancelled += 1,
            Err(_) => t.failed += 1,
        });
        match &result {
            Ok((path, from_cache)) => self.emit(DownloadEvent::Finished {
                id,
                path: path.clone(),
                from_cache: *from_cache,
                global,
            }),
            Err(DownloadError::Cancelled) => self.emit(DownloadEvent::Cancelled { id, global }),
            Err(e) => {
                tracing::warn!(url = %req.url, error = %e, "download failed");
                self.emit(DownloadEvent::Failed {
                    id,
                    error: e.to_string(),
                    global,
                });
            }
        }
        result.map(|(path, _)| path)
    }

    async fn run_inner(
        &self,
        id: u64,
        req: &DownloadRequest,
        cancel: &CancellationToken,
        credited: &mut u64,
    ) -> Result<(PathBuf, bool), DownloadError> {
        // Already there: the cache, or the destination itself.
        if let Some(ck) = &req.checksum
            && let Some(path) = self.cached(req, ck).await?
        {
            self.leave_queue();
            self.credit(credited, req.size.unwrap_or(*credited));
            return Ok((path, true));
        }
        let permit = tokio::select! {
            _ = cancel.cancelled() => {
                self.leave_queue();
                return Err(DownloadError::Cancelled);
            }
            permit = self.inner.permits.clone().acquire_owned() => {
                permit.map_err(|e| DownloadError::Task(e.to_string()))?
            }
        };
        let global = self.update(|t| {
            t.queued = t.queued.saturating_sub(1);
            t.active += 1;
        });
        self.emit(DownloadEvent::Started { id, global });
        let result = self.fetch_verified(id, req, cancel, credited).await;
        self.update(|t| t.active = t.active.saturating_sub(1));
        drop(permit);
        result.map(|p| (p, false))
    }

    fn leave_queue(&self) {
        self.update(|t| t.queued = t.queued.saturating_sub(1));
    }

    /// Moves the global downloaded count so that this job counts for `now` bytes.
    fn credit(&self, credited: &mut u64, now: u64) -> GlobalProgress {
        let before = *credited;
        *credited = now;
        self.update(|t| {
            t.downloaded_bytes = t.downloaded_bytes.saturating_sub(before) + now;
        })
    }

    async fn cached(
        &self,
        req: &DownloadRequest,
        ck: &Checksum,
    ) -> Result<Option<PathBuf>, DownloadError> {
        let dest = req.dest.clone();
        let cached = self.cache_path(ck);
        let ck2 = ck.clone();
        let use_cache = req.cache;
        tokio::task::spawn_blocking(move || -> Result<Option<PathBuf>, DownloadError> {
            if use_cache && cached.is_file() {
                if cached != dest {
                    place_copy(&cached, &dest)?;
                }
                return Ok(Some(dest));
            }
            if dest.is_file() && hash_file(&dest, ck2.algo).ok().as_deref() == Some(&ck2.hex) {
                if use_cache {
                    let _ = place_copy(&dest, &cached);
                }
                return Ok(Some(dest));
            }
            Ok(None)
        })
        .await
        .map_err(|e| DownloadError::Task(e.to_string()))?
    }

    async fn fetch_verified(
        &self,
        id: u64,
        req: &DownloadRequest,
        cancel: &CancellationToken,
        credited: &mut u64,
    ) -> Result<PathBuf, DownloadError> {
        let part = self.partial_path(req);
        if let Some(parent) = part.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DownloadError::io(parent, e))?;
        }
        let mut fresh_retry_done = false;
        loop {
            let resumed = self
                .fetch_with_retries(id, req, &part, cancel, credited)
                .await?;
            match self.verify(req, &part).await {
                Ok(()) => break,
                Err(e) => {
                    let _ = tokio::fs::remove_file(&part).await;
                    self.credit(credited, 0);
                    // A resumed file may have mixed two versions of the file: once from scratch.
                    if resumed && !fresh_retry_done {
                        fresh_retry_done = true;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        let dest = req.dest.clone();
        let cached = req
            .checksum
            .as_ref()
            .filter(|_| req.cache)
            .map(|ck| self.cache_path(ck));
        tokio::task::spawn_blocking(move || -> Result<PathBuf, DownloadError> {
            match cached {
                Some(cached) => {
                    move_file(&part, &cached)?;
                    if cached != dest {
                        place_copy(&cached, &dest)?;
                    }
                }
                None => move_file(&part, &dest)?,
            }
            Ok(dest)
        })
        .await
        .map_err(|e| DownloadError::Task(e.to_string()))?
    }

    /// Fetches into `part`, resuming and retrying. Returns whether some of the data came from an
    /// earlier attempt.
    async fn fetch_with_retries(
        &self,
        id: u64,
        req: &DownloadRequest,
        part: &Path,
        cancel: &CancellationToken,
        credited: &mut u64,
    ) -> Result<bool, DownloadError> {
        let mut attempt = 0;
        let mut resumed_any = false;
        loop {
            match self.fetch_once(id, req, part, cancel, credited).await {
                Ok(resumed) => return Ok(resumed || resumed_any),
                Err(e) if e.is_retryable() && attempt < self.inner.retries => {
                    attempt += 1;
                    resumed_any = true;
                    let backoff = Duration::from_millis(200 * 2u64.pow(attempt.min(6)));
                    tracing::debug!(url = %req.url, error = %e, attempt, "retrying download");
                    tokio::select! {
                        _ = cancel.cancelled() => return Err(DownloadError::Cancelled),
                        _ = tokio::time::sleep(backoff) => {}
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn fetch_once(
        &self,
        id: u64,
        req: &DownloadRequest,
        part: &Path,
        cancel: &CancellationToken,
        credited: &mut u64,
    ) -> Result<bool, DownloadError> {
        let existing = tokio::fs::metadata(part).await.map_or(0, |m| m.len());
        if existing > 0 && req.size == Some(existing) {
            self.credit(credited, existing);
            return Ok(true);
        }
        let mut request = self.inner.client.get(&req.url);
        if existing > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={existing}-"));
        }
        let network = |e: reqwest::Error| DownloadError::Network {
            url: req.url.clone(),
            message: e.to_string(),
        };
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(DownloadError::Cancelled),
            r = request.send() => r.map_err(network)?,
        };
        let status = response.status().as_u16();
        let offset = match status {
            206 if existing > 0 => existing,
            200 => 0,
            416 if existing > 0 => {
                // The part is longer than the file now: start over.
                let _ = tokio::fs::remove_file(part).await;
                self.credit(credited, 0);
                return Err(DownloadError::Network {
                    url: req.url.clone(),
                    message: "range not satisfiable; restarting".into(),
                });
            }
            _ => {
                return Err(DownloadError::Http {
                    url: req.url.clone(),
                    status,
                });
            }
        };
        let total = req
            .size
            .or_else(|| response.content_length().map(|l| l + offset));
        if req.size.is_none()
            && let Some(total) = total
        {
            // Counted once per job, the first time the server says it.
            if *credited == 0 && existing == 0 {
                self.update(|t| t.total_bytes += total);
            }
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(offset > 0)
            .truncate(offset == 0)
            .open(part)
            .await
            .map_err(|e| DownloadError::io(part, e))?;
        let mut done = offset;
        self.credit(credited, done);
        let mut last_event = done;
        let mut stream = response.bytes_stream();
        loop {
            let chunk = tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = file.flush().await;
                    return Err(DownloadError::Cancelled);
                }
                chunk = stream.next() => chunk,
            };
            let Some(chunk) = chunk else { break };
            let chunk = chunk.map_err(network)?;
            file.write_all(&chunk)
                .await
                .map_err(|e| DownloadError::io(part, e))?;
            done += chunk.len() as u64;
            let global = self.credit(credited, done);
            if done - last_event >= PROGRESS_STEP || Some(done) == total {
                last_event = done;
                self.emit(DownloadEvent::Progress {
                    id,
                    downloaded: done,
                    total,
                    global,
                });
            }
        }
        file.flush().await.map_err(|e| DownloadError::io(part, e))?;
        file.sync_all()
            .await
            .map_err(|e| DownloadError::io(part, e))?;
        Ok(offset > 0)
    }

    async fn verify(&self, req: &DownloadRequest, part: &Path) -> Result<(), DownloadError> {
        let len = tokio::fs::metadata(part)
            .await
            .map_err(|e| DownloadError::io(part, e))?
            .len();
        if let Some(size) = req.size
            && size != len
        {
            return Err(DownloadError::Size {
                url: req.url.clone(),
                expected: size,
                actual: len,
            });
        }
        let Some(ck) = req.checksum.clone() else {
            return Ok(());
        };
        let path = part.to_path_buf();
        let actual = tokio::task::spawn_blocking(move || hash_file(&path, ck.algo))
            .await
            .map_err(|e| DownloadError::Task(e.to_string()))?
            .map_err(|e| DownloadError::io(part, e))?;
        if actual == ck.hex {
            Ok(())
        } else {
            Err(DownloadError::Checksum {
                url: req.url.clone(),
                algo: ck.algo.name(),
                expected: ck.hex,
                actual,
            })
        }
    }
}

/// Copies `from` to `to` through a temporary file (no half-written destination).
fn place_copy(from: &Path, to: &Path) -> Result<(), DownloadError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DownloadError::io(parent, e))?;
    }
    let tmp = crate::fsutil::temp_sibling(to);
    std::fs::copy(from, &tmp).map_err(|e| DownloadError::io(&tmp, e))?;
    std::fs::rename(&tmp, to).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        DownloadError::io(to, e)
    })
}

/// Renames `from` to `to` (copying across volumes).
fn move_file(from: &Path, to: &Path) -> Result<(), DownloadError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DownloadError::io(parent, e))?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    place_copy(from, to)?;
    let _ = std::fs::remove_file(from);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    fn body(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 % 251) as u8).collect()
    }

    /// Serves `data`, honouring `Range: bytes=N-` with 206.
    struct Ranged(Vec<u8>);

    impl Respond for Ranged {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let range = request
                .headers
                .get("range")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("bytes="))
                .and_then(|v| v.strip_suffix('-'))
                .and_then(|v| v.parse::<usize>().ok());
            match range {
                Some(start) if start < self.0.len() => ResponseTemplate::new(206)
                    .insert_header(
                        "content-range",
                        format!("bytes {start}-{}/{}", self.0.len() - 1, self.0.len()).as_str(),
                    )
                    .set_body_bytes(self.0[start..].to_vec()),
                Some(_) => ResponseTemplate::new(416),
                None => ResponseTemplate::new(200).set_body_bytes(self.0.clone()),
            }
        }
    }

    fn manager(dir: &Path) -> DownloadManager {
        DownloadManager::new(dir.join("cache"), 3, 2).unwrap()
    }

    #[tokio::test]
    async fn downloads_verifies_and_caches() {
        let server = MockServer::start().await;
        let data = body(300_000);
        Mock::given(method("GET"))
            .and(path("/a.bin"))
            .respond_with(Ranged(data.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        let ck = Checksum::sha256(&hash_bytes(&data, HashAlgo::Sha256)).unwrap();
        let url = format!("{}/a.bin", server.uri());
        let first = m
            .download(
                DownloadRequest::new(&url, dir.path().join("one/a.bin"))
                    .checksum(ck.clone())
                    .size(data.len() as u64),
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read(&first).unwrap(), data);
        // The same hash elsewhere comes from the cache: the mock expects one request only.
        let mut events = m.subscribe();
        let second = m
            .download(DownloadRequest::new(&url, dir.path().join("two/a.bin")).checksum(ck))
            .await
            .unwrap();
        assert_eq!(std::fs::read(second).unwrap(), data);
        let mut from_cache = false;
        while let Ok(e) = events.try_recv() {
            if let DownloadEvent::Finished { from_cache: c, .. } = e {
                from_cache = c;
            }
        }
        assert!(from_cache);
        assert_eq!(m.progress().completed, 2);
    }

    #[tokio::test]
    async fn every_algorithm_is_checked() {
        let server = MockServer::start().await;
        let data = body(1000);
        Mock::given(method("GET"))
            .respond_with(Ranged(data.clone()))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        for algo in [HashAlgo::Sha1, HashAlgo::Sha256, HashAlgo::Sha512] {
            let ck = Checksum::new(algo, &hash_bytes(&data, algo)).unwrap();
            let dest = dir.path().join(format!("{}.bin", algo.name()));
            m.download(
                DownloadRequest::new(format!("{}/x", server.uri()), &dest)
                    .checksum(ck)
                    .no_cache(),
            )
            .await
            .unwrap();
            assert_eq!(std::fs::read(dest).unwrap(), data);
        }
    }

    #[tokio::test]
    async fn a_wrong_checksum_is_refused() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(Ranged(body(5000)))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        let dest = dir.path().join("bad.bin");
        let err = m
            .download(
                DownloadRequest::new(format!("{}/bad", server.uri()), &dest)
                    .checksum(Checksum::sha1(&"0".repeat(40)).unwrap()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Checksum { .. }), "{err}");
        assert!(!dest.exists());
        assert_eq!(m.progress().failed, 1);
    }

    #[tokio::test]
    async fn resumes_with_a_range_request() {
        let server = MockServer::start().await;
        let data = body(200_000);
        Mock::given(method("GET"))
            .respond_with(Ranged(data.clone()))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        let ck = Checksum::sha256(&hash_bytes(&data, HashAlgo::Sha256)).unwrap();
        let req = DownloadRequest::new(format!("{}/big", server.uri()), dir.path().join("big"))
            .checksum(ck);
        // Half of it from an interrupted run.
        let part = m.partial_path(&req);
        std::fs::create_dir_all(part.parent().unwrap()).unwrap();
        std::fs::write(&part, &data[..80_000]).unwrap();
        let path = m.download(req).await.unwrap();
        assert_eq!(std::fs::read(path).unwrap(), data);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers.get("range").unwrap().to_str().unwrap(),
            "bytes=80000-"
        );
        assert!(!part.exists());
    }

    #[tokio::test]
    async fn retries_server_errors() {
        let server = MockServer::start().await;
        let data = body(1234);
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(Ranged(data.clone()))
            .with_priority(2)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        let path = m
            .download(DownloadRequest::new(
                format!("{}/flaky", server.uri()),
                dir.path().join("flaky"),
            ))
            .await
            .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), data);
    }

    #[tokio::test]
    async fn a_404_is_not_retried() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let err = manager(dir.path())
            .download(DownloadRequest::new(
                format!("{}/missing", server.uri()),
                dir.path().join("m"),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, DownloadError::Http { status: 404, .. }));
    }

    #[tokio::test]
    async fn cancellation_stops_a_download() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(body(10))
                    .set_delay(Duration::from_secs(30)),
            )
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        let handle = m.enqueue(DownloadRequest::new(
            format!("{}/slow", server.uri()),
            dir.path().join("slow"),
        ));
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.cancel();
        let err = tokio::time::timeout(Duration::from_secs(5), handle.wait())
            .await
            .expect("cancel is prompt")
            .unwrap_err();
        assert!(matches!(err, DownloadError::Cancelled));
        assert_eq!(m.progress().cancelled, 1);
    }

    #[tokio::test]
    async fn many_files_report_global_progress() {
        let server = MockServer::start().await;
        let data = body(100_000);
        Mock::given(method("GET"))
            .respond_with(Ranged(data.clone()))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let m = manager(dir.path());
        let mut events = m.subscribe();
        let reqs = (0..6)
            .map(|i| {
                DownloadRequest::new(
                    format!("{}/f{i}", server.uri()),
                    dir.path().join(format!("f{i}")),
                )
                .size(data.len() as u64)
            })
            .collect();
        let paths = m.download_all(reqs).await.unwrap();
        assert_eq!(paths.len(), 6);
        let p = m.progress();
        assert_eq!(p.completed, 6);
        assert_eq!(p.total_bytes, 600_000);
        assert_eq!(p.downloaded_bytes, 600_000);
        assert_eq!((p.queued, p.active), (0, 0));
        let mut finished = 0;
        while let Ok(e) = events.try_recv() {
            if matches!(e, DownloadEvent::Finished { .. }) {
                finished += 1;
            }
        }
        assert_eq!(finished, 6);
    }
}
