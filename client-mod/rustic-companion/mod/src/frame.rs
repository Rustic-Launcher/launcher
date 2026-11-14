//! What the game's screen drew this frame, as the widget skin saw it (Rustic's Horizon patches
//! U15 and U17): its buttons (box, label, on or off, the icon drawn on them), the box its content
//! takes, and whether it has lists or tabs. Rustic's own pages lay the screen's buttons out their
//! way and press them (`Host::press_at`); the skin draws a dialog's card behind the content of a
//! screen that is one (from the last frame's box) and gives the screen's main and destructive
//! buttons their look (from the last frame's labels). ADR 0164.
use rusticmc_companion_core::skin::Role;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

/// One of the screen's buttons.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub rect: [f32; 4],
    pub enabled: bool,
    /// Its label (the skin set it), None for an icon button.
    pub label: Option<String>,
    /// The sprite drawn on it (an icon button's icon).
    pub icon: Option<String>,
}

/// A frame of the screen.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub buttons: Vec<Button>,
    /// The box its content takes after its background (GUI pixels).
    pub bounds: Option<[f32; 4]>,
    /// A menu background was drawn (the screen is a menu).
    pub menu: bool,
    /// Lists, tabs, separators or a scroll bar: a page, not a dialog.
    pub paged: bool,
    /// A tooltip is being drawn: what follows is not the screen's content.
    pub tooltip: bool,
    /// Rustic drew a page of its own over the whole screen.
    pub covered: bool,
    /// The boxes of the lines of text the screen drew (after its background).
    pub texts: Vec<[f32; 4]>,
    /// The screen's id, as its render hook named it (None: no screen hook ran).
    pub screen: Option<&'static str>,
    /// The GUI's size.
    pub gui: [f32; 2],
}

impl Frame {
    /// Whether the frame looked like a dialog (a menu with no list nor tab, a few things in the
    /// middle), and its content's box.
    pub fn dialog(&self) -> Option<[f32; 4]> {
        // Settings pages (Horizon's shell draws its own panels) and the pause menu (Rustic's
        // panel) are no dialogs.
        if !self.menu
            || self.paged
            || self.covered
            || matches!(self.screen, Some("options" | "paused" | "mod"))
        {
            return None;
        }
        self.bounds
    }
}

/// The card behind a dialog, as it moves to its content and fades in.
#[derive(Clone, Copy, Debug, Default)]
pub struct Card {
    pub rect: Option<[f32; 4]>,
    pub alpha: f32,
}

pub struct Frames {
    pub current: Frame,
    pub last: Frame,
    pub card: Card,
    /// What the game's screen draws that Rustic lays out its own way (the pause menu's buttons
    /// and title): not drawn at the next frame. Set by a Rustic page; one frame long.
    pub hide_next: Vec<[f32; 4]>,
    hiding: Vec<[f32; 4]>,
    /// The labels of the main and destructive buttons in the game's language, and the language.
    roles: Option<(String, HashMap<String, Role>)>,
}

static FRAMES: LazyLock<Mutex<Frames>> = LazyLock::new(|| {
    Mutex::new(Frames {
        current: Frame::default(),
        last: Frame::default(),
        card: Card::default(),
        hide_next: Vec::new(),
        hiding: Vec::new(),
        roles: None,
    })
});

pub fn frames() -> MutexGuard<'static, Frames> {
    FRAMES.lock().unwrap_or_else(|e| e.into_inner())
}

/// The game's translation keys of the buttons Rustic draws in the accent (the screen's main
/// action) and in red (destructive).
const PRIMARY: &[&str] = &[
    "gui.done",
    "gui.proceed",
    "gui.yes",
    "gui.continue",
    "selectServer.select",
    "selectWorld.select",
    "selectWorld.create",
    "menu.returnToGame",
    "gui.toMenu",
    "gui.toTitle",
    "connect.reconnect",
    "multiplayer.disconnect.back",
    "selectWorld.backupJoinConfirmButton",
    "selectWorld.backupJoinSkipButton",
];
const DANGER: &[&str] = &[
    "selectServer.delete",
    "selectServer.deleteButton",
    "selectWorld.delete",
    "selectWorld.deleteButton",
    "menu.disconnect",
    "menu.returnToMenu",
    "menu.quit",
    "horizon.split.leave",
];

impl Frames {
    /// Once a frame (the skin's `begin`): this frame becomes the last; the card moves.
    pub fn begin(&mut self) {
        self.last = std::mem::take(&mut self.current);
        self.hiding = std::mem::take(&mut self.hide_next);
        let target = self
            .last
            .dialog()
            .and_then(|b| rusticmc_companion_core::skin::card(b, self.last.gui, 1.0).map(|_| b));
        self.card = match (target, self.card.rect) {
            (None, _) => Card::default(),
            (Some(t), None) => Card {
                rect: Some(t),
                alpha: 0.35,
            },
            (Some(t), Some(r)) => {
                // Eased towards the new box (a screen that grows or a new line).
                let k = 0.35;
                let mix = |a: f32, b: f32| a + (b - a) * k;
                let near = (0..4).all(|i| (r[i] - t[i]).abs() < 0.5);
                Card {
                    rect: Some(if near {
                        t
                    } else {
                        [
                            mix(r[0], t[0]),
                            mix(r[1], t[1]),
                            mix(r[2], t[2]),
                            mix(r[3], t[3]),
                        ]
                    }),
                    alpha: (self.card.alpha + 0.3).min(1.0),
                }
            }
        };
    }

    /// Something of the screen's content was drawn over `rect`.
    pub fn content(&mut self, rect: [f32; 4]) {
        let f = &mut self.current;
        if !f.menu || f.tooltip || rect[2] <= 0.0 || rect[3] <= 0.0 {
            return;
        }
        f.bounds = Some(match f.bounds {
            None => rect,
            Some(b) => {
                let x0 = b[0].min(rect[0]);
                let y0 = b[1].min(rect[1]);
                let x1 = (b[0] + b[2]).max(rect[0] + rect[2]);
                let y1 = (b[1] + b[3]).max(rect[1] + rect[3]);
                [x0, y0, x1 - x0, y1 - y0]
            }
        });
    }

    /// Whether what the screen draws over `rect` is hidden (Rustic lays it out its own way): a
    /// box it hid, or something inside one, on a menu.
    pub fn hidden(&self, rect: [f32; 4]) -> bool {
        if !self.current.menu || self.hiding.is_empty() {
            return false;
        }
        let c = centre(rect);
        self.hiding.iter().any(|h| {
            (0..4).all(|i| (h[i] - rect[i]).abs() < 0.5)
                || (rect[2] <= h[2] + 0.5
                    && rect[3] <= h[3] + 0.5
                    && rusticmc_companion_core::paint::inside(*h, c))
        })
    }

    /// A line of text was drawn over `rect`.
    pub fn text_box(&mut self, rect: [f32; 4]) {
        if self.current.menu && !self.current.tooltip && self.current.texts.len() < 256 {
            self.current.texts.push(rect);
        }
        self.content(rect);
    }

    /// The role of a button with label `label`, in language `lang`.
    pub fn role_of(&mut self, label: &str, lang: &str) -> Role {
        if self.roles.as_ref().is_none_or(|(l, _)| l != lang) {
            let mut map = HashMap::new();
            for (keys, role) in [(DANGER, Role::Danger), (PRIMARY, Role::Primary)] {
                for key in keys {
                    let t = horizon_client::lang::get_or(key, "");
                    if !t.trim().is_empty() {
                        map.insert(t.trim().to_owned(), role);
                    }
                }
            }
            self.roles = Some((lang.to_owned(), map));
        }
        self.roles
            .as_ref()
            .and_then(|(_, m)| m.get(label.trim()).copied())
            .unwrap_or_default()
    }

    /// The role of the button the skin is drawing at `rect`: its last frame's label's, or the
    /// main one when it was a dialog's only button.
    pub fn role_at(&mut self, rect: [f32; 4], lang: &str) -> Role {
        let same = |b: &&Button| (0..4).all(|i| (b.rect[i] - rect[i]).abs() < 0.5);
        let label = self
            .last
            .buttons
            .iter()
            .find(same)
            .and_then(|b| b.label.clone());
        let Some(label) = label else {
            return Role::Normal;
        };
        match self.role_of(&label, lang) {
            Role::Normal
                if self.last.dialog().is_some()
                    && self.last.buttons.iter().filter(|b| b.enabled).count() == 1 =>
            {
                Role::Primary
            }
            r => r,
        }
    }
}

/// The game's button whose label is the translation of one of `keys` among `buttons`.
pub fn find<'a>(buttons: &'a [Button], keys: &[&str]) -> Option<&'a Button> {
    let labels: Vec<String> = keys
        .iter()
        .map(|k| horizon_client::lang::get_or(k, ""))
        .filter(|t| !t.trim().is_empty())
        .collect();
    buttons.iter().find(|b| {
        b.label
            .as_deref()
            .is_some_and(|l| labels.iter().any(|t| t.trim() == l.trim()))
    })
}

/// A rectangle's centre.
pub fn centre(r: [f32; 4]) -> [f32; 2] {
    [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dialog_is_a_menu_without_lists_and_its_card_eases_in() {
        let mut f = Frames {
            current: Frame::default(),
            last: Frame::default(),
            card: Card::default(),
            hide_next: Vec::new(),
            hiding: Vec::new(),
            roles: None,
        };
        f.current.menu = true;
        f.current.gui = [427.0, 240.0];
        f.content([100.0, 60.0, 200.0, 20.0]);
        f.content([150.0, 120.0, 100.0, 20.0]);
        assert_eq!(f.current.bounds, Some([100.0, 60.0, 200.0, 80.0]));
        f.current.tooltip = true;
        f.content([0.0, 0.0, 400.0, 230.0]);
        assert_eq!(f.current.bounds, Some([100.0, 60.0, 200.0, 80.0]));
        f.current.tooltip = false;
        f.begin();
        assert_eq!(f.card.rect, Some([100.0, 60.0, 200.0, 80.0]));
        assert!(f.card.alpha < 1.0);
        // A page with a list has no card.
        f.current.menu = true;
        f.current.paged = true;
        f.content([100.0, 60.0, 200.0, 20.0]);
        f.begin();
        assert!(f.card.rect.is_none());
        // Nothing counts before the background.
        f.content([1.0, 1.0, 5.0, 5.0]);
        assert!(f.current.bounds.is_none());
        // What a Rustic page hid is hidden the next frame only, on a menu, with what is on it.
        f.hide_next = vec![[10.0, 10.0, 100.0, 20.0]];
        f.begin();
        assert!(!f.hidden([10.0, 10.0, 100.0, 20.0]));
        f.current.menu = true;
        assert!(f.hidden([10.0, 10.0, 100.0, 20.0]));
        assert!(f.hidden([40.0, 15.0, 30.0, 9.0]));
        assert!(!f.hidden([200.0, 10.0, 100.0, 20.0]));
        f.begin();
        f.current.menu = true;
        assert!(!f.hidden([10.0, 10.0, 100.0, 20.0]));
    }
}
