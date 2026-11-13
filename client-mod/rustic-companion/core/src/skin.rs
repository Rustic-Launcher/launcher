//! The Rustic look of the game's shared widgets (Rustic's Horizon patch U15, `ui::skin`): every
//! screen asks the skin before it draws a GUI sprite, so the buttons, sliders, fields, check boxes,
//! tabs, scroll bars, tooltips and backgrounds of every vanilla and Horizon screen are drawn here,
//! in the title menu's theme: dark rounded plates with a hairline border, the orange accent on
//! hover and focus, smooth labels. Pure geometry: [`widget`] names what a sprite is and [`paint`]
//! lays it out as [`Paint`]s; the game crate paints them.
use crate::paint::Paint;

/// The palette (ARGB), the launcher's dark theme.
pub mod palette {
    /// Over the panorama or the world, under every screen.
    pub const VEIL: u32 = 0xA60B_0C0F;
    pub const VEIL_WORLD: u32 = 0x8C0B_0C0F;
    /// A list's well between the header and the footer.
    pub const WELL: u32 = 0x5C00_0000;
    pub const RULE: u32 = 0x24FF_FFFF;
    pub const FACE: u32 = 0xE618_191D;
    pub const FACE_HOVER: u32 = 0xF024_252B;
    pub const FACE_OFF: u32 = 0x8C14_1518;
    pub const BORDER: u32 = 0x1FFF_FFFF;
    pub const BORDER_OFF: u32 = 0x0FFF_FFFF;
    pub const ACCENT: u32 = 0xFFFF_6A00;
    pub const ACCENT_SOFT: u32 = 0x40FF_6A00;
    pub const ACCENT_TEXT: u32 = 0xFFFF_8A3D;
    pub const INK: u32 = 0xFFF3_F1EE;
    pub const MUTED: u32 = 0xFFA2_9E98;
    pub const INK_OFF: u32 = 0xFF6E_6B67;
    pub const FIELD: u32 = 0xF20D_0E11;
    pub const PANEL: u32 = 0xF513_1418;
    pub const PANEL_RAISED: u32 = 0xF51B_1C21;
    pub const GREEN: u32 = 0xFF4A_DE80;
    pub const DANGER: u32 = 0xFFF0_645A;
}

/// A widget's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Normal,
    Lit,
    Off,
}

/// What a GUI sprite is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    Button(State),
    SliderTrack { lit: bool },
    SliderHandle { lit: bool },
    TextField { lit: bool },
    Checkbox { checked: bool, lit: bool },
    Tab { selected: bool, lit: bool },
    ScrollerTrack,
    Scroller,
    TooltipBackground,
    TooltipFrame,
    MenuBackground { world: bool },
    ListBackground,
    Separator,
    TabHeader,
}

/// The widget sprite `sprite` (its atlas key) draws, None for one the skin leaves to vanilla
/// (icons, the HUD, containers...).
pub fn widget(sprite: &str) -> Option<Widget> {
    let w = match sprite {
        "gui/sprites/widget/button" => Widget::Button(State::Normal),
        "gui/sprites/widget/button_highlighted" => Widget::Button(State::Lit),
        "gui/sprites/widget/button_disabled" => Widget::Button(State::Off),
        "gui/sprites/widget/slider" => Widget::SliderTrack { lit: false },
        "gui/sprites/widget/slider_highlighted" => Widget::SliderTrack { lit: true },
        "gui/sprites/widget/slider_handle" => Widget::SliderHandle { lit: false },
        "gui/sprites/widget/slider_handle_highlighted" => Widget::SliderHandle { lit: true },
        "gui/sprites/widget/text_field" => Widget::TextField { lit: false },
        "gui/sprites/widget/text_field_highlighted" => Widget::TextField { lit: true },
        "gui/sprites/widget/checkbox" => Widget::Checkbox {
            checked: false,
            lit: false,
        },
        "gui/sprites/widget/checkbox_highlighted" => Widget::Checkbox {
            checked: false,
            lit: true,
        },
        "gui/sprites/widget/checkbox_selected" => Widget::Checkbox {
            checked: true,
            lit: false,
        },
        "gui/sprites/widget/checkbox_selected_highlighted" => Widget::Checkbox {
            checked: true,
            lit: true,
        },
        "gui/sprites/widget/tab" => Widget::Tab {
            selected: false,
            lit: false,
        },
        "gui/sprites/widget/tab_highlighted" => Widget::Tab {
            selected: false,
            lit: true,
        },
        "gui/sprites/widget/tab_selected" => Widget::Tab {
            selected: true,
            lit: false,
        },
        "gui/sprites/widget/tab_selected_highlighted" => Widget::Tab {
            selected: true,
            lit: true,
        },
        "gui/sprites/widget/scroller_background" => Widget::ScrollerTrack,
        "gui/sprites/widget/scroller" => Widget::Scroller,
        "gui/sprites/tooltip/background" => Widget::TooltipBackground,
        "gui/sprites/tooltip/frame" => Widget::TooltipFrame,
        "gui/menu_background" => Widget::MenuBackground { world: false },
        "gui/inworld_menu_background" => Widget::MenuBackground { world: true },
        "gui/menu_list_background" | "gui/inworld_menu_list_background" => Widget::ListBackground,
        "gui/header_separator"
        | "gui/footer_separator"
        | "gui/inworld_header_separator"
        | "gui/inworld_footer_separator" => Widget::Separator,
        "gui/tab_header_background" => Widget::TabHeader,
        _ => return None,
    };
    Some(w)
}

/// One screen pixel in GUI pixels at GUI scale `scale`.
fn px(scale: f32) -> f32 {
    1.0 / scale.max(1.0)
}

/// The corner radius of a control `h` GUI pixels high: about a fifth of it, at least 2.
pub fn radius_for(h: f32) -> f32 {
    (h * 0.2).clamp(2.0, 5.0)
}

/// What the skin draws for widget `w` over `rect` at GUI scale `scale`. `track` is the slider
/// track this frame drew last, for the handle's filled part.
pub fn paint(w: Widget, rect: [f32; 4], scale: f32, track: Option<[f32; 4]>) -> Vec<Paint> {
    let [x, y, width, h] = rect;
    let one = px(scale);
    let mut out = Vec::new();
    let round = |rect: [f32; 4], radius: f32, fill: u32, border: u32| Paint::Round {
        rect,
        radius,
        fill,
        border,
    };
    match w {
        Widget::Button(state) => {
            let (fill, border) = match state {
                State::Normal => (palette::FACE, palette::BORDER),
                State::Lit => (palette::FACE_HOVER, palette::ACCENT),
                State::Off => (palette::FACE_OFF, palette::BORDER_OFF),
            };
            out.push(round(rect, radius_for(h), fill, border));
            if state == State::Lit {
                // A soft accent line under the label, Lunar-like.
                let lw = (width * 0.3).min(40.0);
                out.push(Paint::Fill {
                    rect: [
                        x + (width - lw) / 2.0,
                        y + h - 2.0 * one - 1.0,
                        lw,
                        2.0 * one,
                    ],
                    argb: palette::ACCENT,
                });
            }
        }
        Widget::SliderTrack { lit } => {
            let border = if lit {
                palette::ACCENT
            } else {
                palette::BORDER
            };
            out.push(round(rect, radius_for(h), palette::FIELD, border));
        }
        Widget::SliderHandle { lit } => {
            // A thin rail along the bottom (so the label above stays clear), filled in the
            // accent up to the handle, and a round knob on it.
            let rail_y = y + h - 4.5;
            let middle = x + width / 2.0;
            if let Some([tx, ty, tw, _]) = track
                && x >= tx - 1.0
                && x + width <= tx + tw + 1.0
                && (y - ty).abs() < 1.0
            {
                let inset = 4.0;
                out.push(round(
                    [tx + inset, rail_y - 1.0, tw - 2.0 * inset, 2.0],
                    1.0,
                    0x33FF_FFFF,
                    0,
                ));
                if middle > tx + inset {
                    out.push(round(
                        [tx + inset, rail_y - 1.0, middle - tx - inset, 2.0],
                        1.0,
                        if lit { palette::ACCENT } else { 0xE6FF_6A00 },
                        0,
                    ));
                }
            }
            let d = 6.0f32.min(width);
            out.push(round(
                [middle - d / 2.0, rail_y - d / 2.0, d, d],
                d / 2.0,
                if lit { 0xFFFF_FFFF } else { 0xFFE6_E3DF },
                if lit { palette::ACCENT } else { 0 },
            ));
        }
        Widget::TextField { lit } => {
            let border = if lit { palette::ACCENT } else { 0x2EFF_FFFF };
            out.push(round(rect, radius_for(h).min(3.0), palette::FIELD, border));
        }
        Widget::Checkbox { checked, lit } => {
            let side = width.min(h);
            let b = [x + (width - side) / 2.0, y + (h - side) / 2.0, side, side];
            let (fill, border) = match (checked, lit) {
                (true, _) => (
                    palette::ACCENT,
                    if lit { palette::INK } else { palette::ACCENT },
                ),
                (false, true) => (palette::FACE_HOVER, palette::ACCENT),
                (false, false) => (palette::FIELD, 0x33FF_FFFF),
            };
            out.push(round(b, radius_for(side).min(4.0), fill, border));
            if checked {
                // A check mark in two strokes.
                let s = side;
                let t = (s * 0.11).max(1.5);
                let p = |fx: f32, fy: f32| [b[0] + fx * s, b[1] + fy * s];
                let stroke = |a: [f32; 2], c: [f32; 2]| {
                    let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
                    let len = (dx * dx + dy * dy).sqrt().max(0.001);
                    let (nx, ny) = (-dy / len * t / 2.0, dx / len * t / 2.0);
                    Paint::Quad {
                        points: [
                            [a[0] + nx, a[1] + ny],
                            [c[0] + nx, c[1] + ny],
                            [c[0] - nx, c[1] - ny],
                            [a[0] - nx, a[1] - ny],
                        ],
                        argb: 0xFFFF_FFFF,
                    }
                };
                out.push(stroke(p(0.24, 0.52), p(0.43, 0.70)));
                out.push(stroke(p(0.41, 0.71), p(0.77, 0.32)));
            }
        }
        Widget::Tab { selected, lit } => {
            let fill = match (selected, lit) {
                (true, _) => palette::FACE_HOVER,
                (false, true) => palette::FACE,
                (false, false) => 0x9918_191D,
            };
            let border = if lit && !selected {
                palette::ACCENT
            } else {
                palette::BORDER
            };
            out.push(round(
                [x + 1.0, y + 2.0, width - 2.0, h - 2.0],
                radius_for(h).min(4.0),
                fill,
                border,
            ));
            if selected {
                let lw = width - 12.0;
                out.push(Paint::Fill {
                    rect: [x + 6.0, y + h - 2.0, lw.max(2.0), 2.0],
                    argb: palette::ACCENT,
                });
            }
        }
        Widget::ScrollerTrack => {
            out.push(round(
                [x + 1.5, y, (width - 3.0).max(1.0), h],
                1.5,
                0x1AFF_FFFF,
                0,
            ));
        }
        Widget::Scroller => {
            out.push(round(
                [x + 1.0, y, (width - 2.0).max(1.0), h],
                2.0,
                0x8CFF_FFFF,
                0,
            ));
        }
        Widget::TooltipBackground => {
            out.push(round(
                [x + 9.0, y + 9.0, width - 18.0, h - 18.0],
                3.0,
                0xF20F_1013,
                0x40FF_6A00,
            ));
        }
        Widget::TooltipFrame => {}
        Widget::MenuBackground { world } => out.push(Paint::Fill {
            rect,
            argb: if world {
                palette::VEIL_WORLD
            } else {
                palette::VEIL
            },
        }),
        Widget::ListBackground => out.push(Paint::Fill {
            rect,
            argb: palette::WELL,
        }),
        Widget::Separator => out.push(Paint::Fill {
            rect: [x, y + (h - one) / 2.0, width, one],
            argb: palette::RULE,
        }),
        Widget::TabHeader => out.push(Paint::Fill {
            rect,
            argb: 0x8C00_0000,
        }),
    }
    out
}

/// The colour (ARGB) a widget label is set in, from the game's (the HUD's ABGR): white stays the
/// theme's ink, vanilla's grey for what is off the theme's dimmed ink, any other colour its own.
pub fn label_color(abgr: u32) -> u32 {
    let [a, b, g, r] = abgr.to_be_bytes();
    let argb = u32::from_be_bytes([a, r, g, b]);
    match argb & 0x00FF_FFFF {
        0xFF_FFFF => (argb & 0xFF00_0000) | (palette::INK & 0x00FF_FFFF),
        0xA0_A0A0 => (argb & 0xFF00_0000) | (palette::INK_OFF & 0x00FF_FFFF),
        _ => argb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_sprites_are_named_and_the_rest_left_alone() {
        assert_eq!(
            widget("gui/sprites/widget/button_highlighted"),
            Some(Widget::Button(State::Lit))
        );
        assert_eq!(
            widget("gui/sprites/widget/checkbox_selected_highlighted"),
            Some(Widget::Checkbox {
                checked: true,
                lit: true
            })
        );
        assert_eq!(
            widget("gui/inworld_menu_background"),
            Some(Widget::MenuBackground { world: true })
        );
        assert_eq!(widget("gui/sprites/hud/hotbar"), None);
        assert_eq!(widget("gui/sprites/world_list/join"), None);
        assert_eq!(widget("gui/title/minecraft"), None);
    }

    #[test]
    fn buttons_are_rounded_plates_lit_in_the_accent() {
        let r = [10.0, 10.0, 200.0, 20.0];
        let normal = paint(Widget::Button(State::Normal), r, 2.0, None);
        assert!(matches!(
            normal[0],
            Paint::Round {
                rect,
                fill: palette::FACE,
                border: palette::BORDER,
                ..
            } if rect == r
        ));
        let lit = paint(Widget::Button(State::Lit), r, 2.0, None);
        assert!(matches!(
            lit[0],
            Paint::Round {
                border: palette::ACCENT,
                ..
            }
        ));
        assert_eq!(lit.len(), 2);
        // Every paint stays inside the widget.
        for p in lit {
            let rect = match p {
                Paint::Round { rect, .. } | Paint::Fill { rect, .. } => rect,
                _ => continue,
            };
            assert!(rect[0] >= r[0] && rect[0] + rect[2] <= r[0] + r[2]);
            assert!(rect[1] >= r[1] && rect[1] + rect[3] <= r[1] + r[3]);
        }
    }

    #[test]
    fn a_slider_fills_its_track_up_to_the_handle() {
        let track = [0.0, 0.0, 150.0, 20.0];
        let handle = [71.0, 0.0, 8.0, 20.0];
        let p = paint(
            Widget::SliderHandle { lit: false },
            handle,
            2.0,
            Some(track),
        );
        // The rail, its filled part up to the handle's middle, the knob.
        assert_eq!(p.len(), 3);
        let Paint::Round { rect, .. } = p[1] else {
            panic!("the fill second");
        };
        assert_eq!(rect[0], 4.0);
        assert!((rect[0] + rect[2] - 75.0).abs() < 0.01);
        // Below the label's line.
        assert!(rect[1] > 14.0);
        // No rail without the track it belongs to.
        let alone = paint(Widget::SliderHandle { lit: false }, handle, 2.0, None);
        assert_eq!(alone.len(), 1);
    }

    #[test]
    fn a_checked_box_has_its_mark_and_labels_keep_their_colours() {
        let p = paint(
            Widget::Checkbox {
                checked: true,
                lit: false,
            },
            [0.0, 0.0, 20.0, 20.0],
            3.0,
            None,
        );
        assert_eq!(
            p.iter().filter(|p| matches!(p, Paint::Quad { .. })).count(),
            2
        );
        assert_eq!(label_color(u32::MAX), palette::INK);
        // Vanilla's grey (ABGR 0xFFA0A0A0) is the dimmed ink.
        assert_eq!(label_color(0xFFA0_A0A0), palette::INK_OFF);
        // A red label (ABGR) stays red (ARGB).
        assert_eq!(label_color(0xFF55_55FF), 0xFFFF_5555);
        assert!(paint(Widget::TooltipFrame, [0.0; 4], 2.0, None).is_empty());
    }
}
