//! The Rustic main menu over the title screen (ADR 0148): the title's own widgets hidden
//! (`Host::title_widgets_hidden`, Rustic's Horizon patch U13), the menu drawn in
//! `screen_render_post` from [`rusticmc_companion_core::title`]'s layout, its game buttons
//! pressing the title's own (`Host::title_open`), its Store buttons opening the website's store.
//!
//! Every picture and every line of text is the companion's own image: text is set in Inter
//! (SIL OFL 1.1, `assets/Inter-*.ttf`) with ab_glyph, anti-aliased, the logo and icons are
//! resampled, all at the exact pixel size the screen shows them, packed into one picture and
//! uploaded as the mod image (`Host::mod_image`, the HUD atlas slot a native mod may fill), then
//! drawn with `Host::sprite` one image pixel to one screen pixel.
use crate::state::{guard, guard_or, lock};
use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use horizon_client::mod_api::{
    Flow, Host, ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent,
};
use rusticmc_companion_core::title::{
    self, Button, Draw, Face, Glyph, Hit, Image, KeyOutcome, Layout, Rgba, RusticText, StoreItem,
    View, Weight, rustic_text, skin_face, tidy_label,
};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// The title screen's id in the screen hooks (Horizon's `Mode::Main`).
const TITLE: &str = "title";
/// The HUD atlas slot `Host::mod_image` fills.
const MOD_IMAGE: &str = horizon_client::hud::MOD_IMAGE_SPRITE;
/// The website's store (fixed, https): the Store button opens it, the card a product's page.
pub const STORE_URL: &str = "https://rustic-launcher.com/store";
/// A new renderer (a resource reload, the loading screen's hand-over) starts with a blank slot:
/// the images go up again this often while the title shows.
const REUPLOAD: Duration = Duration::from_secs(5);
/// The title counts as shown for this long after its last frame.
const SHOWN: Duration = Duration::from_millis(500);
/// The store's catalogue is asked again this often while the title shows.
const STORE_REFRESH: Duration = Duration::from_secs(600);
/// Rendered images kept for the next pictures (the texts of a hover come and go).
const CACHE_MAX: usize = 256;

/// One image of the mod image, at the screen's pixel size.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Image(Image, [u32; 2]),
    Text {
        text: String,
        size: u32,
        weight: Weight,
        rgb: u32,
    },
}

/// The mod image as uploaded: where each image is in it.
#[derive(Default)]
pub struct Atlas {
    pub places: HashMap<Key, [u32; 4]>,
    pub picture: Option<Rgba>,
    pub uploaded: Option<Instant>,
    /// Images rendered before, by key.
    pub cache: HashMap<Key, Rgba>,
    /// The last set that did not fit (not tried again until the set changes).
    pub too_big: Option<Vec<Key>>,
}

/// The menu's state, in the companion's [`crate::state::State`].
#[derive(Default)]
pub struct TitleMenu {
    /// The button the keyboard is on.
    pub focus: Option<Button>,
    /// The menu as last drawn, for clicks and keys.
    pub layout: Layout,
    /// The last frame the title drew, and when the menu came up.
    pub drawn: Option<Instant>,
    pub since: Option<Instant>,
    /// The images the last frame asked for, and the uploaded ones.
    pub wanted: Vec<Key>,
    pub atlas: Atlas,
    /// The player's face: the skin's, or the default skin's until it is known.
    pub face: Option<Face>,
    pub face_asked: bool,
    pub default_face_tried: bool,
    /// The store's newest products (empty: no card), and when they were asked for.
    pub store: Vec<StoreItem>,
    pub store_asked: Option<Instant>,
}

/// The logo and the icon strip (launcher/assets/brand, `assets/make_assets.py`), decoded once.
pub(crate) static LOGO: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-wordmark.png")));
static ICONS: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-icons.png")));
/// Inter Medium and SemiBold (`assets/make_fonts.py`).
static MEDIUM: LazyLock<Option<FontRef<'static>>> =
    LazyLock::new(|| FontRef::try_from_slice(include_bytes!("../assets/Inter-Medium.ttf")).ok());
static SEMIBOLD: LazyLock<Option<FontRef<'static>>> =
    LazyLock::new(|| FontRef::try_from_slice(include_bytes!("../assets/Inter-SemiBold.ttf")).ok());

/// The window and task bar icon, for the edition's `main` (`horizon_client::window::set_icon_png`).
pub const ICON_PNG: &[u8] = include_bytes!("../assets/rustic-icon-256.png");

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
fn scale_of(size: f32) -> PxScale {
    PxScale::from(size * title::LINE)
}

/// A line's width in screen pixels, as [`render_text`] sets it.
pub fn measure(text: &str, size: f32, weight: Weight) -> f32 {
    let Some(font) = font(weight) else {
        return text.chars().count() as f32 * size * 0.55;
    };
    let sf = font.as_scaled(scale_of(size));
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

/// A line of text in `rgb`, anti-aliased: `ceil(size * LINE)` pixels high, its baseline at the
/// font's ascent, a pixel of room on each side.
pub fn render_text(text: &str, size: f32, weight: Weight, rgb: u32) -> Option<Rgba> {
    let font = font(weight)?;
    let sf = font.as_scaled(scale_of(size));
    let width = measure(text, size, weight).ceil() as u32 + 2;
    let height = title::line_height(size) as u32;
    let baseline = sf.ascent().round();
    let mut cover = vec![0f32; (width * height) as usize];
    let mut caret = 1.0;
    let mut prev = None;
    for c in text.chars() {
        let id = sf.glyph_id(c);
        if let Some(p) = prev {
            caret += sf.kern(p, id);
        }
        let glyph = id.with_scale_and_position(scale_of(size), point(caret, baseline));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let b = outlined.px_bounds();
            outlined.draw(|x, y, c| {
                let (px, py) = (b.min.x as i32 + x as i32, b.min.y as i32 + y as i32);
                if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                    let i = (py as u32 * width + px as u32) as usize;
                    cover[i] = (cover[i] + c).min(1.0);
                }
            });
        }
        caret += sf.h_advance(id);
        prev = Some(id);
    }
    let [_, r, g, bl] = rgb.to_be_bytes();
    let mut im = Rgba::new(width, height);
    for (i, a) in cover.iter().enumerate() {
        im.pixels[i * 4..i * 4 + 4].copy_from_slice(&[r, g, bl, (a * 255.0).round() as u8]);
    }
    Some(im)
}

fn enabled() -> bool {
    lock().cfg.title_menu
}

fn lang() -> String {
    horizon_client::lang::code()
}

/// A button's label in the game's language: the game's own translation keys for its buttons,
/// Rustic's French or English for the others.
fn label(b: Button) -> String {
    if let Some((key, english)) = b.label_key() {
        return tidy_label(&horizon_client::lang::get_or(key, english));
    }
    let text = match b {
        Button::StoreCard => RusticText::StoreNews,
        _ => RusticText::Store,
    };
    rustic_text(text, &lang()).to_owned()
}

/// The signed-in account's name and UUID (never its token).
fn account() -> Option<(String, u128)> {
    horizon_client::auth::current().map(|s| (s.name.clone(), s.uuid))
}

pub fn render(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    guard("title_menu.render", || {
        if event.screen.id != TITLE {
            return;
        }
        if !enabled() {
            host.title_widgets_hidden(false);
            return;
        }
        host.title_widgets_hidden(true);
        let player = account();
        let legal = horizon_client::lang::get_or(
            "horizon.title.not_affiliated",
            "Not affiliated with Mojang or Microsoft",
        );
        let version = crate::window_title();
        let store_title = rustic_text(RusticText::StoreNews, &lang());
        let scale = event.batch.scale.max(1.0);
        let now = Instant::now();
        let mut st = lock();
        let menu = &mut st.title;
        if menu.drawn.is_none_or(|t| now - t > SHOWN) {
            menu.since = Some(now);
        }
        let face = menu.face;
        let layout = title::layout(&View {
            gui: [event.batch.width, event.batch.height],
            scale,
            mouse: event.screen.mouse,
            focus: menu.focus,
            label: &label,
            measure: &measure,
            player: player.as_ref().map(|(n, _)| (n.as_str(), face.as_ref())),
            store: &menu.store,
            store_title,
            time: menu.since.map_or(0.0, |s| (now - s).as_secs_f32()),
            logo_aspect: LOGO
                .as_ref()
                .map_or(4.0, |l| l.width as f32 / l.height as f32),
            version: &version,
            legal: &legal,
        });
        let mut wanted = Vec::new();
        let ready = menu.atlas.uploaded.is_some();
        for d in &layout.draws {
            let (key, at) = match d {
                Draw::Fill { rect, argb } => {
                    host.fill(event.batch, *rect, *argb);
                    continue;
                }
                Draw::Quad { points, argb } => {
                    host.quad(event.batch, *points, *argb);
                    continue;
                }
                Draw::Text {
                    at,
                    text,
                    size,
                    weight,
                    rgb,
                } => (
                    Key::Text {
                        text: text.clone(),
                        size: *size as u32,
                        weight: *weight,
                        rgb: *rgb,
                    },
                    *at,
                ),
                Draw::Image { image, dst } => (
                    Key::Image(
                        *image,
                        [
                            (dst[2] * scale).round().max(1.0) as u32,
                            (dst[3] * scale).round().max(1.0) as u32,
                        ],
                    ),
                    [dst[0], dst[1]],
                ),
            };
            // Not up yet: nothing (a frame away), never the game's own font.
            if ready && let Some([x, y, w, h]) = menu.atlas.places.get(&key) {
                host.sprite(
                    event.batch,
                    MOD_IMAGE,
                    [*x as f32, *y as f32, *w as f32, *h as f32],
                    [at[0], at[1], *w as f32 / scale, *h as f32 / scale],
                );
            }
            if !wanted.contains(&key) {
                wanted.push(key);
            }
        }
        // A focus on a button that went away (the store card) is dropped.
        if menu.focus.is_some_and(|f| !layout.order.contains(&f)) {
            menu.focus = None;
        }
        menu.layout = layout;
        menu.wanted = wanted;
        menu.drawn = Some(now);
    });
}

/// The image of `key`, rendered.
fn render_key(key: &Key) -> Option<Rgba> {
    let side = Glyph::SIDE;
    let glyph = |g: Glyph| {
        ICONS
            .as_ref()
            .map(|s| s.crop([g.index() * side, 0, side, side]))
    };
    match key {
        Key::Image(Image::Logo, [w, h]) => LOGO.as_ref().map(|l| title::resample(l, *w, *h)),
        Key::Image(Image::Icon(g, rgb), [w, h]) => {
            glyph(*g).map(|im| title::resample(&im, *w, *h).tinted(*rgb))
        }
        Key::Image(Image::Tile { hue, glyph: g }, [w, _]) => {
            glyph(*g).map(|im| title::tile(*hue, *w, *w as f32 * 0.14, &im))
        }
        Key::Text {
            text,
            size,
            weight,
            rgb,
        } => render_text(text, *size as f32, *weight, *rgb),
    }
}

/// The images `keys` packed into one picture, rendered (or taken from the cache).
fn build_atlas(atlas: &mut Atlas, keys: &[Key]) -> Option<(Rgba, HashMap<Key, [u32; 4]>)> {
    let mut images = Vec::with_capacity(keys.len());
    let mut kept = Vec::with_capacity(keys.len());
    for key in keys {
        let image = match atlas.cache.get(key) {
            Some(im) => im.clone(),
            None => {
                let Some(im) = render_key(key) else {
                    continue;
                };
                if atlas.cache.len() >= CACHE_MAX {
                    atlas.cache.clear();
                }
                atlas.cache.insert(key.clone(), im.clone());
                im
            }
        };
        images.push(image);
        kept.push(key.clone());
    }
    let size = horizon_client::hud::MOD_IMAGE_SIZE;
    let (picture, places) = title::pack(&images, size, size)?;
    Some((picture, kept.into_iter().zip(places).collect()))
}

/// A new HUD atlas came with the resources: the mod image is blank again.
pub fn resources_reloaded() {
    guard("title_menu.resources_reloaded", || {
        lock().title.atlas.uploaded = None;
    });
}

/// Once a frame, before it is drawn (the `frame` hook): the title's widgets hidden from the
/// first frame on, the images (re)uploaded for what the last frame asked, the player's face, the
/// store's news.
pub fn frame(host: &mut dyn Host) {
    guard("title_menu.frame", || {
        let on = enabled();
        host.title_widgets_hidden(on);
        let now = Instant::now();
        let (shown, started, store_due) = {
            let mut st = lock();
            let started = st.started;
            let menu = &mut st.title;
            let shown = on && menu.drawn.is_some_and(|t| now - t < SHOWN);
            let mut keys = menu.wanted.clone();
            keys.sort();
            let mut have: Vec<Key> = menu.atlas.places.keys().cloned().collect();
            have.sort();
            if shown
                && keys != have
                && !keys.is_empty()
                && menu.atlas.too_big.as_ref() != Some(&keys)
            {
                match build_atlas(&mut menu.atlas, &keys) {
                    Some((picture, places)) => {
                        menu.atlas.picture = Some(picture);
                        menu.atlas.places = places;
                        menu.atlas.uploaded = None;
                        menu.atlas.too_big = None;
                    }
                    None => {
                        log::warn!(
                            "Rustic: the title menu's pictures do not fit the mod image ({} of them)",
                            keys.len()
                        );
                        menu.atlas.too_big = Some(keys);
                    }
                }
            }
            let store_due = menu.store_asked.is_none_or(|t| now - t >= STORE_REFRESH);
            (shown, started, store_due)
        };
        if shown {
            let mut st = lock();
            let atlas = &mut st.title.atlas;
            if atlas.uploaded.is_none_or(|t| now - t >= REUPLOAD)
                && let Some(p) = &atlas.picture
                && host.mod_image(p.width, p.height, &p.pixels)
            {
                atlas.uploaded = Some(now);
            }
        }
        if shown && started && store_due {
            let api = {
                let mut st = lock();
                st.title.store_asked = Some(now);
                st.cfg.api_base.clone()
            };
            fetch_store(api);
        }
        if shown {
            face(host);
        }
    });
}

/// The store's newest products from Rustic Cloud's public catalogue, on a thread of its own;
/// nothing on failure (the card stays hidden).
fn fetch_store(api: String) {
    let spawned = std::thread::Builder::new()
        .name("rustic-store-news".into())
        .spawn(move || {
            let url = format!("{api}/v1/store/catalog?currency=EUR");
            let items = ureq::get(&url)
                .timeout(Duration::from_secs(8))
                .call()
                .ok()
                .and_then(|r| {
                    let mut body = Vec::new();
                    std::io::Read::read_to_end(
                        &mut std::io::Read::take(r.into_reader(), 4 * 1024 * 1024),
                        &mut body,
                    )
                    .ok()?;
                    serde_json::from_slice::<serde_json::Value>(&body).ok()
                })
                .map(|v| title::store_items(&v, 5));
            match items {
                Some(items) => lock().title.store = items,
                None => log::debug!("Rustic: no store news from {url}"),
            }
        });
    if let Err(e) = spawned {
        log::warn!("Rustic: no thread for the store news: {e}");
    }
}

/// Opens `url` in the system's browser, off the game's thread.
fn open_url(url: String) {
    let spawned = std::thread::Builder::new()
        .name("rustic-open".into())
        .spawn(move || {
            if let Err(e) = open::that_detached(&url) {
                log::warn!("Rustic: could not open {url}: {e}");
            }
        });
    if let Err(e) = spawned {
        log::warn!("Rustic: no thread to open a link: {e}");
    }
}

/// The player's face: the default skin's for their UUID at once, then their own skin's once
/// it is downloaded (one request on a thread of its own, as the game asks for it).
fn face(host: &dyn Host) {
    let Some((_, uuid)) = account() else {
        return;
    };
    let (try_default, ask) = {
        let mut st = lock();
        let m = &mut st.title;
        let d = !m.default_face_tried && m.face.is_none();
        m.default_face_tried = true;
        let a = !m.face_asked;
        m.face_asked = true;
        (d, a)
    };
    if try_default
        && let Some(t) = host.client_texture(horizon_client::tab_list::default_skin(uuid))
        && let Some(f) = skin_face(&t.rgba, t.width, t.height)
    {
        let mut st = lock();
        if st.title.face.is_none() {
            st.title.face = Some(f);
        }
    }
    if ask {
        let spawned = std::thread::Builder::new()
            .name("rustic-skin".into())
            .spawn(|| {
                if let Some(f) = own_face() {
                    lock().title.face = Some(f);
                }
            });
        if let Err(e) = spawned {
            log::warn!("Rustic: no thread for the player's skin: {e}");
        }
    }
}

/// The signed-in account's own skin's face (`GET /minecraft/profile`, as the game asks, then the
/// skin from the texture server Horizon accepts).
fn own_face() -> Option<Face> {
    let session = horizon_client::auth::current()?;
    let textures = horizon_client::auth::own_textures(&session)?;
    let url = textures.skin?;
    if !horizon_client::skins::TEXTURE_URIS.iter().any(|u| {
        u.split("{textureId}")
            .next()
            .is_some_and(|p| url.starts_with(p))
    }) {
        return None;
    }
    let response = ureq::get(&url)
        .timeout(Duration::from_secs(10))
        .call()
        .ok()?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(response.into_reader(), 64 * 1024),
        &mut bytes,
    )
    .ok()?;
    let skin = decode_png(&bytes)?;
    skin_face(&skin.pixels, skin.width, skin.height)
}

/// Does what `b` does.
fn press(host: &dyn Host, b: Button) {
    if let Some(action) = b.title_action() {
        if !host.title_open(action) {
            log::debug!("Rustic: the title did not take {action}");
        }
        return;
    }
    match b {
        Button::Store => open_url(STORE_URL.to_owned()),
        Button::StoreCard => {
            let slug = {
                let st = lock();
                st.title
                    .layout
                    .card_item
                    .and_then(|i| st.title.store.get(i))
                    .map(|item| item.slug.clone())
            };
            open_url(match slug {
                Some(slug) => format!("{STORE_URL}/{slug}"),
                None => STORE_URL.to_owned(),
            });
        }
        _ => {}
    }
}

pub fn mouse(host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
    guard_or("title_menu.mouse", Flow::Continue, || {
        if event.screen.id != TITLE || event.button != 0 || !enabled() {
            return Flow::Continue;
        }
        let hit = {
            let mut st = lock();
            st.title.focus = None;
            st.title.layout.hit(event.screen.mouse)
        };
        match hit {
            Some(Hit::Button(b)) => press(host, b),
            Some(Hit::Credits) => {
                host.title_open("credits");
            }
            None => return Flow::Continue,
        }
        Flow::Cancel
    })
}

pub fn key(host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
    guard_or("title_menu.key", Flow::Continue, || {
        if event.screen.id != TITLE || !enabled() {
            return Flow::Continue;
        }
        // GLFW's modifier bits: Shift is 1.
        let shift = event.modifiers & 1 != 0;
        let outcome = {
            let st = lock();
            title::key(&st.title.layout.order, st.title.focus, event.key, shift)
        };
        match outcome {
            KeyOutcome::Focus(b) => {
                lock().title.focus = Some(b);
                Flow::Cancel
            }
            KeyOutcome::Press(b) => {
                press(host, b);
                Flow::Cancel
            }
            KeyOutcome::Ignore => Flow::Continue,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_images_and_fonts_load() {
        let logo = LOGO.as_ref().expect("the wordmark decodes");
        assert_eq!(logo.height, 256);
        assert!(logo.width > logo.height * 3);
        let icons = ICONS.as_ref().expect("the icons decode");
        assert_eq!(
            (icons.width, icons.height),
            (Glyph::SIDE * Glyph::COUNT, Glyph::SIDE)
        );
        let icon = decode_png(ICON_PNG).expect("the window icon decodes");
        assert_eq!((icon.width, icon.height), (256, 256));
        assert!(MEDIUM.is_some() && SEMIBOLD.is_some());
    }

    #[test]
    fn text_is_set_smooth_at_its_size() {
        let size = 15.0;
        let im = render_text("Multijoueur é€", size, Weight::SemiBold, 0xFF6A00).unwrap();
        assert_eq!(im.height, title::line_height(size) as u32);
        assert_eq!(
            im.width,
            measure("Multijoueur é€", size, Weight::SemiBold).ceil() as u32 + 2
        );
        let alphas: Vec<u8> = im.pixels.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
        // Anti-aliased: partly covered pixels, not only on and off.
        assert!(alphas.iter().any(|a| *a > 20 && *a < 235));
        assert!(alphas.iter().filter(|a| **a == 255).count() > 20);
        // The colour asked for.
        let lit = im
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .find(|p| p[3] == 255)
            .unwrap();
        assert_eq!(&lit[..3], &[0xFF, 0x6A, 0x00]);
        // About 0.5 to 0.6 em a character in Inter.
        let w = measure("abcdefghij", 20.0, Weight::Medium);
        assert!((90.0..130.0).contains(&w), "{w}");
        assert!(measure("WWW", 20.0, Weight::SemiBold) > measure("iii", 20.0, Weight::SemiBold));
    }

    #[test]
    fn the_menu_pictures_pack_into_the_mod_image() {
        let mut atlas = Atlas::default();
        let keys = vec![
            Key::Image(Image::Logo, [219, 56]),
            Key::Image(Image::Icon(Glyph::Gear, 0xFF6A00), [18, 18]),
            Key::Image(
                Image::Tile {
                    hue: 20,
                    glyph: Glyph::Crown,
                },
                [64, 64],
            ),
            Key::Text {
                text: "Solo".into(),
                size: 15,
                weight: Weight::SemiBold,
                rgb: 0xF3F1EE,
            },
        ];
        let (picture, places) = build_atlas(&mut atlas, &keys).unwrap();
        assert_eq!(places.len(), 4);
        assert!(picture.width <= 1024 && picture.height <= 1024);
        assert_eq!(places[&keys[0]][2..], [219, 56]);
        // The orange gear is orange where it has pixels.
        let [x, y, w, h] = places[&keys[1]];
        let mut lit = 0;
        for yy in y..y + h {
            for xx in x..x + w {
                let i = ((yy * picture.width + xx) * 4) as usize;
                if picture.pixels[i + 3] > 200 {
                    assert_eq!(&picture.pixels[i..i + 3], &[0xFF, 0x6A, 0x00]);
                    lit += 1;
                }
            }
        }
        assert!(lit > 20);
        assert_eq!(atlas.cache.len(), 4);
        // Too wide for the slot: refused rather than cut.
        let mut atlas = Atlas::default();
        assert!(build_atlas(&mut atlas, &[Key::Image(Image::Logo, [1031, 264])]).is_none());
    }

    #[test]
    fn the_store_url_is_https_and_fixed() {
        assert!(STORE_URL.starts_with("https://rustic-launcher.com/"));
    }
}
