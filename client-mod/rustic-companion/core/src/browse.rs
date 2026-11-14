//! Rustic's Browse Mods page (ADR 0165), in the settings screens' layout: a sidebar ("Rustic /
//! Browse mods", the search field, filters: All, Installed, Updates, the kinds, the categories;
//! Done at its foot) and a main panel with a grid of cards (three columns, two or four by width:
//! the mod's icon, name, version, author, two lines of summary, badges Installed, Update,
//! Native) over a details panel for the hovered or selected mod (its description, author,
//! version, licence, and Install / Update, its switch, its page; a native mod is built from the
//! launcher). Pure: the game crate gives the mods and does the presses through the game's own
//! browser (Rustic's Horizon patch U19).
use crate::kit::{Kit, Look, fold, line, max_scroll};
use crate::lists::{picture, plate};
use crate::paint::{Align, Measure, Paint, Picture, inside, wrap};
use crate::sheet;
use crate::skin::palette;
use crate::title::{Glyph, Weight};

/// One mod as a card shows it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModCard {
    pub id: String,
    pub name: String,
    pub version: String,
    pub authors: String,
    pub summary: String,
    pub description: String,
    pub license: String,
    pub categories: Vec<String>,
    pub icon: Option<Picture>,
    /// A native mod: built into the game by the launcher, not installed here.
    pub native: bool,
    /// The version installed, if any.
    pub installed: Option<String>,
    /// Installed, and the registry has a newer version.
    pub update: bool,
    /// Its switch, when installed (content mods).
    pub on: Option<bool>,
    /// Its install runs now.
    pub installing: bool,
}

/// A sidebar filter.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Installed,
    Updates,
    Content,
    Native,
    Category(String),
}

impl Filter {
    pub fn keeps(&self, c: &ModCard) -> bool {
        match self {
            Filter::All => true,
            Filter::Installed => c.installed.is_some(),
            Filter::Updates => c.update,
            Filter::Content => !c.native,
            Filter::Native => c.native,
            Filter::Category(cat) => c.categories.iter().any(|x| x == cat),
        }
    }
}

/// What a press lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Filter(Filter),
    /// A card: selects it (a second press soon after installs it).
    Card(String),
    Install(String),
    Toggle(String),
    Page(String),
    /// A native mod: the launcher builds it.
    Build(String),
    Search,
    Done,
    Nothing,
}

/// Where the keyboard is (Tab's stops; the arrows walk the grid).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    Search,
    Filter(usize),
    Card(String),
    Primary,
    Switch,
    Page,
    Done,
}

/// How the registry's index reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Loading,
    Ready,
    Failed(String),
}

/// The page's words, already translated.
#[derive(Clone, Debug, Default)]
pub struct Labels {
    pub brand: String,
    pub title: String,
    pub placeholder: String,
    pub all: String,
    pub installed: String,
    pub updates: String,
    pub kinds: String,
    pub content: String,
    pub native: String,
    pub categories: String,
    pub done: String,
    pub install: String,
    pub update: String,
    pub installing: String,
    pub up_to_date: String,
    pub update_badge: String,
    pub build: String,
    pub open_page: String,
    pub enabled: String,
    /// `{}` is the authors.
    pub by: String,
    pub loading: String,
    pub failed: String,
    pub nothing_found: String,
    /// `{n}` mods.
    pub count: String,
}

/// What the page shows this frame.
pub struct View<'a> {
    pub gui: [f32; 2],
    pub mouse: [f32; 2],
    pub measure: Measure<'a>,
    pub labels: &'a Labels,
    pub query: &'a str,
    pub search_focused: bool,
    pub caret: bool,
    pub cards: &'a [ModCard],
    pub filter: &'a Filter,
    /// The categories offered in the sidebar, most used first.
    pub categories: &'a [String],
    pub selected: Option<&'a str>,
    pub scroll: f32,
    pub focus: Option<Focus>,
    pub status: Status,
    /// The last install's outcome (ok or not).
    pub message: Option<(&'a str, bool)>,
}

/// What was laid out.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub paints: Vec<Paint>,
    pub hits: Vec<([f32; 4], Hit)>,
    pub area: [f32; 4],
    pub max_scroll: f32,
    /// The cards shown, in order: box and id.
    pub cards: Vec<([f32; 4], String)>,
    pub columns: usize,
    /// Tab's stops, in order.
    pub stops: Vec<([f32; 4], Focus, Hit)>,
    /// The mod the details panel shows.
    pub detailed: Option<String>,
}

impl Layout {
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, at))
            .map(|(_, h)| h.clone())
    }
}

/// Whether `c` matches `query` (folded): name, id, summary, authors, categories.
pub fn matches(c: &ModCard, query: &str) -> bool {
    let q = fold(query.trim());
    q.is_empty()
        || [&c.name, &c.id, &c.summary, &c.authors]
            .iter()
            .any(|f| fold(f).contains(&q))
        || c.categories.iter().any(|x| fold(x).contains(&q))
}

/// The grid's columns for a main panel `w` wide.
pub fn columns(w: f32) -> usize {
    if w < 360.0 {
        2
    } else if w < 720.0 {
        3
    } else {
        4
    }
}

const CARD_H: f32 = 76.0;
const GAP: f32 = 6.0;
const DETAILS_H: f32 = 86.0;
const FILTER_H: f32 = 14.0;

fn badges(k: &mut Kit<'_, Hit>, l: &Labels, c: &ModCard, x: f32, cy: f32, room: f32) {
    let mut pills: Vec<(&str, u32, u32)> = Vec::new();
    if c.native {
        pills.push((&l.native, 0x405A_A9E6, 0xFF5A_A9E6));
    }
    if c.update {
        pills.push((&l.update_badge, palette::ACCENT_SOFT, palette::ACCENT_TEXT));
    } else if c.installed.is_some() {
        pills.push((&l.installed, 0x404A_DE80, palette::GREEN));
    }
    let mut px = x;
    for (t, fill, ink) in pills {
        let w = k.pill_width(t, 9.0);
        if px + w > x + room {
            break;
        }
        px += k.pill(px, cy, 9.0, t, fill, ink) + 3.0;
    }
}

fn card(k: &mut Kit<'_, Hit>, l: &Labels, r: [f32; 4], c: &ModCard, selected: bool) {
    plate(k, r, selected);
    k.hit(r, Hit::Card(c.id.clone()));
    let pic = [r[0] + 7.0, r[1] + 7.0, 22.0, 22.0];
    picture(k, pic, &c.id, Glyph::Sparkle, c.icon.as_ref());
    let tx = pic[0] + pic[2] + 6.0;
    let room = r[0] + r[2] - tx - 6.0;
    k.text_fit(
        tx,
        r[1] + 12.5,
        room,
        &c.name,
        7.5,
        Weight::SemiBold,
        palette::INK,
    );
    let by = l.by.replace("{}", &c.authors);
    let meta = if c.version.is_empty() {
        by
    } else {
        format!("{} · {by}", c.version)
    };
    k.text_fit(
        tx,
        r[1] + 23.0,
        room,
        &meta,
        6.0,
        Weight::Medium,
        palette::MUTED,
    );
    let lines = wrap(&c.summary, r[2] - 14.0, 6.3, Weight::Medium, 2, k.measure);
    for (i, s) in lines.iter().enumerate() {
        k.text(
            r[0] + 7.0,
            r[1] + 39.0 + i as f32 * line(6.3),
            s,
            6.3,
            Weight::Medium,
            0xFFC9_C5BF,
            Align::Left,
        );
    }
    badges(k, l, c, r[0] + 7.0, r[1] + r[3] - 9.0, r[2] - 14.0);
}

/// The details panel's buttons and their stops.
fn details(
    k: &mut Kit<'_, Hit>,
    v: &View<'_>,
    r: [f32; 4],
    c: Option<&ModCard>,
    stops: &mut Vec<([f32; 4], Focus, Hit)>,
) {
    let l = v.labels;
    k.round(r, 7.0, palette::PANEL_RAISED, palette::BORDER);
    let Some(c) = c else {
        let text = match (&v.status, v.message) {
            (_, Some((m, _))) => m.to_owned(),
            (Status::Loading, _) => l.loading.clone(),
            (Status::Failed(why), _) => format!("{} {why}", l.failed),
            (Status::Ready, _) => l.count.replace("{n}", &v.cards.len().to_string()),
        };
        sheet::empty_line(k, r, &text);
        return;
    };
    let pic = [r[0] + 10.0, r[1] + 10.0, 36.0, 36.0];
    picture(k, pic, &c.id, Glyph::Sparkle, c.icon.as_ref());
    // The actions on the right.
    let bw = 108.0f32.min(r[2] * 0.3);
    let bx = r[0] + r[2] - 10.0 - bw;
    let bh = 18.0;
    let mut by = r[1] + 10.0;
    let (label, hit, on, look) = if c.native {
        (
            l.build.as_str(),
            Hit::Build(c.id.clone()),
            true,
            Look::Secondary,
        )
    } else if c.installing {
        (l.installing.as_str(), Hit::Nothing, false, Look::Primary)
    } else if c.update {
        (
            l.update.as_str(),
            Hit::Install(c.id.clone()),
            true,
            Look::Primary,
        )
    } else if c.installed.is_some() {
        (l.up_to_date.as_str(), Hit::Nothing, false, Look::Primary)
    } else {
        (
            l.install.as_str(),
            Hit::Install(c.id.clone()),
            true,
            Look::Primary,
        )
    };
    let br = [bx, by, bw, bh];
    k.button_on(
        br,
        label,
        look,
        v.focus == Some(Focus::Primary),
        on,
        hit.clone(),
    );
    if on {
        stops.push((br, Focus::Primary, hit));
    }
    by += bh + 5.0;
    if let Some(on) = c.on {
        let cy = by + 6.0;
        k.text_fit(
            bx,
            cy,
            bw - 24.0,
            &l.enabled,
            6.5,
            Weight::Medium,
            palette::INK,
        );
        let sr = sheet::switch(k, bx + bw, cy, on, true, Hit::Toggle(c.id.clone()));
        if v.focus == Some(Focus::Switch) {
            k.round(
                [sr[0] - 1.5, sr[1] - 1.5, sr[2] + 3.0, sr[3] + 3.0],
                6.0,
                0,
                palette::ACCENT,
            );
        }
        stops.push((sr, Focus::Switch, Hit::Toggle(c.id.clone())));
        by += 14.0;
    }
    if !c.native {
        let pr = [bx, by, bw, 15.0];
        k.button(
            pr,
            &l.open_page,
            Look::Ghost,
            v.focus == Some(Focus::Page),
            Hit::Page(c.id.clone()),
        );
        stops.push((pr, Focus::Page, Hit::Page(c.id.clone())));
    }
    // What it is.
    let tx = pic[0] + pic[2] + 9.0;
    let room = bx - 10.0 - tx;
    k.text_fit(
        tx,
        r[1] + 15.0,
        room,
        &c.name,
        9.0,
        Weight::SemiBold,
        palette::INK,
    );
    let mut meta: Vec<String> = Vec::new();
    if !c.version.is_empty() {
        meta.push(c.version.clone());
    }
    if !c.authors.is_empty() {
        meta.push(l.by.replace("{}", &c.authors));
    }
    if !c.license.is_empty() {
        meta.push(c.license.clone());
    }
    k.text_fit(
        tx,
        r[1] + 27.0,
        room,
        &meta.join(" · "),
        6.3,
        Weight::Medium,
        palette::MUTED,
    );
    let text = if c.description.trim().is_empty() {
        &c.summary
    } else {
        &c.description
    };
    let lines = wrap(
        text,
        r[0] + r[2] - 10.0 - bw - 10.0 - tx,
        6.4,
        Weight::Medium,
        4,
        k.measure,
    );
    for (i, s) in lines.iter().enumerate() {
        k.text(
            tx,
            r[1] + 39.0 + i as f32 * line(6.4),
            s,
            6.4,
            Weight::Medium,
            0xFFC9_C5BF,
            Align::Left,
        );
    }
    if let Some((m, ok)) = v.message {
        k.text_fit(
            tx,
            r[1] + r[3] - 7.0,
            room,
            m,
            6.0,
            Weight::Medium,
            if ok { palette::GREEN } else { palette::DANGER },
        );
    }
}

/// The sidebar's filters: (label, filter), with the headings between groups as `None` filters.
fn filters(v: &View<'_>) -> Vec<(String, Option<Filter>)> {
    let l = v.labels;
    let mut out = vec![
        (l.all.clone(), Some(Filter::All)),
        (l.installed.clone(), Some(Filter::Installed)),
        (l.updates.clone(), Some(Filter::Updates)),
        (l.kinds.clone(), None),
        (l.content.clone(), Some(Filter::Content)),
        (l.native.clone(), Some(Filter::Native)),
    ];
    if !v.categories.is_empty() {
        out.push((l.categories.clone(), None));
        for c in v.categories {
            out.push((c.clone(), Some(Filter::Category(c.clone()))));
        }
    }
    out
}

/// Lays the page out.
pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let l = v.labels;
    let mut k: Kit<'_, Hit> = Kit::new(v.measure, v.mouse);
    let mut stops: Vec<([f32; 4], Focus, Hit)> = Vec::new();
    k.hit([0.0, 0.0, gw, gh], Hit::Nothing);
    k.fill([0.0, 0.0, gw, gh], 0xFF0B_0C0F);
    // The sidebar.
    let sw = (gw * 0.2).clamp(112.0, 150.0);
    k.fill([0.0, 0.0, sw, gh], 0xF20F_1013);
    k.fill([sw - 0.5, 0.0, 0.5, gh], palette::RULE);
    let sx = 10.0;
    let inner = sw - 20.0;
    k.text(
        sx,
        13.0,
        &l.brand,
        6.5,
        Weight::SemiBold,
        palette::ACCENT_TEXT,
        Align::Left,
    );
    k.text_fit(
        sx,
        25.0,
        inner,
        &l.title,
        10.0,
        Weight::SemiBold,
        palette::INK,
    );
    let search = [sx, 37.0, inner, 16.0];
    k.field(
        search,
        v.query,
        &l.placeholder,
        v.search_focused || v.focus == Some(Focus::Search),
        v.caret,
        Hit::Search,
    );
    stops.push((search, Focus::Search, Hit::Search));
    let done_r = [sx, gh - 30.0, inner, 20.0];
    let mut y = 60.0;
    let mut fi = 0;
    for (label, f) in filters(v) {
        if y + FILTER_H > done_r[1] - 6.0 {
            break;
        }
        match f {
            None => {
                y += 4.0;
                k.text_fit(
                    sx + 2.0,
                    y + 5.0,
                    inner,
                    &label,
                    6.0,
                    Weight::SemiBold,
                    0xFF7A_7670,
                );
                y += 11.0;
            }
            Some(f) => {
                let r = [sx, y, inner, FILTER_H - 1.0];
                let on = *v.filter == f;
                let focused = v.focus == Some(Focus::Filter(fi));
                let lit = k.over(r) || focused;
                if on {
                    k.round(r, 4.0, palette::ACCENT_SOFT, 0);
                    k.round([r[0], r[1] + 3.0, 2.0, r[3] - 6.0], 1.0, palette::ACCENT, 0);
                } else if lit {
                    k.round(r, 4.0, palette::FACE_HOVER, 0);
                }
                if focused {
                    k.round(r, 4.0, 0, palette::ACCENT);
                }
                k.text_fit(
                    r[0] + 7.0,
                    r[1] + r[3] / 2.0,
                    r[2] - 10.0,
                    &label,
                    6.6,
                    if on { Weight::SemiBold } else { Weight::Medium },
                    if on { palette::INK } else { 0xFFC9_C5BF },
                );
                let hit = Hit::Filter(f);
                k.hit(r, hit.clone());
                stops.push((r, Focus::Filter(fi), hit));
                fi += 1;
                y += FILTER_H;
            }
        }
    }
    k.button(
        done_r,
        &l.done,
        Look::Primary,
        v.focus == Some(Focus::Done),
        Hit::Done,
    );
    // The main panel: the grid, then the details.
    let mx = sw + 10.0;
    let mw = (gw - mx - 10.0).max(60.0);
    let shown: Vec<&ModCard> = v
        .cards
        .iter()
        .filter(|c| v.filter.keeps(c) && matches(c, v.query))
        .collect();
    let head = format!(
        "{} · {}",
        match v.filter {
            Filter::All => l.all.as_str(),
            Filter::Installed => l.installed.as_str(),
            Filter::Updates => l.updates.as_str(),
            Filter::Content => l.content.as_str(),
            Filter::Native => l.native.as_str(),
            Filter::Category(c) => c.as_str(),
        },
        l.count.replace("{n}", &shown.len().to_string())
    );
    k.text_fit(
        mx + 2.0,
        13.0,
        mw,
        &head,
        7.0,
        Weight::SemiBold,
        palette::MUTED,
    );
    let details_r = [mx, gh - DETAILS_H - 8.0, mw, DETAILS_H];
    let area = [mx, 22.0, mw, (details_r[1] - 6.0 - 22.0).max(0.0)];
    let cols = columns(mw);
    let cw = ((mw - GAP * (cols as f32 - 1.0) - 4.0) / cols as f32).floor();
    let rows = shown.len().div_ceil(cols);
    let content = rows as f32 * (CARD_H + GAP);
    let max = max_scroll(content, area[3]);
    let scroll = v.scroll.clamp(0.0, max);
    let mut out = Layout {
        area,
        max_scroll: max,
        columns: cols,
        ..Layout::default()
    };
    k.out.push(Paint::ClipBegin(area));
    if shown.is_empty() {
        let text = match &v.status {
            Status::Loading => l.loading.clone(),
            Status::Failed(why) if v.cards.is_empty() => format!("{} {why}", l.failed),
            _ => l.nothing_found.clone(),
        };
        sheet::empty_line(&mut k, area, &text);
    }
    let mut hovered: Option<&ModCard> = None;
    for (i, c) in shown.iter().enumerate() {
        let r = [
            mx + (i % cols) as f32 * (cw + GAP),
            area[1] + (i / cols) as f32 * (CARD_H + GAP) - scroll,
            cw,
            CARD_H,
        ];
        if r[1] + r[3] < area[1] || r[1] > area[1] + area[3] {
            continue;
        }
        let selected =
            v.selected == Some(c.id.as_str()) || v.focus == Some(Focus::Card(c.id.clone()));
        let before = k.hits.len();
        card(&mut k, l, r, c, selected);
        for (hr, _) in &mut k.hits[before..] {
            let y0 = hr[1].max(area[1]);
            let y1 = (hr[1] + hr[3]).min(area[1] + area[3]);
            hr[1] = y0;
            hr[3] = (y1 - y0).max(0.0);
        }
        if inside(r, v.mouse) && inside(area, v.mouse) {
            hovered = Some(c);
        }
        out.cards.push((r, c.id.clone()));
    }
    k.out.push(Paint::ClipEnd);
    k.scrollbar(area, content, scroll);
    let detailed = hovered.or_else(|| {
        v.selected
            .and_then(|id| v.cards.iter().find(|c| c.id == id))
    });
    out.detailed = detailed.map(|c| c.id.clone());
    details(&mut k, v, details_r, detailed, &mut stops);
    stops.push((done_r, Focus::Done, Hit::Done));
    out.paints = k.out;
    out.hits = k.hits;
    out.stops = stops;
    out
}

/// The card `delta` places from `current` in the grid (`columns` wide), clamped; the first
/// with none.
pub fn step(cards: &[([f32; 4], String)], current: Option<&str>, delta: isize) -> Option<String> {
    if cards.is_empty() {
        return None;
    }
    let at = current.and_then(|c| cards.iter().position(|(_, id)| id == c));
    let next = match at {
        None => 0,
        Some(i) => (i as isize + delta).clamp(0, cards.len() as isize - 1) as usize,
    };
    Some(cards[next].1.clone())
}

/// The categories of `cards`, most used first (ties by name), at most `max`.
pub fn top_categories(cards: &[ModCard], max: usize) -> Vec<String> {
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for c in cards {
        for x in &c.categories {
            *counts.entry(x.as_str()).or_default() += 1;
        }
    }
    let mut v: Vec<(&str, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    v.into_iter().take(max).map(|(c, _)| c.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    fn card_of(id: &str, native: bool, installed: bool) -> ModCard {
        ModCard {
            id: id.into(),
            name: id.into(),
            version: "1.0".into(),
            authors: "Ann".into(),
            summary: "Does things".into(),
            categories: vec!["utility".into()],
            native,
            installed: installed.then(|| "1.0".into()),
            on: installed.then_some(true),
            ..ModCard::default()
        }
    }

    #[test]
    fn a_grid_of_cards_filters_and_details() {
        assert_eq!((columns(300.0), columns(500.0), columns(900.0)), (2, 3, 4));
        let labels = Labels {
            install: "Install".into(),
            build: "Build it from the launcher".into(),
            by: "by {}".into(),
            count: "{n} mods".into(),
            ..Labels::default()
        };
        let cards = [
            card_of("a", false, false),
            card_of("b", true, false),
            card_of("c", false, true),
        ];
        let cats = top_categories(&cards, 5);
        assert_eq!(cats, ["utility"]);
        let mut v = View {
            gui: [854.0, 480.0],
            mouse: [0.0, 0.0],
            measure: &test_measure,
            labels: &labels,
            query: "",
            search_focused: false,
            caret: false,
            cards: &cards,
            filter: &Filter::All,
            categories: &cats,
            selected: Some("a"),
            scroll: 0.0,
            focus: None,
            status: Status::Ready,
            message: None,
        };
        let l = layout(&v);
        assert_eq!(l.cards.len(), 3);
        assert_eq!(l.columns, 3);
        assert_eq!(l.detailed.as_deref(), Some("a"));
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Install("a".into())));
        v.selected = Some("b");
        let l = layout(&v);
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Build("b".into())));
        v.filter = &Filter::Installed;
        let l = layout(&v);
        assert_eq!(l.cards.len(), 1);
        assert_eq!(step(&l.cards, None, 1).as_deref(), Some("c"));
        let f = Filter::Native;
        v.filter = &f;
        assert_eq!(layout(&v).cards.len(), 1);
    }
}
