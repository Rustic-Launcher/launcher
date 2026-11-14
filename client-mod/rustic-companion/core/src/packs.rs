//! Rustic's Resource Packs page (and Create World's Data Packs, the same screen; ADR 0165): the
//! game's pack screen redone as a launcher page over it — the title and a search field, two
//! columns of cards, "Available" and "Selected" (in load order, the top one applied last, over
//! the others), each card with the pack's icon, its name, two lines of its description and a
//! badge when it was made for another version; Add or Remove and the order arrows on hover or
//! focus; Open pack folder, Cancel and Done in the footer. Pure: the game crate gives the packs
//! and does the presses through the game's own pack screen (Rustic's Horizon patch U19).
use crate::kit::{Kit, Look, fold, line, max_scroll};
use crate::lists::{picture, plate};
use crate::paint::{Align, Measure, Paint, Picture, fit, inside, wrap};
use crate::sheet::{self, FOOTER, HEADER, Icon};
use crate::skin::palette;
use crate::title::{Glyph, Weight};

/// One pack as a card shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct PackRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<Picture>,
    /// Made for another version: the badge's text.
    pub warning: Option<String>,
    /// What can be done now: add, remove, up, down.
    pub can: [bool; 4],
}

/// Which column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Available,
    Selected,
}

/// A card's action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Remove,
    Up,
    Down,
}

/// What a press lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// A card (its pack's id): focuses it.
    Card(Side, String),
    Op(String, Op),
    Search,
    Folder,
    Cancel,
    Done,
    Nothing,
}

/// Where the keyboard is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    Card(String),
    Search,
    Folder,
    Cancel,
    Done,
}

/// The page's words (Rustic's and the game's own, already translated).
#[derive(Clone, Debug, Default)]
pub struct Labels {
    pub title: String,
    pub available: String,
    pub selected: String,
    /// What the selected column's order means.
    pub order: String,
    pub placeholder: String,
    pub folder: String,
    pub cancel: String,
    pub done: String,
    pub none_available: String,
    pub none_selected: String,
    pub nothing_found: String,
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
    pub available: &'a [PackRow],
    pub selected: &'a [PackRow],
    pub scroll: [f32; 2],
    pub focus: Option<Focus>,
}

/// What was laid out: the paints, the hit boxes, each column's list area and scroll, the cards.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub paints: Vec<Paint>,
    pub hits: Vec<([f32; 4], Hit)>,
    pub areas: [[f32; 4]; 2],
    pub max_scroll: [f32; 2],
    /// The cards shown, by column, in order: their box and pack id.
    pub cards: [Vec<([f32; 4], String)>; 2],
}

impl Layout {
    /// The topmost hit at `at`.
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, at))
            .map(|(_, h)| h.clone())
    }
    /// The column at `at`, for the wheel.
    pub fn side_at(&self, at: [f32; 2]) -> Option<usize> {
        (0..2).find(|&i| inside(self.areas[i], at))
    }
}

const CARD: f32 = 40.0;
const GAP: f32 = 5.0;
/// The footer and Tab's stops after the search field.
pub const STOPS: [Focus; 4] = [Focus::Search, Focus::Folder, Focus::Cancel, Focus::Done];

/// Whether `row` matches `query` (folded): its name, description or id.
pub fn matches(row: &PackRow, query: &str) -> bool {
    let q = fold(query.trim());
    q.is_empty()
        || fold(&row.name).contains(&q)
        || fold(&row.description).contains(&q)
        || fold(&row.id).contains(&q)
}

fn card(
    k: &mut Kit<'_, Hit>,
    r: [f32; 4],
    row: &PackRow,
    side: Side,
    focused: bool,
    clip: [f32; 4],
) {
    let shown = k.over(r) && inside(clip, k.mouse) || focused;
    plate(k, r, focused);
    k.hit(r, Hit::Card(side, row.id.clone()));
    let pic = [r[0] + 6.0, r[1] + 6.0, 28.0, 28.0];
    picture(k, pic, &row.id, Glyph::Bag, row.icon.as_ref());
    // The actions at the right, shown on hover or focus.
    let bs = 12.0;
    let ops: Vec<(Op, Icon, bool)> = match side {
        Side::Available => vec![(Op::Add, Icon::Plus, row.can[0])],
        Side::Selected => vec![
            (Op::Up, Icon::Up, row.can[2]),
            (Op::Down, Icon::Down, row.can[3]),
            (Op::Remove, Icon::Minus, row.can[1]),
        ],
    };
    let ops_w = if shown {
        ops.len() as f32 * (bs + 3.0) + 4.0
    } else {
        0.0
    };
    let tx = pic[0] + pic[2] + 7.0;
    let room = r[0] + r[2] - tx - 6.0 - ops_w;
    let name_cy = r[1] + 11.0;
    let mut name_room = room;
    if let Some(w) = &row.warning {
        let pw = k.pill_width(w, 9.0).min(room * 0.5);
        let label = fit(w, pw - 6.0, 5.6, Weight::SemiBold, k.measure);
        k.pill(
            r[0] + r[2] - 6.0 - ops_w - pw,
            name_cy,
            9.0,
            &label,
            0x40F0_645A,
            palette::DANGER,
        );
        name_room -= pw + 4.0;
    }
    k.text_fit(
        tx,
        name_cy,
        name_room,
        &row.name,
        7.5,
        Weight::SemiBold,
        palette::INK,
    );
    let lines = wrap(&row.description, room, 6.3, Weight::Medium, 2, k.measure);
    for (i, l) in lines.iter().enumerate() {
        k.text(
            tx,
            r[1] + 21.5 + i as f32 * line(6.3),
            l,
            6.3,
            Weight::Medium,
            palette::MUTED,
            Align::Left,
        );
    }
    if shown {
        let mut x = r[0] + r[2] - 6.0 - bs;
        for (op, icon, on) in ops.iter().rev() {
            sheet::icon_button(
                k,
                [x, r[1] + (r[3] - bs) / 2.0, bs, bs],
                *icon,
                *on,
                Hit::Op(row.id.clone(), *op),
            );
            x -= bs + 3.0;
        }
    }
}

/// Lays the page out.
pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let mut k: Kit<'_, Hit> = Kit::new(v.measure, v.mouse);
    let l = v.labels;
    // Everything inside the page is eaten.
    k.hit([0.0, 0.0, gw, gh], Hit::Nothing);
    k.fill([0.0, 0.0, gw, gh], 0xFF0B_0C0F);
    sheet::title(&mut k, gw, &l.title);
    let search = sheet::search_rect(gw, 320.0);
    k.field(
        search,
        v.query,
        &l.placeholder,
        v.search_focused || v.focus == Some(Focus::Search),
        v.caret,
        Hit::Search,
    );
    // The two columns.
    let w = (gw - 24.0).min(780.0);
    let x0 = ((gw - w) / 2.0).round();
    let col_w = ((w - 12.0) / 2.0).floor();
    let top = HEADER + 4.0;
    let bottom = gh - FOOTER - 6.0;
    let mut out = Layout::default();
    for (i, (side, rows, name)) in [
        (Side::Available, v.available, &l.available),
        (Side::Selected, v.selected, &l.selected),
    ]
    .into_iter()
    .enumerate()
    {
        let x = x0 + i as f32 * (col_w + 12.0);
        let shown: Vec<&PackRow> = rows.iter().filter(|r| matches(r, v.query)).collect();
        // The column's heading and count; the order's meaning under "Selected".
        let hw = k.text(
            x + 2.0,
            top + 6.0,
            name,
            7.5,
            Weight::SemiBold,
            palette::INK,
            Align::Left,
        );
        k.text(
            x + 2.0 + hw + 5.0,
            top + 6.0,
            &shown.len().to_string(),
            6.5,
            Weight::Medium,
            palette::MUTED,
            Align::Left,
        );
        if side == Side::Selected {
            k.text_fit(
                x + 2.0,
                top + 15.0,
                col_w - 4.0,
                &l.order,
                6.0,
                Weight::Medium,
                0xFF7A_7670,
            );
        }
        let area_top = top + 22.0;
        let area = [x, area_top, col_w, (bottom - area_top).max(0.0)];
        k.round(area, 6.0, 0x4000_0000, palette::BORDER_OFF);
        out.areas[i] = area;
        let content = shown.len() as f32 * (CARD + GAP) + GAP;
        out.max_scroll[i] = max_scroll(content, area[3]);
        let scroll = v.scroll[i].clamp(0.0, out.max_scroll[i]);
        k.out.push(Paint::ClipBegin(area));
        if shown.is_empty() {
            let text = if !v.query.trim().is_empty() {
                &l.nothing_found
            } else if side == Side::Available {
                &l.none_available
            } else {
                &l.none_selected
            };
            sheet::empty_line(&mut k, area, text);
        }
        let mut y = area[1] + GAP - scroll;
        let cw = col_w - 2.0 * GAP - if out.max_scroll[i] > 0.0 { 4.0 } else { 0.0 };
        for row in shown {
            let r = [x + GAP, y, cw, CARD];
            y += CARD + GAP;
            if r[1] + r[3] < area[1] || r[1] > area[1] + area[3] {
                continue;
            }
            let focused = v.focus == Some(Focus::Card(row.id.clone()));
            let hits_before = k.hits.len();
            card(&mut k, r, row, side, focused, area);
            // Hit boxes outside the list's view are cut to it.
            for (hr, _) in &mut k.hits[hits_before..] {
                let y0 = hr[1].max(area[1]);
                let y1 = (hr[1] + hr[3]).min(area[1] + area[3]);
                hr[1] = y0;
                hr[3] = (y1 - y0).max(0.0);
            }
            out.cards[i].push((r, row.id.clone()));
        }
        k.out.push(Paint::ClipEnd);
        k.scrollbar(area, content, scroll);
    }
    // The footer: Open pack folder on the left, Cancel and Done on the right.
    let cy = sheet::footer(&mut k, gw, gh);
    let h = sheet::BUTTON_H;
    let fw = k.button_width(&l.folder, h).max(70.0);
    k.button(
        [x0, cy - h / 2.0, fw, h],
        &l.folder,
        Look::Secondary,
        v.focus == Some(Focus::Folder),
        Hit::Folder,
    );
    let dw = k.button_width(&l.done, h).max(80.0);
    let cw = k.button_width(&l.cancel, h).max(60.0);
    let right = x0 + w;
    k.button(
        [right - dw, cy - h / 2.0, dw, h],
        &l.done,
        Look::Primary,
        v.focus == Some(Focus::Done),
        Hit::Done,
    );
    k.button(
        [right - dw - 6.0 - cw, cy - h / 2.0, cw, h],
        &l.cancel,
        Look::Ghost,
        v.focus == Some(Focus::Cancel),
        Hit::Cancel,
    );
    out.paints = k.out;
    out.hits = k.hits;
    out
}

/// The card after (or before) `current` in its column, in the order shown; the first card with
/// none.
pub fn step(cards: &[([f32; 4], String)], current: Option<&str>, down: bool) -> Option<String> {
    if cards.is_empty() {
        return None;
    }
    let at = current.and_then(|c| cards.iter().position(|(_, id)| id == c));
    let next = match (at, down) {
        (None, _) => 0,
        (Some(i), true) => (i + 1).min(cards.len() - 1),
        (Some(i), false) => i.saturating_sub(1),
    };
    Some(cards[next].1.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    fn row(id: &str, can: [bool; 4]) -> PackRow {
        PackRow {
            id: id.into(),
            name: id.into(),
            description: "A pack".into(),
            icon: None,
            warning: None,
            can,
        }
    }

    #[test]
    fn two_columns_their_cards_and_the_footer() {
        let labels = Labels {
            title: "Select Resource Packs".into(),
            done: "Done".into(),
            ..Labels::default()
        };
        let available = [row("file/A", [true, false, false, false])];
        let selected = [
            row("file/B", [false, true, false, true]),
            row("vanilla", [false, false, true, false]),
        ];
        let v = View {
            gui: [800.0, 480.0],
            mouse: [0.0, 0.0],
            measure: &test_measure,
            labels: &labels,
            query: "",
            search_focused: false,
            caret: false,
            available: &available,
            selected: &selected,
            scroll: [0.0, 0.0],
            focus: Some(Focus::Card("file/B".into())),
        };
        let l = layout(&v);
        assert_eq!(l.cards[0].len(), 1);
        assert_eq!(l.cards[1].len(), 2);
        // The focused card shows its actions: up is off (it is on top), down and remove are on.
        let ops: Vec<&Hit> = l
            .hits
            .iter()
            .map(|(_, h)| h)
            .filter(|h| matches!(h, Hit::Op(..)))
            .collect();
        assert_eq!(
            ops,
            [
                &Hit::Op("file/B".into(), Op::Remove),
                &Hit::Op("file/B".into(), Op::Down),
            ]
        );
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Done));
        let done = l.hits.iter().find(|(_, h)| *h == Hit::Done).unwrap().0;
        assert_eq!(l.hit([done[0] + 1.0, done[1] + 1.0]), Some(Hit::Done));
        // A search keeps the matching cards only.
        let v = View { query: "vani", ..v };
        let l = layout(&v);
        assert_eq!(l.cards[1].len(), 1);
        assert!(l.cards[0].is_empty());
        assert_eq!(
            step(&l.cards[1], None, true).as_deref(),
            Some("vanilla"),
            "the first card"
        );
    }
}
