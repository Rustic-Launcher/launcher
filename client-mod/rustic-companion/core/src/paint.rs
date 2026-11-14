//! What Rustic's screens and widgets draw, as data: the game crate paints it with the companion's
//! canvas (smooth text in Inter, round corners and pictures from the HUD atlas's mod slot). All
//! coordinates are GUI pixels; a text's size is its em in GUI pixels.
use crate::title::{Image, Weight};

/// How a line of text sits on its `x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// A picture the canvas can draw.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Picture {
    /// The title menu's pictures (the logo, the icon strip's glyphs, store tiles).
    Title(Image),
    /// A language's flag by the game's language code (`fr_fr`), drawn by [`crate::flags`].
    Flag(String),
    /// A picture downloaded from Rustic's CDN (a server's icon or banner), by its URL.
    Remote(String),
}

/// One thing to draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Fill {
        rect: [f32; 4],
        argb: u32,
    },
    /// A box with round corners (`radius` GUI pixels) and a one-screen-pixel border (none when
    /// `border` is fully transparent).
    Round {
        rect: [f32; 4],
        radius: f32,
        fill: u32,
        border: u32,
    },
    /// A rectangle shading from `left` to `right` (ARGB), left to right.
    Gradient {
        rect: [f32; 4],
        left: u32,
        right: u32,
    },
    /// A quadrilateral (a triangle when two corners are the same point).
    Quad {
        points: [[f32; 2]; 4],
        argb: u32,
    },
    /// A line of text, its 1-em box's top at `y`.
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        weight: Weight,
        argb: u32,
        align: Align,
    },
    /// `picture` stretched over `rect`, multiplied by `tint` (0xFFFFFFFF: as it is).
    Picture {
        picture: Picture,
        rect: [f32; 4],
        tint: u32,
    },
    /// What follows until [`Paint::ClipEnd`] shows only inside `rect`.
    ClipBegin([f32; 4]),
    ClipEnd,
}

/// A line's width in GUI pixels: `(text, size, weight)`.
pub type Measure<'a> = &'a dyn Fn(&str, f32, Weight) -> f32;

pub fn inside(r: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= r[0] && at[0] < r[0] + r[2] && at[1] >= r[1] && at[1] < r[1] + r[3]
}

/// Rustic's full screens (Discover, languages) are at least this many units high: a 1080p
/// window shows them at about two window pixels per unit, like a desktop app, whatever the
/// game's GUI scale.
pub const SCREEN_UNITS_HIGH: f32 = 480.0;

/// GUI pixels per unit of Rustic's full screens: 1 at small GUI scales, less when the GUI
/// scale would make them fewer than [`SCREEN_UNITS_HIGH`] units high.
pub fn screen_unit(gui_scale: f32, window_height_px: f32) -> f32 {
    let scale = gui_scale.max(1.0);
    let wanted = (window_height_px / SCREEN_UNITS_HIGH).max(1.0);
    wanted.min(scale) / scale
}

/// `r` (in screen units) in GUI pixels, `unit` GUI pixels per unit.
pub fn scale_rect(r: [f32; 4], unit: f32) -> [f32; 4] {
    [r[0] * unit, r[1] * unit, r[2] * unit, r[3] * unit]
}

/// `paints` laid out in screen units, in GUI pixels (`unit` GUI pixels per unit).
pub fn rescaled(paints: &[Paint], unit: f32) -> Vec<Paint> {
    let mut out = paints.to_vec();
    if (unit - 1.0).abs() < f32::EPSILON {
        return out;
    }
    for p in &mut out {
        match p {
            Paint::Fill { rect, .. }
            | Paint::Gradient { rect, .. }
            | Paint::Picture { rect, .. } => {
                *rect = scale_rect(*rect, unit);
            }
            Paint::Round { rect, radius, .. } => {
                *rect = scale_rect(*rect, unit);
                *radius *= unit;
            }
            Paint::Quad { points, .. } => {
                for pt in points.iter_mut() {
                    pt[0] *= unit;
                    pt[1] *= unit;
                }
            }
            Paint::Text { x, y, size, .. } => {
                *x *= unit;
                *y *= unit;
                *size *= unit;
            }
            Paint::ClipBegin(rect) => *rect = scale_rect(*rect, unit),
            Paint::ClipEnd => {}
        }
    }
    out
}

/// `text` cut to `width` with an ellipsis.
pub fn fit(text: &str, width: f32, size: f32, weight: Weight, measure: Measure<'_>) -> String {
    if measure(text, size, weight) <= width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>().trim_end().to_owned() + "…";
        if measure(&cut, size, weight) <= width {
            return cut;
        }
    }
    String::new()
}

/// `text` broken into at most `max_lines` lines of `width`, the last one cut with an ellipsis.
pub fn wrap(
    text: &str,
    width: f32,
    size: f32,
    weight: Weight,
    max_lines: usize,
    measure: Measure<'_>,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let candidate = if line.is_empty() {
            words[i].to_owned()
        } else {
            format!("{line} {}", words[i])
        };
        if measure(&candidate, size, weight) <= width || line.is_empty() {
            line = candidate;
            i += 1;
        } else {
            lines.push(std::mem::take(&mut line));
            if lines.len() == max_lines {
                break;
            }
        }
    }
    if !line.is_empty() && lines.len() < max_lines {
        lines.push(line);
    }
    if i < words.len()
        && let Some(last) = lines.last_mut()
    {
        let rest = format!("{last} {}", words[i..].join(" "));
        *last = fit(&rest, width, size, weight, measure);
    } else if let Some(last) = lines.last_mut() {
        *last = fit(last, width, size, weight, measure);
    }
    lines
}

/// ARGB `argb` with its alpha multiplied by `a` (0-1).
pub fn fade(argb: u32, a: f32) -> u32 {
    let alpha = ((argb >> 24) as f32 * a.clamp(0.0, 1.0)).round() as u32;
    (argb & 0x00FF_FFFF) | (alpha << 24)
}

#[cfg(test)]
pub(crate) fn test_measure(t: &str, size: f32, _: Weight) -> f32 {
    t.chars().count() as f32 * size * 0.55
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_screens_keep_desktop_sizes_at_large_gui_scales() {
        // 1080p at GUI scale 4: about two window pixels per unit, not four.
        assert!((screen_unit(4.0, 1080.0) * 4.0 - 2.25).abs() < 1e-4);
        // At most the GUI scale, never below one window pixel per unit.
        assert!((screen_unit(2.0, 1080.0) - 1.0).abs() < 1e-6);
        assert!((screen_unit(3.0, 600.0) * 3.0 - 1.25).abs() < 1e-4);
        assert!((screen_unit(1.0, 400.0) - 1.0).abs() < 1e-6);
        let p = rescaled(
            &[Paint::Text {
                x: 10.0,
                y: 20.0,
                text: "a".into(),
                size: 8.0,
                weight: Weight::Medium,
                argb: 0,
                align: Align::Left,
            }],
            0.5,
        );
        assert!(matches!(
            p[0],
            Paint::Text {
                x: 5.0,
                y: 10.0,
                size: 4.0,
                ..
            }
        ));
        assert_eq!(scale_rect([2.0, 4.0, 6.0, 8.0], 0.5), [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn long_text_is_cut_and_wrapped() {
        let m: Measure<'_> = &test_measure;
        assert_eq!(fit("Hello", 100.0, 10.0, Weight::Medium, m), "Hello");
        let cut = fit("A rather long server name", 60.0, 10.0, Weight::Medium, m);
        assert!(cut.ends_with('…') && test_measure(&cut, 10.0, Weight::Medium) <= 60.0);
        let lines = wrap(
            "one two three four five six seven eight nine ten",
            60.0,
            10.0,
            Weight::Medium,
            2,
            m,
        );
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'));
        for l in &lines {
            assert!(test_measure(l, 10.0, Weight::Medium) <= 60.0, "{l}");
        }
        assert_eq!(wrap("short", 60.0, 10.0, Weight::Medium, 2, m), ["short"]);
    }

    #[test]
    fn fading_keeps_the_colour() {
        assert_eq!(fade(0xFF12_3456, 0.5), 0x8012_3456);
        assert_eq!(fade(0x8012_3456, 2.0), 0x8012_3456);
        assert!(inside([0.0, 0.0, 10.0, 10.0], [9.9, 0.0]));
        assert!(!inside([0.0, 0.0, 10.0, 10.0], [10.0, 0.0]));
    }
}
