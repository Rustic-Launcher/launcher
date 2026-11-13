//! The Rustic look of every screen's widgets (launcher ADR 0160): the widget skin Rustic's Horizon
//! patch U15 lets the executable install (`horizon_client::ui::skin`). Buttons, sliders, fields,
//! check boxes, tabs, scroll bars, tooltips and backgrounds are drawn from
//! [`rusticmc_companion_core::skin`]'s plates with the [`crate::canvas`], their labels and the
//! screens' centred lines set smooth in Inter. Everything else stays the game's.
use crate::canvas::{Canvas, abgr, canvas, measure_px};
use horizon_client::hud::{HudAtlas, HudBatch};
use horizon_client::text::Text;
use horizon_client::ui::skin::{LabelBox, LabelKind, Skin};
use rusticmc_companion_core::paint::fade;
use rusticmc_companion_core::skin::{self as look, Widget, label_color};
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

fn draw_label(c: &mut Canvas, batch: &mut HudBatch, text: &Text, at: LabelBox, color: u32) -> bool {
    let widget = at.kind == LabelKind::Widget;
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
    let baseline = ((at.y + 7.0) * s).round();
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

impl Skin for RusticSkin {
    fn begin(&self, atlas: &HudAtlas) {
        guarded("begin", || {
            canvas().begin(atlas);
            true
        });
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
            return false;
        };
        guarded(sprite, || {
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
            for p in look::paint(widget, rect, batch.scale, track) {
                let p = if alpha < 1.0 { faded(p, alpha) } else { p };
                c.paint(batch, &p);
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
            let mut c = canvas();
            c.ready() && draw_label(&mut c, batch, text, at, color)
        })
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
    fn labels_split_into_coloured_runs_and_odd_ones_stay_vanilla() {
        let t = Text::literal("Done");
        let r = runs(&t, 0xFFF3_F1EE, true).unwrap();
        assert_eq!(r, vec![("Done".to_owned(), 0xFFF3_F1EE, Weight::SemiBold)]);
        let r = runs(&t, 0xFFF3_F1EE, false).unwrap();
        assert_eq!(r[0].2, Weight::Medium);
    }
}
