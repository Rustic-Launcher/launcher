//! The social overlay's layout in GUI pixels: a panel in the middle of the screen with three
//! tabs (friends, invites, messages), a row per entry and its action. Pure geometry: the game
//! crate draws [`Draw`]s with Horizon's HUD helpers (`fill`, `text`) on its own mod screen and
//! hands clicks to [`Layout::hit`].
use crate::protocol::SocialSnapshot;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Friends,
    Invites,
    Messages,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Friends, Tab::Invites, Tab::Messages];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Friends => "Friends",
            Tab::Invites => "Invites",
            Tab::Messages => "Messages",
        }
    }
}

/// What a click does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Tab(Tab),
    /// Invite this friend (UUID text) to the server the player is on.
    InviteToMyServer(String),
    /// Accept this invite.
    Accept(String),
    Close,
}

/// One thing to draw: a filled box, or a line of text.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Fill {
        rect: [f32; 4],
        argb: u32,
    },
    Text {
        at: [f32; 2],
        text: String,
        rgb: u32,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub draws: Vec<Draw>,
    pub hits: Vec<([f32; 4], Action)>,
}

impl Layout {
    /// The action under `at`, if any.
    pub fn hit(&self, at: [f32; 2]) -> Option<&Action> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| {
                at[0] >= r[0] && at[0] < r[0] + r[2] && at[1] >= r[1] && at[1] < r[1] + r[3]
            })
            .map(|(_, a)| a)
    }
}

const PANEL: u32 = 0xE012_1018;
const ROW: u32 = 0x3326_2A36;
const ACCENT: u32 = 0xFFE0_7A3C;
const BUTTON: u32 = 0xFF3A_2F2A;
const WHITE: u32 = 0xFF_FFFF;
const ROW_H: f32 = 14.0;

/// What the overlay knows to draw.
pub struct View<'a> {
    pub tab: Tab,
    pub social: &'a SocialSnapshot,
    pub signed_in: bool,
    /// The server address the player is on (None in single player, where "invite" is off).
    pub server: Option<&'a str>,
    /// GUI size.
    pub screen: [f32; 2],
}

/// Lays the overlay out.
pub fn layout(v: &View<'_>) -> Layout {
    let mut l = Layout::default();
    let w = (v.screen[0] - 40.0).clamp(160.0, 300.0);
    let h = (v.screen[1] - 40.0).clamp(120.0, 220.0);
    let x = ((v.screen[0] - w) / 2.0).floor();
    let y = ((v.screen[1] - h) / 2.0).floor();
    l.draws.push(Draw::Fill {
        rect: [x, y, w, h],
        argb: PANEL,
    });
    l.draws.push(Draw::Text {
        at: [x + 6.0, y + 5.0],
        text: "Rustic".into(),
        rgb: ACCENT & 0xFF_FFFF,
    });
    let close = [x + w - 14.0, y + 3.0, 11.0, 11.0];
    l.draws.push(Draw::Fill {
        rect: close,
        argb: BUTTON,
    });
    l.draws.push(Draw::Text {
        at: [close[0] + 3.0, close[1] + 2.0],
        text: "x".into(),
        rgb: WHITE,
    });
    l.hits.push((close, Action::Close));
    // Tabs.
    let tab_w = ((w - 12.0) / 3.0).floor();
    for (i, t) in Tab::ALL.into_iter().enumerate() {
        let r = [x + 6.0 + i as f32 * tab_w, y + 18.0, tab_w - 2.0, 12.0];
        let count = match t {
            Tab::Friends => v.social.friends.iter().filter(|f| f.online).count(),
            Tab::Invites => v.social.invites.len(),
            Tab::Messages => v.social.messages.len(),
        };
        l.draws.push(Draw::Fill {
            rect: r,
            argb: if t == v.tab { ACCENT } else { BUTTON },
        });
        let label = if count > 0 {
            format!("{} ({count})", t.label())
        } else {
            t.label().to_owned()
        };
        l.draws.push(Draw::Text {
            at: [r[0] + 4.0, r[1] + 2.0],
            text: label,
            rgb: WHITE,
        });
        l.hits.push((r, Action::Tab(t)));
    }
    let top = y + 36.0;
    let rows = ((h - 42.0) / ROW_H).floor().max(0.0) as usize;
    let row = |i: usize, text: String, button: Option<(&str, Action)>, l: &mut Layout| {
        let r = [x + 6.0, top + i as f32 * ROW_H, w - 12.0, ROW_H - 2.0];
        l.draws.push(Draw::Fill { rect: r, argb: ROW });
        l.draws.push(Draw::Text {
            at: [r[0] + 3.0, r[1] + 2.0],
            text,
            rgb: WHITE,
        });
        if let Some((label, action)) = button {
            let b = [r[0] + r[2] - 52.0, r[1] + 1.0, 50.0, ROW_H - 4.0];
            l.draws.push(Draw::Fill {
                rect: b,
                argb: BUTTON,
            });
            l.draws.push(Draw::Text {
                at: [b[0] + 3.0, b[1] + 1.0],
                text: label.into(),
                rgb: WHITE,
            });
            l.hits.push((b, action));
        }
    };
    if !v.signed_in {
        row(
            0,
            "Sign in from Rustic Launcher to see your friends.".into(),
            None,
            &mut l,
        );
        return l;
    }
    match v.tab {
        Tab::Friends => {
            let mut friends: Vec<_> = v.social.friends.iter().collect();
            friends.sort_by_key(|f| (!f.online, f.name.to_lowercase()));
            if friends.is_empty() {
                row(
                    0,
                    "No friends yet: add some in Rustic Launcher.".into(),
                    None,
                    &mut l,
                );
            }
            for (i, f) in friends.into_iter().take(rows).enumerate() {
                let status = match (&f.activity, f.online) {
                    (Some(a), true) => format!("{} - {a}", f.name),
                    (None, true) => format!("{} - online", f.name),
                    (_, false) => format!("{} - offline", f.name),
                };
                let invite = (f.online && v.server.is_some())
                    .then(|| ("Invite", Action::InviteToMyServer(f.uuid.clone())));
                row(i, status, invite, &mut l);
            }
        }
        Tab::Invites => {
            if v.social.invites.is_empty() {
                row(0, "No invites.".into(), None, &mut l);
            }
            for (i, inv) in v.social.invites.iter().take(rows).enumerate() {
                row(
                    i,
                    format!("{} invites you to {}", inv.from, inv.server),
                    Some(("Join", Action::Accept(inv.id.clone()))),
                    &mut l,
                );
            }
        }
        Tab::Messages => {
            if v.social.messages.is_empty() {
                row(0, "No messages.".into(), None, &mut l);
            }
            for (i, m) in v.social.messages.iter().rev().take(rows).enumerate() {
                row(i, format!("{}: {}", m.from, m.text), None, &mut l);
            }
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Friend, Invite};

    fn social() -> SocialSnapshot {
        SocialSnapshot {
            friends: vec![
                Friend {
                    uuid: "b".into(),
                    name: "Bob".into(),
                    online: false,
                    activity: None,
                },
                Friend {
                    uuid: "a".into(),
                    name: "Ana".into(),
                    online: true,
                    activity: Some("Rustic SMP".into()),
                },
            ],
            invites: vec![Invite {
                id: "i1".into(),
                from: "Ana".into(),
                server: "Rustic SMP".into(),
            }],
            messages: vec![],
            emotes: vec![],
        }
    }

    #[test]
    fn friends_online_first_with_invite_on_a_server() {
        let s = social();
        let l = layout(&View {
            tab: Tab::Friends,
            social: &s,
            signed_in: true,
            server: Some("play.example.org"),
            screen: [427.0, 240.0],
        });
        let texts: Vec<&str> = l
            .draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        let ana = texts.iter().position(|t| t.starts_with("Ana")).unwrap();
        let bob = texts.iter().position(|t| t.starts_with("Bob")).unwrap();
        assert!(ana < bob);
        let invite = l
            .hits
            .iter()
            .find(|(_, a)| *a == Action::InviteToMyServer("a".into()))
            .unwrap()
            .0;
        assert_eq!(
            l.hit([invite[0] + 1.0, invite[1] + 1.0]),
            Some(&Action::InviteToMyServer("a".into()))
        );
        // Offline friends and single player get no invite button.
        assert_eq!(
            l.hits
                .iter()
                .filter(|(_, a)| matches!(a, Action::InviteToMyServer(_)))
                .count(),
            1
        );
        let solo = layout(&View {
            server: None,
            tab: Tab::Friends,
            social: &s,
            signed_in: true,
            screen: [427.0, 240.0],
        });
        assert!(
            !solo
                .hits
                .iter()
                .any(|(_, a)| matches!(a, Action::InviteToMyServer(_)))
        );
    }

    #[test]
    fn invites_tab_and_signed_out() {
        let s = social();
        let l = layout(&View {
            tab: Tab::Invites,
            social: &s,
            signed_in: true,
            server: None,
            screen: [427.0, 240.0],
        });
        assert!(
            l.hits
                .iter()
                .any(|(_, a)| *a == Action::Accept("i1".into()))
        );
        assert!(l.hits.iter().any(|(_, a)| *a == Action::Close));
        let out = layout(&View {
            tab: Tab::Invites,
            social: &s,
            signed_in: false,
            server: None,
            screen: [427.0, 240.0],
        });
        assert!(!out.hits.iter().any(|(_, a)| matches!(a, Action::Accept(_))));
        assert_eq!(out.hit([-5.0, -5.0]), None);
    }
}
