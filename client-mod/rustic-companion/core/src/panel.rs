//! Rustic's pause menu (ADR 0164): the game's own buttons (Back to Game, Advancements,
//! Statistics, Options, Open to LAN, Disconnect...) laid out as the launcher's panel — a card
//! with the screen's title and where the player is, Resume full width in the accent, the others
//! in a two-column grid, the way out (Disconnect, Save and Quit) at the bottom in red. Pure: the
//! game crate gives the buttons (as the game drew them), draws the paints and presses the game's
//! own button for a press.
use crate::kit::{Kit, Look};
use crate::paint::{Align, Measure, Paint};
use crate::skin::{Role, palette};
use crate::title::Weight;

/// One of the screen's buttons.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub label: String,
    pub role: Role,
    pub enabled: bool,
}

/// What the panel is drawn from.
pub struct View<'a> {
    pub gui: [f32; 2],
    pub mouse: [f32; 2],
    pub measure: Measure<'a>,
    pub title: &'a str,
    /// Where the player is ("Singleplayer", a server's address), under the title.
    pub subtitle: &'a str,
    pub entries: &'a [Entry],
    /// The keyboard's focus (an index into `entries`).
    pub focus: Option<usize>,
}

pub struct Layout {
    pub paints: Vec<Paint>,
    /// Each entry's box and index, in the keyboard's order.
    pub hits: Vec<([f32; 4], usize)>,
    pub card: [f32; 4],
}

/// The panel.
pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let mut k: Kit<'_, Option<usize>> = Kit::new(v.measure, v.mouse);
    // The world stays under a dark veil (the game blurs it); the panel takes every press.
    k.fill([0.0, 0.0, gw, gh], 0xE00B_0C0F);
    let w = (gw - 24.0).min(300.0);
    let pad = 14.0;
    let inner = w - 2.0 * pad;
    let primary: Vec<usize> = (0..v.entries.len())
        .filter(|&i| v.entries[i].role == Role::Primary)
        .collect();
    let danger: Vec<usize> = (0..v.entries.len())
        .filter(|&i| v.entries[i].role == Role::Danger)
        .collect();
    let normal: Vec<usize> = (0..v.entries.len())
        .filter(|&i| v.entries[i].role == Role::Normal)
        .collect();
    let big = 26.0;
    let small = 22.0;
    let gap = 6.0;
    let head = 40.0;
    let rows = normal.len().div_ceil(2);
    let h = head
        + primary.len() as f32 * (big + gap)
        + if rows > 0 {
            8.0 + rows as f32 * (small + gap)
        } else {
            0.0
        }
        + if danger.is_empty() {
            0.0
        } else {
            10.0 + danger.len() as f32 * (small + gap)
        }
        + pad
        - gap;
    let card = [
        ((gw - w) / 2.0).round(),
        ((gh - h) / 2.0).round().max(6.0),
        w,
        h,
    ];
    k.round(card, 8.0, palette::PANEL, palette::BORDER);
    k.text(
        card[0] + w / 2.0,
        card[1] + 16.0,
        v.title,
        10.0,
        Weight::SemiBold,
        palette::INK,
        Align::Center,
    );
    if !v.subtitle.is_empty() {
        let sub = crate::paint::fit(v.subtitle, inner, 6.5, Weight::Medium, v.measure);
        k.text(
            card[0] + w / 2.0,
            card[1] + 28.0,
            &sub,
            6.5,
            Weight::Medium,
            palette::MUTED,
            Align::Center,
        );
    }
    let x = card[0] + pad;
    let mut y = card[1] + head;
    let mut boxes: Vec<([f32; 4], usize)> = Vec::new();
    for &i in &primary {
        boxes.push(([x, y, inner, big], i));
        y += big + gap;
    }
    if rows > 0 {
        y += 8.0;
        let half = ((inner - gap) / 2.0).floor();
        for (n, &i) in normal.iter().enumerate() {
            let col = n % 2;
            let last_alone = n + 1 == normal.len() && normal.len() % 2 == 1;
            let bw = if last_alone { inner } else { half };
            let bx = if col == 0 { x } else { x + inner - half };
            boxes.push(([bx, y, bw, small], i));
            if col == 1 || last_alone {
                y += small + gap;
            }
        }
    }
    if !danger.is_empty() {
        y += 4.0;
        k.fill([x, y, inner, 0.5], palette::RULE);
        y += 6.0;
        for &i in &danger {
            boxes.push(([x, y, inner, small], i));
            y += small + gap;
        }
    }
    let mut hits = Vec::new();
    for (r, i) in boxes {
        let e = &v.entries[i];
        let look = match e.role {
            Role::Primary => Look::Primary,
            Role::Danger => Look::Danger,
            Role::Normal => Look::Secondary,
        };
        k.button_on(r, &e.label, look, v.focus == Some(i), e.enabled, Some(i));
        hits.push((r, i));
    }
    Layout {
        paints: k.out,
        hits,
        card,
    }
}

/// The entry the keyboard goes to from `focus` (Tab, the arrows): the next enabled one in the
/// panel's order, `back` the previous.
pub fn step(l: &Layout, entries: &[Entry], focus: Option<usize>, back: bool) -> Option<usize> {
    let order: Vec<usize> = l
        .hits
        .iter()
        .map(|(_, i)| *i)
        .filter(|&i| entries[i].enabled)
        .collect();
    if order.is_empty() {
        return None;
    }
    let n = order.len();
    let at = focus.and_then(|f| order.iter().position(|&i| i == f));
    Some(match (at, back) {
        (None, false) => order[0],
        (None, true) => order[n - 1],
        (Some(p), false) => order[(p + 1) % n],
        (Some(p), true) => order[(p + n - 1) % n],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    fn entries() -> Vec<Entry> {
        [
            ("Back to Game", Role::Primary, true),
            ("Advancements", Role::Normal, true),
            ("Statistics", Role::Normal, true),
            ("Options...", Role::Normal, true),
            ("Open to LAN", Role::Normal, false),
            ("Save and Quit to Title", Role::Danger, true),
        ]
        .into_iter()
        .map(|(l, role, enabled)| Entry {
            label: l.into(),
            role,
            enabled,
        })
        .collect()
    }

    #[test]
    fn resume_leads_the_grid_and_the_way_out_closes_it() {
        let e = entries();
        let l = layout(&View {
            gui: [854.0, 480.0],
            mouse: [0.0, 0.0],
            measure: &test_measure,
            title: "Game Menu",
            subtitle: "Singleplayer",
            entries: &e,
            focus: Some(0),
        });
        assert_eq!(l.hits.len(), 6);
        let at = |i: usize| l.hits.iter().find(|(_, j)| *j == i).unwrap().0;
        // Resume first and full width; the grid two by two; the danger last.
        assert!(at(0)[1] < at(1)[1]);
        assert_eq!(at(1)[1], at(2)[1]);
        assert!(at(2)[0] > at(1)[0]);
        assert!(at(5)[1] > at(4)[1]);
        assert!((at(0)[2] - at(5)[2]).abs() < 0.01);
        // Inside the card.
        for (r, _) in &l.hits {
            assert!(r[0] >= l.card[0] && r[0] + r[2] <= l.card[0] + l.card[2] + 0.01);
            assert!(r[1] >= l.card[1] && r[1] + r[3] <= l.card[1] + l.card[3] + 0.01);
        }
        // The keyboard skips what is off.
        assert_eq!(step(&l, &e, Some(3), false), Some(5));
        assert_eq!(step(&l, &e, Some(0), true), Some(5));
        assert_eq!(step(&l, &e, None, false), Some(0));
    }
}
