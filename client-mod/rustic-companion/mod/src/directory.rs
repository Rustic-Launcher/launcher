//! Rustic Cloud's server directory for the Discover screen (ADR 0158): its lists, its catalog and
//! the servers' pictures, fetched on threads of their own (never on the game's), each list kept
//! a minute (the API's own cache time), pictures decoded once (PNG, WebP or JPEG, by the game's
//! own decoders) into the canvas. A join is counted with the anonymous `plays` call.
use crate::canvas::canvas;
use crate::state::lock;
use rusticmc_companion_core::discover::{self, Listing, Query, Status};
use rusticmc_companion_core::title::{self, Rgba};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A list is asked again after this long.
const FRESH: Duration = Duration::from_secs(60);
/// A failed list is asked again after this long.
const RETRY: Duration = Duration::from_secs(15);
/// Pictures downloaded at once.
const PICTURES_AT_ONCE: usize = 4;
/// The largest answer read (lists, the catalog).
const MAX_JSON: u64 = 4 * 1024 * 1024;
/// The largest picture read.
const MAX_PICTURE: u64 = 4 * 1024 * 1024;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent(&format!("rustic-companion/{}", env!("CARGO_PKG_VERSION")))
        .build()
}

/// `GET url` as JSON; None on any failure.
pub fn get_json(url: &str) -> Option<serde_json::Value> {
    let r = agent().get(url).call().ok()?;
    let mut body = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(r.into_reader(), MAX_JSON),
        &mut body,
    )
    .ok()?;
    serde_json::from_slice(&body).ok()
}

fn get_bytes(url: &str, cap: u64) -> Option<Vec<u8>> {
    let r = agent().get(url).call().ok()?;
    let mut body = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(r.into_reader(), cap), &mut body).ok()?;
    Some(body)
}

#[derive(Default)]
struct Entry {
    items: Vec<Listing>,
    loaded: Option<Instant>,
    asked: Option<Instant>,
    in_flight: bool,
    failed: bool,
}

#[derive(Default)]
pub struct Directory {
    lists: HashMap<Query, Entry>,
    pub tags: Vec<String>,
    catalog_asked: Option<Instant>,
    pictures_asked: HashSet<String>,
    pictures_in_flight: usize,
}

impl Directory {
    /// Asks for `q` unless a fresh answer or a request is there.
    pub fn want(&mut self, q: &Query, api: &str, now: Instant) {
        let e = self.lists.entry(q.clone()).or_default();
        if e.in_flight {
            return;
        }
        let due = match e.loaded {
            Some(at) if !e.failed => now - at >= FRESH,
            _ => e.asked.is_none_or(|a| now - a >= RETRY),
        };
        if !due {
            return;
        }
        e.in_flight = true;
        e.asked = Some(now);
        let (key, url) = (q.clone(), q.url(api));
        let spawned = std::thread::Builder::new()
            .name("rustic-discover".into())
            .spawn(move || {
                let items = get_json(&url).and_then(|v| discover::parse_page(&v));
                if items.is_none() {
                    log::debug!("Rustic: no server list from {url}");
                }
                let mut st = lock();
                let e = st.directory.lists.entry(key).or_default();
                e.in_flight = false;
                match items {
                    Some(items) => {
                        e.items = items;
                        e.loaded = Some(Instant::now());
                        e.failed = false;
                    }
                    None => e.failed = true,
                }
            });
        if spawned.is_err()
            && let Some(e) = self.lists.get_mut(q)
        {
            e.in_flight = false;
            e.failed = true;
        }
    }

    /// The catalog's tags, once.
    pub fn want_catalog(&mut self, api: &str, now: Instant) {
        if !self.tags.is_empty() || self.catalog_asked.is_some_and(|t| now - t < RETRY) {
            return;
        }
        self.catalog_asked = Some(now);
        let url = discover::catalog_url(api);
        let _ = std::thread::Builder::new()
            .name("rustic-discover".into())
            .spawn(move || {
                if let Some(v) = get_json(&url) {
                    let tags = discover::parse_catalog(&v);
                    lock().directory.tags = tags;
                }
            });
    }

    /// Asks every list again (Retry).
    pub fn retry(&mut self) {
        for e in self.lists.values_mut() {
            if e.failed {
                e.asked = None;
                e.loaded = None;
            }
        }
        self.catalog_asked = None;
    }

    pub fn items(&self, q: &Query) -> &[Listing] {
        self.lists.get(q).map_or(&[], |e| &e.items)
    }

    /// Whether `q` has answered (even with nothing).
    pub fn answered(&self, q: &Query) -> bool {
        self.lists.get(q).is_some_and(|e| e.loaded.is_some())
    }

    /// How the screen's main lists stand.
    pub fn status(&self) -> Status {
        let main = [Query::Featured, Query::Trending, Query::All];
        if main.iter().any(|q| self.answered(q)) {
            Status::Ready
        } else if main
            .iter()
            .all(|q| self.lists.get(q).is_some_and(|e| e.failed && !e.in_flight))
        {
            Status::Failed
        } else {
            Status::Loading
        }
    }

    /// Every listing the screen knows, by id.
    pub fn find(&self, id: &str) -> Option<Listing> {
        self.lists
            .values()
            .flat_map(|e| e.items.iter())
            .find(|l| l.id == id)
            .cloned()
    }

    /// Downloads and decodes `url` into the canvas, once, a few at a time.
    pub fn want_picture(&mut self, url: &str, banner: bool) {
        if self.pictures_asked.contains(url) || self.pictures_in_flight >= PICTURES_AT_ONCE {
            return;
        }
        self.pictures_asked.insert(url.to_owned());
        self.pictures_in_flight += 1;
        let url = url.to_owned();
        let spawned = std::thread::Builder::new()
            .name("rustic-picture".into())
            .spawn(move || {
                let picture = get_bytes(&url, MAX_PICTURE).and_then(|b| decode(&b, banner));
                match picture {
                    Some(im) => {
                        canvas().remote.insert(url, Arc::new(im));
                    }
                    None => log::debug!("Rustic: no picture from {url}"),
                }
                let mut st = lock();
                st.directory.pictures_in_flight = st.directory.pictures_in_flight.saturating_sub(1);
            });
        if spawned.is_err() {
            self.pictures_in_flight = self.pictures_in_flight.saturating_sub(1);
        }
    }
}

/// A picture as RGBA, cut down to what the screen shows at most (an icon 128 pixels, a banner
/// 640 wide).
fn decode(bytes: &[u8], banner: bool) -> Option<Rgba> {
    let (w, h, pixels) = horizon_client::hud::decode_image_bytes(bytes).ok()?;
    let im = Rgba {
        width: w,
        height: h,
        pixels,
    };
    let (max_w, max_h) = if banner { (640, 256) } else { (128, 128) };
    let k = (w as f32 / max_w as f32).max(h as f32 / max_h as f32);
    Some(if k > 1.0 {
        title::resample(
            &im,
            (w as f32 / k).round() as u32,
            (h as f32 / k).round() as u32,
        )
    } else {
        im
    })
}

/// Counts a join (anonymous, the API deduplicates), on a thread of its own.
pub fn count_play(api: &str, id: &str) {
    let url = discover::plays_url(api, id);
    let _ = std::thread::Builder::new()
        .name("rustic-discover".into())
        .spawn(move || {
            if let Err(e) = agent().post(&url).call() {
                log::debug!("Rustic: play not counted ({url}): {e}");
            }
        });
}
