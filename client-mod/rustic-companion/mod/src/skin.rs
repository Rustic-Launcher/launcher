//! The Rustic look of every screen's widgets (launcher ADR 0160): the widget skin Rustic's Horizon
//! patch U15 lets the executable install (`horizon_client::ui::skin`). Buttons, sliders, fields,
//! check boxes, tabs, scroll bars, tooltips and backgrounds are drawn from
//! [`rusticmc_companion_core::skin`]'s plates with the [`crate::canvas`], their labels and the
//! screens' centred lines set smooth in Inter. The rest of a menu's text (titles, list rows,
//! descriptions: Rustic's patch U17, `Skin::text`) is set in Inter too, each glyph in the place
//! of the game's so the screens keep their layout; a dialog gets a card behind its content and
//! its main and destructive buttons their colours ([`crate::frame`], ADR 0164). Everything else
//! stays the game's.
use crate::canvas::{Canvas, abgr, canvas, measure_px};
use crate::frame::{self, frames};
use horizon_client::hud::{HudAtlas, HudBatch};
use horizon_client::text::Text;
use horizon_client::ui::skin::{LabelBox, LabelKind, Skin, TextLine};
use rusticmc_companion_core::paint::fade;
use rusticmc_companion_core::skin::{self as look, Role, Widget, label_color, palette};
use rusticmc_companion_core::title::Weight;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the Rustic theme is on (`rustic_theme` in the companion's config), read on every
/// widget without taking the companion's lock.
pub static ON: AtomicBool = AtomicBool::new(true);

pub struct RusticSkin;

/// The skin the edition installs.
pub static SKIN: RusticSkin = RusticSkin;

/// The look of the settings pages and overlays Horizon draws itself (its graphics settings,
/// the on-screen keyboard, the boot screen: `horizon_client::menu_style`), in Rustic's palette
/// and Inter, sizes as Horizon's design system has them.
pub fn menu_style() -> horizon_client::menu_style::MenuStyle {
    use horizon_client::menu_style::{Fonts, MenuStyle, OverlayColours, SettingsColours, Sizes};
    MenuStyle {
        settings: SettingsColours {
            page: 0x0b0c0f,
            ink: 0xf3f1ee,
            ink_soft: 0xd6d3cf,
            muted: 0xa29e98,
            outline: 0x050506,
            accent: 0xff6a00,
            face: 0xe85f0a,
            face_hover: 0xff7a1f,
            success: 0x4ade80,
            warn: 0xf5b83d,
            danger: 0xf0645a,
        },
        overlay: OverlayColours {
            page: 0x0b0c0f,
            backdrop_muted: 0xa29e98,
            ink: 0xf3f1ee,
            ink_soft: 0xd6d3cf,
            rule: 0x2a2b30,
            accent: 0xff6a00,
            on_accent: 0xffffff,
        },
        sizes: Sizes {
            caption: 12.0,
            label: 13.0,
            ui: 14.0,
            body: 15.0,
            title: 28.0,
            display: 44.0,
            micro: 11.0,
            radius: 8.0,
            radius_plate: 12.0,
            control_large: 40.0,
        },
        fonts: Some(Fonts {
            regular: crate::canvas::INTER_MEDIUM,
            medium: crate::canvas::INTER_MEDIUM,
            semibold: crate::canvas::INTER_SEMIBOLD,
            mono: crate::canvas::INTER_MEDIUM,
        }),
    }
}

/// Runs a drawing body; a panic (which would end the game) is logged and the widget left to
/// vanilla.
fn guarded(what: &str, f: impl FnOnce() -> bool) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(drawn) => drawn,
        Err(_) => {
            log::error!("Rustic skin: {what} failed; vanilla's drawn instead");
            ON.store(false, Ordering::Relaxed);
            false
        }
    }
}

/// A label's runs: text in one colour (ARGB) and weight.
fn runs(text: &Text, base: u32, widget: bool) -> Option<Vec<(String, u32, Weight)>> {
    let mut out: Vec<(String, u32, Weight)> = Vec::new();
    for (c, style) in text.chars() {
        if style.obfuscated == Some(true)
            || style.inline_object.is_some()
            || style
                .font
                .as_deref()
                .is_some_and(|f| f != "minecraft:default" && f != "default")
        {
            return None;
        }
        let color = match style.color {
            Some(rgb) => (base & 0xFF00_0000) | (rgb & 0x00FF_FFFF),
            None => base,
        };
        let weight = if widget || style.bold == Some(true) {
            Weight::SemiBold
        } else {
            Weight::Medium
        };
        match out.last_mut() {
            Some((s, c0, w0)) if *c0 == color && *w0 == weight => s.push(c),
            _ => out.push((c.to_string(), color, weight)),
        }
    }
    Some(out)
}

/// The em of a label in GUI pixels.
const WIDGET_EM: f32 = 8.0;
const LINE_EM: f32 = 8.5;
/// A menu line's em: its glyphs sit in the game's 6-pixel cells.
const TEXT_EM: f32 = 8.0;

fn draw_label(c: &mut Canvas, batch: &mut HudBatch, text: &Text, at: LabelBox, color: u32) -> bool {
    let widget = at.kind == LabelKind::Widget;
    // A slider's label sits over its rail: lifted so the two keep apart.
    let lift = if widget
        && c.last_track.is_some_and(|t| {
            rusticmc_companion_core::paint::inside(t, [at.center_x, (at.top + at.bottom) / 2.0])
        }) {
        look::SLIDER_LABEL_LIFT
    } else {
        0.0
    };
    let base = label_color(color);
    let Some(mut parts) = runs(text, base, widget) else {
        return false;
    };
    let s = batch.scale.max(1.0);
    let px = ((if widget { WIDGET_EM } else { LINE_EM }) * s).round();
    let width: f32 = parts.iter().map(|(t, _, w)| measure_px(t, px, *w)).sum();
    let (left, right) = (at.left * s, at.right * s);
    let room = right - left - 2.0;
    // Too wide: one run, cut with an ellipsis.
    if width > room {
        let all: String = parts.iter().map(|(t, _, _)| t.as_str()).collect();
        let weight = parts.first().map_or(Weight::Medium, |p| p.2);
        let measure = |t: &str, size: f32, w: Weight| measure_px(t, size, w);
        let cut = rusticmc_companion_core::paint::fit(&all, room, px, weight, &measure);
        parts = vec![(cut, base, weight)];
    }
    let width: f32 = parts.iter().map(|(t, _, w)| measure_px(t, px, *w)).sum();
    let half = width / 2.0;
    let cx = (at.center_x * s).max(left + half).min(right - half);
    let start = (cx - half).round();
    // Vanilla's 9-pixel line has its baseline 7 pixels down.
    let baseline = ((at.y + 7.0 - lift) * s).round();
    if !widget {
        // A soft shadow keeps titles readable over the panorama.
        let mut pen = start + 1.0;
        for (t, argb, w) in &parts {
            let shadow = abgr(fade(0x9000_0000, (argb >> 24) as f32 / 255.0));
            pen = c.text_px(batch, pen, baseline + 1.0, t, px, *w, shadow);
        }
    }
    let mut pen = start;
    for (t, argb, w) in &parts {
        pen = c.text_px(batch, pen, baseline, t, px, *w, abgr(*argb));
    }
    true
}

/// A menu line's colour (ARGB) from the game's: white is the theme's ink, vanilla's greys the
/// theme's muted inks, any other colour its own.
pub fn line_color(argb: u32) -> u32 {
    let a = argb & 0xFF00_0000;
    let rgb = match argb & 0x00FF_FFFF {
        0xFF_FFFF => palette::INK,
        0xA0_A0A0 | 0xAA_AAAA | 0xE0_E0E0 => palette::MUTED,
        0x80_8080 | 0x70_7070 => 0xFF8A_867F,
        0x55_5555 | 0x40_4040 => 0xFF6E_6B67,
        other => other,
    };
    a | (rgb & 0x00FF_FFFF)
}

/// A cluster of a menu line: its characters (a base and the marks the game draws on it), where
/// the game's glyph starts and its advance, its colour (ARGB), weight, underline and strike.
struct Cluster {
    text: String,
    at: f32,
    adv: f32,
    color: u32,
    weight: Weight,
    under: bool,
    strike: bool,
}

/// A menu screen's line (`Skin::text`): each glyph of Inter centred in the game's glyph's
/// place, so carets, selections and the text after it stay where the screen put them; a mark
/// the game gives no advance (Thai, combining accents) is set with the glyph before it.
fn draw_line(c: &mut Canvas, batch: &mut HudBatch, line: &TextLine<'_>) -> bool {
    let base = {
        let [a, b, g, r] = line.color.to_be_bytes();
        u32::from_be_bytes([a, r, g, b])
    };
    let s = batch.scale.max(1.0);
    let px = (TEXT_EM * s).round();
    // Vanilla's 9-pixel line has its baseline 7 pixels down.
    let baseline = ((line.y + 7.0) * s).round();
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut pen = line.x;
    let mut count = 0;
    for ((ch, style), adv) in line.text.chars().zip(line.advances.iter().copied()) {
        count += 1;
        if style.obfuscated == Some(true)
            || style.inline_object.is_some()
            || style
                .font
                .as_deref()
                .is_some_and(|f| f != "minecraft:default" && f != "default")
        {
            return false;
        }
        let color = line_color(match style.color {
            Some(rgb) => (base & 0xFF00_0000) | (rgb & 0x00FF_FFFF),
            None => base,
        });
        let weight = if style.bold == Some(true) {
            Weight::SemiBold
        } else {
            Weight::Medium
        };
        match clusters.last_mut() {
            Some(last) if adv <= 0.0 && !ch.is_whitespace() => last.text.push(ch),
            _ => clusters.push(Cluster {
                text: ch.to_string(),
                at: pen,
                adv,
                color,
                weight,
                under: style.underlined == Some(true),
                strike: style.strikethrough == Some(true),
            }),
        }
        pen += adv;
    }
    if count != line.advances.len() {
        return false;
    }
    // Inter's own spacing, fitted to the game's width: a slightly larger size, then a little
    // tracking, so the line ends about where the game's would (carets, the text after it).
    let vanilla: f32 = line.advances.iter().sum::<f32>() * s;
    let natural = |px: f32| -> f32 {
        clusters
            .iter()
            .map(|k| measure_px(&k.text, px, k.weight))
            .sum()
    };
    let w0 = natural(px);
    let px = if w0 > 0.0 {
        (px * (vanilla / w0).clamp(1.0, 1.06)).round()
    } else {
        px
    };
    let widths: Vec<f32> = clusters
        .iter()
        .map(|k| measure_px(&k.text, px, k.weight))
        .collect();
    let gaps = clusters.len().saturating_sub(1).max(1) as f32;
    let track = ((vanilla - widths.iter().sum::<f32>()) / gaps).clamp(-0.25 * s, 0.12 * s);
    // A line the screen centred (a title, a dialog's message) stays centred; others start where
    // the game's would.
    let natural: f32 = widths.iter().sum::<f32>() + track * gaps;
    let centred = ((line.x + vanilla / s / 2.0) - batch.width / 2.0).abs() < 1.5;
    let start = if centred && natural < vanilla {
        (line.x * s + (vanilla - natural) / 2.0).round()
    } else {
        (line.x * s).round()
    };
    for shadow in [true, false] {
        if shadow && !line.shadow {
            continue;
        }
        let mut pen = start;
        for (k, w) in clusters.iter().zip(&widths) {
            let alpha = (k.color >> 24) as f32 / 255.0;
            let ink = if shadow {
                abgr(fade(0x8000_0000, alpha))
            } else {
                abgr(k.color)
            };
            let off = if shadow { 1.0 } else { 0.0 };
            if !k.text.trim().is_empty() {
                c.text_px(
                    batch,
                    pen.round() + off,
                    baseline + off,
                    &k.text,
                    px,
                    k.weight,
                    ink,
                );
            }
            pen += w + track;
            if !shadow && (k.under || k.strike) {
                let y = if k.under { line.y + 8.0 } else { line.y + 4.0 };
                c.fill(batch, [k.at, y, k.adv, 1.0 / s], k.color);
            }
        }
    }
    true
}

impl Skin for RusticSkin {
    fn begin(&self, atlas: &HudAtlas) {
        guarded("begin", || {
            canvas().begin(atlas);
            frames().begin();
            true
        });
    }

    fn text(&self, batch: &mut HudBatch, _atlas: &HudAtlas, line: &TextLine<'_>) -> bool {
        if !ON.load(Ordering::Relaxed) || line.color >> 24 < 8 || line.advances.is_empty() {
            return false;
        }
        guarded("text", || {
            let width: f32 = line.advances.iter().sum();
            let rect = [line.x, line.y, width, 9.0];
            if frames().hidden(rect) {
                frames().text_box(rect);
                return true;
            }
            let drawn = {
                let mut c = canvas();
                c.ready() && draw_line(&mut c, batch, line)
            };
            if drawn {
                frames().text_box(rect);
            }
            drawn
        })
    }

    fn sprite(
        &self,
        batch: &mut HudBatch,
        _atlas: &HudAtlas,
        sprite: &str,
        rect: [f32; 4],
        color: u32,
    ) -> bool {
        if !ON.load(Ordering::Relaxed) {
            return false;
        }
        let Some(widget) = look::widget(sprite) else {
            // An icon on a button the screen just drew: the button's.
            let mut f = frames();
            let hide = f.hidden(rect);
            if let Some(b) = f.current.buttons.last_mut()
                && rusticmc_companion_core::paint::inside(b.rect, frame::centre(rect))
                && b.icon.is_none()
            {
                b.icon = Some(sprite.to_owned());
            }
            return hide;
        };
        guarded(sprite, || {
            let role = record(widget, rect, batch);
            if !matches!(widget, Widget::MenuBackground { .. }) && frames().hidden(rect) {
                return true;
            }
            let mut c = canvas();
            if !c.ready() {
                return false;
            }
            let track = match widget {
                Widget::SliderTrack { .. } => {
                    c.last_track = Some(rect);
                    None
                }
                Widget::SliderHandle { .. } => c.last_track,
                _ => None,
            };
            // The title's fade-in tints its widgets' alpha.
            let alpha = (color >> 24) as f32 / 255.0;
            for p in look::paint_as(widget, rect, batch.scale, track, role) {
                let p = if alpha < 1.0 { faded(p, alpha) } else { p };
                c.paint(batch, &p);
            }
            // A dialog's card, over the whole screen's background.
            if let Widget::MenuBackground { .. } = widget
                && full_screen(rect, batch)
            {
                let card = frames().card;
                if let Some(r) = card.rect
                    && let Some(paints) =
                        look::card(r, [batch.width, batch.height], card.alpha * alpha)
                {
                    c.paint_all(batch, &paints);
                }
            }
            true
        })
    }

    fn label(
        &self,
        batch: &mut HudBatch,
        _atlas: &HudAtlas,
        text: &Text,
        at: LabelBox,
        color: u32,
    ) -> bool {
        if !ON.load(Ordering::Relaxed) || color >> 24 < 8 {
            return false;
        }
        guarded("label", || {
            let hidden = frames().hidden([at.left, at.top, at.right - at.left, at.bottom - at.top]);
            let drawn = hidden || {
                let mut c = canvas();
                c.ready() && draw_label(&mut c, batch, text, at, color)
            };
            let mut f = frames();
            match at.kind {
                LabelKind::Widget => {
                    let mid = [at.center_x, (at.top + at.bottom) / 2.0];
                    if let Some(b) = f
                        .current
                        .buttons
                        .iter_mut()
                        .rev()
                        .find(|b| rusticmc_companion_core::paint::inside(b.rect, mid))
                    {
                        b.label = Some(text.plain());
                    }
                }
                LabelKind::Line => {
                    let w = text.plain().chars().count() as f32 * 5.0;
                    let x = (at.center_x - w / 2.0).max(at.left);
                    f.text_box([x, at.y, w.min(at.right - x), 9.0]);
                }
            }
            drawn
        })
    }
}

/// Whether `rect` is the whole screen (a screen's background, not a panel's).
fn full_screen(rect: [f32; 4], batch: &HudBatch) -> bool {
    rect[0] <= 0.5 && rect[1] <= 0.5 && rect[2] >= batch.width - 0.5
}

/// Notes what the screen drew for the frame's record ([`crate::frame`]); the look a button
/// takes (its last frame's label's).
fn record(widget: Widget, rect: [f32; 4], batch: &HudBatch) -> Role {
    let mut f = frames();
    match widget {
        Widget::MenuBackground { .. } => {
            if full_screen(rect, batch) {
                f.current.menu = true;
                f.current.gui = [batch.width, batch.height];
            }
            Role::Normal
        }
        Widget::Button(state) => {
            let role = f.role_at(rect, &horizon_client::lang::code());
            f.current.buttons.push(frame::Button {
                rect,
                enabled: state != look::State::Off,
                label: None,
                icon: None,
            });
            f.content(rect);
            role
        }
        Widget::TextField { .. }
        | Widget::Checkbox { .. }
        | Widget::SliderTrack { .. }
        | Widget::SliderHandle { .. } => {
            f.content(rect);
            Role::Normal
        }
        Widget::Tab { .. }
        | Widget::ListBackground
        | Widget::Separator
        | Widget::TabHeader
        | Widget::ScrollerTrack
        | Widget::Scroller => {
            f.current.paged = true;
            Role::Normal
        }
        Widget::TooltipBackground | Widget::TooltipFrame => {
            f.current.tooltip = true;
            Role::Normal
        }
    }
}

/// `p` with its colours' alpha times `a`.
fn faded(
    p: rusticmc_companion_core::paint::Paint,
    a: f32,
) -> rusticmc_companion_core::paint::Paint {
    use rusticmc_companion_core::paint::Paint;
    match p {
        Paint::Fill { rect, argb } => Paint::Fill {
            rect,
            argb: fade(argb, a),
        },
        Paint::Round {
            rect,
            radius,
            fill,
            border,
        } => Paint::Round {
            rect,
            radius,
            fill: fade(fill, a),
            border: fade(border, a),
        },
        Paint::Quad { points, argb } => Paint::Quad {
            points,
            argb: fade(argb, a),
        },
        Paint::Gradient { rect, left, right } => Paint::Gradient {
            rect,
            left: fade(left, a),
            right: fade(right, a),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_greys_read_as_the_themes_inks() {
        assert_eq!(line_color(0xFFFF_FFFF), palette::INK);
        assert_eq!(line_color(0xFFA0_A0A0), palette::MUTED);
        assert_eq!(line_color(0x80FF_5555), 0x80FF_5555);
    }

    #[test]
    fn labels_split_into_coloured_runs_and_odd_ones_stay_vanilla() {
        let t = Text::literal("Done");
        let r = runs(&t, 0xFFF3_F1EE, true).unwrap();
        assert_eq!(r, vec![("Done".to_owned(), 0xFFF3_F1EE, Weight::SemiBold)]);
        let r = runs(&t, 0xFFF3_F1EE, false).unwrap();
        assert_eq!(r[0].2, Weight::Medium);
    }
}
