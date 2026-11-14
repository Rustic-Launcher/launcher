//! Rustic's language screen over the title (the menu's Language button): the game's languages
//! (`Host::languages`) as a searchable grid with flags, the one in use lit; a press applies it at
//! once as the game's Language screen does (`Host::set_language`, Rustic's Horizon patch U16),
//! and the screen stays so the change shows; Back or Escape returns to the menu.
use super::gui_measure;
use crate::canvas::canvas;
use crate::state::lock;
use crate::title_menu::TitleView;
use horizon_client::mod_api::{
    Flow, Host, ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent,
};
use rusticmc_companion_core::i18n::{self, T};
use rusticmc_companion_core::langs::{self, Hit, Key, Language};
use rusticmc_companion_core::paint::{self, rescaled, scale_rect};
use std::time::Instant;

#[derive(Default)]
pub struct LangScreen {
    pub all: Vec<Language>,
    pub query: String,
    pub scroll: f32,
    pub focus: Option<usize>,
    pub opened: Option<Instant>,
    /// The code in use, as last applied.
    pub current: String,
    pub layout: Option<langs::Layout>,
    /// Scroll to the current language at the next frame.
    pub reveal: bool,
    /// GUI pixels per layout unit at the last frame (0 before it).
    pub unit: f32,
}

/// Opens the screen over the menu.
pub fn open(host: &dyn Host) {
    let raw = host.languages();
    let mut st = lock();
    let s = &mut st.languages;
    s.all = langs::sorted(&raw);
    s.query.clear();
    s.scroll = 0.0;
    s.focus = None;
    s.opened = Some(Instant::now());
    s.current = horizon_client::lang::code();
    s.reveal = true;
    st.title.view = TitleView::Languages;
}

fn close() {
    let mut st = lock();
    st.title.view = TitleView::Menu;
    st.languages.layout = None;
}

pub fn render(_host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    let lang = horizon_client::lang::code();
    let scale = event.batch.scale.max(1.0);
    // Laid out in screen units, drawn in GUI pixels (as Discover).
    let unit = paint::screen_unit(scale, event.batch.height * scale);
    let measure = gui_measure(scale * unit);
    let mouse = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
    let mut guard = lock();
    let st = &mut *guard;
    let s = &mut st.languages;
    if s.all.is_empty() {
        // Nothing declared (no resources): back to the menu rather than an empty screen.
        st.title.view = TitleView::Menu;
        return;
    }
    let shown = langs::filter(&s.all, &s.query);
    let time = s.opened.map_or(0.0, |t| t.elapsed().as_secs_f32());
    let title = horizon_client::lang::get_or("options.language", i18n::t(T::LanguageTitle, &lang));
    let title = title
        .trim_end_matches("...")
        .trim_end_matches('…')
        .to_owned();
    let back = horizon_client::lang::get_or("gui.done", i18n::t(T::Back, &lang));
    let view = |scroll: f32| langs::View {
        gui: [event.batch.width / unit, event.batch.height / unit],
        mouse,
        shown: &shown,
        current: &s.current,
        query: &s.query,
        search_focused: true,
        caret: ((time * 2.0) as u32).is_multiple_of(2),
        scroll,
        focus: s.focus,
        title: &title,
        back: &back,
        placeholder: i18n::t(T::LanguageSearch, &lang),
        none: i18n::t(T::LanguageNone, &lang),
        measure: &measure,
    };
    let mut layout = langs::layout(&view(s.scroll));
    if s.reveal {
        if let Some(i) = shown.iter().position(|l| l.code == s.current) {
            let to = langs::scroll_to(i, layout.columns, 0.0, layout.grid[3], layout.row_step);
            let to = (to + layout.grid[3] / 2.0 - layout.row_step).clamp(0.0, layout.max_scroll);
            layout = langs::layout(&view(to));
            s.scroll = to;
        }
        s.reveal = false;
    }
    s.scroll = s.scroll.clamp(0.0, layout.max_scroll);
    let mut c = canvas();
    if c.ready() {
        c.paint_all(event.batch, &rescaled(&layout.paints, unit));
    }
    drop(c);
    if layout.max_scroll > 0.0 {
        horizon_client::widgets::scroll_area(
            scale_rect(layout.grid, unit),
            s.scroll * unit,
            layout.max_scroll * unit,
        );
    }
    for (r, h) in &layout.hits {
        let label = match h {
            langs::Hit::Pick(code) => shown
                .iter()
                .find(|l| &l.code == code)
                .map(|l| format!("{} ({})", l.name, l.region))
                .unwrap_or_default(),
            langs::Hit::Back => back.clone(),
            langs::Hit::Search => i18n::t(T::LanguageSearch, &lang).to_owned(),
        };
        super::register(&label, scale_rect(*r, unit));
    }
    let count = shown.len();
    if s.focus.is_some_and(|f| f >= count) {
        s.focus = None;
    }
    s.layout = Some(layout);
    s.unit = unit;
}

fn pick(host: &dyn Host, code: &str) {
    // A host call: no lock held.
    if host.set_language(code) {
        lock().languages.current = code.to_owned();
    } else {
        log::warn!("Rustic: the game did not take language {code}");
    }
}

pub fn mouse(host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
    if event.button != 0 {
        return Flow::Cancel;
    }
    let hit = {
        let st = lock();
        let unit = if st.languages.unit > 0.0 {
            st.languages.unit
        } else {
            1.0
        };
        let at = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
        st.languages.layout.as_ref().and_then(|l| l.hit(at))
    };
    lock().languages.focus = None;
    match hit {
        Some(Hit::Pick(code)) => pick(host, &code),
        Some(Hit::Back) => close(),
        Some(Hit::Search) | None => {}
    }
    Flow::Cancel
}

pub fn key(host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
    let (outcome, code) = {
        let st = lock();
        let s = &st.languages;
        let shown = langs::filter(&s.all, &s.query);
        let columns = s.layout.as_ref().map_or(1, |l| l.columns);
        let outcome = langs::key(event.key, s.focus, shown.len(), columns);
        let code = match outcome {
            Key::Pick(i) => shown.get(i).map(|l| l.code.clone()),
            _ => None,
        };
        (outcome, code)
    };
    match outcome {
        Key::Focus(i) => {
            let mut st = lock();
            let s = &mut st.languages;
            s.focus = Some(i);
            if let Some(l) = &s.layout {
                s.scroll = langs::scroll_to(i, l.columns, s.scroll, l.grid[3], l.row_step);
            }
        }
        Key::Pick(_) => {
            if let Some(code) = code {
                pick(host, &code);
            }
        }
        Key::Back => close(),
        Key::Erase => {
            let mut st = lock();
            st.languages.query.pop();
            st.languages.scroll = 0.0;
            st.languages.focus = None;
        }
        Key::Ignore => {}
    }
    Flow::Cancel
}

pub fn typed(_host: &dyn Host, c: char) -> Flow {
    if c.is_control() {
        return Flow::Cancel;
    }
    let mut st = lock();
    let s = &mut st.languages;
    if s.query.chars().count() < 40 {
        s.query.push(c);
        s.scroll = 0.0;
        s.focus = None;
    }
    Flow::Cancel
}

pub fn scroll(_host: &dyn Host, delta: f32) -> Flow {
    let mut st = lock();
    let s = &mut st.languages;
    let max = s.layout.as_ref().map_or(0.0, |l| l.max_scroll);
    s.scroll = (s.scroll - delta * 24.0).clamp(0.0, max);
    Flow::Cancel
}
