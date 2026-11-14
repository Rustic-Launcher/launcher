//! The pieces Rustic's option pages share (Resource Packs, Browse Mods, Create World, ADR 0165):
//! the header with the page's title and its search field, the footer bar, small arrow buttons and
//! a switch, in the launcher's look. Coordinates are layout units (`paint::screen_unit`).
use crate::kit::Kit;
use crate::paint::{Align, Paint};
use crate::skin::palette;
use crate::title::Weight;

/// The header's height (the title, then the search row).
pub const HEADER: f32 = 58.0;
/// The footer bar's height.
pub const FOOTER: f32 = 40.0;
/// A footer button's height.
pub const BUTTON_H: f32 = 22.0;

/// The page's title centred at the top.
pub fn title<H: Clone>(k: &mut Kit<'_, H>, gw: f32, text: &str) {
    k.text(
        gw / 2.0,
        18.5,
        text,
        11.0,
        Weight::SemiBold,
        palette::INK,
        Align::Center,
    );
}

/// The search row's box under the title, `width` at most.
pub fn search_rect(gw: f32, width: f32) -> [f32; 4] {
    let w = (gw - 24.0).min(width);
    [((gw - w) / 2.0).round(), 31.0, w, 18.0]
}

/// The footer bar's plate at the bottom of a `gw` by `gh` page; its centre line.
pub fn footer<H: Clone>(k: &mut Kit<'_, H>, gw: f32, gh: f32) -> f32 {
    let top = gh - FOOTER;
    k.fill([0.0, top, gw, FOOTER], 0xFF0F_1013);
    k.fill([0.0, top, gw, 0.5], palette::RULE);
    top + FOOTER / 2.0
}

/// A small square button with an arrow (up or down) or a sign (`+`, `−`) on it.
pub fn icon_button<H: Clone>(k: &mut Kit<'_, H>, r: [f32; 4], icon: Icon, on: bool, h: H) {
    let lit = on && k.over(r);
    let (fill, ink) = match (on, lit) {
        (false, _) => (palette::FACE_OFF, palette::INK_OFF),
        (true, true) => (palette::ACCENT, 0xFFFF_FFFF),
        (true, false) => (0xF22A_2B31, palette::INK),
    };
    k.round(r, 3.0, fill, if on { palette::BORDER } else { 0 });
    let [cx, cy] = [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0];
    let s = r[3] * 0.22;
    match icon {
        Icon::Up | Icon::Down => {
            let d = if icon == Icon::Up { -1.0 } else { 1.0 };
            k.out.push(Paint::Quad {
                points: [
                    [cx - s * 1.2, cy - d * s * 0.6],
                    [cx + s * 1.2, cy - d * s * 0.6],
                    [cx, cy + d * s * 0.8],
                    [cx, cy + d * s * 0.8],
                ],
                argb: ink,
            });
        }
        Icon::Plus | Icon::Minus => {
            let t = (r[3] * 0.06).max(0.6);
            k.fill([cx - s * 1.3, cy - t, s * 2.6, 2.0 * t], ink);
            if icon == Icon::Plus {
                k.fill([cx - t, cy - s * 1.3, 2.0 * t, s * 2.6], ink);
            }
        }
    }
    if on {
        k.hit(r, h);
    }
}

/// A text field (a form's, no magnifier): its text or placeholder, the caret while focused.
pub fn input<H: Clone>(
    k: &mut Kit<'_, H>,
    rect: [f32; 4],
    text: &str,
    placeholder: &str,
    focused: bool,
    caret: bool,
    h: H,
) {
    let lit = focused || k.over(rect);
    let border = if focused {
        palette::ACCENT
    } else if lit {
        0x40FF_FFFF
    } else {
        0x26FF_FFFF
    };
    k.round(rect, (rect[3] * 0.3).min(5.0), palette::FIELD, border);
    let size = (rect[3] * 0.44).clamp(5.5, 8.0);
    let tx = rect[0] + rect[3] * 0.4;
    let cy = rect[1] + rect[3] / 2.0;
    let room = rect[0] + rect[2] - rect[3] * 0.4 - tx;
    let w = if text.is_empty() {
        k.text_fit(
            tx,
            cy,
            room,
            placeholder,
            size,
            Weight::Medium,
            palette::MUTED,
        );
        0.0
    } else {
        // The end of the text shows when it is too long.
        let mut shown: String = text.to_owned();
        while (k.measure)(&shown, size, Weight::Medium) > room && !shown.is_empty() {
            shown.remove(0);
        }
        k.text(
            tx,
            cy,
            &shown,
            size,
            Weight::Medium,
            palette::INK,
            Align::Left,
        )
    };
    if focused && caret {
        k.fill(
            [tx + w + 0.5, cy - size * 0.55, 0.75, size * 1.1],
            palette::ACCENT,
        );
    }
    k.hit(rect, h);
}

/// What an [`icon_button`] shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Up,
    Down,
    Plus,
    Minus,
}

/// A switch (on: the accent) whose right edge is `right`, centred on `cy`; its box.
pub fn switch<H: Clone>(
    k: &mut Kit<'_, H>,
    right: f32,
    cy: f32,
    on: bool,
    enabled: bool,
    h: H,
) -> [f32; 4] {
    let (w, ht) = (18.0, 10.0);
    let r = [right - w, cy - ht / 2.0, w, ht];
    let track = match (enabled, on) {
        (false, _) => palette::FACE_OFF,
        (true, true) => palette::ACCENT,
        (true, false) => 0xFF3A_3B42,
    };
    k.round(
        r,
        ht / 2.0,
        track,
        if enabled { 0 } else { palette::BORDER_OFF },
    );
    let knob = ht - 3.0;
    let kx = if on {
        r[0] + w - knob - 1.5
    } else {
        r[0] + 1.5
    };
    let ink = if enabled {
        0xFFFF_FFFF
    } else {
        palette::INK_OFF
    };
    k.round([kx, cy - knob / 2.0, knob, knob], knob / 2.0, ink, 0);
    if enabled {
        k.hit(r, h);
    }
    r
}

/// A dashed-free empty state: one muted line centred in `r`.
pub fn empty_line<H: Clone>(k: &mut Kit<'_, H>, r: [f32; 4], text: &str) {
    let lines = crate::paint::wrap(text, r[2] - 16.0, 7.0, Weight::Medium, 3, k.measure);
    let lh = crate::kit::line(7.0);
    let top = r[1] + r[3] / 2.0 - lh * lines.len() as f32 / 2.0;
    for (i, l) in lines.iter().enumerate() {
        k.text(
            r[0] + r[2] / 2.0,
            top + lh * (i as f32 + 0.5),
            l,
            7.0,
            Weight::Medium,
            palette::MUTED,
            Align::Center,
        );
    }
}

/// The next of `n` focus stops after `at` (back: before), wrapping; the first with none.
pub fn step_focus(at: Option<usize>, n: usize, back: bool) -> Option<usize> {
    if n == 0 {
        return None;
    }
    Some(match (at, back) {
        (None, false) => 0,
        (None, true) => n - 1,
        (Some(i), false) => (i + 1) % n,
        (Some(i), true) => (i + n - 1) % n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    #[test]
    fn small_pieces_take_presses_only_when_on() {
        let mut k: Kit<'_, u8> = Kit::new(&test_measure, [0.0, 0.0]);
        icon_button(&mut k, [0.0, 0.0, 12.0, 12.0], Icon::Up, true, 1);
        icon_button(&mut k, [20.0, 0.0, 12.0, 12.0], Icon::Plus, false, 2);
        switch(&mut k, 100.0, 10.0, true, true, 3);
        switch(&mut k, 140.0, 10.0, false, false, 4);
        let hits: Vec<u8> = k.hits.iter().map(|(_, h)| *h).collect();
        assert_eq!(hits, [1, 3]);
        assert_eq!(step_focus(None, 3, false), Some(0));
        assert_eq!(step_focus(Some(0), 3, true), Some(2));
        assert_eq!(step_focus(Some(2), 3, false), Some(0));
        assert_eq!(step_focus(None, 0, false), None);
        assert_eq!(search_rect(800.0, 320.0)[2], 320.0);
    }
}
