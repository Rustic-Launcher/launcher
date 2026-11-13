//! The Rustic main menu over the title screen (ADRs 0148, 0160): the title's own widgets hidden
//! (`Host::title_widgets_hidden`, Rustic's Horizon patch U13), the menu drawn in
//! `screen_render_post` from [`rusticmc_companion_core::title`]'s layout, its game buttons
//! pressing the title's own (`Host::title_open`), Discover opening the multiplayer screen on its
//! Discover tab ([`crate::screens`]), the Store buttons opening the website's store and Language
//! opening Rustic's language screen ([`crate::screens::language`]) in place of the game's page.
//!
//! Every picture and every line of text is the companion's own, drawn with the
//! [`crate::canvas`]: text set in Inter, the logo and icons resampled at the exact pixel size the
//! screen shows them.
use crate::canvas::{self, canvas};
use crate::state::{guard, guard_or, lock};
use horizon_client::mod_api::{
    Flow, Host, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent,
};
use rusticmc_companion_core::paint::{Align, Picture};
use rusticmc_companion_core::title::{
    self, Button, Draw, Face, Hit, KeyOutcome, Layout, RusticText, StoreItem, View, Weight,
    rustic_text, skin_face, tidy_label,
};
use std::time::{Duration, Instant};

/// The title screen's id in the screen hooks (Horizon's `Mode::Main`).
pub const TITLE: &str = "title";
/// The website's store (fixed, https): the Store button opens it, the card a product's page.
pub const STORE_URL: &str = "https://rustic-launcher.com/store";
/// The title counts as shown for this long after its last frame.
const SHOWN: Duration = Duration::from_millis(500);
/// The store's catalogue is asked again this often while the title shows.
const STORE_REFRESH: Duration = Duration::from_secs(600);

/// What the title shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TitleView {
    #[default]
    Menu,
    /// Rustic's language screen.
    Languages,
}

/// The menu's state, in the companion's [`crate::state::State`].
#[derive(Default)]
pub struct TitleMenu {
    pub view: TitleView,
    /// The button the keyboard is on.
    pub focus: Option<Button>,
    /// The menu as last drawn, for clicks and keys.
    pub layout: Layout,
    /// The last frame the title drew, and when the menu came up.
    pub drawn: Option<Instant>,
    pub since: Option<Instant>,
    /// The player's face: the skin's, or the default skin's until it is known.
    pub face: Option<Face>,
    pub face_asked: bool,
    pub default_face_tried: bool,
    /// The store's newest products (empty: no card), and when they were asked for.
    pub store: Vec<StoreItem>,
    pub store_asked: Option<Instant>,
}

/// The window and task bar icon, for the edition's `main` (`horizon_client::window::set_icon_png`).
pub const ICON_PNG: &[u8] = include_bytes!("../assets/rustic-icon-256.png");

/// A line's width in screen pixels (the title's layout works in screen pixels).
pub fn measure(text: &str, size: f32, weight: Weight) -> f32 {
    canvas::measure_px(text, size, weight)
}

fn enabled() -> bool {
    lock().cfg.title_menu
}

pub fn lang() -> String {
    horizon_client::lang::code()
}

/// A button's label in the game's language: the game's own translation keys for its buttons,
/// Rustic's for the others.
fn label(b: Button) -> String {
    if let Some((key, english)) = b.label_key() {
        return tidy_label(&horizon_client::lang::get_or(key, english));
    }
    let text = match b {
        Button::StoreCard => RusticText::StoreNews,
        Button::Discover => RusticText::Discover,
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
        let now = Instant::now();
        let view = {
            let mut st = lock();
            let menu = &mut st.title;
            if menu.drawn.is_none_or(|t| now - t > SHOWN) {
                menu.since = Some(now);
                menu.view = TitleView::Menu;
            }
            menu.drawn = Some(now);
            menu.view
        };
        match view {
            TitleView::Menu => render_menu(event, now),
            TitleView::Languages => crate::screens::language::render(host, event),
        }
    });
}

fn render_menu(event: &mut ScreenRenderPostEvent<'_>, now: Instant) {
    let player = account();
    let legal = horizon_client::lang::get_or(
        "horizon.title.not_affiliated",
        "Not affiliated with Mojang or Microsoft",
    );
    let version = crate::window_title();
    let store_title = rustic_text(RusticText::StoreNews, &lang());
    let scale = event.batch.scale.max(1.0);
    let mut st = lock();
    let menu = &mut st.title;
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
        logo_aspect: canvas::banner_aspect(),
        version: &version,
        legal: &legal,
    });
    let mut c = canvas();
    if c.ready() {
        for d in &layout.draws {
            match d {
                Draw::Fill { rect, argb } => c.fill(event.batch, *rect, *argb),
                Draw::Quad { points, argb } => c.quad(event.batch, *points, *argb),
                Draw::Text {
                    at,
                    text,
                    size,
                    weight,
                    rgb,
                } => {
                    // The layout's text sizes are screen pixels: an em of `size`.
                    c.text(
                        event.batch,
                        at[0],
                        at[1],
                        text,
                        *size / scale,
                        *weight,
                        0xFF00_0000 | rgb,
                        Align::Left,
                    );
                }
                Draw::Image { image, dst } => {
                    c.picture(event.batch, &Picture::Title(*image), *dst, 0xFFFF_FFFF)
                }
            }
        }
    }
    drop(c);
    for (r, h) in &layout.hits {
        match h {
            Hit::Button(b) => crate::screens::register(&label(*b), *r),
            Hit::Credits => crate::screens::register(&legal, *r),
        }
    }
    // A focus on a button that went away (the store card) is dropped.
    if menu.focus.is_some_and(|f| !layout.order.contains(&f)) {
        menu.focus = None;
    }
    menu.layout = layout;
}

/// A new HUD atlas came with the resources: the canvas starts over.
pub fn resources_reloaded() {
    guard("title_menu.resources_reloaded", || {
        canvas().forget();
    });
}

/// Once a frame, before it is drawn (the `frame` hook): the title's widgets hidden from the
/// first frame on, the player's face, the store's news.
pub fn frame(host: &mut dyn Host) {
    guard("title_menu.frame", || {
        let on = enabled();
        host.title_widgets_hidden(on);
        let now = Instant::now();
        let (shown, started, store_due) = {
            let st = lock();
            let menu = &st.title;
            let shown = on && menu.drawn.is_some_and(|t| now - t < SHOWN);
            let store_due = menu.store_asked.is_none_or(|t| now - t >= STORE_REFRESH);
            (shown, st.started, store_due)
        };
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
            let items = crate::directory::get_json(&url).map(|v| title::store_items(&v, 5));
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
pub fn open_url(url: String) {
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
    let skin = canvas::decode_png(&bytes)?;
    skin_face(&skin.pixels, skin.width, skin.height)
}

/// Does what `b` does.
fn press(host: &dyn Host, b: Button) {
    match b {
        Button::Language => {
            crate::screens::language::open(host);
            return;
        }
        Button::Discover => {
            crate::screens::open_discover(host);
            return;
        }
        _ => {}
    }
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

fn view() -> TitleView {
    lock().title.view
}

pub fn mouse(host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
    guard_or("title_menu.mouse", Flow::Continue, || {
        if event.screen.id != TITLE || !enabled() {
            return Flow::Continue;
        }
        if view() == TitleView::Languages {
            return crate::screens::language::mouse(host, event);
        }
        if event.button != 0 {
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
        if view() == TitleView::Languages {
            return crate::screens::language::key(host, event);
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

pub fn chars(host: &dyn Host, event: &mut ScreenCharTypedPreEvent<'_>) -> Flow {
    guard_or("title_menu.chars", Flow::Continue, || {
        if event.screen.id != TITLE || !enabled() || view() != TitleView::Languages {
            return Flow::Continue;
        }
        crate::screens::language::typed(host, event.c)
    })
}

pub fn scroll(host: &dyn Host, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
    guard_or("title_menu.scroll", Flow::Continue, || {
        if event.screen.id != TITLE || !enabled() || view() != TitleView::Languages {
            return Flow::Continue;
        }
        crate::screens::language::scroll(host, event.scroll[1] as f32)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_icon_decodes() {
        let icon = canvas::decode_png(ICON_PNG).expect("the window icon decodes");
        assert_eq!((icon.width, icon.height), (256, 256));
    }

    #[test]
    fn the_store_url_is_https_and_fixed() {
        assert!(STORE_URL.starts_with("https://rustic-launcher.com/"));
    }
}
