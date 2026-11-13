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
