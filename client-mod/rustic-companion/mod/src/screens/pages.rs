//! Rustic's world list and server list (ADR 0164): the title's Singleplayer screen
//! (`world_select`) and the multiplayer screen's "My servers" tab redone as the launcher's pages
//! ([`rusticmc_companion_core::lists`]) over the game's own, which still runs underneath: the
//! rows come from it (`Host::world_entries`, `Host::server_entries`), a card's press selects its
//! row (`Host::title_select`) and the action bar presses the game's own buttons
//! (`Host::press_at`, Rustic's Horizon patch U17), so Join, Play, Edit, Delete, Add, Direct
//! Connection, Re-Create and Refresh do exactly what they do in the game, their confirmations and
//! forms included (shown as Rustic dialogs by the skin).
use super::{gui_measure, register};
use crate::canvas::{canvas, decode_png};
use crate::frame::{self, frames};
use crate::state::{guard, guard_or, lock};
use horizon_client::mod_api::{
    Flow, Host, NativeServerEntry, NativeServerStatus, NativeWorldEntry, ScreenCharTypedPreEvent,
    ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent,
    ScreenRenderPostEvent,
};
use horizon_client::text::Text;
use rusticmc_companion_core::discover::Tab;
use rusticmc_companion_core::i18n::{self, T};
use rusticmc_companion_core::lists::{
    self, Act, Button, Hit, Layout, Reach, Rows, ServerRow, Tone, View, WorldRow,
};
use rusticmc_companion_core::paint::{self, Picture, rescaled, scale_rect};
use rusticmc_companion_core::skin::palette;
use rusticmc_companion_core::title::Rgba;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The world list's screen id.
pub const WORLDS: &str = "world_select";

/// Which page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Servers,
    Worlds,
}

/// A page's state.
#[derive(Default)]
pub struct PageUi {
    pub query: String,
    pub search_focused: bool,
    pub scroll: f32,
    pub focus: Option<usize>,
    pub opened: Option<Instant>,
    pub last_frame: Option<Instant>,
    pub layout: Option<Layout>,
    /// GUI pixels per layout unit at the last frame (0 before it).
    pub unit: f32,
    /// The game's own button for each act, as this frame drew it: its box and whether it is on.
    pub vanilla: Vec<(Act, [f32; 4], bool, String)>,
    /// The last press on a card (its row, when): a second one soon after opens it.
    pub last_click: Option<(usize, Instant)>,
    /// The picture each world (by folder) or server (by address) shows, and what it came from.
    pub pictures: HashMap<String, (u64, Option<String>)>,
    /// The selected row, as the game said this frame.
    pub selected: Option<usize>,
    /// Rows' names by index (for the narrator's labels).
    pub names: HashMap<usize, String>,
}

/// The game's keys of each page's buttons, by act (the first that is there is the one).
fn keys(kind: Kind, act: Act) -> &'static [&'static str] {
    match (kind, act) {
        (Kind::Servers, Act::Primary) => &["selectServer.select"],
        (Kind::Servers, Act::Direct) => &["selectServer.direct"],
        (Kind::Servers, Act::Create) => &["selectServer.add"],
        (Kind::Servers, Act::Edit) => &["selectServer.edit"],
        (Kind::Servers, Act::Delete) => &["selectServer.delete"],
        (Kind::Servers, Act::Refresh) => &["selectServer.refresh"],
        (Kind::Worlds, Act::Primary) => &["selectWorld.select"],
        (Kind::Worlds, Act::Create) => &["selectWorld.create"],
        (Kind::Worlds, Act::Edit) => &["selectWorld.edit"],
        (Kind::Worlds, Act::Delete) => &["selectWorld.delete"],
        (Kind::Worlds, Act::Recreate) => &["selectWorld.recreate"],
        (_, Act::Back) => &["gui.back", "gui.cancel"],
        _ => &[],
    }
}

/// The acts of a page, in the action bar's order.
fn acts(kind: Kind) -> &'static [Act] {
    match kind {
        Kind::Servers => &[
            Act::Back,
            Act::Refresh,
            Act::Direct,
            Act::Create,
            Act::Edit,
            Act::Delete,
            Act::Primary,
        ],
        Kind::Worlds => &[
            Act::Back,
            Act::Create,
            Act::Edit,
            Act::Delete,
            Act::Recreate,
            Act::Primary,
        ],
    }
}

fn page(st: &mut crate::state::State, kind: Kind) -> &mut PageUi {
    match kind {
        Kind::Servers => &mut st.servers_page,
        Kind::Worlds => &mut st.worlds_page,
    }
}

/// Whether Rustic draws this page now.
pub fn shown(screen: &str) -> Option<Kind> {
    let st = lock();
    if !st.cfg.rustic_theme {
        return None;
    }
    match screen {
        WORLDS => Some(Kind::Worlds),
        super::MULTIPLAYER if st.discover.tab == Tab::MyServers => Some(Kind::Servers),
        _ => None,
    }
}

/// The game's language.
fn lang() -> String {
    horizon_client::lang::code()
}

/// A styled text's lines as runs of `(text, ARGB)`, the theme's inks for the game's greys.
fn text_lines(text: &Text, default: u32) -> Vec<Vec<(String, u32)>> {
    let resolved = text.resolved();
    let mut lines: Vec<Vec<(String, u32)>> = vec![Vec::new()];
    for (c, style) in resolved.chars() {
        if c == '\n' {
            lines.push(Vec::new());
            continue;
        }
        if c.is_control() {
            continue;
        }
        let color = style
            .color
            .map_or(default, |rgb| crate::skin::line_color(0xFF00_0000 | rgb));
        let line = lines.last_mut().expect("a line");
        match line.last_mut() {
            Some((s, c0)) if *c0 == color => s.push(c),
            _ => line.push((c.to_string(), color)),
        }
    }
    for l in &mut lines {
        // Leading and trailing spaces of a MOTD are layout tricks of the game's font.
        if let Some((s, _)) = l.first_mut() {
            *s = s.trim_start().to_owned();
        }
        if let Some((s, _)) = l.last_mut() {
            *s = s.trim_end().to_owned();
        }
        l.retain(|(s, _)| !s.is_empty());
    }
    lines.retain(|l| !l.is_empty());
    lines
}

/// The picture a server shows (its favicon, put on the canvas once per icon).
fn favicon(ui: &mut PageUi, e: &NativeServerEntry) -> Option<Picture> {
    let icon: &Arc<Vec<u8>> = e.icon.as_ref()?;
    if icon.len() != 64 * 64 * 4 {
        return None;
    }
    let id = Arc::as_ptr(icon) as u64;
    let key = format!("favicon:{}:{id:x}", e.address);
    let known = ui.pictures.get(&e.address).map(|(i, _)| *i);
    if known != Some(id) {
        let mut c = canvas();
        if let Some((_, Some(old))) = ui.pictures.get(&e.address) {
            c.remote.remove(old);
        }
        c.remote.insert(
            key.clone(),
            Arc::new(Rgba {
                width: 64,
                height: 64,
                pixels: icon.to_vec(),
            }),
        );
        ui.pictures
            .insert(e.address.clone(), (id, Some(key.clone())));
    }
    Some(Picture::Remote(key))
}

/// The picture a world shows (its `icon.png`, read once per save).
fn world_icon(host: &dyn Host, ui: &mut PageUi, w: &NativeWorldEntry) -> Option<Picture> {
    let id = w.last_played as u64;
    if ui.pictures.get(&w.folder).map(|(i, _)| *i) != Some(id) {
        // A host call: no lock of the canvas held.
        let picture = host
            .world_icon(&w.folder)
            .and_then(|png| decode_png(&png))
            .map(|im| {
                let key = format!("world:{}:{id:x}", w.folder);
                let mut c = canvas();
                if let Some((_, Some(old))) = ui.pictures.get(&w.folder) {
                    c.remote.remove(old);
                }
                c.remote.insert(key.clone(), Arc::new(im));
                key
            });
        ui.pictures.insert(w.folder.clone(), (id, picture));
    }
    ui.pictures
        .get(&w.folder)
        .and_then(|(_, k)| k.clone())
        .map(Picture::Remote)
}

/// A server's card from the game's row.
fn server_row(ui: &mut PageUi, i: usize, e: &NativeServerEntry) -> ServerRow {
    let mut motd = text_lines(&e.motd, palette::MUTED);
    let (reach, players) = match &e.status {
        NativeServerStatus::Pinging => (Reach::Pinging, None),
        NativeServerStatus::Online {
            ping_ms, players, ..
        } => (Reach::Online { ping_ms: *ping_ms }, *players),
        NativeServerStatus::Incompatible { players, version } => (
            Reach::Incompatible {
                version: version.plain(),
            },
            *players,
        ),
        NativeServerStatus::Unreachable(_) => {
            motd = vec![vec![(
                horizon_client::lang::get_or(
                    "multiplayer.status.cannot_connect",
                    "Can't connect to server",
                ),
                palette::DANGER,
            )]];
            (Reach::Unreachable, None)
        }
        NativeServerStatus::Lan => (Reach::Lan, None),
    };
    let (name, motd) = if e.lan {
        // A LAN world's name is in its announcement; the game's "LAN World" under it.
        let plain = e.motd.plain();
        let name = if plain.trim().is_empty() {
            e.name.clone()
        } else {
            plain.trim().to_owned()
        };
        (name, vec![vec![(e.name.clone(), palette::MUTED)]])
    } else {
        (e.name.clone(), motd)
    };
    ServerRow {
        index: i,
        name,
        address: e.address.clone(),
        lan: e.lan,
        motd,
        players,
        reach,
        icon: favicon(ui, e),
        selected: e.selected,
    }
}

/// A world's card from the game's row.
fn world_row(
    host: &dyn Host,
    ui: &mut PageUi,
    i: usize,
    w: &NativeWorldEntry,
    lang: &str,
    now: i64,
) -> WorldRow {
    let g = |k: &str, f: &str| horizon_client::lang::get_or(k, f);
    let mut badges: Vec<(String, Tone)> = Vec::new();
    if w.hardcore {
        badges.push((g("selectWorld.gameMode.hardcore", "Hardcore"), Tone::Danger));
    } else {
        let (k, f, tone) = match w.game_mode {
            1 => ("selectWorld.gameMode.creative", "Creative", Tone::Info),
            2 => ("selectWorld.gameMode.adventure", "Adventure", Tone::Warn),
            3 => ("selectWorld.gameMode.spectator", "Spectator", Tone::Neutral),
            _ => ("selectWorld.gameMode.survival", "Survival", Tone::Good),
        };
        badges.push((g(k, f), tone));
    }
    if w.commands {
        badges.push((g("selectWorld.cheats", "Cheats"), Tone::Accent));
    }
    if w.experimental {
        badges.push((g("selectWorld.experimental", "Experimental"), Tone::Warn));
    }
    match &w.incompatible {
        Some(v) => badges.push((v.clone(), Tone::Danger)),
        None if !w.version.is_empty() => badges.push((w.version.clone(), Tone::Neutral)),
        None => {}
    }
    let when = lists::ago(now, w.last_played, lang);
    let detail = if w.locked {
        g("selectWorld.locked", "Locked by another running instance")
    } else if w.name != w.folder {
        format!("{when} · {}", w.folder)
    } else {
        when
    };
    WorldRow {
        index: i,
        name: w.name.clone(),
        detail,
        badges,
        icon: world_icon(host, ui, w),
        selected: w.selected,
        playable: !w.locked && w.incompatible.is_none(),
    }
}

pub fn render(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>, kind: Kind) {
    guard("pages.render", || {
        let now = Instant::now();
        let lang = lang();
        // The game's buttons this frame, by act.
        let vanilla: Vec<(Act, [f32; 4], bool, String)> = {
            let f = frames();
            acts(kind)
                .iter()
                .filter_map(|&a| {
                    frame::find(&f.current.buttons, keys(kind, a))
                        .map(|b| (a, b.rect, b.enabled, b.label.clone().unwrap_or_default()))
                })
                .collect()
        };
        // The rows (host calls: no lock held).
        let servers = (kind == Kind::Servers).then(|| host.server_entries());
        let worlds = (kind == Kind::Worlds).then(|| host.world_entries());
        let scale = event.batch.scale.max(1.0);
        let unit = paint::screen_unit(scale, event.batch.height * scale);
        let measure = gui_measure(scale * unit);
        let gui = [event.batch.width / unit, event.batch.height / unit];
        let mouse = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
        let wall = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let mut st = lock();
        let switch = st.cfg.title_menu;
        let tab = st.discover.tab;
        // Taken out of the state while host calls run (world icons).
        let mut ui = std::mem::take(page(&mut st, kind));
        drop(st);
        if ui
            .last_frame
            .is_none_or(|t| now - t > Duration::from_millis(500))
        {
            // Opened (again): from the top.
            ui.opened = Some(now);
            ui.scroll = 0.0;
            ui.focus = None;
            ui.search_focused = false;
        }
        ui.last_frame = Some(now);
        let time = ui.opened.map_or(0.0, |t| (now - t).as_secs_f32());
        let mut saved = Vec::new();
        let mut lan = Vec::new();
        let mut world_rows = Vec::new();
        ui.names.clear();
        ui.selected = None;
        if let Some(entries) = &servers {
            for (i, e) in entries.iter().enumerate() {
                let row = server_row(&mut ui, i, e);
                ui.names.insert(i, row.name.clone());
                if row.selected {
                    ui.selected = Some(i);
                }
                if e.lan {
                    lan.push(row);
                } else {
                    saved.push(row);
                }
            }
        }
        if let Some(entries) = &worlds {
            for (i, w) in entries.iter().enumerate() {
                let row = world_row(host, &mut ui, i, w, &lang, wall);
                ui.names.insert(i, row.name.clone());
                if row.selected {
                    ui.selected = Some(i);
                }
                world_rows.push(row);
            }
        }
        let buttons: Vec<Button> = vanilla
            .iter()
            .map(|(act, _, enabled, label)| Button {
                act: *act,
                label: label.clone(),
                enabled: *enabled,
            })
            .collect();
        let title_text = horizon_client::lang::get_or("menu.singleplayer", "Singleplayer");
        let multiplayer = horizon_client::lang::get_or("menu.multiplayer", "Multiplayer");
        let placeholder = match kind {
            Kind::Servers => i18n::t(T::SearchServers, &lang).to_owned(),
            Kind::Worlds => horizon_client::lang::get_or("selectWorld.search", "Search for worlds"),
        };
        // The game's placeholder may start in lower case ("search for worlds").
        let placeholder: String = {
            let mut c = placeholder.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        };
        let open_label = match kind {
            Kind::Servers => i18n::t(T::Join, &lang),
            Kind::Worlds => i18n::t(T::Play, &lang),
        };
        let scanning = horizon_client::lang::get_or(
            "lanServer.scanning",
            "Scanning for games on your local network",
        );
        let discover = i18n::t(T::Discover, &lang);
        let rows = match kind {
            Kind::Servers => Rows::Servers {
                saved: &saved,
                lan: &lan,
                time,
            },
            Kind::Worlds => Rows::Worlds(&world_rows),
        };
        let caret = ((time * 2.0) as u32).is_multiple_of(2);
        let l = lists::layout(&View {
            gui,
            mouse,
            lang: &lang,
            measure: &measure,
            title: match kind {
                Kind::Worlds => Some(&title_text),
                Kind::Servers if !switch => Some(&multiplayer),
                Kind::Servers => None,
            },
            tab,
            query: &ui.query,
            placeholder: &placeholder,
            search_focused: ui.search_focused,
            caret,
            rows,
            buttons: &buttons,
            open_label,
            discover_label: (kind == Kind::Servers && switch).then_some(discover),
            scanning: &scanning,
            scroll: ui.scroll,
            focus: ui.focus,
        });
        {
            let mut c = canvas();
            if c.ready() {
                c.paint_all(event.batch, &rescaled(&l.paints, unit));
            }
        }
        frames().current.covered = true;
        // The game's list under the page can't be reached: its widgets go, ours come.
        horizon_client::widgets::cover_frame();
        if l.max_scroll > 0.0 {
            horizon_client::widgets::scroll_area(
                scale_rect(l.area, unit),
                ui.scroll * unit,
                l.max_scroll * unit,
            );
        }
        for (r, h) in &l.hits {
            let in_view = r[1] >= l.area[1] - 0.5 && r[1] + r[3] <= l.area[1] + l.area[3] + 0.5;
            let name = |i: &usize| ui.names.get(i).cloned().unwrap_or_default();
            let label = match h {
                Hit::Row(i) if in_view => name(i),
                Hit::Open(i) if in_view => format!("{open_label} {}", name(i)),
                Hit::Act(a) => vanilla
                    .iter()
                    .find(|(x, ..)| x == a)
                    .map(|(.., l)| l.clone())
                    .unwrap_or_default(),
                Hit::Search => placeholder.clone(),
                Hit::Tab(Tab::MyServers) => i18n::t(T::MyServers, &lang).to_owned(),
                Hit::Tab(Tab::Discover) => discover.to_owned(),
                Hit::Discover => discover.to_owned(),
                _ => continue,
            };
            register(&label, scale_rect(*r, unit));
        }
        ui.scroll = ui.scroll.clamp(0.0, l.max_scroll);
        ui.unit = unit;
        ui.vanilla = vanilla;
        ui.layout = Some(l);
        let mut st = lock();
        *page(&mut st, kind) = ui;
    });
}

/// Presses the game's own button for `act`.
fn press_act(host: &dyn Host, kind: Kind, act: Act) {
    let at = {
        let mut st = lock();
        page(&mut st, kind)
            .vanilla
            .iter()
            .find(|(a, ..)| *a == act)
            .map(|(_, r, ..)| frame::centre(*r))
    };
    match at {
        Some([x, y]) => {
            host.press_at(x, y);
        }
        None => log::warn!("Rustic: the game's button for {act:?} is not on the screen"),
    }
}

/// Selects row `i` and opens it (Join, Play).
fn open(host: &dyn Host, kind: Kind, i: usize) {
    if host.title_select(i) {
        press_act(host, kind, Act::Primary);
    }
}

fn press(host: &dyn Host, kind: Kind, hit: Hit) {
    match hit {
        Hit::Row(i) => {
            let now = Instant::now();
            let double = {
                let mut st = lock();
                let ui = page(&mut st, kind);
                ui.search_focused = false;
                let double = ui
                    .last_click
                    .is_some_and(|(j, t)| j == i && now - t < Duration::from_millis(400));
                ui.last_click = if double { None } else { Some((i, now)) };
                double
            };
            if double {
                open(host, kind, i);
            } else {
                host.title_select(i);
            }
        }
        Hit::Open(i) => open(host, kind, i),
        Hit::Act(a) => press_act(host, kind, a),
        Hit::Search => {
            let mut st = lock();
            page(&mut st, kind).search_focused = true;
        }
        Hit::Tab(t) => {
            let mut st = lock();
            st.discover.tab = t;
            st.discover.scroll = 0.0;
            st.discover.focus = None;
            st.discover.opened = Some(Instant::now());
        }
        Hit::Discover => {
            let mut st = lock();
            st.discover.tab = Tab::Discover;
            st.discover.scroll = 0.0;
            st.discover.opened = Some(Instant::now());
        }
        Hit::Nothing => {
            let mut st = lock();
            page(&mut st, kind).search_focused = false;
        }
    }
}

pub fn mouse(
    host: &dyn Host,
    event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    kind: Kind,
) -> Flow {
    guard_or("pages.mouse", Flow::Continue, || {
        let hit = {
            let mut st = lock();
            let ui = page(&mut st, kind);
            let unit = if ui.unit > 0.0 { ui.unit } else { 1.0 };
            let at = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
            ui.focus = None;
            ui.layout.as_ref().and_then(|l| l.hit(at))
        };
        if event.button == 0
            && let Some(h) = hit
        {
            press(host, kind, h);
        }
        Flow::Cancel
    })
}

pub fn scroll(event: &mut ScreenMouseScrolledPreEvent<'_>, kind: Kind) -> Flow {
    guard_or("pages.scroll", Flow::Continue, || {
        let mut st = lock();
        let ui = page(&mut st, kind);
        let max = ui.layout.as_ref().map_or(0.0, |l| l.max_scroll);
        ui.scroll = (ui.scroll - event.scroll[1] as f32 * 24.0).clamp(0.0, max);
        Flow::Cancel
    })
}

pub fn key(host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>, kind: Kind) -> Flow {
    guard_or("pages.key", Flow::Continue, || {
        let shift = event.modifiers & 1 != 0;
        match event.key {
            "key.keyboard.escape" => {
                let mut st = lock();
                let ui = page(&mut st, kind);
                if ui.search_focused {
                    ui.search_focused = false;
                    return Flow::Cancel;
                }
                // Leaves the screen, as the game's Back.
                Flow::Continue
            }
            "key.keyboard.backspace" => {
                let mut st = lock();
                let ui = page(&mut st, kind);
                if ui.search_focused || !ui.query.is_empty() {
                    ui.query.pop();
                    ui.scroll = 0.0;
                }
                Flow::Cancel
            }
            "key.keyboard.up" | "key.keyboard.down" => {
                let (next, area_reveal) = {
                    let mut st = lock();
                    let ui = page(&mut st, kind);
                    ui.search_focused = false;
                    ui.focus = None;
                    let Some(l) = &ui.layout else {
                        return Flow::Cancel;
                    };
                    let next = lists::step(&l.cards, ui.selected, event.key == "key.keyboard.down");
                    let r =
                        next.and_then(|n| l.cards.iter().find(|(_, i)| *i == n).map(|(r, _)| *r));
                    (
                        next,
                        r.map(|r| lists::reveal(r, l.area, ui.scroll, l.max_scroll)),
                    )
                };
                if let Some(s) = area_reveal {
                    let mut st = lock();
                    page(&mut st, kind).scroll = s;
                }
                if let Some(i) = next {
                    host.title_select(i);
                }
                Flow::Cancel
            }
            "key.keyboard.tab" | "key.keyboard.left" | "key.keyboard.right" => {
                let mut st = lock();
                let ui = page(&mut st, kind);
                let Some(l) = &ui.layout else {
                    return Flow::Cancel;
                };
                let n = l.focusable().len();
                if n == 0 {
                    return Flow::Cancel;
                }
                let back = shift || event.key == "key.keyboard.left";
                ui.focus = Some(match (ui.focus, back) {
                    (None, false) => 0,
                    (None, true) => n - 1,
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                });
                ui.search_focused = false;
                Flow::Cancel
            }
            "key.keyboard.enter" | "key.keyboard.keypad.enter" => {
                let hit = {
                    let mut st = lock();
                    let ui = page(&mut st, kind);
                    let focused = ui
                        .focus
                        .and_then(|f| ui.layout.as_ref()?.focusable().get(f).cloned())
                        .map(|(_, h)| h);
                    match focused {
                        Some(h) => Some(h),
                        None if ui.selected.is_some() => Some(Hit::Act(Act::Primary)),
                        None => None,
                    }
                };
                if let Some(h) = hit {
                    press(host, kind, h);
                }
                Flow::Cancel
            }
            "key.keyboard.delete" => {
                if lock_page_selected(kind) {
                    press_act(host, kind, Act::Delete);
                }
                Flow::Cancel
            }
            "key.keyboard.f5" if kind == Kind::Servers => {
                press_act(host, kind, Act::Refresh);
                Flow::Cancel
            }
            // Nothing reaches the game's list under the page.
            _ => Flow::Cancel,
        }
    })
}

fn lock_page_selected(kind: Kind) -> bool {
    let mut st = lock();
    page(&mut st, kind).selected.is_some()
}

pub fn chars(event: &mut ScreenCharTypedPreEvent<'_>, kind: Kind) -> Flow {
    guard_or("pages.chars", Flow::Continue, || {
        let c = event.c;
        if c.is_control() {
            return Flow::Cancel;
        }
        let mut st = lock();
        let ui = page(&mut st, kind);
        // Typing anywhere searches.
        ui.search_focused = true;
        ui.focus = None;
        if ui.query.chars().count() < 64 {
            ui.query.push(c);
            ui.scroll = 0.0;
        }
        Flow::Cancel
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motds_keep_their_colours_and_lines() {
        let t = Text::legacy("§aGreen §fwhite\n§7second line ");
        let lines = text_lines(&t, palette::MUTED);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0][0].0, "Green ");
        assert_eq!(lines[0][0].1 & 0x00FF_FFFF, 0x55_FF55);
        assert_eq!(lines[0][1].1, palette::INK);
        assert_eq!(lines[1], vec![("second line".to_owned(), palette::MUTED)]);
        assert!(text_lines(&Text::literal("   "), palette::MUTED).is_empty());
    }

    #[test]
    fn each_act_presses_a_button_of_the_game() {
        for kind in [Kind::Servers, Kind::Worlds] {
            for act in acts(kind) {
                assert!(!keys(kind, *act).is_empty(), "{kind:?} {act:?}");
            }
        }
    }
}
