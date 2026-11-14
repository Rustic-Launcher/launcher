//! Rustic's pause menu (ADR 0164): the game's pause screen (`paused`) still runs underneath;
//! its buttons, as the skin saw them this frame ([`crate::frame`]), are laid out as the
//! launcher's panel ([`rusticmc_companion_core::panel`]) and a press presses the game's own
//! (`Host::press_at`, Rustic's Horizon patch U17), so every one does what it does in the game
//! (and a mod's own additions come along). Escape resumes as in the game.
use super::{gui_measure, register};
use crate::canvas::canvas;
use crate::frame::{self, frames};
use crate::state::{guard, guard_or, lock};
use horizon_client::mod_api::{
    Flow, Host, ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent,
};
use rusticmc_companion_core::paint::{self, rescaled, scale_rect};
use rusticmc_companion_core::panel::{self, Entry, Layout};
use rusticmc_companion_core::skin::Role;

/// The pause screen's id.
pub const PAUSED: &str = "paused";

#[derive(Default)]
pub struct PauseUi {
    pub entries: Vec<Entry>,
    /// The game's button behind each entry.
    pub targets: Vec<[f32; 4]>,
    pub layout: Option<Layout>,
    pub focus: Option<usize>,
    pub unit: f32,
}

/// The label of an icon button by its icon (the game draws them unlabelled).
fn icon_label(icon: &str) -> Option<String> {
    let key = match icon {
        "gui/sprites/pause_menu/bug" => "menu.reportBugs",
        "gui/sprites/pause_menu/social_interactions" => "menu.sendFeedback",
        "gui/sprites/friends/friends" => "gui.friends.open",
        "gui/sprites/pause_menu/player_reporting" => "menu.playerReporting",
        _ => return None,
    };
    let t = horizon_client::lang::get_or(key, "");
    (!t.trim().is_empty()).then_some(t)
}

/// Whether Rustic draws the pause menu now: the theme is on and the game drew its buttons.
pub fn shown(screen: &str) -> bool {
    if screen != PAUSED {
        return false;
    }
    let st = lock();
    st.cfg.rustic_theme && !st.pause.entries.is_empty()
}

pub fn render(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    guard("pause.render", || {
        let theme = lock().cfg.rustic_theme;
        if event.screen.id != PAUSED || !theme {
            lock().pause.entries.clear();
            return;
        }
        let lang = horizon_client::lang::code();
        let (entries, targets): (Vec<Entry>, Vec<[f32; 4]>) = {
            let mut f = frames();
            let buttons = f.current.buttons.clone();
            buttons
                .iter()
                .filter_map(|b| {
                    let label = b
                        .label
                        .clone()
                        .filter(|l| !l.trim().is_empty())
                        .or_else(|| b.icon.as_deref().and_then(icon_label))?;
                    let role = f.role_of(&label, &lang);
                    Some((
                        Entry {
                            label,
                            role,
                            enabled: b.enabled,
                        },
                        b.rect,
                    ))
                })
                .unzip()
        };
        if entries.is_empty() {
            // The pause screen without its menu (F3 + Escape): the game's title only.
            lock().pause.entries.clear();
            return;
        }
        // The first button resumes, whatever its words.
        let mut entries = entries;
        if !entries.iter().any(|e| e.role == Role::Primary) {
            entries[0].role = Role::Primary;
        }
        let scale = event.batch.scale.max(1.0);
        let unit = paint::screen_unit(scale, event.batch.height * scale);
        let measure = gui_measure(scale * unit);
        let gui = [event.batch.width / unit, event.batch.height / unit];
        let mouse = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
        let where_ = place(host);
        let title = horizon_client::lang::get_or("menu.game", "Game Menu");
        let focus = lock().pause.focus;
        let l = panel::layout(&panel::View {
            gui,
            mouse,
            measure: &measure,
            title: &title,
            subtitle: &where_,
            entries: &entries,
            focus,
        });
        {
            let mut c = canvas();
            if c.ready() {
                c.paint_all(event.batch, &rescaled(&l.paints, unit));
            }
        }
        {
            // The game's own pause menu is not drawn under the panel from the next frame on.
            let mut f = frames();
            f.current.covered = true;
            // Every button the screen drew (named or not), and its lines of text.
            let mut hide: Vec<[f32; 4]> = f.current.buttons.iter().map(|b| b.rect).collect();
            hide.extend(f.current.texts.iter().copied());
            f.hide_next = hide;
        }
        horizon_client::widgets::cover_frame();
        for (r, i) in &l.hits {
            if entries[*i].enabled {
                register(&entries[*i].label, scale_rect(*r, unit));
            }
        }
        let mut st = lock();
        let p = &mut st.pause;
        p.entries = entries;
        p.targets = targets;
        p.layout = Some(l);
        p.unit = unit;
    });
}

/// Where the player is, under the panel's title: the world's name in single player, the saved
/// server's name and its address on a server.
fn place(host: &dyn Host) -> String {
    use horizon_client::mod_api::NativeWorldId;
    let single = || horizon_client::lang::get_or("menu.singleplayer", "Singleplayer");
    match host.client_state().and_then(|c| c.world) {
        Some(NativeWorldId::Singleplayer(folder)) if !folder.is_empty() => {
            format!("{} \u{b7} {folder}", single())
        }
        Some(NativeWorldId::Server(address)) => {
            let name = host
                .server_list()
                .into_iter()
                .find(|(_, a, hidden)| !hidden && a.eq_ignore_ascii_case(&address))
                .map(|(n, ..)| n);
            match name {
                Some(n) if !n.trim().is_empty() => format!("{n} \u{b7} {address}"),
                _ => address,
            }
        }
        _ => single(),
    }
}

/// Presses the game's button behind entry `i`.
fn press(host: &dyn Host, i: usize) {
    let at = {
        let st = lock();
        let p = &st.pause;
        p.entries
            .get(i)
            .filter(|e| e.enabled)
            .and(p.targets.get(i))
            .map(|r| frame::centre(*r))
    };
    if let Some([x, y]) = at {
        host.press_at(x, y);
    }
}

pub fn mouse(host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
    guard_or("pause.mouse", Flow::Continue, || {
        let hit = {
            let mut st = lock();
            let p = &mut st.pause;
            let unit = if p.unit > 0.0 { p.unit } else { 1.0 };
            let at = [event.screen.mouse[0] / unit, event.screen.mouse[1] / unit];
            p.focus = None;
            p.layout.as_ref().and_then(|l| {
                l.hits
                    .iter()
                    .find(|(r, _)| paint::inside(*r, at))
                    .map(|(_, i)| *i)
            })
        };
        if event.button == 0
            && let Some(i) = hit
        {
            press(host, i);
        }
        Flow::Cancel
    })
}

pub fn key(host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
    guard_or("pause.key", Flow::Continue, || {
        let back = event.modifiers & 1 != 0;
        match event.key {
            "key.keyboard.tab" | "key.keyboard.down" | "key.keyboard.up" | "key.keyboard.right"
            | "key.keyboard.left" => {
                let mut st = lock();
                let p = &mut st.pause;
                if let Some(l) = &p.layout {
                    let back = back || matches!(event.key, "key.keyboard.up" | "key.keyboard.left");
                    p.focus = panel::step(l, &p.entries, p.focus, back);
                }
                Flow::Cancel
            }
            "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.space" => {
                let focus = lock().pause.focus;
                if let Some(i) = focus {
                    press(host, i);
                    Flow::Cancel
                } else {
                    Flow::Continue
                }
            }
            // Escape resumes, F-keys and the rest do what they do in the game.
            _ => Flow::Continue,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_known_icons_are_named() {
        assert!(icon_label("gui/sprites/hud/heart").is_none());
    }
}
