//! The pieces Rustic's own screens are built from (the language picker, the server directory, the
//! multiplayer switch), in the title menu's look: plates, buttons, chips, a search field, a
//! scroll bar. Each piece adds its [`Paint`]s and, when it can be pressed, its hit box.
use crate::paint::{Align, Measure, Paint, fit, inside};
use crate::skin::palette;
use crate::title::{LINE, Weight};

/// How a button looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// The accent (Join).
    Primary,
    /// A dark plate.
    Secondary,
    /// Text only until hovered.
    Ghost,
}

/// Builds a screen: what to draw and where presses land (`H`).
pub struct Kit<'a, H> {
    pub out: Vec<Paint>,
    pub hits: Vec<([f32; 4], H)>,
    pub measure: Measure<'a>,
    pub mouse: [f32; 2],
}

/// The height of a line of text of `size`.
pub fn line(size: f32) -> f32 {
    size * LINE
}

impl<'a, H: Clone> Kit<'a, H> {
    pub fn new(measure: Measure<'a>, mouse: [f32; 2]) -> Self {
        Kit {
            out: Vec::new(),
            hits: Vec::new(),
            measure,
            mouse,
        }
    }

    pub fn over(&self, r: [f32; 4]) -> bool {
        inside(r, self.mouse)
    }

    pub fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.out.push(Paint::Fill { rect, argb });
    }

    pub fn round(&mut self, rect: [f32; 4], radius: f32, fill: u32, border: u32) {
        self.out.push(Paint::Round {
            rect,
            radius,
            fill,
            border,
        });
    }

    /// A line of text whose box is vertically centred on `cy`; its width.
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        x: f32,
        cy: f32,
        text: &str,
        size: f32,
        weight: Weight,
        argb: u32,
        align: Align,
    ) -> f32 {
        let w = (self.measure)(text, size, weight);
        if !text.is_empty() {
            self.out.push(Paint::Text {
                x,
                y: cy - line(size) / 2.0,
                text: text.to_owned(),
                size,
                weight,
                argb,
                align,
            });
        }
        w
    }

    /// `text` cut to `width`, left-aligned at `x`, centred on `cy`.
    #[allow(clippy::too_many_arguments)]
    pub fn text_fit(
        &mut self,
        x: f32,
        cy: f32,
        width: f32,
        text: &str,
        size: f32,
        weight: Weight,
        argb: u32,
    ) {
        let t = fit(text, width, size, weight, self.measure);
        self.text(x, cy, &t, size, weight, argb, Align::Left);
    }

    pub fn hit(&mut self, rect: [f32; 4], h: H) {
        self.hits.push((rect, h));
    }

    /// A button with its label centred; `focused` lights it as a hover does.
    pub fn button(&mut self, rect: [f32; 4], label: &str, look: Look, focused: bool, h: H) {
        let lit = focused || self.over(rect);
        let (fill, border, ink) = match (look, lit) {
            (Look::Primary, false) => (0xF2E8_5F0A, 0, palette::INK),
            (Look::Primary, true) => (palette::ACCENT, 0xFFFF_B37A, 0xFFFF_FFFF),
            (Look::Secondary, false) => (palette::FACE, palette::BORDER, palette::INK),
            (Look::Secondary, true) => (palette::FACE_HOVER, palette::ACCENT, palette::INK),
            (Look::Ghost, false) => (0, 0, palette::MUTED),
            (Look::Ghost, true) => (0x14FF_FFFF, palette::BORDER, palette::INK),
        };
        self.round(rect, (rect[3] * 0.22).clamp(2.0, 5.0), fill, border);
        let size = (rect[3] * 0.42).clamp(5.5, 8.5);
        let label = fit(label, rect[2] - 6.0, size, Weight::SemiBold, self.measure);
        self.text(
            rect[0] + rect[2] / 2.0,
            rect[1] + rect[3] / 2.0,
            &label,
            size,
            Weight::SemiBold,
            ink,
            Align::Center,
        );
        self.hit(rect, h);
    }

    /// A chip (a category, a segment); the selected one in the accent.
    pub fn chip(&mut self, rect: [f32; 4], label: &str, selected: bool, h: H) {
        let lit = self.over(rect);
        let (fill, border, ink) = match (selected, lit) {
            (true, _) => (palette::ACCENT_SOFT, palette::ACCENT, palette::ACCENT_TEXT),
            (false, true) => (palette::FACE_HOVER, 0x33FF_FFFF, palette::INK),
            (false, false) => (palette::FACE, palette::BORDER, palette::MUTED),
        };
        self.round(rect, rect[3] / 2.0, fill, border);
        let size = (rect[3] * 0.46).clamp(5.0, 7.5);
        self.text(
            rect[0] + rect[2] / 2.0,
            rect[1] + rect[3] / 2.0,
            label,
            size,
            Weight::SemiBold,
            ink,
            Align::Center,
        );
        self.hit(rect, h);
    }

    /// A chip's width for `label` at height `h`.
    pub fn chip_width(&self, label: &str, h: f32) -> f32 {
        let size = (h * 0.46).clamp(5.0, 7.5);
        ((self.measure)(label, size, Weight::SemiBold) + h * 1.1).ceil()
    }

    /// A search field: its text or placeholder, a caret while focused.
    pub fn field(
        &mut self,
        rect: [f32; 4],
        text: &str,
        placeholder: &str,
        focused: bool,
        caret: bool,
        h: H,
    ) {
        let lit = focused || self.over(rect);
        self.round(
            rect,
            (rect[3] * 0.3).min(5.0),
            palette::FIELD,
            if focused {
                palette::ACCENT
            } else if lit {
                0x40FF_FFFF
            } else {
                0x26FF_FFFF
            },
        );
        let size = (rect[3] * 0.44).clamp(5.5, 8.0);
        let pad = rect[3] * 0.4;
        let cy = rect[1] + rect[3] / 2.0;
        // The magnifier: a ring and its handle.
        let r = rect[3] * 0.17;
        let c = [rect[0] + pad + r, cy - r * 0.2];
        self.out.push(Paint::Round {
            rect: [c[0] - r, c[1] - r, 2.0 * r, 2.0 * r],
            radius: r,
            fill: 0,
            border: palette::MUTED,
        });
        let t = 0.6;
        let a = [c[0] + r * 0.7, c[1] + r * 0.7];
        let b = [c[0] + r * 1.6, c[1] + r * 1.6];
        self.out.push(Paint::Quad {
            points: [
                [a[0] - t, a[1] + t],
                [a[0] + t, a[1] - t],
                [b[0] + t, b[1] - t],
                [b[0] - t, b[1] + t],
            ],
            argb: palette::MUTED,
        });
        let tx = c[0] + r * 2.4 + 2.0;
        let room = rect[0] + rect[2] - pad - tx;
        if text.is_empty() {
            self.text_fit(
                tx,
                cy,
                room,
                placeholder,
                size,
                Weight::Medium,
                palette::MUTED,
            );
            if focused && caret {
                self.fill([tx, cy - size * 0.55, 0.75, size * 1.1], palette::ACCENT);
            }
        } else {
            // The end of the text shows when it is too long.
            let mut shown: String = text.to_owned();
            while (self.measure)(&shown, size, Weight::Medium) > room && !shown.is_empty() {
                shown.remove(0);
            }
            let w = self.text(
                tx,
                cy,
                &shown,
                size,
                Weight::Medium,
                palette::INK,
                Align::Left,
            );
            if focused && caret {
                self.fill(
                    [tx + w + 0.5, cy - size * 0.55, 0.75, size * 1.1],
                    palette::ACCENT,
                );
            }
        }
        self.hit(rect, h);
    }

    /// A thin scroll bar at the right of `area` for `content` pixels scrolled by `scroll`.
    pub fn scrollbar(&mut self, area: [f32; 4], content: f32, scroll: f32) {
        let [x, y, w, h] = area;
        if content <= h {
            return;
        }
        let bar = (h * h / content).max(16.0).min(h);
        let top = y + (scroll / (content - h)).clamp(0.0, 1.0) * (h - bar);
        self.round([x + w - 3.0, y, 2.0, h], 1.0, 0x14FF_FFFF, 0);
        self.round([x + w - 3.0, top, 2.0, bar], 1.0, 0x80FF_FFFF, 0);
    }
}

/// The largest scroll of `content` in a view `view` high.
pub fn max_scroll(content: f32, view: f32) -> f32 {
    (content - view).max(0.0)
}

/// A string folded for search: lower case, Latin accents dropped.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
            'ç' | 'ć' | 'č' => 'c',
            'ď' | 'đ' => 'd',
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
            'ł' => 'l',
            'ñ' | 'ń' | 'ň' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => 'o',
            'ř' => 'r',
            'ś' | 'š' | 'ş' => 's',
            'ť' | 'ţ' => 't',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
            'ý' | 'ÿ' => 'y',
            'ź' | 'ż' | 'ž' => 'z',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    #[test]
    fn pieces_register_where_they_can_be_pressed() {
        let mut k: Kit<'_, u8> = Kit::new(&test_measure, [15.0, 15.0]);
        k.button([10.0, 10.0, 60.0, 16.0], "Join", Look::Primary, false, 1);
        k.chip([80.0, 10.0, 30.0, 12.0], "PvP", true, 2);
        k.field([0.0, 40.0, 120.0, 16.0], "", "Search", true, true, 3);
        assert_eq!(k.hits.len(), 3);
        assert_eq!(k.hits[0].1, 1);
        // The hovered primary button is the full accent.
        assert!(k.out.iter().any(|p| matches!(
            p,
            Paint::Round {
                fill: palette::ACCENT,
                ..
            }
        )));
        // The placeholder and the caret show in an empty focused field.
        assert!(
            k.out
                .iter()
                .any(|p| matches!(p, Paint::Text { text, .. } if text == "Search"))
        );
    }

    #[test]
    fn search_ignores_case_and_accents() {
        assert_eq!(fold("Français"), "francais");
        assert_eq!(fold("ČEŠTINA"), "cestina");
        assert_eq!(fold("日本語"), "日本語");
        assert_eq!(max_scroll(100.0, 40.0), 60.0);
        assert_eq!(max_scroll(10.0, 40.0), 0.0);
    }
}
