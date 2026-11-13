//! The Rustic language screen, over the title (in place of the game's Language page): every
//! language the game has, as a searchable grid of cards (flag, native name, region), the one in
//! use lit. Choosing one applies it as the game's own Language screen does (`Host::set_language`,
//! Rustic's Horizon patch U16). Pure layout and keys, in GUI pixels.
use crate::kit::{Kit, Look, fold, max_scroll};
use crate::paint::{Align, Measure, Paint, Picture};
use crate::skin::palette;
use crate::title::Weight;

/// A language as the game declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Language {
    pub code: String,
    /// Its own name (`Français`).
    pub name: String,
    /// Its region in its own words (`Canada`).
    pub region: String,
    folded: String,
}

impl Language {
    pub fn new(code: &str, name: &str, region: &str) -> Self {
        Language {
            code: code.to_owned(),
            name: name.to_owned(),
            region: region.to_owned(),
            folded: fold(&format!("{name} {region} {code}")),
        }
    }
}

/// The game's languages sorted by name (then region), as the grid shows them.
pub fn sorted(raw: &[(String, String, String)]) -> Vec<Language> {
    let mut all: Vec<Language> = raw.iter().map(|(c, n, r)| Language::new(c, n, r)).collect();
    all.sort_by(|a, b| {
        (fold(&a.name), fold(&a.region), &a.code).cmp(&(fold(&b.name), fold(&b.region), &b.code))
    });
    all
}

/// The languages matching `query` (every word in the name, region or code; accents and case
/// ignored).
pub fn filter<'a>(all: &'a [Language], query: &str) -> Vec<&'a Language> {
    let words: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
    all.iter()
        .filter(|l| words.iter().all(|w| l.folded.contains(w.as_str())))
        .collect()
}

/// What a press on the screen lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Pick(String),
    Back,
    Search,
}

/// What the screen is drawn from.
pub struct View<'a> {
    pub gui: [f32; 2],
    pub mouse: [f32; 2],
    pub shown: &'a [&'a Language],
    /// The code in use.
    pub current: &'a str,
    pub query: &'a str,
    pub search_focused: bool,
    pub caret: bool,
    pub scroll: f32,
    /// The card the keyboard is on, an index into `shown`.
    pub focus: Option<usize>,
    pub title: &'a str,
    pub back: &'a str,
    pub placeholder: &'a str,
    pub none: &'a str,
    pub measure: Measure<'a>,
}

pub struct Layout {
    pub paints: Vec<Paint>,
    pub hits: Vec<([f32; 4], Hit)>,
    pub columns: usize,
    pub max_scroll: f32,
    /// The grid's visible area.
    pub grid: [f32; 4],
    /// A card's height with its gap (a row's step).
    pub row_step: f32,
}

/// The card's height.
const CARD_H: f32 = 26.0;
const GAP: f32 = 4.0;

pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let mut k: Kit<'_, Hit> = Kit::new(v.measure, v.mouse);
    // The panel, centred.
    let pw = (gw - 24.0).min(560.0);
    let ph = gh - 24.0;
    let panel = [((gw - pw) / 2.0).round(), 12.0, pw.round(), ph.round()];
    k.fill([0.0, 0.0, gw, gh], 0x8C05_0507);
    k.round(panel, 6.0, palette::PANEL, palette::BORDER);
    let pad = 12.0;
    let [px, py, pw, ph] = panel;
    // Header: the title, the search field.
    let head_cy = py + pad + 8.0;
    k.text(
        px + pad,
        head_cy,
        v.title,
        11.0,
        Weight::SemiBold,
        palette::INK,
        Align::Left,
    );
    let fw = (pw * 0.45).clamp(100.0, 220.0);
    k.field(
        [px + pw - pad - fw, head_cy - 8.0, fw, 16.0],
        v.query,
        v.placeholder,
        v.search_focused,
        v.caret,
        Hit::Search,
    );
    // Footer: Back.
    let foot_h = 20.0;
    let back = [px + pw - pad - 90.0, py + ph - pad - foot_h, 90.0, foot_h];
    k.button(back, v.back, Look::Secondary, false, Hit::Back);
    // The grid between them.
    let top = head_cy + 8.0 + 10.0;
    let bottom = back[1] - 8.0;
    let grid = [px + pad, top, pw - 2.0 * pad, (bottom - top).max(0.0)];
    let columns = ((grid[2] + GAP) / (150.0 + GAP)).floor().clamp(1.0, 4.0) as usize;
    let cw = ((grid[2] - 4.0 - GAP * (columns as f32 - 1.0)) / columns as f32).floor();
    let rows = v.shown.len().div_ceil(columns);
    let content = rows as f32 * (CARD_H + GAP) - GAP;
    let max = max_scroll(content, grid[3]);
    let scroll = v.scroll.clamp(0.0, max);
    k.out.push(Paint::ClipBegin(grid));
    if v.shown.is_empty() {
        k.text(
            grid[0] + grid[2] / 2.0,
            grid[1] + 30.0,
            v.none,
            8.0,
            Weight::Medium,
            palette::MUTED,
            Align::Center,
        );
    }
    for (i, lang) in v.shown.iter().enumerate() {
        let (row, col) = (i / columns, i % columns);
        let r = [
            grid[0] + col as f32 * (cw + GAP),
            grid[1] + row as f32 * (CARD_H + GAP) - scroll,
            cw,
            CARD_H,
        ];
        if r[1] + r[3] < grid[1] || r[1] > grid[1] + grid[3] {
            continue;
        }
        let current = lang.code == v.current;
        let lit = v.focus == Some(i) || (k.over(r) && k.over(grid));
        let (fill, border) = match (current, lit) {
            (true, _) => (0x33FF_6A00, palette::ACCENT),
            (false, true) => (palette::FACE_HOVER, 0x4DFF_FFFF),
            (false, false) => (palette::FACE, palette::BORDER),
        };
        k.round(r, 4.0, fill, border);
        // The flag, 3:2.
        let fh = 12.0;
        let flag = [r[0] + 7.0, r[1] + (CARD_H - fh) / 2.0, 18.0, fh];
        k.out.push(Paint::Picture {
            picture: Picture::Flag(lang.code.clone()),
            rect: flag,
            tint: 0xFFFF_FFFF,
        });
        if crate::flags::flag_for(&lang.code).is_none() {
            let code: String = lang.code.chars().take(3).collect::<String>().to_uppercase();
            k.text(
                flag[0] + flag[2] / 2.0,
                flag[1] + fh / 2.0,
                &code,
                4.5,
                Weight::SemiBold,
                palette::INK,
                Align::Center,
            );
        }
        let tx = flag[0] + flag[2] + 6.0;
        let room = r[0] + r[2] - tx - if current { 14.0 } else { 5.0 };
        k.text_fit(
            tx,
            r[1] + 9.5,
            room,
            &lang.name,
            7.0,
            Weight::SemiBold,
            if current {
                palette::ACCENT_TEXT
            } else {
                palette::INK
            },
        );
        k.text_fit(
            tx,
            r[1] + 18.5,
            room,
            &lang.region,
            5.5,
            Weight::Medium,
            palette::MUTED,
        );
        if current {
            // A check mark at the right.
            let c = [r[0] + r[2] - 9.0, r[1] + CARD_H / 2.0];
            let t = 0.7;
            for (a, b) in [
                ([c[0] - 3.0, c[1]], [c[0] - 1.0, c[1] + 2.2]),
                ([c[0] - 1.2, c[1] + 2.2], [c[0] + 3.2, c[1] - 2.6]),
            ] {
                k.out.push(Paint::Quad {
                    points: [
                        [a[0], a[1] - t],
                        [b[0], b[1] - t],
                        [b[0], b[1] + t],
                        [a[0], a[1] + t],
                    ],
                    argb: palette::ACCENT,
                });
            }
        }
        // The part of the card inside the grid takes presses.
        let y0 = r[1].max(grid[1]);
        let y1 = (r[1] + r[3]).min(grid[1] + grid[3]);
        if y1 - y0 >= 4.0 {
            k.hit([r[0], y0, r[2], y1 - y0], Hit::Pick(lang.code.clone()));
        }
    }
    k.out.push(Paint::ClipEnd);
    k.scrollbar([grid[0], grid[1], grid[2] + 2.0, grid[3]], content, scroll);
    Layout {
        paints: k.out,
        hits: k.hits,
        columns,
        max_scroll: max,
        grid,
        row_step: CARD_H + GAP,
    }
}

impl Layout {
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| crate::paint::inside(*r, at))
            .map(|(_, h)| h.clone())
    }
}

/// What a key does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// Move the keyboard to card `n`.
    Focus(usize),
    Pick(usize),
    Back,
    /// Delete the search's last character.
    Erase,
    Ignore,
}

/// A key (`key.keyboard.*`) on the grid of `count` cards in `columns` columns, the keyboard on
/// `focus`.
pub fn key(name: &str, focus: Option<usize>, count: usize, columns: usize) -> Key {
    let columns = columns.max(1);
    let last = count.saturating_sub(1);
    let step = |delta: isize| -> Key {
        if count == 0 {
            return Key::Ignore;
        }
        let at = match focus {
            None => 0,
            Some(f) => (f as isize + delta).clamp(0, last as isize) as usize,
        };
        Key::Focus(at)
    };
    match name {
        "key.keyboard.escape" => Key::Back,
        "key.keyboard.backspace" => Key::Erase,
        "key.keyboard.right" | "key.keyboard.tab" => step(1),
        "key.keyboard.left" => step(-1),
        "key.keyboard.down" => step(columns as isize),
        "key.keyboard.up" => step(-(columns as isize)),
        "key.keyboard.enter" | "key.keyboard.keypad.enter" => match focus {
            Some(f) if f < count => Key::Pick(f),
            _ if count == 1 => Key::Pick(0),
            _ => Key::Ignore,
        },
        _ => Key::Ignore,
    }
}

/// The scroll that shows card `index` of a grid of `columns` columns.
pub fn scroll_to(index: usize, columns: usize, scroll: f32, view_h: f32, step: f32) -> f32 {
    let row = (index / columns.max(1)) as f32;
    let (top, bottom) = (row * step, row * step + step);
    if top < scroll {
        top
    } else if bottom > scroll + view_h {
        bottom - view_h
    } else {
        scroll
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    fn raw() -> Vec<(String, String, String)> {
        [
            ("en_us", "English", "United States"),
            ("fr_fr", "Français", "France"),
            ("fr_ca", "Français", "Canada"),
            ("de_de", "Deutsch", "Deutschland"),
            ("ja_jp", "日本語", "日本"),
        ]
        .iter()
        .map(|(a, b, c)| ((*a).into(), (*b).into(), (*c).into()))
        .collect()
    }

    #[test]
    fn languages_sort_by_name_and_search_by_name_region_or_code() {
        let all = sorted(&raw());
        let names: Vec<&str> = all.iter().map(|l| l.code.as_str()).collect();
        assert_eq!(names, ["de_de", "en_us", "fr_ca", "fr_fr", "ja_jp"]);
        let codes =
            |q: &str| -> Vec<String> { filter(&all, q).iter().map(|l| l.code.clone()).collect() };
        assert_eq!(codes("francais"), ["fr_ca", "fr_fr"]);
        assert_eq!(codes("FRAN canada"), ["fr_ca"]);
        assert_eq!(codes("ja_"), ["ja_jp"]);
        assert_eq!(codes("日本"), ["ja_jp"]);
        assert_eq!(codes("").len(), 5);
        assert!(codes("klingon").is_empty());
    }

    fn view<'a>(shown: &'a [&'a Language], mouse: [f32; 2]) -> View<'a> {
        View {
            gui: [640.0, 360.0],
            mouse,
            shown,
            current: "fr_fr",
            query: "",
            search_focused: false,
            caret: false,
            scroll: 0.0,
            focus: None,
            title: "Language",
            back: "Back",
            placeholder: "Search languages",
            none: "No language matches",
            measure: &test_measure,
        }
    }

    #[test]
    fn the_grid_lays_cards_in_columns_and_lights_the_current_one() {
        let all = sorted(&raw());
        let shown: Vec<&Language> = all.iter().collect();
        let l = layout(&view(&shown, [-1.0, -1.0]));
        assert_eq!(l.columns, 3);
        assert_eq!(l.max_scroll, 0.0);
        // Every card visible in the grid takes presses.
        assert_eq!(
            l.hits
                .iter()
                .filter(|(_, h)| matches!(h, Hit::Pick(_)))
                .count(),
            5
        );
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Back));
        // A flag for every card, and the current card in the accent.
        let flags = l
            .paints
            .iter()
            .filter(|p| matches!(p, Paint::Picture { .. }))
            .count();
        assert_eq!(flags, 5);
        assert!(l.paints.iter().any(|p| matches!(
            p,
            Paint::Text { text, argb, .. } if text == "Français" && *argb == palette::ACCENT_TEXT
        )));
        // A press on the first card picks it.
        let g = l.grid;
        let l = layout(&view(&shown, [g[0] + 5.0, g[1] + 5.0]));
        assert_eq!(
            l.hit([g[0] + 5.0, g[1] + 5.0]),
            Some(Hit::Pick("de_de".into()))
        );
    }

    #[test]
    fn many_languages_scroll_and_the_keyboard_walks_the_grid() {
        let raw: Vec<(String, String, String)> = (0..143)
            .map(|i| {
                (
                    format!("l{i:03}_xx"),
                    format!("Lang {i:03}"),
                    "Region".into(),
                )
            })
            .collect();
        let all = sorted(&raw);
        let shown: Vec<&Language> = all.iter().collect();
        let l = layout(&view(&shown, [-1.0, -1.0]));
        assert!(l.max_scroll > 0.0);
        assert_eq!(key("key.keyboard.down", Some(0), 143, 3), Key::Focus(3));
        assert_eq!(key("key.keyboard.up", Some(1), 143, 3), Key::Focus(0));
        assert_eq!(key("key.keyboard.right", None, 143, 3), Key::Focus(0));
        assert_eq!(
            key("key.keyboard.right", Some(142), 143, 3),
            Key::Focus(142)
        );
        assert_eq!(key("key.keyboard.enter", Some(4), 143, 3), Key::Pick(4));
        assert_eq!(key("key.keyboard.enter", None, 1, 3), Key::Pick(0));
        assert_eq!(key("key.keyboard.escape", None, 0, 3), Key::Back);
        assert_eq!(key("key.keyboard.a", None, 3, 3), Key::Ignore);
        assert_eq!(scroll_to(30, 3, 0.0, 100.0, 30.0), 230.0);
        assert_eq!(scroll_to(0, 3, 230.0, 100.0, 30.0), 0.0);
        assert_eq!(scroll_to(10, 3, 70.0, 100.0, 30.0), 70.0);
    }
}
