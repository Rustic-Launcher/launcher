//! Rustic's Resource Packs (and Data Packs), Browse Mods and Create World pages (ADR 0165), drawn
//! over the game's own screens, which still run underneath: what they show comes from them
//! (`Host::options_packs`, `Host::options_mods`, `Host::create_world`) and what their buttons do
//! is done by them (`Host::options_act`, `Host::create_world_act`, Rustic's Horizon patch U19),
//! so applying packs reloads the resources, installs are checked, and the world is created by
//! the game exactly as from its own screens. Their layouts are in the core crate
//! ([`rusticmc_companion_core::packs`], [`browse`](rusticmc_companion_core::browse),
//! [`create`](rusticmc_companion_core::create)).
use super::{gui_measure, register};
use crate::canvas::canvas;
use crate::frame::frames;
use crate::state::{guard, guard_or, lock};
use horizon_client::mod_api::{
    Flow, Host, NativeCreateAct, NativeMods, NativeOptionsAct, NativePackAct, NativePackCompat,
    NativePacks, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent,
};
use horizon_client::mod_browser::Status as BrowserStatus;
use rusticmc_companion_core::browse::{self, Filter, ModCard};
use rusticmc_companion_core::create::{self, Field, Form};
use rusticmc_companion_core::i18n::{self, T};
use rusticmc_companion_core::packs::{self, Op, PackRow};
use rusticmc_companion_core::paint::{self, Picture, rescaled, scale_rect};
use rusticmc_companion_core::title::Rgba;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// Which page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    Packs,
    Browse,
    Create,
}

/// The pack page's state.
#[derive(Default)]
pub struct PacksUi {
    pub query: String,
    pub search_focused: bool,
    pub scroll: [f32; 2],
    pub focus: Option<packs::Focus>,
    pub layout: Option<packs::Layout>,
    pub unit: f32,
    pub last_frame: Option<Instant>,
    pub opened: Option<Instant>,
    /// Pack names by id (the narrator's labels).
    pub names: HashMap<String, String>,
    /// Each pack's icon on the canvas: the icon's pointer and its key.
    pub icons: HashMap<String, (usize, String)>,
    /// The packs as last read (for the keyboard's moves).
    pub rows: [Vec<PackRow>; 2],
}

/// One project of the registry's index, as Browse Mods shows it.
#[derive(Clone, Debug, Default)]
pub struct Project {
    pub id: String,
    pub native: bool,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub categories: Vec<String>,
    pub icon: Option<String>,
    pub version: String,
}

/// The browse page's state.
#[derive(Default)]
pub struct BrowseUi {
    pub query: String,
    pub search_focused: bool,
    pub scroll: f32,
    pub focus: Option<browse::Focus>,
    pub filter: Filter,
    pub selected: Option<String>,
    pub layout: Option<browse::Layout>,
    pub unit: f32,
    pub last_frame: Option<Instant>,
    pub opened: Option<Instant>,
    pub last_click: Option<(String, Instant)>,
    /// The registry's index as the game last saved it (`mod-browser/horizon-index.json`): when
    /// the file was written and its projects.
    pub index: Option<(SystemTime, Vec<Project>)>,
    pub index_checked: Option<Instant>,
    pub names: HashMap<String, String>,
}

/// The Create World form's state.
#[derive(Default)]
pub struct CreateUi {
    pub typing: Option<Field>,
    pub scroll: f32,
    pub focus: Option<usize>,
    pub layout: Option<create::Layout>,
    pub unit: f32,
    pub last_frame: Option<Instant>,
    pub opened: Option<Instant>,
    pub form: Form,
}

#[derive(Default)]
pub struct SheetsUi {
    pub packs: PacksUi,
    pub browse: BrowseUi,
    pub create: CreateUi,
}

/// Whether Rustic draws one of its pages over the screen `screen` now.
pub fn shown(host: &dyn Host, screen: &str) -> Option<Sheet> {
    if !lock().cfg.rustic_theme {
        return None;
    }
    match screen {
        "options" => match host.options_page()? {
            "packs" | "data_packs" => host
                .options_packs()
                .is_some_and(|p| !p.confirming)
                .then_some(Sheet::Packs),
            "mod_browser" | "mod_project" => Some(Sheet::Browse),
            _ => None,
        },
        "world_create" => host
            .create_world()
            .is_some_and(|c| !c.covered)
            .then_some(Sheet::Create),
        _ => None,
    }
}

fn lang() -> String {
    horizon_client::lang::code()
}

fn g(key: &str, fallback: &str) -> String {
    horizon_client::lang::get_or(key, fallback)
}

/// The game's text `key` with its `%s` filled with `{}` (for the layouts' own filling).
fn g_fmt(key: &str, fallback: &str) -> String {
    g(key, fallback).replacen("%s", "{}", 1)
}

/// A page's geometry this frame: GUI pixels per layout unit, the page's size and the mouse in
/// layout units, and GUI pixels per layout unit times the GUI scale (for text measures).
fn geometry(event: &ScreenRenderPostEvent<'_>) -> (f32, [f32; 2], [f32; 2], f32) {
    let scale = event.batch.scale.max(1.0);
    let unit = paint::screen_unit(scale, event.batch.height * scale);
    let gui = [event.batch.width / unit, event.batch.height / unit];
    let mouse = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
    (unit, gui, mouse, scale * unit)
}

/// Opened (again) when no frame was drawn for a moment.
fn reopened(last: &mut Option<Instant>, opened: &mut Option<Instant>, now: Instant) -> bool {
    let again = last.is_none_or(|t| now - t > Duration::from_millis(500));
    if again {
        *opened = Some(now);
    }
    *last = Some(now);
    again
}

fn caret(opened: Option<Instant>, now: Instant) -> bool {
    let t = opened.map_or(0.0, |o| (now - o).as_secs_f32());
    ((t * 2.0) as u32).is_multiple_of(2)
}

fn paint(event: &mut ScreenRenderPostEvent<'_>, paints: &[paint::Paint], unit: f32) {
    let mut c = canvas();
    if c.ready() {
        c.paint_all(event.batch, &rescaled(paints, unit));
    }
    drop(c);
    frames().current.covered = true;
    // The game's widgets under the page can't be reached: ours come instead.
    horizon_client::widgets::cover_frame();
}

pub fn render(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>, sheet: Sheet) {
    // A controller's A presses Rustic's pieces, not the screen underneath.
    host.pad_clicks_to_hooks(true);
    match sheet {
        Sheet::Packs => guard("sheets.packs", || render_packs(host, event)),
        Sheet::Browse => guard("sheets.browse", || render_browse(host, event)),
        Sheet::Create => guard("sheets.create", || render_create(host, event)),
    }
}

// --- Resource Packs -------------------------------------------------------------------------

/// A pack's icon put on the canvas once per icon.
fn pack_icon(ui: &mut PacksUi, id: &str, icon: Option<&Arc<[u8]>>, size: u32) -> Option<Picture> {
    let icon = icon?;
    if icon.len() != (size * size * 4) as usize {
        return None;
    }
    let ptr = icon.as_ptr() as usize;
    if ui.icons.get(id).map(|(p, _)| *p) != Some(ptr) {
        let key = format!("pack:{id}:{ptr:x}");
        let mut c = canvas();
        if let Some((_, old)) = ui.icons.get(id) {
            c.remote.remove(old);
        }
        c.remote.insert(
            key.clone(),
            Arc::new(Rgba {
                width: size,
                height: size,
                pixels: icon.to_vec(),
            }),
        );
        ui.icons.insert(id.to_owned(), (ptr, key));
    }
    ui.icons.get(id).map(|(_, k)| Picture::Remote(k.clone()))
}

fn pack_rows(ui: &mut PacksUi, p: &NativePacks, lang: &str) -> [Vec<PackRow>; 2] {
    let warning = i18n::t(T::IncompatiblePack, lang).to_owned();
    let mut out: [Vec<PackRow>; 2] = Default::default();
    for (i, list) in [&p.available, &p.selected].into_iter().enumerate() {
        for pack in list {
            let icon = pack_icon(ui, &pack.id, pack.icon.as_ref(), pack.icon_size);
            out[i].push(PackRow {
                id: pack.id.clone(),
                name: unformatted(&pack.title.plain()),
                description: unformatted(&pack.description.plain()).replace('\n', " "),
                icon,
                warning: (pack.compat != NativePackCompat::Compatible).then(|| warning.clone()),
                can: [
                    pack.can_select,
                    pack.can_unselect,
                    pack.can_move_up,
                    pack.can_move_down,
                ],
            });
        }
    }
    out
}

/// `text` without the legacy formatting codes a pack's own text may carry (`§6`, `§r`).
fn unformatted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

fn pack_labels(lang: &str, data: bool) -> packs::Labels {
    packs::Labels {
        title: if data {
            g("dataPack.title", "Select Data Packs")
        } else {
            g("resourcePack.title", "Select Resource Packs")
        },
        available: i18n::t(T::PacksAvailable, lang).to_owned(),
        selected: i18n::t(T::PacksSelected, lang).to_owned(),
        order: i18n::t(T::LoadOrder, lang).to_owned(),
        placeholder: i18n::t(T::SearchPacks, lang).to_owned(),
        folder: i18n::t(T::OpenPackFolder, lang).to_owned(),
        cancel: g("gui.cancel", "Cancel"),
        done: g("gui.done", "Done"),
        none_available: i18n::t(T::PacksNoneAvailable, lang).to_owned(),
        none_selected: i18n::t(T::PacksNoneSelected, lang).to_owned(),
        nothing_found: i18n::t(T::NothingFound, lang).to_owned(),
    }
}

fn render_packs(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    let Some(p) = host.options_packs() else {
        return;
    };
    let now = Instant::now();
    let lang = lang();
    let (unit, gui, mouse, mscale) = geometry(event);
    let measure = gui_measure(mscale);
    let mut st = lock();
    let mut ui = std::mem::take(&mut st.sheets.packs);
    drop(st);
    if reopened(&mut ui.last_frame, &mut ui.opened, now) {
        ui.scroll = [0.0, 0.0];
        ui.focus = None;
        ui.query.clear();
        ui.search_focused = false;
    }
    let rows = pack_rows(&mut ui, &p, &lang);
    let labels = pack_labels(&lang, p.data);
    let l = packs::layout(&packs::View {
        gui,
        mouse,
        measure: &measure,
        labels: &labels,
        query: &ui.query,
        search_focused: ui.search_focused,
        caret: caret(ui.opened, now),
        available: &rows[0],
        selected: &rows[1],
        scroll: ui.scroll,
        focus: ui.focus.clone(),
    });
    paint(event, &l.paints, unit);
    ui.names = rows
        .iter()
        .flatten()
        .map(|r| (r.id.clone(), r.name.clone()))
        .collect();
    for (r, h) in &l.hits {
        let label = match h {
            packs::Hit::Card(_, id) => {
                let row = rows.iter().flatten().find(|x| &x.id == id);
                row.map_or(String::new(), |x| format!("{}, {}", x.name, x.description))
            }
            packs::Hit::Op(id, op) => {
                let name = ui.names.get(id).cloned().unwrap_or_default();
                let verb = match op {
                    Op::Add => g("gui.packSelection.select", "Select"),
                    Op::Remove => g("gui.packSelection.unselect", "Unselect"),
                    Op::Up => g("gui.packSelection.moveUp", "Move up"),
                    Op::Down => g("gui.packSelection.moveDown", "Move down"),
                };
                format!("{verb}: {name}")
            }
            packs::Hit::Search => labels.placeholder.clone(),
            packs::Hit::Folder => labels.folder.clone(),
            packs::Hit::Cancel => labels.cancel.clone(),
            packs::Hit::Done => labels.done.clone(),
            packs::Hit::Nothing => continue,
        };
        register(&label, scale_rect(*r, unit));
    }
    for i in 0..2 {
        ui.scroll[i] = ui.scroll[i].clamp(0.0, l.max_scroll[i]);
        if l.max_scroll[i] > 0.0 {
            horizon_client::widgets::scroll_area(
                scale_rect(l.areas[i], unit),
                ui.scroll[i] * unit,
                l.max_scroll[i] * unit,
            );
        }
    }
    ui.rows = rows;
    ui.unit = unit;
    ui.layout = Some(l);
    lock().sheets.packs = ui;
}

fn pack_press(host: &dyn Host, hit: packs::Hit) {
    let act = |a: NativeOptionsAct| {
        host.options_act(a);
    };
    match hit {
        packs::Hit::Card(_, id) => {
            let mut st = lock();
            st.sheets.packs.focus = Some(packs::Focus::Card(id));
            st.sheets.packs.search_focused = false;
        }
        packs::Hit::Op(id, op) => {
            let a = match op {
                Op::Add => NativePackAct::Select,
                Op::Remove => NativePackAct::Unselect,
                Op::Up => NativePackAct::Up,
                Op::Down => NativePackAct::Down,
            };
            {
                let mut st = lock();
                st.sheets.packs.focus = Some(packs::Focus::Card(id.clone()));
            }
            act(NativeOptionsAct::Pack(id, a));
        }
        packs::Hit::Search => {
            let mut st = lock();
            st.sheets.packs.search_focused = true;
        }
        packs::Hit::Folder => act(NativeOptionsAct::PackFolder),
        packs::Hit::Cancel => {
            act(NativeOptionsAct::PackRestore);
            act(NativeOptionsAct::Leave);
        }
        packs::Hit::Done => act(NativeOptionsAct::Leave),
        packs::Hit::Nothing => lock().sheets.packs.search_focused = false,
    }
}

/// The focused card's side and id.
fn focused_card(ui: &PacksUi) -> Option<(usize, String)> {
    let Some(packs::Focus::Card(id)) = &ui.focus else {
        return None;
    };
    let side = ui.rows.iter().position(|r| r.iter().any(|x| &x.id == id))?;
    Some((side, id.clone()))
}

fn packs_key(host: &dyn Host, event: &ScreenKeyPressedPreEvent<'_>) -> Flow {
    let shift = event.modifiers & 1 != 0;
    let key = event.key;
    let mut st = lock();
    let ui = &mut st.sheets.packs;
    match key {
        "key.keyboard.escape" => {
            if ui.search_focused || !ui.query.is_empty() {
                ui.search_focused = false;
                ui.query.clear();
                return Flow::Cancel;
            }
            drop(st);
            // As the game's own: leaving applies the selection.
            host.options_act(NativeOptionsAct::Leave);
        }
        "key.keyboard.backspace" => {
            ui.query.pop();
            ui.scroll = [0.0, 0.0];
        }
        "key.keyboard.up" | "key.keyboard.down" => {
            let down = key == "key.keyboard.down";
            ui.search_focused = false;
            if shift && let Some((1, id)) = focused_card(ui) {
                drop(st);
                let a = if down {
                    NativePackAct::Down
                } else {
                    NativePackAct::Up
                };
                host.options_act(NativeOptionsAct::Pack(id, a));
                return Flow::Cancel;
            }
            let Some(l) = &ui.layout else {
                return Flow::Cancel;
            };
            let side = focused_card(ui).map_or(0, |(s, _)| s);
            let current = focused_card(ui).map(|(_, id)| id);
            if let Some(next) = packs::step(&l.cards[side], current.as_deref(), down) {
                if let Some((r, _)) = l.cards[side].iter().find(|(_, id)| *id == next) {
                    let area = l.areas[side];
                    ui.scroll[side] = rusticmc_companion_core::lists::reveal(
                        *r,
                        area,
                        ui.scroll[side],
                        l.max_scroll[side],
                    );
                }
                ui.focus = Some(packs::Focus::Card(next));
            }
        }
        "key.keyboard.left" | "key.keyboard.right" => {
            ui.search_focused = false;
            let to = usize::from(key == "key.keyboard.right");
            if let Some(l) = &ui.layout
                && let Some(first) = l.cards[to].first()
            {
                ui.focus = Some(packs::Focus::Card(first.1.clone()));
            }
        }
        "key.keyboard.tab" => {
            ui.search_focused = false;
            let at = ui
                .focus
                .as_ref()
                .and_then(|f| packs::STOPS.iter().position(|s| s == f));
            let next = super::sheets_step(at, packs::STOPS.len(), shift);
            ui.focus = next.map(|i| packs::STOPS[i].clone());
        }
        "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.space" => {
            let hit = match ui.focus.clone() {
                Some(packs::Focus::Search) => {
                    ui.search_focused = true;
                    None
                }
                Some(packs::Focus::Folder) => Some(packs::Hit::Folder),
                Some(packs::Focus::Cancel) => Some(packs::Hit::Cancel),
                Some(packs::Focus::Done) => Some(packs::Hit::Done),
                Some(packs::Focus::Card(_)) => focused_card(ui).map(|(side, id)| {
                    packs::Hit::Op(id, if side == 0 { Op::Add } else { Op::Remove })
                }),
                None => None,
            };
            drop(st);
            if let Some(h) = hit {
                pack_press(host, h);
            }
        }
        _ => {}
    }
    Flow::Cancel
}

// --- Browse Mods ----------------------------------------------------------------------------

/// The registry's index the game saved (`mod-browser/horizon-index.json` in the game folder):
/// its mods and native mods.
fn read_index(path: &std::path::Path) -> Option<Vec<Project>> {
    let bytes = std::fs::read(path).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let s = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_owned();
    let list = |v: &serde_json::Value| -> Vec<String> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| x.as_str().map(str::to_owned))
            .collect()
    };
    Some(
        v["projects"]
            .as_array()?
            .iter()
            .filter_map(|p| {
                let native = match p["kind"].as_str()? {
                    "mod" => false,
                    "native-mod" => true,
                    _ => return None,
                };
                Some(Project {
                    id: s(&p["id"]),
                    native,
                    name: s(&p["name"]),
                    summary: s(&p["summary"]),
                    description: s(&p["description"]),
                    authors: list(&p["authors"]),
                    license: s(&p["license"]),
                    categories: list(&p["categories"]),
                    icon: p["icon"]
                        .as_str()
                        .filter(|u| u.starts_with("https://"))
                        .map(str::to_owned),
                    version: p["versions"]
                        .as_array()
                        .and_then(|vs| vs.first())
                        .map(|v| s(&v["version"]))
                        .unwrap_or_default(),
                })
            })
            .collect(),
    )
}

fn index_path(host: &dyn Host) -> PathBuf {
    host.instance_dir()
        .join("mod-browser")
        .join("horizon-index.json")
}

/// The markdown description as plain text on one line.
fn plain(markdown: &str) -> String {
    horizon_client::mod_browser::plain_lines(markdown).join(" ")
}

fn mod_cards(ui: &mut BrowseUi, mods: &NativeMods, want_icon: &mut Vec<String>) -> Vec<ModCard> {
    let b = &mods.browser;
    let projects: &[Project] = ui.index.as_ref().map_or(&[], |(_, p)| p);
    let by_id: HashMap<&str, &Project> = projects.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut out = Vec::new();
    let mut icon = |id: &str| -> Option<Picture> {
        let url = by_id.get(id)?.icon.clone()?;
        let have = canvas().remote.contains_key(&url);
        if !have {
            want_icon.push(url.clone());
        }
        have.then_some(Picture::Remote(url))
    };
    // The content mods this game installs (the game's own listings).
    for l in &b.listings {
        let installed = mods.installed.iter().find(|m| m.id == l.id && !m.dev);
        out.push(ModCard {
            id: l.id.clone(),
            name: l.name.clone(),
            version: l.version.clone(),
            authors: l.authors.join(", "),
            summary: l.summary.clone(),
            description: plain(&l.description),
            license: l.license.clone(),
            categories: l.categories.clone(),
            icon: icon(&l.id),
            native: false,
            installed: l.installed.clone(),
            update: l.has_update(),
            on: installed.filter(|_| l.installed.is_some()).map(|m| m.on),
            installing: b.installing.as_deref() == Some(l.id.as_str()),
        });
    }
    // The registry's native mods: built into the game by the launcher.
    for p in projects.iter().filter(|p| p.native) {
        let installed = mods.installed.iter().any(|m| m.native && m.id == p.id);
        out.push(ModCard {
            id: p.id.clone(),
            name: p.name.clone(),
            version: p.version.clone(),
            authors: p.authors.join(", "),
            summary: p.summary.clone(),
            description: plain(&p.description),
            license: p.license.clone(),
            categories: p.categories.clone(),
            icon: icon(&p.id),
            native: true,
            installed: installed.then(String::new),
            update: false,
            on: None,
            installing: false,
        });
    }
    out
}

fn browse_labels(lang: &str) -> browse::Labels {
    browse::Labels {
        brand: "Rustic".into(),
        title: g("horizon.mods.browse.title", "Browse Mods"),
        placeholder: g("horizon.mods.browse.search", "Search mods...")
            .trim_end_matches('…')
            .trim_end_matches("...")
            .to_owned(),
        all: i18n::t(T::All, lang).to_owned(),
        installed: g("horizon.mods.browse.installed", "Installed"),
        updates: i18n::t(T::Updates, lang).to_owned(),
        kinds: i18n::t(T::Kind, lang).to_owned(),
        content: i18n::t(T::KindContent, lang).to_owned(),
        native: i18n::t(T::OriginNative, lang).to_owned(),
        categories: i18n::t(T::Categories, lang).to_owned(),
        done: g("gui.done", "Done"),
        install: g("horizon.mods.browse.install", "Install"),
        update: g("horizon.mods.browse.update", "Update"),
        installing: g("horizon.mods.browse.installing", "Installing..."),
        up_to_date: g("horizon.mods.browse.installed", "Installed"),
        update_badge: g("horizon.mods.browse.update", "Update"),
        build: i18n::t(T::BuildInLauncher, lang).to_owned(),
        open_page: g("horizon.mods.browse.open_page", "Open Page"),
        enabled: i18n::t(T::Enabled, lang).to_owned(),
        by: g_fmt("horizon.mods.browse.by", "by %s"),
        loading: g(
            "horizon.mods.browse.loading",
            "Reading Horizon's mod registry...",
        ),
        failed: g(
            "horizon.mods.browse.failed",
            "Couldn't read the mod registry: %s",
        )
        .replace("%s", "")
        .trim()
        .to_owned(),
        nothing_found: i18n::t(T::NothingFound, lang).to_owned(),
        count: i18n::t(T::ModsCount, lang).to_owned(),
    }
}

fn render_browse(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    let Some(mods) = host.options_mods() else {
        return;
    };
    let now = Instant::now();
    let lang = lang();
    let (unit, gui, mouse, mscale) = geometry(event);
    let measure = gui_measure(mscale);
    let path = index_path(host);
    let mut st = lock();
    let mut ui = std::mem::take(&mut st.sheets.browse);
    drop(st);
    if reopened(&mut ui.last_frame, &mut ui.opened, now) {
        ui.scroll = 0.0;
        ui.focus = None;
        ui.search_focused = false;
    }
    // The index the game saved, read again when it changes (looked at every 2 s).
    if ui
        .index_checked
        .is_none_or(|t| now - t > Duration::from_secs(2))
    {
        ui.index_checked = Some(now);
        let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if let Some(stamp) = stamp
            && ui.index.as_ref().map(|(s, _)| *s) != Some(stamp)
            && let Some(projects) = read_index(&path)
        {
            ui.index = Some((stamp, projects));
        }
    }
    let mut want = Vec::new();
    let cards = mod_cards(&mut ui, &mods, &mut want);
    if !want.is_empty() {
        let mut st = lock();
        for url in want.iter().take(4) {
            st.directory.want_picture(url, false);
        }
    }
    let categories = browse::top_categories(&cards, 12);
    let labels = browse_labels(&lang);
    let status = match &mods.browser.status {
        BrowserStatus::Ready => browse::Status::Ready,
        BrowserStatus::Failed(why) => browse::Status::Failed(why.clone()),
        _ if !cards.is_empty() => browse::Status::Ready,
        _ => browse::Status::Loading,
    };
    let message = mods
        .browser
        .message
        .as_ref()
        .map(|(m, ok)| (m.as_str(), *ok));
    let filter = ui.filter.clone();
    let l = browse::layout(&browse::View {
        gui,
        mouse,
        measure: &measure,
        labels: &labels,
        query: &ui.query,
        search_focused: ui.search_focused,
        caret: caret(ui.opened, now),
        cards: &cards,
        filter: &filter,
        categories: &categories,
        selected: ui.selected.as_deref(),
        scroll: ui.scroll,
        focus: ui.focus.clone(),
        status,
        message,
    });
    paint(event, &l.paints, unit);
    ui.names = cards
        .iter()
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();
    for (r, h) in &l.hits {
        let name = |id: &String| ui.names.get(id).cloned().unwrap_or_default();
        let label = match h {
            browse::Hit::Card(id) => {
                let c = cards.iter().find(|c| &c.id == id);
                c.map_or(String::new(), |c| format!("{}, {}", c.name, c.summary))
            }
            browse::Hit::Install(id) => format!("{}: {}", labels.install, name(id)),
            browse::Hit::Toggle(id) => format!("{}: {}", labels.enabled, name(id)),
            browse::Hit::Page(id) => format!("{}: {}", labels.open_page, name(id)),
            browse::Hit::Build(id) => format!("{}: {}", labels.build, name(id)),
            browse::Hit::Filter(f) => match f {
                Filter::All => labels.all.clone(),
                Filter::Installed => labels.installed.clone(),
                Filter::Updates => labels.updates.clone(),
                Filter::Content => labels.content.clone(),
                Filter::Native => labels.native.clone(),
                Filter::Category(c) => c.clone(),
            },
            browse::Hit::Search => labels.placeholder.clone(),
            browse::Hit::Done => labels.done.clone(),
            browse::Hit::Nothing => continue,
        };
        register(&label, scale_rect(*r, unit));
    }
    ui.scroll = ui.scroll.clamp(0.0, l.max_scroll);
    if l.max_scroll > 0.0 {
        horizon_client::widgets::scroll_area(
            scale_rect(l.area, unit),
            ui.scroll * unit,
            l.max_scroll * unit,
        );
    }
    ui.unit = unit;
    ui.layout = Some(l);
    lock().sheets.browse = ui;
}

fn browse_press(host: &dyn Host, hit: browse::Hit) {
    match hit {
        browse::Hit::Filter(f) => {
            let mut st = lock();
            let ui = &mut st.sheets.browse;
            ui.filter = f;
            ui.scroll = 0.0;
            ui.search_focused = false;
        }
        browse::Hit::Card(id) => {
            let now = Instant::now();
            let mut st = lock();
            let ui = &mut st.sheets.browse;
            ui.search_focused = false;
            let double = ui
                .last_click
                .as_ref()
                .is_some_and(|(j, t)| *j == id && now - *t < Duration::from_millis(400));
            ui.last_click = if double {
                None
            } else {
                Some((id.clone(), now))
            };
            ui.selected = Some(id.clone());
            ui.focus = Some(browse::Focus::Card(id.clone()));
            drop(st);
            if double {
                host.options_act(NativeOptionsAct::InstallMod(id));
            }
        }
        browse::Hit::Install(id) => {
            host.options_act(NativeOptionsAct::InstallMod(id));
        }
        browse::Hit::Toggle(id) => {
            host.options_act(NativeOptionsAct::ToggleMod(id));
        }
        browse::Hit::Page(id) => {
            host.options_act(NativeOptionsAct::ModPage(id));
        }
        browse::Hit::Build(_) => crate::title_menu::open_url(super::FIND_MODS_URL.to_owned()),
        browse::Hit::Search => lock().sheets.browse.search_focused = true,
        browse::Hit::Done => {
            host.options_act(NativeOptionsAct::Leave);
        }
        browse::Hit::Nothing => lock().sheets.browse.search_focused = false,
    }
}

fn browse_key(host: &dyn Host, event: &ScreenKeyPressedPreEvent<'_>) -> Flow {
    let shift = event.modifiers & 1 != 0;
    let key = event.key;
    let mut st = lock();
    let ui = &mut st.sheets.browse;
    match key {
        "key.keyboard.escape" => {
            if ui.search_focused || !ui.query.is_empty() {
                ui.search_focused = false;
                ui.query.clear();
                return Flow::Cancel;
            }
            drop(st);
            host.options_act(NativeOptionsAct::Leave);
        }
        "key.keyboard.backspace" => {
            ui.query.pop();
            ui.scroll = 0.0;
        }
        "key.keyboard.up" | "key.keyboard.down" | "key.keyboard.left" | "key.keyboard.right" => {
            ui.search_focused = false;
            let Some(l) = &ui.layout else {
                return Flow::Cancel;
            };
            let cols = l.columns.max(1) as isize;
            let delta = match key {
                "key.keyboard.up" => -cols,
                "key.keyboard.down" => cols,
                "key.keyboard.left" => -1,
                _ => 1,
            };
            let current = match &ui.focus {
                Some(browse::Focus::Card(id)) => Some(id.clone()),
                _ => ui.selected.clone(),
            };
            if let Some(next) = browse::step(&l.cards, current.as_deref(), delta) {
                if let Some((r, _)) = l.cards.iter().find(|(_, id)| *id == next) {
                    ui.scroll =
                        rusticmc_companion_core::lists::reveal(*r, l.area, ui.scroll, l.max_scroll);
                }
                ui.selected = Some(next.clone());
                ui.focus = Some(browse::Focus::Card(next));
            }
        }
        "key.keyboard.tab" => {
            ui.search_focused = false;
            let Some(l) = &ui.layout else {
                return Flow::Cancel;
            };
            let at = ui
                .focus
                .as_ref()
                .and_then(|f| l.stops.iter().position(|(_, s, _)| s == f));
            ui.focus = super::sheets_step(at, l.stops.len(), shift).map(|i| l.stops[i].1.clone());
        }
        "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.space" => {
            let hit = match (&ui.focus, &ui.layout) {
                (Some(browse::Focus::Search), _) => {
                    ui.search_focused = true;
                    None
                }
                (Some(browse::Focus::Card(_)), Some(l)) => l
                    .stops
                    .iter()
                    .find(|(_, s, _)| *s == browse::Focus::Primary)
                    .map(|(_, _, h)| h.clone()),
                (Some(f), Some(l)) => l
                    .stops
                    .iter()
                    .find(|(_, s, _)| s == f)
                    .map(|(_, _, h)| h.clone()),
                _ => None,
            };
            drop(st);
            if let Some(h) = hit {
                browse_press(host, h);
            }
        }
        _ => {}
    }
    Flow::Cancel
}

// --- Create World ---------------------------------------------------------------------------

fn create_labels() -> create::Labels {
    let mode = |k: &str, f: &str| g(&format!("selectWorld.gameMode.{k}"), f);
    let difficulty = |k: &str, f: &str| g(&format!("options.difficulty.{k}"), f);
    create::Labels {
        title: g("selectWorld.create", "Create New World"),
        game: g("createWorld.tab.game.title", "Game"),
        world: g("createWorld.tab.world.title", "World"),
        more: g("createWorld.tab.more.title", "More"),
        name: g("selectWorld.enterName", "World Name"),
        game_mode: g("selectWorld.gameMode", "Game Mode"),
        modes: [
            mode("survival", "Survival"),
            mode("hardcore", "Hardcore"),
            mode("creative", "Creative"),
        ],
        difficulty: g("options.difficulty", "Difficulty"),
        difficulties: [
            difficulty("peaceful", "Peaceful"),
            difficulty("easy", "Easy"),
            difficulty("normal", "Normal"),
            difficulty("hard", "Hard"),
        ],
        commands: g("selectWorld.allowCommands", "Allow Commands"),
        seed: g("selectWorld.enterSeed", "Seed for the world generator"),
        seed_hint: g("selectWorld.seedInfo", "Leave blank for a random seed"),
        world_type: g("selectWorld.mapType", "World Type"),
        customize: g("selectWorld.customizeType", "Customize"),
        structures: g("selectWorld.mapFeatures", "Generate Structures"),
        bonus_chest: g("selectWorld.bonusItems", "Bonus Chest"),
        game_rules: g("selectWorld.gameRules", "Game Rules"),
        experiments: g("selectWorld.experiments", "Experiments"),
        data_packs: g("selectWorld.dataPacks", "Data Packs"),
        create: g("selectWorld.create", "Create New World"),
        cancel: g("gui.cancel", "Cancel"),
    }
}

fn render_create(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    let Some(c) = host.create_world() else {
        return;
    };
    let now = Instant::now();
    let (unit, gui, mouse, mscale) = geometry(event);
    let measure = gui_measure(mscale);
    let mut st = lock();
    let mut ui = std::mem::take(&mut st.sheets.create);
    drop(st);
    if reopened(&mut ui.last_frame, &mut ui.opened, now) {
        ui.scroll = 0.0;
        ui.focus = None;
        // The name takes the keyboard, as the game's own screen opens.
        ui.typing = Some(Field::Name);
        host.create_world_act(NativeCreateAct::Focus(0));
    }
    let difficulty = ["peaceful", "easy", "normal", "hard"]
        .iter()
        .position(|d| *d == c.difficulty)
        .unwrap_or(2) as u8;
    let form = Form {
        name: c.name.clone(),
        seed: c.seed.clone(),
        game_mode: c.game_mode,
        difficulty,
        commands: c.commands,
        world_type: g(c.world_type, c.world_type),
        customizable: c.customizable,
        structures: c.structures,
        bonus_chest: c.bonus_chest,
        hardcore: c.game_mode == 1,
        data_packs: c.data_packs,
    };
    let labels = create_labels();
    let l = create::layout(&create::View {
        gui,
        mouse,
        measure: &measure,
        labels: &labels,
        form: &form,
        typing: ui.typing,
        caret: caret(ui.opened, now),
        scroll: ui.scroll,
        focus: ui.focus,
    });
    paint(event, &l.paints, unit);
    for (r, h) in &l.hits {
        let label = match h {
            create::Hit::Field(Field::Name) => format!("{}: {}", labels.name, form.name),
            create::Hit::Field(Field::Seed) => format!("{}: {}", labels.seed, form.seed),
            create::Hit::Mode(m) => format!("{}: {}", labels.game_mode, labels.modes[*m as usize]),
            create::Hit::Difficulty(d) => {
                format!(
                    "{}: {}",
                    labels.difficulty, labels.difficulties[*d as usize]
                )
            }
            create::Hit::Commands => labels.commands.clone(),
            create::Hit::TypePrev | create::Hit::TypeNext => {
                format!("{}: {}", labels.world_type, form.world_type)
            }
            create::Hit::Customize => labels.customize.clone(),
            create::Hit::Structures => labels.structures.clone(),
            create::Hit::BonusChest => labels.bonus_chest.clone(),
            create::Hit::GameRules => labels.game_rules.clone(),
            create::Hit::Experiments => labels.experiments.clone(),
            create::Hit::DataPacks => labels.data_packs.clone(),
            create::Hit::Create => labels.create.clone(),
            create::Hit::Cancel => labels.cancel.clone(),
            create::Hit::Nothing => continue,
        };
        register(&label, scale_rect(*r, unit));
    }
    ui.scroll = ui.scroll.clamp(0.0, l.max_scroll);
    if l.max_scroll > 0.0 {
        horizon_client::widgets::scroll_area(
            scale_rect(l.area, unit),
            ui.scroll * unit,
            l.max_scroll * unit,
        );
    }
    ui.form = form;
    ui.unit = unit;
    ui.layout = Some(l);
    lock().sheets.create = ui;
}

fn create_press(host: &dyn Host, hit: create::Hit) {
    let press = |id: u8, step: i32| {
        host.create_world_act(NativeCreateAct::Press(id, step));
    };
    let form = {
        let mut st = lock();
        let ui = &mut st.sheets.create;
        if !matches!(hit, create::Hit::Field(_)) && ui.typing.take().is_some() {
            host.create_world_act(NativeCreateAct::Focus(2));
        }
        ui.form.clone()
    };
    match hit {
        create::Hit::Field(f) => focus_field(host, Some(f)),
        create::Hit::Mode(m) => {
            for _ in 0..create::steps(form.game_mode, m, 3) {
                press(20, 1);
            }
        }
        create::Hit::Difficulty(d) => {
            for _ in 0..create::steps(form.difficulty, d, 4) {
                press(23, 1);
            }
        }
        create::Hit::Commands => press(24, 1),
        create::Hit::TypePrev => press(25, -1),
        create::Hit::TypeNext => press(25, 1),
        create::Hit::Customize => press(26, 1),
        create::Hit::Structures => press(27, 1),
        create::Hit::BonusChest => press(28, 1),
        create::Hit::GameRules => press(29, 1),
        create::Hit::Experiments => press(30, 1),
        create::Hit::DataPacks => press(31, 1),
        create::Hit::Create => press(21, 1),
        create::Hit::Cancel => press(22, 1),
        create::Hit::Nothing => {}
    }
}

/// Gives the keyboard to one of the form's fields (the game's own: typing, paste, selection and
/// the cursor keys are its), or to none.
fn focus_field(host: &dyn Host, field: Option<Field>) {
    lock().sheets.create.typing = field;
    host.create_world_act(NativeCreateAct::Focus(match field {
        Some(Field::Name) => 0,
        Some(Field::Seed) => 1,
        None => 2,
    }));
}

fn create_key(host: &dyn Host, event: &ScreenKeyPressedPreEvent<'_>) -> Flow {
    let shift = event.modifiers & 1 != 0;
    let key = event.key;
    let typing = lock().sheets.create.typing.is_some();
    match key {
        "key.keyboard.escape" => {
            if typing {
                focus_field(host, None);
            } else {
                create_press(host, create::Hit::Cancel);
            }
        }
        "key.keyboard.tab" | "key.keyboard.up" | "key.keyboard.down" => {
            let field = {
                let mut st = lock();
                let ui = &mut st.sheets.create;
                let Some(l) = &ui.layout else {
                    return Flow::Cancel;
                };
                let back = shift || key == "key.keyboard.up";
                let next = super::sheets_step(ui.focus, l.stops.len(), back);
                ui.focus = next;
                if let Some(i) = next {
                    let r = l.stops[i].0;
                    ui.scroll =
                        rusticmc_companion_core::lists::reveal(r, l.area, ui.scroll, l.max_scroll);
                }
                next.and_then(|i| match l.stops[i].1 {
                    create::Hit::Field(f) => Some(f),
                    _ => None,
                })
            };
            focus_field(host, field);
        }
        // The game's own field edits its text (backspace, the cursor keys, copy and paste...).
        _ if typing && !matches!(key, "key.keyboard.enter" | "key.keyboard.keypad.enter") => {
            return Flow::Continue;
        }
        "key.keyboard.left" | "key.keyboard.right" if !typing => {
            let right = key == "key.keyboard.right";
            let hit = {
                let st = lock();
                let ui = &st.sheets.create;
                ui.focus
                    .and_then(|i| ui.layout.as_ref()?.stops.get(i).map(|s| s.1.clone()))
            };
            let step = if right { 1 } else { -1 };
            match hit {
                Some(create::Hit::Mode(_)) => {
                    host.create_world_act(NativeCreateAct::Press(20, step));
                }
                Some(create::Hit::Difficulty(_)) => {
                    host.create_world_act(NativeCreateAct::Press(23, step));
                }
                Some(create::Hit::TypeNext) => {
                    host.create_world_act(NativeCreateAct::Press(25, step));
                }
                _ => {}
            }
        }
        "key.keyboard.enter" | "key.keyboard.keypad.enter" => {
            let hit = {
                let mut st = lock();
                let ui = &mut st.sheets.create;
                if ui.typing.is_some() && ui.focus.is_none() {
                    // Enter in the name field with nothing focused: create, as the game's.
                    Some(create::Hit::Create)
                } else {
                    ui.focus
                        .and_then(|i| ui.layout.as_ref()?.stops.get(i).map(|s| s.1.clone()))
                }
            };
            if let Some(h) = hit {
                create_press(host, h);
            }
        }
        "key.keyboard.space" if !typing => {
            let hit = {
                let st = lock();
                let ui = &st.sheets.create;
                ui.focus
                    .and_then(|i| ui.layout.as_ref()?.stops.get(i).map(|s| s.1.clone()))
            };
            if let Some(h) = hit {
                create_press(host, h);
            }
        }
        _ => {}
    }
    Flow::Cancel
}

// --- Input ----------------------------------------------------------------------------------

pub fn mouse(host: &dyn Host, event: &ScreenMouseButtonPressedPreEvent<'_>, sheet: Sheet) -> Flow {
    guard_or("sheets.mouse", Flow::Cancel, || {
        let at = |unit: f32| {
            let unit = if unit > 0.0 { unit } else { 1.0 };
            [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit]
        };
        if event.button != 0 {
            return Flow::Cancel;
        }
        match sheet {
            Sheet::Packs => {
                let hit = {
                    let mut st = lock();
                    let ui = &mut st.sheets.packs;
                    ui.search_focused = false;
                    ui.layout.as_ref().and_then(|l| l.hit(at(ui.unit)))
                };
                if let Some(h) = hit {
                    pack_press(host, h);
                }
            }
            Sheet::Browse => {
                let hit = {
                    let mut st = lock();
                    let ui = &mut st.sheets.browse;
                    ui.search_focused = false;
                    ui.layout.as_ref().and_then(|l| l.hit(at(ui.unit)))
                };
                if let Some(h) = hit {
                    browse_press(host, h);
                }
            }
            Sheet::Create => {
                let hit = {
                    let st = lock();
                    let ui = &st.sheets.create;
                    ui.layout.as_ref().and_then(|l| l.hit(at(ui.unit)))
                };
                match hit {
                    Some(h) => create_press(host, h),
                    None => lock().sheets.create.typing = None,
                }
            }
        }
        Flow::Cancel
    })
}

pub fn scroll(event: &ScreenMouseScrolledPreEvent<'_>, sheet: Sheet) -> Flow {
    guard_or("sheets.scroll", Flow::Cancel, || {
        let dy = event.scroll[1] as f32 * 24.0;
        let mut st = lock();
        match sheet {
            Sheet::Packs => {
                let ui = &mut st.sheets.packs;
                let unit = if ui.unit > 0.0 { ui.unit } else { 1.0 };
                let at = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
                if let Some(l) = &ui.layout
                    && let Some(i) = l.side_at(at)
                {
                    ui.scroll[i] = (ui.scroll[i] - dy).clamp(0.0, l.max_scroll[i]);
                }
            }
            Sheet::Browse => {
                let ui = &mut st.sheets.browse;
                let max = ui.layout.as_ref().map_or(0.0, |l| l.max_scroll);
                ui.scroll = (ui.scroll - dy).clamp(0.0, max);
            }
            Sheet::Create => {
                let ui = &mut st.sheets.create;
                let max = ui.layout.as_ref().map_or(0.0, |l| l.max_scroll);
                ui.scroll = (ui.scroll - dy).clamp(0.0, max);
            }
        }
        Flow::Cancel
    })
}

pub fn key(host: &dyn Host, event: &ScreenKeyPressedPreEvent<'_>, sheet: Sheet) -> Flow {
    guard_or("sheets.key", Flow::Cancel, || match sheet {
        Sheet::Packs => packs_key(host, event),
        Sheet::Browse => browse_key(host, event),
        Sheet::Create => create_key(host, event),
    })
}

pub fn chars(_host: &dyn Host, event: &ScreenCharTypedPreEvent<'_>, sheet: Sheet) -> Flow {
    guard_or("sheets.chars", Flow::Cancel, || {
        let c = event.c;
        if sheet == Sheet::Create {
            // The game's own field types it (see `focus_field`).
            return Flow::Continue;
        }
        if c.is_control() {
            return Flow::Cancel;
        }
        match sheet {
            Sheet::Packs => {
                let mut st = lock();
                let ui = &mut st.sheets.packs;
                ui.search_focused = true;
                if ui.query.chars().count() < 64 {
                    ui.query.push(c);
                    ui.scroll = [0.0, 0.0];
                }
            }
            Sheet::Browse => {
                let mut st = lock();
                let ui = &mut st.sheets.browse;
                ui.search_focused = true;
                if ui.query.chars().count() < 64 {
                    ui.query.push(c);
                    ui.scroll = 0.0;
                }
            }
            Sheet::Create => {}
        }
        Flow::Cancel
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_codes_are_left_out() {
        assert_eq!(unformatted("§6Warm§r menus"), "Warm menus");
        assert_eq!(unformatted("plain"), "plain");
        assert_eq!(unformatted("end§"), "end");
    }

    #[test]
    fn the_registry_index_gives_mods_and_native_mods() {
        let dir = std::env::temp_dir().join(format!("rustic-index-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("horizon-index.json");
        std::fs::write(
            &path,
            r#"{"projects":[
                {"id":"a","kind":"mod","name":"A","authors":["Ann"],"categories":["utility"],
                 "icon":"https://example.invalid/a.png","versions":[{"version":"1.2"}]},
                {"id":"b","kind":"native-mod","name":"B","icon":"http://insecure.invalid/b.png"},
                {"id":"s","kind":"shader","name":"S"}]}"#,
        )
        .unwrap();
        let p = read_index(&path).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].version, "1.2");
        assert_eq!(p[0].icon.as_deref(), Some("https://example.invalid/a.png"));
        assert!(p[1].native && p[1].icon.is_none(), "https only");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(read_index(&path).is_none());
    }
}
