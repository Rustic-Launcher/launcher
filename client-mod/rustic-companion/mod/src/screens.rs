//! Rustic's own screens over the game's (ADR 0160):
//!
//! - the multiplayer screen's "My servers | Discover" switch, over its title;
//! - the Discover tab: Rustic Cloud's server directory over the whole multiplayer screen
//!   ([`rusticmc_companion_core::discover`], data from [`crate::directory`]), which takes every
//!   press and key while it shows (Escape still leaves the screen, as on the list); Join connects
//!   at once and Add to list saves the server (`Host::join_server`, `Host::add_server`, Rustic's
//!   Horizon patch U16); the title's Discover button opens the multiplayer screen on this tab;
//! - [`language`]: Rustic's language screen over the title.
use crate::canvas::{canvas, measure_px};
use crate::state::{guard, guard_or, lock};
use horizon_client::mod_api::{
    Flow, Host, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent,
};
use rusticmc_companion_core::discover::{self, Hit, Layout, Listing, Query, Tab};
use rusticmc_companion_core::kit::Kit;
use rusticmc_companion_core::title::Weight;
use std::time::{Duration, Instant};

pub mod language;

/// The multiplayer list's screen id.
pub const MULTIPLAYER: &str = "multiplayer";
/// A search waits this long after the last key before it is asked.
const SEARCH_WAIT: Duration = Duration::from_millis(300);
/// The server list is read again this often while the screen shows.
const SERVERS_EVERY: Duration = Duration::from_secs(3);

/// The Discover tab's state.
#[derive(Default)]
pub struct DiscoverUi {
    pub tab: Tab,
    /// The title's Discover button was pressed: the multiplayer screen opens on Discover.
    pub pending: bool,
    pub query: String,
    pub search_focused: bool,
    pub tag: Option<String>,
    /// The search asked for, once the typing stopped.
    pub asked: Option<Query>,
    pub typed: Option<Instant>,
    pub scroll: f32,
    pub focus: Option<usize>,
    pub opened: Option<Instant>,
    pub layout: Option<Layout>,
    pub switch_hits: Vec<([f32; 4], Hit)>,
    /// The game's server list (`name, address, hidden`), and when it was read.
    pub servers: Vec<(String, String, bool)>,
    pub servers_at: Option<Instant>,
    /// Servers joined or added from here this session (shown as in the list at once).
    pub added: Vec<String>,
    pub last_frame: Option<Instant>,
}

/// Registers a piece that can be pressed as one of the screen's widgets, by its label: the
/// game's controller support reaches it, the narrator reads it, and the control socket's `ui`
/// lists it (`click <id>` presses it at its centre, through the window's own events).
pub fn register(label: &str, rect: [f32; 4]) {
    if rect[2] > 0.0 && rect[3] > 0.0 && !label.trim().is_empty() {
        horizon_client::widgets::record(label, rect, true);
    }
}

/// GUI-pixel measure at GUI scale `scale`.
pub fn gui_measure(scale: f32) -> impl Fn(&str, f32, Weight) -> f32 {
    move |t: &str, size: f32, w: Weight| measure_px(t, size * scale, w) / scale
}

/// The title's Discover button: the multiplayer screen, on its Discover tab.
pub fn open_discover(host: &dyn Host) {
    lock().discover.pending = true;
    if !host.title_open("multiplayer") {
        lock().discover.pending = false;
    }
}

fn lang() -> String {
    crate::title_menu::lang()
}

/// What the tab's search asks for now (None: the sections).
fn search_query(ui: &DiscoverUi) -> Option<Query> {
    (!ui.query.trim().is_empty() || ui.tag.is_some()).then(|| Query::Search {
        q: ui.query.trim().to_owned(),
        tag: ui.tag.clone(),
    })
}

pub fn render(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    guard("screens.render", || {
        if event.screen.id != MULTIPLAYER {
            if event.screen.id == crate::title_menu::TITLE {
                let mut st = lock();
                if !st.discover.pending {
                    st.discover.tab = Tab::MyServers;
                }
            }
            return;
        }
        if !lock().cfg.title_menu {
            return;
        }
        let now = Instant::now();
        let servers_due = {
            let mut st = lock();
            let ui = &mut st.discover;
            if std::mem::take(&mut ui.pending) {
                ui.tab = Tab::Discover;
                ui.opened = Some(now);
                ui.scroll = 0.0;
            }
            if ui
                .last_frame
                .is_none_or(|t| now - t > Duration::from_millis(500))
            {
                ui.opened = Some(now);
                ui.servers_at = None;
            }
            ui.last_frame = Some(now);
            ui.servers_at.is_none_or(|t| now - t >= SERVERS_EVERY)
        };
        if servers_due {
            // A host call: no lock held.
            let servers = host.server_list();
            let mut st = lock();
            st.discover.servers = servers;
            st.discover.servers_at = Some(now);
        }
        let scale = event.batch.scale.max(1.0);
        let measure = gui_measure(scale);
        let gui = [event.batch.width, event.batch.height];
        let mouse = event.screen.mouse;
        let lang = lang();
        let mut st = lock();
        let api = st.cfg.api_base.clone();
        let tab = st.discover.tab;
        let mut layout = None;
        if tab == Tab::Discover {
            // The lists, the catalog and the search once the typing stopped.
            let search = search_query(&st.discover);
            let typed_long_ago = st.discover.typed.is_none_or(|t| now - t >= SEARCH_WAIT);
            if typed_long_ago {
                st.discover.asked = search.clone();
            }
            let dir = &mut st.directory;
            for q in [Query::Featured, Query::Trending, Query::All] {
                dir.want(&q, &api, now);
            }
            dir.want_catalog(&api, now);
            let asked = st.discover.asked.clone();
            if let Some(q) = &asked {
                st.directory.want(q, &api, now);
            }
            let st = &mut *st;
            let ui = &st.discover;
            let dir = &st.directory;
            let featured = dir.items(&Query::Featured);
            let trending = dir.items(&Query::Trending);
            let all = dir.items(&Query::All);
            let results = asked.as_ref().map(|q| dir.items(q));
            let mut pool: Vec<&Listing> = featured.iter().chain(trending).chain(all).collect();
            if let Some(r) = results {
                pool.extend(r.iter());
            }
            // Recently played: joined by address first (newest first), then the saved list.
            let mut recent_addresses: Vec<String> = ui.added.iter().rev().cloned().collect();
            recent_addresses.extend(ui.servers.iter().filter(|s| s.2).map(|s| s.1.clone()));
            recent_addresses.extend(ui.servers.iter().filter(|s| !s.2).map(|s| s.1.clone()));
            let recent = discover::recently_played(&pool, &recent_addresses, 3);
            let shown: Vec<String> = ui
                .servers
                .iter()
                .filter(|s| !s.2)
                .map(|s| discover::same_address(&s.1))
                .chain(ui.added.iter().map(|a| discover::same_address(a)))
                .collect();
            let listed = |a: &str| shown.contains(&discover::same_address(a));
            // Results only once the search answered; the sections meanwhile.
            let results = asked.as_ref().filter(|q| dir.answered(q)).and(results);
            let time = ui.opened.map_or(0.0, |t| (now - t).as_secs_f32());
            let caret = ((time * 2.0) as u32).is_multiple_of(2);
            let l = discover::layout(&discover::View {
                gui,
                mouse,
                lang: &lang,
                measure: &measure,
                query: &ui.query,
                search_focused: ui.search_focused,
                caret,
                tags: &dir.tags,
                tag: ui.tag.as_deref(),
                featured,
                trending,
                all,
                recent: &recent,
                results,
                status: dir.status(),
                listed: &listed,
                scroll: ui.scroll,
                time,
                focus: ui.focus,
            });
            // The pictures the screen shows.
            let mut wanted: Vec<(String, bool)> = Vec::new();
            for p in &l.paints {
                if let rusticmc_companion_core::paint::Paint::Picture {
                    picture: rusticmc_companion_core::paint::Picture::Remote(url),
                    rect,
                    ..
                } = p
                {
                    wanted.push((url.clone(), rect[2] > 100.0));
                }
            }
            for (url, banner) in wanted {
                st.directory.want_picture(&url, banner);
            }
            layout = Some(l);
        }
        let mut c = canvas();
        if !c.ready() {
            return;
        }
        if let Some(l) = &layout {
            c.paint_all(event.batch, &l.paints);
        }
        let mut kit: Kit<'_, Hit> = Kit::new(&measure, mouse);
        discover::switch(&mut kit, gui[0], tab, &lang);
        c.paint_all(event.batch, &kit.out);
        drop(c);
        if let Some(l) = &layout {
            // The list under the tab can't be reached: its widgets go, ours come.
            horizon_client::widgets::cover_frame();
            if l.max_scroll > 0.0 {
                let ui = &st.discover;
                horizon_client::widgets::scroll_area(l.area, ui.scroll, l.max_scroll);
            }
        }
        for (r, h) in &kit.hits {
            if let Hit::Tab(t) = h {
                let key = match t {
                    Tab::MyServers => rusticmc_companion_core::i18n::T::MyServers,
                    Tab::Discover => rusticmc_companion_core::i18n::T::Discover,
                };
                register(rusticmc_companion_core::i18n::t(key, &lang), *r);
            }
        }
        if let Some(l) = &layout {
            let area = l.area;
            for (r, h) in &l.hits {
                let visible = r[1] >= area[1] - 0.5 && r[1] + r[3] <= area[1] + area[3] + 0.5;
                let name = |id: &str| {
                    st.directory
                        .find(id)
                        .map_or_else(|| id.to_owned(), |l| l.name)
                };
                let label = match h {
                    Hit::Join(id) if visible => format!(
                        "{} {}",
                        rusticmc_companion_core::i18n::t(
                            rusticmc_companion_core::i18n::T::Join,
                            &lang
                        ),
                        name(id)
                    ),
                    Hit::Add(id) if visible => format!(
                        "{} {}",
                        rusticmc_companion_core::i18n::t(
                            rusticmc_companion_core::i18n::T::AddToList,
                            &lang
                        ),
                        name(id)
                    ),
                    Hit::Category(tag) if visible => match tag {
                        Some(t) => rusticmc_companion_core::i18n::tag_name(t, &lang),
                        None => rusticmc_companion_core::i18n::t(
                            rusticmc_companion_core::i18n::T::All,
                            &lang,
                        )
                        .to_owned(),
                    },
                    Hit::Retry => rusticmc_companion_core::i18n::t(
                        rusticmc_companion_core::i18n::T::Retry,
                        &lang,
                    )
                    .to_owned(),
                    Hit::Search => rusticmc_companion_core::i18n::t(
                        rusticmc_companion_core::i18n::T::Search,
                        &lang,
                    )
                    .to_owned(),
                    _ => continue,
                };
                register(&label, *r);
            }
        }
        let ui = &mut st.discover;
        if let Some(l) = &layout {
            ui.scroll = ui.scroll.clamp(0.0, l.max_scroll);
        }
        ui.layout = layout;
        ui.switch_hits = kit.hits;
    });
}

/// Whether the multiplayer screen shows the Discover tab now.
fn discovering(screen: &str) -> bool {
    if screen != MULTIPLAYER {
        return false;
    }
    let st = lock();
    st.discover.tab == Tab::Discover && st.cfg.title_menu
}

fn press(host: &dyn Host, hit: Hit) {
    match hit {
        Hit::Tab(t) => {
            let mut st = lock();
            st.discover.tab = t;
            st.discover.scroll = 0.0;
            st.discover.focus = None;
            st.discover.search_focused = false;
            st.discover.opened = Some(Instant::now());
        }
        Hit::Search => lock().discover.search_focused = true,
        Hit::Category(tag) => {
            let mut st = lock();
            st.discover.tag = tag;
            st.discover.scroll = 0.0;
            st.discover.typed = None;
        }
        Hit::Retry => lock().directory.retry(),
        Hit::Hero(i) => {
            // Turn the hero to that server: move the clock to its turn.
            let mut st = lock();
            let now = Instant::now();
            let n = st.directory.items(&Query::Featured).len().max(1);
            let at = (i % n) as f32 * discover::HERO_TURN + 0.01;
            st.discover.opened = now.checked_sub(Duration::from_secs_f32(at));
        }
        Hit::Join(id) => {
            let (listing, api) = {
                let st = lock();
                (st.directory.find(&id), st.cfg.api_base.clone())
            };
            if let Some(l) = listing {
                // A host call: no lock held.
                if host.join_server(&l.address) {
                    crate::directory::count_play(&api, &l.id);
                    let mut st = lock();
                    st.discover.added.retain(|a| a != &l.address);
                    st.discover.added.push(l.address.clone());
                } else {
                    log::warn!("Rustic: could not join {}", l.address);
                }
            }
        }
        Hit::Add(id) => {
            let listing = lock().directory.find(&id);
            if let Some(l) = listing
                && host.add_server(&l.name, &l.address)
            {
                let mut st = lock();
                st.discover.added.push(l.address.clone());
                st.discover.servers_at = None;
            }
        }
        Hit::Nothing => lock().discover.search_focused = false,
    }
}

pub fn mouse(host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
    guard_or("screens.mouse", Flow::Continue, || {
        if event.screen.id != MULTIPLAYER || !lock().cfg.title_menu {
            return Flow::Continue;
        }
        let at = event.screen.mouse;
        let switch_hit = lock()
            .discover
            .switch_hits
            .iter()
            .find(|(r, _)| rusticmc_companion_core::paint::inside(*r, at))
            .map(|(_, h)| h.clone());
        if let Some(h) = switch_hit {
            if event.button == 0 {
                press(host, h);
            }
            return Flow::Cancel;
        }
        if !discovering(event.screen.id) {
            return Flow::Continue;
        }
        let hit = lock().discover.layout.as_ref().and_then(|l| l.hit(at));
        {
            let mut st = lock();
            st.discover.focus = None;
            if !matches!(hit, Some(Hit::Search)) {
                st.discover.search_focused = false;
            }
        }
        if event.button == 0
            && let Some(h) = hit
        {
            press(host, h);
        }
        Flow::Cancel
    })
}

pub fn scroll(_host: &dyn Host, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
    guard_or("screens.scroll", Flow::Continue, || {
        if !discovering(event.screen.id) {
            return Flow::Continue;
        }
        let mut st = lock();
        let max = st.discover.layout.as_ref().map_or(0.0, |l| l.max_scroll);
        st.discover.scroll = (st.discover.scroll - event.scroll[1] as f32 * 24.0).clamp(0.0, max);
        Flow::Cancel
    })
}

pub fn key(host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
    guard_or("screens.key", Flow::Continue, || {
        if !discovering(event.screen.id) {
            return Flow::Continue;
        }
        let shift = event.modifiers & 1 != 0;
        match event.key {
            "key.keyboard.escape" => {
                let mut st = lock();
                if st.discover.search_focused {
                    st.discover.search_focused = false;
                    return Flow::Cancel;
                }
                // Leaves the screen, as on the list.
                Flow::Continue
            }
            "key.keyboard.backspace" => {
                let mut st = lock();
                if st.discover.search_focused || !st.discover.query.is_empty() {
                    st.discover.query.pop();
                    st.discover.typed = Some(Instant::now());
                    st.discover.scroll = 0.0;
                }
                Flow::Cancel
            }
            "key.keyboard.tab" | "key.keyboard.down" | "key.keyboard.up" | "key.keyboard.right"
            | "key.keyboard.left" => {
                let mut st = lock();
                let Some(l) = &st.discover.layout else {
                    return Flow::Cancel;
                };
                let items = l.focusable();
                if items.is_empty() {
                    return Flow::Cancel;
                }
                let n = items.len();
                let back = shift || matches!(event.key, "key.keyboard.up" | "key.keyboard.left");
                let next = match (st.discover.focus, back) {
                    (None, false) => 0,
                    (None, true) => n - 1,
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                };
                let area = l.area;
                let r = items[next].0;
                // Scroll the focused piece into view.
                let ui = &mut st.discover;
                if r[1] < area[1] {
                    ui.scroll -= area[1] - r[1] + 4.0;
                } else if r[1] + r[3] > area[1] + area[3] {
                    ui.scroll += r[1] + r[3] - (area[1] + area[3]) + 4.0;
                }
                ui.scroll = ui.scroll.max(0.0);
                ui.focus = Some(next);
                ui.search_focused = false;
                Flow::Cancel
            }
            "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.space" => {
                let hit = {
                    let st = lock();
                    st.discover
                        .focus
                        .and_then(|f| st.discover.layout.as_ref()?.focusable().get(f).cloned())
                };
                if let Some((_, h)) = hit {
                    press(host, h);
                }
                Flow::Cancel
            }
            // Nothing reaches the list under the tab (Delete would remove a server).
            _ => Flow::Cancel,
        }
    })
}

pub fn chars(_host: &dyn Host, event: &mut ScreenCharTypedPreEvent<'_>) -> Flow {
    guard_or("screens.chars", Flow::Continue, || {
        if !discovering(event.screen.id) {
            return Flow::Continue;
        }
        let c = event.c;
        if c.is_control() {
            return Flow::Cancel;
        }
        let mut st = lock();
        let ui = &mut st.discover;
        // Typing anywhere searches.
        ui.search_focused = true;
        ui.focus = None;
        if ui.query.chars().count() < 100 {
            ui.query.push(c);
            ui.typed = Some(Instant::now());
            ui.scroll = 0.0;
        }
        Flow::Cancel
    })
}
