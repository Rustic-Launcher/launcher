//! The Discord presence's own pictures (ADR 0173): the server's favicon or the singleplayer world's
//! icon, which the companion sends on its presence line. Discord shows an image only from a URL, so
//! each picture is made a 64x64 PNG here, named by its SHA-256, and sent once to Rustic Cloud
//! (`POST /v1/me/presence-icons`), which re-encodes it and serves it from the CDN. What was sent is
//! remembered in `presence-icons.json` (picture hash → URL) and sent again after a week, which
//! keeps the cloud's copy from being cleared as unused.
//!
//! Nothing is sent when the presence is off, when the server's address is hidden (or private),
//! or when no Rustic account is signed in; the generic art asset shows then.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ImageEncoder, ImageFormat, ImageReader, Limits, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::cloud_link::ApiCloud;
use crate::discord::{Discord, Place};

/// The size the cloud stores and Discord shows (it scales the small image down anyway).
pub const SIZE: u32 = 64;
/// A picture sent this long ago is sent again (the cloud forgets pictures unused for 60 days).
pub const RESEND_AFTER_SECS: u64 = 7 * 24 * 3600;
/// Remembered pictures not used for this long are forgotten.
const FORGET_AFTER_SECS: u64 = 30 * 24 * 3600;
/// After a failed upload, the same picture waits this long before another try.
const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);

/// The picture as sent: a 64x64 RGBA PNG of its pixels only, and its SHA-256 (hex). `None` for
/// anything that isn't a PNG of 8 to 512 pixels a side.
pub fn prepare(png: &[u8]) -> Option<(Vec<u8>, String)> {
    let mut reader = ImageReader::with_format(Cursor::new(png), ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(512);
    limits.max_image_height = Some(512);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().ok()?;
    let (w, h) = (img.width(), img.height());
    if !(8..=512).contains(&w) || !(8..=512).contains(&h) {
        return None;
    }
    let rgba = fit(&img.to_rgba8());
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)
        .write_image(rgba.as_raw(), SIZE, SIZE, image::ExtendedColorType::Rgba8)
        .ok()?;
    let sha = hex(&Sha256::digest(&out));
    Some((out, sha))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `img` scaled to fit 64x64, its proportions kept, centred on a transparent square.
fn fit(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    if (w, h) == (SIZE, SIZE) {
        return img.clone();
    }
    let scale = f64::from(SIZE) / f64::from(w.max(h));
    let nw = ((f64::from(w) * scale).round() as u32).clamp(1, SIZE);
    let nh = ((f64::from(h) * scale).round() as u32).clamp(1, SIZE);
    let scaled = image::imageops::resize(img, nw, nh, image::imageops::FilterType::Lanczos3);
    let mut out = RgbaImage::new(SIZE, SIZE);
    image::imageops::overlay(
        &mut out,
        &scaled,
        i64::from((SIZE - nw) / 2),
        i64::from((SIZE - nh) / 2),
    );
    out
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    url: String,
    /// When it was last sent (unix seconds).
    sent: u64,
    /// When it was last shown (unix seconds).
    used: u64,
}

/// The pictures already on the CDN, by the hash of the picture as sent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sent {
    #[serde(default)]
    pictures: HashMap<String, Entry>,
}

impl Sent {
    fn load(file: &std::path::Path) -> Self {
        std::fs::read(file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, file: &std::path::Path) {
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let tmp = file.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, file);
            }
        }
    }

    /// Its URL when it was sent recently enough; marks it used.
    pub fn fresh(&mut self, sha: &str, now: u64) -> Option<String> {
        let e = self.pictures.get_mut(sha)?;
        if now.saturating_sub(e.sent) >= RESEND_AFTER_SECS {
            return None;
        }
        e.used = now;
        Some(e.url.clone())
    }

    pub fn record(&mut self, sha: &str, url: &str, now: u64) {
        self.pictures.insert(
            sha.to_owned(),
            Entry {
                url: url.to_owned(),
                sent: now,
                used: now,
            },
        );
        self.pictures
            .retain(|_, e| now.saturating_sub(e.used) < FORGET_AFTER_SECS);
    }
}

/// Only Rustic's own CDN URLs are shown (the cloud names them; anything else is ignored).
fn usable_url(url: &str) -> bool {
    url.starts_with("https://") && url.ends_with(".png") && !url.contains(char::is_whitespace)
}

/// The uploader: pictures in, URLs to the presence out, on its own task.
pub struct PresenceIcons {
    tx: mpsc::UnboundedSender<(Place, Vec<u8>)>,
}

impl PresenceIcons {
    pub fn start(
        cloud: ApiCloud,
        rustic: Arc<rusticmc_auth::RusticAccount>,
        discord: Discord,
        file: PathBuf,
    ) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<(Place, Vec<u8>)>();
        tauri::async_runtime::spawn(async move {
            let mut sent = Sent::load(&file);
            let mut failed: HashMap<String, Instant> = HashMap::new();
            while let Some((place, png)) = rx.recv().await {
                // The player may have changed their mind (or place) meanwhile.
                if !crate::discord::icon_allowed(&place, discord.options())
                    || discord.place().as_ref() != Some(&place)
                {
                    continue;
                }
                let Some((small, sha)) =
                    tauri::async_runtime::spawn_blocking(move || prepare(&png))
                        .await
                        .ok()
                        .flatten()
                else {
                    continue;
                };
                let now = rusticmc_launcher_core::time::now();
                if let Some(url) = sent.fresh(&sha, now) {
                    discord.set_icon(&place, url);
                    sent.save(&file);
                    continue;
                }
                if failed.get(&sha).is_some_and(|t| t.elapsed() < RETRY_AFTER) {
                    continue;
                }
                // Signed out of Rustic: the generic picture stays.
                let Ok((token, _)) = rustic.access_token_with_expiry().await else {
                    continue;
                };
                match cloud.presence_icon(&token, small).await {
                    Ok(icon) if usable_url(&icon.url) => {
                        failed.remove(&sha);
                        sent.record(&sha, &icon.url, now);
                        sent.save(&file);
                        discord.set_icon(&place, icon.url);
                    }
                    Ok(icon) => tracing::debug!(url = %icon.url, "presence icon URL refused"),
                    Err(e) => {
                        tracing::debug!(error = %e, "presence icon not sent");
                        failed.insert(sha, Instant::now());
                    }
                }
            }
        });
        Self { tx }
    }

    /// A picture the game sent for `place` (already checked against the player's settings).
    pub fn send(&self, place: Place, png: Vec<u8>) {
        let _ = self.tx.send((place, png));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn png(img: &RgbaImage) -> Vec<u8> {
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    fn decode(bytes: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .unwrap()
            .to_rgba8()
    }

    #[test]
    fn pictures_are_made_64_pixel_pngs_named_by_their_hash() {
        let mut fav = RgbaImage::new(64, 64);
        fav.put_pixel(1, 2, Rgba([255, 106, 0, 255]));
        let (out, sha) = prepare(&png(&fav)).unwrap();
        assert_eq!(sha.len(), 64);
        assert_eq!(decode(&out).get_pixel(1, 2), &Rgba([255, 106, 0, 255]));
        assert_eq!(
            prepare(&png(&fav)).unwrap().1,
            sha,
            "the same picture, the same name"
        );
        let big = RgbaImage::from_pixel(256, 128, Rgba([9, 9, 9, 255]));
        let small = decode(&prepare(&png(&big)).unwrap().0);
        assert_eq!(small.dimensions(), (64, 64));
        assert_eq!(small.get_pixel(32, 2)[3], 0, "proportions kept");
        assert_eq!(small.get_pixel(32, 32), &Rgba([9, 9, 9, 255]));
        assert!(prepare(b"\x89PNG\r\n\x1a\nnope").is_none());
        assert!(prepare(&png(&RgbaImage::new(4, 4))).is_none());
        assert!(prepare(&png(&RgbaImage::new(1024, 64))).is_none());
    }

    #[test]
    fn a_picture_is_sent_once_a_week_and_forgotten_after_a_month_unused() {
        let mut s = Sent::default();
        let t = 1_000_000;
        assert_eq!(s.fresh("a", t), None);
        s.record("a", "https://cdn.example/presence/a.png", t);
        assert_eq!(
            s.fresh("a", t + 60).as_deref(),
            Some("https://cdn.example/presence/a.png")
        );
        assert_eq!(s.fresh("a", t + RESEND_AFTER_SECS), None, "sent again");
        // "a" was last shown at t + 60 (the fresh look above).
        s.record(
            "b",
            "https://cdn.example/presence/b.png",
            t + 60 + FORGET_AFTER_SECS,
        );
        assert!(!s.pictures.contains_key("a"), "a month unused");
        assert!(s.pictures.contains_key("b"));
        // It survives a restart.
        let dir =
            std::env::temp_dir().join(format!("rustic-presence-icons-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("presence-icons.json");
        s.save(&file);
        assert_eq!(Sent::load(&file), s);
        assert_eq!(Sent::load(&dir.join("missing.json")), Sent::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_https_png_urls_are_shown() {
        assert!(usable_url(
            "https://cdn.rustic-launcher.com/presence/ab.png"
        ));
        assert!(!usable_url(
            "http://cdn.rustic-launcher.com/presence/ab.png"
        ));
        assert!(!usable_url("https://x/a.png b"));
        assert!(!usable_url("rustic_multiplayer"));
    }
}
