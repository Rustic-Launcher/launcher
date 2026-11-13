//! The companion's canvas: everything Rustic draws in the game's GUI that is not a flat colour —
//! smooth text set in Inter (SIL OFL 1.1, `assets/`), the round corners of its plates, its
//! pictures (the logo, icons, flags, server icons) — is rasterised here, at the screen's exact
//! pixel size, into the HUD atlas's mod slot (`horizon_client::hud::MOD_IMAGE_SPRITE`, the slot a
//! native mod may fill), and drawn from there one atlas pixel to one screen pixel.
//!
//! Pieces are packed on shelves as they are first drawn and uploaded with the frame's own batch
//! (`HudBatch::patches`, as the game uploads its glyphs), so a new glyph shows in the frame that
//! asks for it. When the slot is full, the frame draws what it has and the next starts over; a new
//! atlas (every resource reload) starts over too ([`Canvas::begin`], from the widget skin's
//! `begin`, Rustic's Horizon patch U15).
use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont, point};
use horizon_client::font::Patch;
use horizon_client::hud::{HudAtlas, HudBatch, Rect};
use rusticmc_companion_core::paint::{Align, Paint, Picture};
use rusticmc_companion_core::title::{self, Glyph, Image, Rgba, Weight};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

/// Inter Medium and SemiBold (`assets/make_fonts.py`).
pub(crate) const INTER_MEDIUM: &[u8] = include_bytes!("../assets/Inter-Medium.ttf");
pub(crate) const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");
static MEDIUM: LazyLock<Option<FontRef<'static>>> =
    LazyLock::new(|| FontRef::try_from_slice(INTER_MEDIUM).ok());
static SEMIBOLD: LazyLock<Option<FontRef<'static>>> =
    LazyLock::new(|| FontRef::try_from_slice(INTER_SEMIBOLD).ok());

/// The logo, the official banner and the icon strip (launcher/assets/brand, `assets/make_assets.py`).
pub(crate) static LOGO: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-wordmark.png")));
static BANNER: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-banner.png")));
static ICONS: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-icons.png")));

/// The banner's width over its height.
pub fn banner_aspect() -> f32 {
    BANNER
        .as_ref()
        .map_or(4.0, |l| l.width as f32 / l.height as f32)
}

pub(crate) fn decode_png(bytes: &[u8]) -> Option<Rgba> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Rgba {
        width: info.width,
        height: info.height,
        pixels,
    })
}

fn font(weight: Weight) -> Option<&'static FontRef<'static>> {
    match weight {
        Weight::Medium => MEDIUM.as_ref(),
        Weight::SemiBold => SEMIBOLD.as_ref(),
    }
}

/// ab_glyph's scale is the line's height: Inter's em is that over its line height.
fn scale_of(px: f32) -> PxScale {
    PxScale::from(px * title::LINE)
}

/// A line's width in screen pixels at an em of `px` screen pixels.
pub fn measure_px(text: &str, px: f32, weight: Weight) -> f32 {
    let Some(font) = font(weight) else {
        return text.chars().count() as f32 * px * 0.55;
    };
    let sf = font.as_scaled(scale_of(px));
    let mut width = 0.0;
    let mut prev = None;
    for c in text.chars() {
        let id = sf.glyph_id(c);
        if let Some(p) = prev {
            width += sf.kern(p, id);
        }
        width += sf.h_advance(id);
        prev = Some(id);
    }
    width
}

/// The ARGB colour as the HUD's vertices take it (ABGR).
pub fn abgr(argb: u32) -> u32 {
    let [a, r, g, b] = argb.to_be_bytes();
    u32::from_be_bytes([a, b, g, r])
}

/// Where the mod slot is in the atlas.
#[derive(Clone, Copy, Debug)]
struct Slot {
    /// The atlas's identity (a new one comes with every resource reload).
    id: usize,
    atlas: [u32; 2],
    origin: [u32; 2],
    size: [u32; 2],
    white: Rect,
}

/// A piece of the canvas.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    Glyph(Weight, u16, u16),
    /// A quarter disc (`false`) or a one-pixel quarter ring of this radius in pixels.
    Corner(u16, bool),
    Picture(Picture, u32, u32),
}

#[derive(Clone, Copy, Debug)]
struct Placed {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    /// A glyph's bitmap offset from the pen.
    left: f32,
    top: f32,
}

#[derive(Default)]
pub struct Canvas {
    slot: Option<Slot>,
    shelf: (u32, u32, u32),
    placed: HashMap<Key, Option<Placed>>,
    /// The slot filled up this frame: start over at the next.
    full: bool,
    /// Pictures downloaded from Rustic's CDN, decoded, by URL.
    pub remote: HashMap<String, Arc<Rgba>>,
    /// The slider track drawn last (its handle fills it up to itself).
    pub last_track: Option<[f32; 4]>,
    /// The open clip: the batch's vertex count at its start, and its rectangle.
    clip: Option<(usize, [f32; 4])>,
}

static CANVAS: LazyLock<Mutex<Canvas>> = LazyLock::new(|| Mutex::new(Canvas::default()));

/// The canvas (never held across a game call that may draw widgets).
pub fn canvas() -> MutexGuard<'static, Canvas> {
    CANVAS.lock().unwrap_or_else(|e| e.into_inner())
}

impl Canvas {
    /// Once a frame, with the atlas the frame draws from.
    pub fn begin(&mut self, atlas: &HudAtlas) {
        let slot = atlas
            .sprites
            .get(horizon_client::hud::MOD_IMAGE_SPRITE)
            .map(|r| Slot {
                id: std::ptr::from_ref(atlas) as usize,
                atlas: [atlas.width, atlas.height],
                origin: [
                    (r.u0 * atlas.width as f32).round() as u32,
                    (r.v0 * atlas.height as f32).round() as u32,
                ],
                size: [r.w, r.h],
                white: atlas.font.white(),
            });
        let key = |s: &Option<Slot>| s.map(|s| (s.id, s.atlas, s.origin, s.size));
        if key(&slot) != key(&self.slot) || self.full {
            self.placed.clear();
            self.shelf = (0, 0, 0);
            self.full = false;
        }
        self.slot = slot;
        self.last_track = None;
        self.clip = None;
    }

    /// A new atlas came with the resources: everything goes up again.
    pub fn forget(&mut self) {
        self.slot = None;
        self.placed.clear();
        self.shelf = (0, 0, 0);
    }

    /// Whether the canvas has a slot to draw from.
    pub fn ready(&self) -> bool {
        self.slot.is_some()
    }

    /// Room for `w` x `h` pixels in the slot, a pixel apart from the rest.
    fn room(&mut self, w: u32, h: u32) -> Option<[u32; 2]> {
        let slot = self.slot?;
        let (mut x, mut y, mut row) = self.shelf;
        if w > slot.size[0] || h > slot.size[1] {
            return None;
        }
        if x + w > slot.size[0] {
            x = 0;
            y += row + 1;
            row = 0;
        }
        if y + h > slot.size[1] {
            self.full = true;
            return None;
        }
        self.shelf = (x + w + 1, y, row.max(h));
        Some([x, y])
    }

    /// Puts `rgba` (`w` x `h`) in the slot, uploaded with `batch`.
    fn upload(&mut self, batch: &mut HudBatch, w: u32, h: u32, rgba: Vec<u8>) -> Option<[u32; 2]> {
        let at = self.room(w, h)?;
        let slot = self.slot?;
        batch.patches.push(Patch {
            x: slot.origin[0] + at[0],
            y: slot.origin[1] + at[1],
            width: w,
            height: h,
            rgba,
        });
        Some(at)
    }

    /// The atlas rectangle of a placed piece, its corners swapped to flip it.
    fn uv(&self, p: &Placed, flip_x: bool, flip_y: bool) -> Option<Rect> {
        let slot = self.slot?;
        let (aw, ah) = (slot.atlas[0] as f32, slot.atlas[1] as f32);
        let (x0, y0) = (
            (slot.origin[0] + p.x) as f32 / aw,
            (slot.origin[1] + p.y) as f32 / ah,
        );
        let (x1, y1) = (
            (slot.origin[0] + p.x + p.w) as f32 / aw,
            (slot.origin[1] + p.y + p.h) as f32 / ah,
        );
        let (u0, u1) = if flip_x { (x1, x0) } else { (x0, x1) };
        let (v0, v1) = if flip_y { (y1, y0) } else { (y0, y1) };
        Some(Rect {
            u0,
            v0,
            u1,
            v1,
            w: p.w,
            h: p.h,
        })
    }

    /// A piece, rasterised and uploaded the first time it is asked for.
    fn piece(&mut self, batch: &mut HudBatch, key: &Key) -> Option<Placed> {
        if let Some(p) = self.placed.get(key) {
            return *p;
        }
        if self.full {
            return None;
        }
        let made = match key {
            Key::Glyph(weight, id, quarter) => {
                let font = font(*weight)?;
                let px = f32::from(*quarter) / 4.0;
                let glyph = GlyphId(*id).with_scale_and_position(scale_of(px), point(0.0, 0.0));
                match font.outline_glyph(glyph) {
                    Some(outline) => {
                        let b = outline.px_bounds();
                        let (w, h) = (b.width() as u32, b.height() as u32);
                        let mut alpha = vec![0f32; (w * h) as usize];
                        outline.draw(|x, y, c| {
                            if x < w && y < h {
                                alpha[(y * w + x) as usize] = c;
                            }
                        });
                        let rgba = alpha
                            .iter()
                            // Light text on dark plates reads thin: a gentle boost.
                            .flat_map(|a| [255, 255, 255, (a.powf(0.85) * 255.0).round() as u8])
                            .collect();
                        let at = self.upload(batch, w, h, rgba);
                        at.map(|[x, y]| Placed {
                            x,
                            y,
                            w,
                            h,
                            left: b.min.x,
                            top: b.min.y,
                        })
                    }
                    // A space: nothing to draw, never to ask again.
                    None => {
                        self.placed.insert(key.clone(), None);
                        return None;
                    }
                }
            }
            Key::Corner(r, ring) => {
                let n = u32::from(*r);
                let rgba = corner_mask(n, *ring);
                self.upload(batch, n, n, rgba).map(|[x, y]| Placed {
                    x,
                    y,
                    w: n,
                    h: n,
                    left: 0.0,
                    top: 0.0,
                })
            }
            Key::Picture(picture, w, h) => {
                let im = self.render_picture(picture, *w, *h)?;
                let (w, h) = (im.width, im.height);
                self.upload(batch, w, h, im.pixels).map(|[x, y]| Placed {
                    x,
                    y,
                    w,
                    h,
                    left: 0.0,
                    top: 0.0,
                })
            }
        };
        if made.is_some() {
            self.placed.insert(key.clone(), made);
        }
        made
    }

    fn render_picture(&self, picture: &Picture, w: u32, h: u32) -> Option<Rgba> {
        let side = Glyph::SIDE;
        let glyph = |g: Glyph| {
            ICONS
                .as_ref()
                .map(|s| s.crop([g.index() * side, 0, side, side]))
        };
        match picture {
            Picture::Title(Image::Logo) => BANNER.as_ref().map(|l| title::resample(l, w, h)),
            Picture::Title(Image::Icon(g, rgb)) => {
                glyph(*g).map(|im| title::resample(&im, w, h).tinted(*rgb))
            }
            Picture::Title(Image::Tile { hue, glyph: g }) => {
                glyph(*g).map(|im| title::tile(*hue, w, w as f32 * 0.14, &im))
            }
            Picture::Flag(code) => Some(rusticmc_companion_core::flags::render(
                code,
                w,
                h,
                (h as f32 * 0.14).max(1.0),
            )),
            Picture::Remote(url) => self.remote.get(url).map(|im| title::resample(im, w, h)),
        }
    }

    fn scale(batch: &HudBatch) -> f32 {
        batch.scale.max(1.0)
    }

    /// A rectangle in screen pixels, in `color` (ABGR).
    fn rect_px(&self, batch: &mut HudBatch, [x, y, w, h]: [i32; 4], color: u32) {
        let Some(slot) = self.slot else {
            return;
        };
        if w <= 0 || h <= 0 {
            return;
        }
        let s = Self::scale(batch);
        let mut r = slot.white;
        let (cu, cv) = ((r.u0 + r.u1) / 2.0, (r.v0 + r.v1) / 2.0);
        (r.u0, r.u1, r.v0, r.v1) = (cu, cu, cv, cv);
        batch.rect_uv(
            x as f32 / s,
            y as f32 / s,
            w as f32 / s,
            h as f32 / s,
            r,
            color,
        );
    }

    /// A corner mask `r` pixels square at `(x, y)`, flipped to the corner it is.
    #[allow(clippy::too_many_arguments)]
    fn corner_px(
        &mut self,
        batch: &mut HudBatch,
        x: i32,
        y: i32,
        r: i32,
        ring: bool,
        flip: (bool, bool),
        color: u32,
    ) {
        if r <= 0 {
            return;
        }
        let Some(p) = self.piece(batch, &Key::Corner(r as u16, ring)) else {
            return;
        };
        let Some(uv) = self.uv(&p, flip.0, flip.1) else {
            return;
        };
        let s = Self::scale(batch);
        batch.rect_uv(
            x as f32 / s,
            y as f32 / s,
            r as f32 / s,
            r as f32 / s,
            uv,
            color,
        );
    }

    /// A box with round corners of `r` pixels, filled in `fill`, `border` around it a pixel
    /// thick (ABGR; alpha 0 for none), all in screen pixels, no pixel covered twice.
    #[allow(clippy::too_many_arguments)]
    fn round_px(
        &mut self,
        batch: &mut HudBatch,
        [x, y, w, h]: [i32; 4],
        r: i32,
        fill: u32,
        border: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.min(w / 2).min(h / 2).max(0);
        let has_border = border >> 24 != 0;
        // The fill's own box and radius.
        let (fx, fy, fw, fh, fr) = if has_border {
            (x + 1, y + 1, w - 2, h - 2, (r - 1).max(0))
        } else {
            (x, y, w, h, r)
        };
        if fill >> 24 != 0 && fw > 0 && fh > 0 {
            if fr == 0 {
                self.rect_px(batch, [fx, fy, fw, fh], fill);
            } else {
                self.rect_px(batch, [fx + fr, fy, fw - 2 * fr, fh], fill);
                self.rect_px(batch, [fx, fy + fr, fr, fh - 2 * fr], fill);
                self.rect_px(batch, [fx + fw - fr, fy + fr, fr, fh - 2 * fr], fill);
                for (cx, cy, flip) in [
                    (fx, fy, (false, false)),
                    (fx + fw - fr, fy, (true, false)),
                    (fx, fy + fh - fr, (false, true)),
                    (fx + fw - fr, fy + fh - fr, (true, true)),
                ] {
                    self.corner_px(batch, cx, cy, fr, false, flip, fill);
                }
            }
        }
        if has_border {
            if r <= 1 {
                self.rect_px(batch, [x, y, w, 1], border);
                self.rect_px(batch, [x, y + h - 1, w, 1], border);
                self.rect_px(batch, [x, y + 1, 1, h - 2], border);
                self.rect_px(batch, [x + w - 1, y + 1, 1, h - 2], border);
            } else {
                self.rect_px(batch, [x + r, y, w - 2 * r, 1], border);
                self.rect_px(batch, [x + r, y + h - 1, w - 2 * r, 1], border);
                self.rect_px(batch, [x, y + r, 1, h - 2 * r], border);
                self.rect_px(batch, [x + w - 1, y + r, 1, h - 2 * r], border);
                for (cx, cy, flip) in [
                    (x, y, (false, false)),
                    (x + w - r, y, (true, false)),
                    (x, y + h - r, (false, true)),
                    (x + w - r, y + h - r, (true, true)),
                ] {
                    self.corner_px(batch, cx, cy, r, true, flip, border);
                }
            }
        }
    }

    /// GUI pixels to whole screen pixels.
    fn px_rect(batch: &HudBatch, [x, y, w, h]: [f32; 4]) -> [i32; 4] {
        let s = Self::scale(batch);
        let (x0, y0) = ((x * s).round() as i32, (y * s).round() as i32);
        let (x1, y1) = (((x + w) * s).round() as i32, ((y + h) * s).round() as i32);
        [x0, y0, x1 - x0, y1 - y0]
    }

    /// Whether `r` (GUI pixels) shows inside the open clip.
    fn visible(&self, r: [f32; 4]) -> bool {
        match self.clip {
            Some((_, c)) => {
                r[0] < c[0] + c[2] && r[0] + r[2] > c[0] && r[1] < c[1] + c[3] && r[1] + r[3] > c[1]
            }
            None => true,
        }
    }

    pub fn fill(&mut self, batch: &mut HudBatch, rect: [f32; 4], argb: u32) {
        if !self.visible(rect) {
            return;
        }
        let r = Self::px_rect(batch, rect);
        self.rect_px(batch, r, abgr(argb));
    }

    pub fn round(
        &mut self,
        batch: &mut HudBatch,
        rect: [f32; 4],
        radius: f32,
        fill: u32,
        border: u32,
    ) {
        if !self.visible(rect) {
            return;
        }
        let r = Self::px_rect(batch, rect);
        let radius = (radius * Self::scale(batch)).round() as i32;
        self.round_px(batch, r, radius, abgr(fill), abgr(border));
    }

    /// A rectangle shading from `left` to `right` (ARGB), left to right.
    pub fn gradient(&mut self, batch: &mut HudBatch, rect: [f32; 4], left: u32, right: u32) {
        if !self.visible(rect) {
            return;
        }
        let [x, y, w, h] = Self::px_rect(batch, rect);
        let start = batch.vertices.len();
        self.rect_px(batch, [x, y, w, h], abgr(left));
        // Two triangles of corners 0 1 2 and 0 2 3 (top left, top right, bottom right, bottom
        // left): the right corners are the 2nd, 3rd and 5th vertices.
        if batch.vertices.len() == start + 6 {
            for i in [1, 2, 4] {
                batch.vertices[start + i].color = abgr(right);
            }
        }
    }

    pub fn quad(&mut self, batch: &mut HudBatch, points: [[f32; 2]; 4], argb: u32) {
        let Some(slot) = self.slot else {
            return;
        };
        let w = slot.white;
        let uv = [(w.u0 + w.u1) / 2.0, (w.v0 + w.v1) / 2.0];
        batch.quad(points, [uv; 4], abgr(argb));
    }

    /// A line of text whose em is `px` screen pixels, its baseline at `baseline` (screen pixels
    /// from the top) and its pen starting at `x` (screen pixels); the pen's end.
    #[allow(clippy::too_many_arguments)]
    pub fn text_px(
        &mut self,
        batch: &mut HudBatch,
        x: f32,
        baseline: f32,
        text: &str,
        px: f32,
        weight: Weight,
        color: u32,
    ) -> f32 {
        let Some(f) = font(weight) else {
            return x;
        };
        let sf = f.as_scaled(scale_of(px));
        let quarter = (px * 4.0).round().clamp(1.0, 4000.0) as u16;
        let s = Self::scale(batch);
        let mut pen = x;
        let mut prev = None;
        let baseline = baseline.round();
        for c in text.chars() {
            let id = sf.glyph_id(c);
            if let Some(p) = prev {
                pen += sf.kern(p, id);
            }
            prev = Some(id);
            let key = Key::Glyph(weight, id.0, quarter);
            if let Some(p) = self.piece(batch, &key)
                && let Some(uv) = self.uv(&p, false, false)
            {
                let gx = pen.round() + p.left;
                let gy = baseline + p.top;
                batch.rect_uv(gx / s, gy / s, p.w as f32 / s, p.h as f32 / s, uv, color);
            }
            pen += sf.h_advance(id);
        }
        pen
    }

    /// The ascent of a line of `px` screen pixels (its baseline from its box's top).
    pub fn ascent_px(px: f32, weight: Weight) -> f32 {
        font(weight).map_or(px * title::ASCENT, |f| f.as_scaled(scale_of(px)).ascent())
    }

    /// `picture` over `rect` (GUI pixels), multiplied by `tint` (ARGB).
    pub fn picture(&mut self, batch: &mut HudBatch, picture: &Picture, rect: [f32; 4], tint: u32) {
        if !self.visible(rect) {
            return;
        }
        let [x, y, w, h] = Self::px_rect(batch, rect);
        if w <= 0 || h <= 0 {
            return;
        }
        // Big pictures (a server's banner) are kept at most this large and stretched.
        let (max_w, max_h) = (640, 256);
        let k = (w as f32 / max_w as f32)
            .max(h as f32 / max_h as f32)
            .max(1.0);
        let (pw, ph) = (
            ((w as f32 / k).round() as u32).max(1),
            ((h as f32 / k).round() as u32).max(1),
        );
        let key = Key::Picture(picture.clone(), pw, ph);
        let Some(p) = self.piece(batch, &key) else {
            return;
        };
        let Some(uv) = self.uv(&p, false, false) else {
            return;
        };
        let s = Self::scale(batch);
        batch.rect_uv(
            x as f32 / s,
            y as f32 / s,
            w as f32 / s,
            h as f32 / s,
            uv,
            abgr(tint),
        );
    }

    /// A line of text, its box's top at `y`, its em `size` (GUI pixels); its width (GUI).
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        batch: &mut HudBatch,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: Weight,
        argb: u32,
        align: Align,
    ) -> f32 {
        let s = Self::scale(batch);
        let px = (size * s).round().max(4.0);
        let width = measure_px(text, px, weight);
        let line_h = px * title::LINE;
        if !self.visible([x - width / s, y, width * 2.0 / s + 1.0, line_h / s]) {
            return width / s;
        }
        let x_px = match align {
            Align::Left => x * s,
            Align::Center => x * s - width / 2.0,
            Align::Right => x * s - width,
        };
        let baseline = y * s + Self::ascent_px(px, weight);
        self.text_px(batch, x_px.round(), baseline, text, px, weight, abgr(argb));
        width / s
    }

    /// Paints `paints` (a screen's or a widget's).
    pub fn paint_all(&mut self, batch: &mut HudBatch, paints: &[Paint]) {
        for p in paints {
            self.paint(batch, p);
        }
        // An unclosed clip ends with the paints.
        self.clip_end(batch);
    }

    pub fn paint(&mut self, batch: &mut HudBatch, p: &Paint) {
        match p {
            Paint::Fill { rect, argb } => self.fill(batch, *rect, *argb),
            Paint::Round {
                rect,
                radius,
                fill,
                border,
            } => self.round(batch, *rect, *radius, *fill, *border),
            Paint::Quad { points, argb } => self.quad(batch, *points, *argb),
            Paint::Gradient { rect, left, right } => self.gradient(batch, *rect, *left, *right),
            Paint::Text {
                x,
                y,
                text,
                size,
                weight,
                argb,
                align,
            } => {
                self.text(batch, *x, *y, text, *size, *weight, *argb, *align);
            }
            Paint::Picture {
                picture,
                rect,
                tint,
            } => self.picture(batch, picture, *rect, *tint),
            Paint::ClipBegin(r) => {
                self.clip_end(batch);
                self.clip = Some((batch.vertices.len(), *r));
            }
            Paint::ClipEnd => self.clip_end(batch),
        }
    }

    fn clip_end(&mut self, batch: &mut HudBatch) {
        if let Some((start, [x, y, w, h])) = self.clip.take()
            && batch.vertices.len() > start
        {
            batch.clip_since(start, x, y, x + w, y + h);
        }
    }
}

/// The top-left quarter of a circle of radius `n` centred on `(n, n)`, 4x4 supersampled, white:
/// the disc, or with `ring` the one-pixel ring at its edge.
fn corner_mask(n: u32, ring: bool) -> Vec<u8> {
    let r = n as f32;
    let cover = |x: u32, y: u32, radius: f32| -> f32 {
        if radius <= 0.0 {
            return 0.0;
        }
        let mut inside = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                let (dx, dy) = (r - px, r - py);
                if dx * dx + dy * dy <= radius * radius {
                    inside += 1;
                }
            }
        }
        inside as f32 / 16.0
    };
    let mut out = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let a = if ring {
                cover(x, y, r) - cover(x, y, r - 1.0)
            } else {
                cover(x, y, r)
            };
            out.extend_from_slice(&[255, 255, 255, (a.clamp(0.0, 1.0) * 255.0).round() as u8]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_images_and_fonts_load() {
        let logo = LOGO.as_ref().expect("the wordmark decodes");
        assert_eq!(logo.height, 256);
        let banner = BANNER.as_ref().expect("the banner decodes");
        assert!(banner.width > banner.height * 3);
        let icons = ICONS.as_ref().expect("the icons decode");
        assert_eq!(
            (icons.width, icons.height),
            (Glyph::SIDE * Glyph::COUNT, Glyph::SIDE)
        );
        assert!(MEDIUM.is_some() && SEMIBOLD.is_some());
        let w = measure_px("abcdefghij", 20.0, Weight::Medium);
        assert!((90.0..130.0).contains(&w), "{w}");
    }

    #[test]
    fn corner_masks_are_full_inside_and_rings_their_edge() {
        let disc = corner_mask(8, false);
        let ring = corner_mask(8, true);
        let a = |m: &[u8], x: u32, y: u32| m[((y * 8 + x) * 4 + 3) as usize];
        // The bottom right of the top-left quarter is inside the disc, the top left outside.
        assert_eq!(a(&disc, 7, 7), 255);
        assert_eq!(a(&disc, 0, 0), 0);
        assert_eq!(a(&ring, 7, 7), 0);
        assert!(a(&ring, 7, 0) > 200);
        // The ring is the disc's edge only: never more than the disc.
        for i in (3..disc.len()).step_by(4) {
            assert!(ring[i] <= disc[i]);
        }
    }

    #[test]
    fn colours_turn_into_the_huds_order() {
        assert_eq!(abgr(0xFF11_2233), 0xFF33_2211);
        assert_eq!(abgr(0x8000_00FF), 0x80FF_0000);
    }
}
