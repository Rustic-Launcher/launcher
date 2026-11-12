//! The Rustic badge (ADR 0072): the Rustic logo left of the names of the players who use Rustic,
//! in the player list, over their heads and before their chat lines, in the place of Horizon's
//! mark (one badge only: Rustic's for Rustic users; Horizon's own mark stays for Horizon-only
//! players; none for the others).
//!
//! - **Who**: the cloud says which of the UUIDs the game sees are linked to an active Rustic
//!   account ([`BadgeCache`], asked in batches of [`MAX_BATCH`] off the render thread, cached for
//!   [`TTL_RUSTIC`] / [`TTL_OTHER`], at most one request in flight and one every [`MIN_GAP`],
//!   backing off after failures). Offline nobody gets one, the window's own player included
//!   unless the Rustic session says it is them ([`shows_badge`]).
//! - **The picture**: the official mark, set at the exact screen size of the game's text
//!   ([`badge_image`]), then laid into the 64x64 sprite the game's hooks take so that the game's
//!   nearest sampling of the whole sprite into an 8x8 GUI box gives back that render pixel for
//!   pixel ([`sprite_pixels`]).
use crate::backoff::Backoff;
use crate::title::{Rgba, resample};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The largest batch the lookup takes (`POST /v1/public/minecraft/lookup`: 100 UUIDs).
pub const MAX_BATCH: usize = 100;
/// How long "uses Rustic" holds before it is asked again.
pub const TTL_RUSTIC: Duration = Duration::from_secs(15 * 60);
/// How long "does not use Rustic" holds (shorter: a player may link an account meanwhile).
pub const TTL_OTHER: Duration = Duration::from_secs(5 * 60);
/// At least this long between two requests (the lookup allows 120 a minute per address).
pub const MIN_GAP: Duration = Duration::from_secs(2);
/// Players remembered at most (least recently seen go first).
pub const MAX_PLAYERS: usize = 4096;

/// The badge's sprite id in the game's `hooks/<hex>` atlas names: under the companion's prefix
/// ([`crate::store::SPRITE_BASE`]), far above the cosmetics' own ids.
pub const SPRITE: u64 = crate::store::SPRITE_BASE | 0xFFFF_FF00;
/// The badge's sprite as the GUI atlas names it (`hooks/<id in hex>`): what an inline sprite of
/// a chat line names.
pub fn sprite_name() -> String {
    format!("hooks/{SPRITE:x}")
}

/// The sprites the game's hooks take are this many pixels square.
pub const SPRITE_SIDE: u32 = 64;
/// The badge's box in GUI pixels (the game draws a mark, or an inline sprite, 8 square).
pub const BOX: u32 = 8;

#[derive(Clone, Copy, Debug)]
struct Known {
    rustic: bool,
    expires: Instant,
    seen: Instant,
}

/// Who uses Rustic, as the cloud last said.
#[derive(Debug, Default)]
pub struct BadgeCache {
    known: HashMap<u128, Known>,
    in_flight: Vec<u128>,
    last_sent: Option<Instant>,
    backoff: Backoff,
}

impl BadgeCache {
    /// Whether the cloud said `uuid` uses Rustic (the last answer holds while a new one is asked).
    pub fn is_rustic(&self, uuid: u128) -> bool {
        self.known.get(&uuid).is_some_and(|k| k.rustic)
    }

    /// Whether the cloud answered for `uuid` at all.
    pub fn knows(&self, uuid: u128) -> bool {
        self.known.contains_key(&uuid)
    }

    /// The next batch to ask for among `wanted` (the listed players, those in view, chat
    /// senders): never asked or past their TTL, at most [`MAX_BATCH`], none while one is in
    /// flight, within [`MIN_GAP`] of the last or while backing off. Marks them in flight.
    pub fn next_batch(&mut self, wanted: &[u128], now: Instant) -> Vec<u128> {
        for u in wanted {
            if let Some(k) = self.known.get_mut(u) {
                k.seen = now;
            }
        }
        let gap = self
            .last_sent
            .is_some_and(|t| now.saturating_duration_since(t) < MIN_GAP);
        if gap || !self.in_flight.is_empty() || !self.backoff.ready(now) {
            return Vec::new();
        }
        let mut batch: Vec<u128> = Vec::new();
        for &u in wanted {
            let due = self.known.get(&u).is_none_or(|k| now >= k.expires);
            if u != 0 && due && !batch.contains(&u) {
                batch.push(u);
                if batch.len() == MAX_BATCH {
                    break;
                }
            }
        }
        if !batch.is_empty() {
            self.last_sent = Some(now);
            self.in_flight = batch.clone();
        }
        batch
    }

    /// The cloud answered for the batch in flight: `rustic` holds those of them who use Rustic
    /// (the others in the batch do not).
    pub fn answered(&mut self, rustic: &[u128], now: Instant) {
        self.backoff.succeeded();
        for uuid in std::mem::take(&mut self.in_flight) {
            let yes = rustic.contains(&uuid);
            let ttl = if yes { TTL_RUSTIC } else { TTL_OTHER };
            self.known.insert(
                uuid,
                Known {
                    rustic: yes,
                    expires: now + ttl,
                    seen: now,
                },
            );
        }
        self.evict();
    }

    /// The batch in flight failed: what was known stays; wait before asking again.
    pub fn failed(&mut self, now: Instant) {
        self.in_flight.clear();
        self.backoff.failed(now);
    }

    fn evict(&mut self) {
        if self.known.len() <= MAX_PLAYERS {
            return;
        }
        let mut by_age: Vec<(u128, Instant)> =
            self.known.iter().map(|(u, k)| (*u, k.seen)).collect();
        by_age.sort_by_key(|(_, seen)| *seen);
        for (u, _) in by_age.into_iter().take(self.known.len() - MAX_PLAYERS) {
            self.known.remove(&u);
        }
    }

    pub fn len(&self) -> usize {
        self.known.len()
    }

    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }
}

/// Who gets the badge: the window's own player when the Rustic session is theirs (`own`: the
/// session's UUID while signed in), anyone the cloud says uses Rustic. Nobody else, and nobody
/// without an answer (offline: no badge).
pub fn shows_badge(uuid: u128, own: Option<u128>, cache: &BadgeCache) -> bool {
    uuid != 0 && (own == Some(uuid) || cache.is_rustic(uuid))
}

/// The side of the badge's render in screen pixels at GUI scale `scale`: the 8 GUI pixel box,
/// within what a 64-pixel sprite holds (GUI scale 1 to 8).
pub fn side_for(scale: f32) -> u32 {
    let s = if scale.is_finite() { scale } else { 1.0 };
    ((BOX as f32 * s).round() as u32).clamp(BOX, SPRITE_SIDE)
}

/// Where the logo goes in a `side`-pixel render of the 8x8 GUI box (`[x, y, w, h]`, screen
/// pixels): within the text's cap height (GUI rows 1 to 7 of the box, the box's top row being a
/// pixel above the glyphs), its last column left empty so a pixel separates it from the name,
/// a screen pixel in from each edge for its outline, its proportions (`aspect`: width over
/// height) kept, centred in the box both ways (as Horizon's mark sits in the row).
pub fn logo_rect(side: u32, aspect: f32) -> [u32; 4] {
    let u = side as f32 / BOX as f32;
    let (bx, by, bw, bh) = (1.0, u + 1.0, 7.0 * u - 2.0, 7.0 * u - 2.0);
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    let (w, h) = if aspect >= bw / bh {
        (bw, bw / aspect)
    } else {
        (bh * aspect, bh)
    };
    let (w, h) = (w.round().max(1.0), h.round().max(1.0));
    let x = (bx + (bw - w) / 2.0).round();
    // Centred on the whole box's height (its middle is the row's), kept inside the outline.
    let y = ((side as f32 - h) / 2.0)
        .round()
        .clamp(1.0, (side as f32 - 1.0 - h).max(1.0));
    [x as u32, y as u32, w as u32, h as u32]
}

/// The mark alone, from the wordmark (`assets/rustic-wordmark.png`: the mark, a gap, "Rustic"):
/// the columns up to the first empty one after the mark, trimmed to what is drawn. None when
/// nothing is drawn.
pub fn mark_of_wordmark(wordmark: &Rgba) -> Option<Rgba> {
    let alpha = |x: u32, y: u32| wordmark.pixels[((y * wordmark.width + x) * 4 + 3) as usize];
    let column_drawn = |x: u32| (0..wordmark.height).any(|y| alpha(x, y) > 0);
    let first = (0..wordmark.width).find(|&x| column_drawn(x))?;
    let end = (first..wordmark.width)
        .find(|&x| !column_drawn(x))
        .unwrap_or(wordmark.width);
    let row_drawn = |y: u32| (first..end).any(|x| alpha(x, y) > 0);
    let top = (0..wordmark.height).find(|&y| row_drawn(y))?;
    let bottom = (top..wordmark.height)
        .rev()
        .find(|&y| row_drawn(y))
        .map_or(wordmark.height, |y| y + 1);
    Some(wordmark.crop([first, top, end - first, bottom - top]))
}

/// The badge at `side` screen pixels: the mark resampled smooth into [`logo_rect`], over a
/// subtle dark outline a screen pixel wide so it reads on any background (the game adds the
/// text's own shadow in the player list and chat).
pub fn badge_image(mark: &Rgba, side: u32) -> Rgba {
    let mut out = Rgba::new(side, side);
    let aspect = mark.width as f32 / mark.height.max(1) as f32;
    let [x, y, w, h] = logo_rect(side, aspect);
    let logo = resample(mark, w, h);
    // The outline: the logo's alpha spread a pixel round, black at 45 %.
    let mut halo = Rgba::new(side, side);
    for ly in 0..h {
        for lx in 0..w {
            let a = logo.pixels[((ly * w + lx) * 4 + 3) as usize];
            if a == 0 {
                continue;
            }
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (hx, hy) = (x as i32 + lx as i32 + dx, y as i32 + ly as i32 + dy);
                if hx < 0 || hy < 0 || hx >= side as i32 || hy >= side as i32 {
                    continue;
                }
                let i = ((hy as u32 * side + hx as u32) * 4 + 3) as usize;
                halo.pixels[i] = halo.pixels[i].max((u32::from(a) * 115 / 255) as u8);
            }
        }
    }
    out.draw_over(&halo, 0, 0);
    out.draw_over(&logo, x, y);
    out
}

/// `image` (`side` square, at most [`SPRITE_SIDE`]) laid into a 64x64 sprite so that sampling
/// the nearest texel of the whole sprite drawn over `side` screen pixels finds pixel for pixel
/// `image` again: texel `t` holds the pixel under its centre, `(t + 0.5) * side / 64`.
pub fn sprite_pixels(image: &Rgba) -> Vec<u8> {
    let n = SPRITE_SIDE;
    let mut out = vec![0u8; (n * n * 4) as usize];
    let (w, h) = (image.width.max(1), image.height.max(1));
    for ty in 0..n {
        let sy = (((ty as f32 + 0.5) * h as f32 / n as f32) as u32).min(h - 1);
        for tx in 0..n {
            let sx = (((tx as f32 + 0.5) * w as f32 / n as f32) as u32).min(w - 1);
            let s = ((sy * image.width + sx) * 4) as usize;
            let d = ((ty * n + tx) * 4) as usize;
            if s + 4 <= image.pixels.len() {
                out[d..d + 4].copy_from_slice(&image.pixels[s..s + 4]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_in_batches_one_at_a_time_and_keeps_answers() {
        let t = Instant::now();
        let mut c = BadgeCache::default();
        let wanted: Vec<u128> = (1..=130).collect();
        let b = c.next_batch(&wanted, t);
        assert_eq!(b.len(), MAX_BATCH);
        // One in flight at a time.
        assert!(c.next_batch(&wanted, t + MIN_GAP).is_empty());
        c.answered(&[1, 7], t);
        assert!(c.is_rustic(1) && c.is_rustic(7));
        assert!(!c.is_rustic(2) && c.knows(2));
        assert!(!c.knows(101));
        // The rest, but not within the minimum gap.
        assert!(
            c.next_batch(&wanted, t + Duration::from_millis(500))
                .is_empty()
        );
        let rest = c.next_batch(&wanted, t + MIN_GAP);
        assert_eq!(rest, (101..=130).collect::<Vec<_>>());
        c.answered(&[], t + MIN_GAP);
        // Nobody until a TTL ends: "no" first, "yes" later.
        let later = t + MIN_GAP * 2;
        assert!(c.next_batch(&wanted, later).is_empty());
        let others = c.next_batch(&wanted, t + TTL_OTHER + MIN_GAP);
        assert_eq!(others.len(), MAX_BATCH);
        assert!(!others.contains(&1) && !others.contains(&7));
        // A failure keeps what was known and backs off.
        c.failed(t + TTL_OTHER + MIN_GAP);
        assert!(c.is_rustic(1));
        assert!(
            c.next_batch(&wanted, t + TTL_OTHER + MIN_GAP * 3)
                .is_empty()
        );
        assert!(
            !c.next_batch(&wanted, t + TTL_OTHER + Duration::from_secs(60))
                .is_empty()
        );
        // Duplicates and the nil UUID are never asked.
        let mut c = BadgeCache::default();
        assert_eq!(c.next_batch(&[0, 5, 5, 6], t), vec![5, 6]);
    }

    #[test]
    fn forgets_the_least_recently_seen_beyond_the_limit() {
        let t = Instant::now();
        let mut c = BadgeCache::default();
        let mut at = t;
        let mut n = 1u128;
        while c.len() < MAX_PLAYERS {
            let batch: Vec<u128> = (n..n + MAX_BATCH as u128).collect();
            assert_eq!(c.next_batch(&batch, at).len(), MAX_BATCH);
            c.answered(&batch, at);
            n += MAX_BATCH as u128;
            at += MIN_GAP;
        }
        let newest: Vec<u128> = (n..n + 10).collect();
        c.next_batch(&newest, at);
        c.answered(&[], at);
        assert_eq!(c.len(), MAX_PLAYERS);
        // 4100 then 4110 known: the 14 forgotten were all of the first (oldest) batch.
        assert_eq!((1..=100).filter(|u| c.knows(*u)).count(), 86);
        assert!(newest.iter().all(|u| c.knows(*u)));
    }

    #[test]
    fn who_gets_the_badge() {
        let t = Instant::now();
        let mut c = BadgeCache::default();
        c.next_batch(&[2, 3], t);
        c.answered(&[2], t);
        // Rustic users by the cloud's answer; the own player by the session, even unasked.
        assert!(shows_badge(2, None, &c));
        assert!(!shows_badge(3, None, &c));
        assert!(shows_badge(9, Some(9), &c));
        assert!(!shows_badge(9, None, &c));
        assert!(!shows_badge(4, Some(9), &c));
        assert!(!shows_badge(0, Some(0), &c));
        // Offline: nothing answered, nobody but a signed-in own player.
        let offline = BadgeCache::default();
        assert!(!shows_badge(2, None, &offline));
    }

    #[test]
    fn the_sprite_is_named_as_the_game_names_hook_sprites() {
        assert_eq!(sprite_name(), "hooks/52555354ffffff00");
    }

    #[test]
    fn sides_follow_the_gui_scale_within_a_sprite() {
        assert_eq!(side_for(1.0), 8);
        assert_eq!(side_for(2.0), 16);
        assert_eq!(side_for(3.0), 24);
        assert_eq!(side_for(4.0), 32);
        assert_eq!(side_for(8.0), 64);
        assert_eq!(side_for(12.0), 64);
        assert_eq!(side_for(0.5), 8);
        assert_eq!(side_for(f32::NAN), 8);
    }

    #[test]
    fn the_logo_is_centred_with_a_pixel_before_the_name() {
        // The mark is wider than high (458 x 394 in its SVG).
        let aspect = 458.0 / 394.0;
        for scale in 1..=8u32 {
            let side = side_for(scale as f32);
            let u = scale;
            let [x, y, w, h] = logo_rect(side, aspect);
            // Inside the box, its last GUI column (the gap) and top row left free.
            assert!(x + w <= 7 * u, "scale {scale}: {x}+{w}");
            assert!(y >= 1, "scale {scale}: top {y}");
            assert!(y + h < 8 * u, "scale {scale}: bottom {y}+{h}");
            // Centred in the box: its middle on the box's middle, to a screen pixel.
            let middle = 2 * y + h;
            assert!(
                middle.abs_diff(side) <= 1,
                "scale {scale}: middle {middle} of {side}"
            );
            // As large as the box allows: width-bound for this aspect.
            assert!(w + 2 >= 7 * u - 1, "scale {scale}: width {w}");
            assert!((w as f32 / h as f32 - aspect).abs() < 0.35, "scale {scale}");
        }
        // A tall picture is height-bound.
        let [_, _, w, h] = logo_rect(32, 0.5);
        assert_eq!(h, 26);
        assert_eq!(w, 13);
    }

    fn sample(sprite: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
        // The GPU's nearest sampling of a 64-texel sprite stretched over `side` pixels.
        let t = |p: u32| (((p as f32 + 0.5) * SPRITE_SIDE as f32 / side as f32) as u32).min(63);
        let i = ((t(y) * SPRITE_SIDE + t(x)) * 4) as usize;
        sprite[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn the_sprite_gives_back_the_render_pixel_for_pixel() {
        for side in (8..=64).step_by(8).chain([12, 20, 36]) {
            let mut im = Rgba::new(side, side);
            for (i, p) in im.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                *p = [
                    (i % 251) as u8,
                    (i / 7 % 253) as u8,
                    (i * 13 % 255) as u8,
                    255,
                ];
            }
            let sprite = sprite_pixels(&im);
            assert_eq!(sprite.len(), 64 * 64 * 4);
            for y in 0..side {
                for x in 0..side {
                    let i = ((y * side + x) * 4) as usize;
                    assert_eq!(
                        sample(&sprite, side, x, y),
                        im.pixels[i..i + 4],
                        "side {side} at {x},{y}"
                    );
                }
            }
        }
    }

    /// A wordmark: an orange and white "mark" 30 x 20, a gap, then "text".
    fn wordmark() -> Rgba {
        let mut im = Rgba::new(80, 24);
        for y in 2..22 {
            for x in 3..33 {
                let i = ((y * 80 + x) * 4) as usize;
                let c = if y > 15 && x < 12 {
                    [255, 106, 0, 255]
                } else {
                    [255, 255, 255, 255]
                };
                im.pixels[i..i + 4].copy_from_slice(&c);
            }
            for x in 40..78 {
                let i = ((y * 80 + x) * 4) as usize;
                im.pixels[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        im
    }

    #[test]
    fn the_mark_comes_out_of_the_wordmark() {
        let mark = mark_of_wordmark(&wordmark()).unwrap();
        assert_eq!((mark.width, mark.height), (30, 20));
        assert!(mark_of_wordmark(&Rgba::new(10, 10)).is_none());
    }

    #[test]
    fn the_badge_is_the_mark_with_an_outline_and_a_free_last_column() {
        let mark = mark_of_wordmark(&wordmark()).unwrap();
        for side in [8, 16, 24, 32, 64] {
            let b = badge_image(&mark, side);
            assert_eq!((b.width, b.height), (side, side));
            let at = |x: u32, y: u32| {
                let i = ((y * side + x) * 4) as usize;
                [
                    b.pixels[i],
                    b.pixels[i + 1],
                    b.pixels[i + 2],
                    b.pixels[i + 3],
                ]
            };
            let u = side / BOX;
            // The gap column and the row above the glyphs stay clear.
            for y in 0..side {
                for x in 7 * u..side {
                    assert_eq!(at(x, y)[3], 0, "side {side} gap at {x},{y}");
                }
            }
            for x in 0..side {
                for y in 0..u {
                    assert_eq!(at(x, y)[3], 0, "side {side} top at {x},{y}");
                }
            }
            let pixels: Vec<[u8; 4]> = (0..side * side).map(|i| at(i % side, i / side)).collect();
            // White and orange of the mark, and the dark outline round it.
            assert!(
                pixels
                    .iter()
                    .any(|p| p[3] == 255 && p[0] > 240 && p[2] > 240)
            );
            assert!(pixels.iter().any(|p| p[3] > 200 && p[0] > 200 && p[2] < 80));
            if side >= 16 {
                assert!(pixels.iter().any(|p| p[3] > 0 && p[0] < 40 && p[1] < 40));
            }
        }
    }
}
