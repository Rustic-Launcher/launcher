//! Rustic's world list and server list (ADR 0164): the title's Singleplayer and Multiplayer
//! screens redone as the launcher's pages — a header (the page's title, or the multiplayer
//! screen's "My servers | Discover" switch, then a search field), the worlds or servers as cards
//! (picture, name, details, badges, status), the selected one in the accent, and an action bar
//! at the bottom (Back on the left, the page's actions, the primary one on the right). Pure: the
//! game crate gives the rows and the labels, draws the paints and presses the game's own
//! buttons for the actions.
use crate::discover::Tab;
use crate::i18n::{self, T};
use crate::kit::{Kit, Look, fold, max_scroll};
use crate::paint::{Align, Measure, Paint, Picture, fit, inside};
use crate::skin::palette;
use crate::title::{Glyph, Image, Weight, hue_of};

/// What an action bar button does (the game's own button it presses).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    Back,
    /// Join Server, Play Selected World.
    Primary,
    /// Create New World, Add Server.
    Create,
    /// Direct Connection.
    Direct,
    Edit,
    Delete,
    /// Re-Create (worlds).
    Recreate,
    /// Refresh (servers).
    Refresh,
}

/// What a press lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// A card: selects its row (the game's row index).
    Row(usize),
    /// A card's own Join / Play button.
    Open(usize),
    Act(Act),
    Search,
    Tab(Tab),
    /// The empty state's "Discover" button.
    Discover,
    /// Anything else inside the screen (eaten).
    Nothing,
}

/// A button of the action bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub act: Act,
    pub label: String,
    pub enabled: bool,
}

/// How a server answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Reach {
    Pinging,
    Online {
        ping_ms: u32,
    },
    /// Another version; the server's.
    Incompatible {
        version: String,
    },
    Unreachable,
    /// A LAN world (no ping).
    Lan,
}

/// A server card.
#[derive(Clone, Debug, PartialEq)]
pub struct ServerRow {
    /// The game's row index (saved servers then LAN worlds).
    pub index: usize,
    pub name: String,
    pub address: String,
    pub lan: bool,
    /// The MOTD's lines (at most two), each a run of `(text, ARGB)`.
    pub motd: Vec<Vec<(String, u32)>>,
    pub players: Option<(i64, i64)>,
    pub reach: Reach,
    pub icon: Option<Picture>,
    pub selected: bool,
}

/// A badge's colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Accent,
    Good,
    Info,
    Warn,
    Danger,
}

impl Tone {
    /// (fill, ink) ARGB.
    pub fn colors(self) -> (u32, u32) {
        match self {
            Tone::Neutral => (0x24FF_FFFF, palette::MUTED),
            Tone::Accent => (palette::ACCENT_SOFT, palette::ACCENT_TEXT),
            Tone::Good => (0x2E4A_DE80, 0xFF6E_E7A0),
            Tone::Info => (0x2E60_A5FA, 0xFF93_C5FD),
            Tone::Warn => (0x2EF5_B83D, 0xFFF8_CC6B),
            Tone::Danger => (0x33F0_645A, 0xFFF5_8C84),
        }
    }
}

/// A world card.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldRow {
    pub index: usize,
    pub name: String,
    /// "Played 2 h ago · New World (folder)".
    pub detail: String,
    pub badges: Vec<(String, Tone)>,
    pub icon: Option<Picture>,
    pub selected: bool,
    /// Play is possible (not locked, not from another series).
    pub playable: bool,
}

/// The page's rows.
pub enum Rows<'a> {
    Servers {
        saved: &'a [ServerRow],
        lan: &'a [ServerRow],
        /// Seconds since the screen opened (the scanning dot pulses).
        time: f32,
    },
    Worlds(&'a [WorldRow]),
}

/// What the page is drawn from.
pub struct View<'a> {
    pub gui: [f32; 2],
    pub mouse: [f32; 2],
    pub lang: &'a str,
    pub measure: Measure<'a>,
    /// The page title (the world list); None shows the multiplayer switch on `tab`.
    pub title: Option<&'a str>,
    pub tab: Tab,
    pub query: &'a str,
    pub placeholder: &'a str,
    pub search_focused: bool,
    pub caret: bool,
    pub rows: Rows<'a>,
    /// The action bar, left to right; `Act::Back` goes left, `Act::Primary` right.
    pub buttons: &'a [Button],
    /// The card's own Join / Play label.
    pub open_label: &'a str,
    /// The empty state's Discover button (servers), when it has one.
    pub discover_label: Option<&'a str>,
    /// The game's "Scanning for games on your local network" (servers).
    pub scanning: &'a str,
    pub scroll: f32,
    /// The keyboard's focus among [`Layout::focusable`].
    pub focus: Option<usize>,
}

pub struct Layout {
    pub paints: Vec<Paint>,
    pub hits: Vec<([f32; 4], Hit)>,
    pub max_scroll: f32,
    /// The scrolled area.
    pub area: [f32; 4],
    /// Each card's box (unscrolled at the current scroll) by its row index, in order.
    pub cards: Vec<([f32; 4], usize)>,
}

impl Layout {
    /// What a press at `at` lands on: in the scrolled area, the cards; outside it, the header
    /// and the action bar.
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        let in_area = inside(self.area, at);
        self.hits
            .iter()
            .rev()
            .filter(|(_, h)| in_area || !matches!(h, Hit::Row(_) | Hit::Open(_) | Hit::Discover))
            .find(|(r, _)| inside(*r, at))
            .map(|(_, h)| h.clone())
    }
    /// The pieces Tab walks, in order: the switch's tabs, the search field, the action bar (the
    /// arrows walk the cards).
    pub fn focusable(&self) -> Vec<([f32; 4], Hit)> {
        self.hits
            .iter()
            .filter(|(r, h)| {
                matches!(h, Hit::Tab(_) | Hit::Search | Hit::Act(_)) && !inside(self.area, rc(*r))
            })
            .cloned()
            .collect()
    }
}

/// A rectangle's centre.
fn rc(r: [f32; 4]) -> [f32; 2] {
    [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0]
}

/// The column's width and left edge in a screen `gw` wide.
pub fn column(gw: f32) -> (f32, f32) {
    let w = (gw - 24.0).clamp(160.0, 620.0);
    (((gw - w) / 2.0).round(), w)
}

/// The header's height (the switch or the title, then the search row).
pub const HEADER: f32 = 58.0;
/// The action bar's height.
pub const FOOTER: f32 = 40.0;
const CARD: f32 = 46.0;
const GAP: f32 = 6.0;

/// The page title, centred where the multiplayer switch sits.
fn title(k: &mut Kit<'_, Hit>, gw: f32, text: &str) {
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

/// The action bar: Back on the left, the others right-aligned, the primary last.
fn action_bar(k: &mut Kit<'_, Hit>, v: &View<'_>, focus_at: &mut usize) {
    let [gw, gh] = v.gui;
    let top = gh - FOOTER;
    k.fill([0.0, top, gw, FOOTER], 0xFF0F_1013);
    k.fill([0.0, top, gw, 0.5], palette::RULE);
    let h = 22.0;
    let cy = top + FOOTER / 2.0;
    let gap = 6.0;
    let (cx, cw) = column(gw);
    let widths: Vec<f32> = v
        .buttons
        .iter()
        .map(|b| {
            let min = if b.act == Act::Primary { 92.0 } else { 54.0 };
            k.button_width(&b.label, h).max(min)
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len().max(1) - 1) as f32 + 12.0;
    // The column, or the whole width when the labels need it, and narrower buttons past that.
    let (x0, room) = if total <= cw {
        (cx, cw)
    } else {
        (12.0, gw - 24.0)
    };
    let squeeze = if total > room { room / total } else { 1.0 };
    let width = |i: usize| (widths[i] * squeeze).floor();
    let mut left = x0;
    let mut right = x0 + room;
    let order: Vec<usize> = (0..v.buttons.len()).collect();
    // Back first (left), then the rest from the right, primary outermost.
    let mut placed: Vec<(usize, [f32; 4])> = Vec::new();
    for &i in order.iter().filter(|&&i| v.buttons[i].act == Act::Back) {
        let r = [left, cy - h / 2.0, width(i), h];
        left += width(i) + gap;
        placed.push((i, r));
    }
    for &i in order
        .iter()
        .rev()
        .filter(|&&i| v.buttons[i].act != Act::Back)
    {
        let r = [right - width(i), cy - h / 2.0, width(i), h];
        right -= width(i) + gap;
        placed.push((i, r));
    }
    // Tab order: left to right.
    placed.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]));
    for (i, r) in placed {
        let b = &v.buttons[i];
        let look = match b.act {
            Act::Primary => Look::Primary,
            Act::Back => Look::Ghost,
            Act::Delete => Look::Danger,
            _ => Look::Secondary,
        };
        let focused = b.enabled && v.focus == Some(*focus_at);
        if b.enabled {
            *focus_at += 1;
        }
        k.button_on(r, &b.label, look, focused, b.enabled, Hit::Act(b.act));
    }
}

/// A section's heading at `y`, and an optional muted note after it; its height.
fn heading(
    k: &mut Kit<'_, Hit>,
    x: f32,
    y: f32,
    w: f32,
    text: &str,
    note: Option<(&str, f32)>,
) -> f32 {
    let tw = k.text(
        x + 2.0,
        y + 6.0,
        text,
        7.5,
        Weight::SemiBold,
        palette::MUTED,
        Align::Left,
    );
    if let Some((note, pulse)) = note {
        // A pulsing dot: the list keeps looking.
        let dx = x + 2.0 + tw + 8.0;
        let a = 0.45 + 0.55 * (pulse * std::f32::consts::TAU).sin().abs();
        k.round(
            [dx, y + 4.0, 4.0, 4.0],
            2.0,
            crate::paint::fade(palette::GREEN, a),
            0,
        );
        k.text_fit(
            dx + 8.0,
            y + 6.0,
            w - (dx + 8.0 - x),
            note,
            6.5,
            Weight::Medium,
            0xFF7A_7670,
        );
    }
    16.0
}

/// A card's plate: hovered, selected (the accent) or plain.
fn plate(k: &mut Kit<'_, Hit>, r: [f32; 4], selected: bool) {
    let lit = k.over(r);
    let (fill, border) = match (selected, lit) {
        (true, _) => (0x33FF_6A00, palette::ACCENT),
        (false, true) => (palette::FACE_HOVER, 0x33FF_FFFF),
        (false, false) => (palette::FACE, palette::BORDER),
    };
    k.round(r, 6.0, fill, border);
    if selected {
        // The accent's edge on the left, as the launcher marks the current item.
        k.round(
            [r[0] + 1.0, r[1] + 8.0, 2.0, r[3] - 16.0],
            1.0,
            palette::ACCENT,
            0,
        );
    }
}

/// A square picture with a generated tile under it (shown while it loads, or for none).
fn picture(k: &mut Kit<'_, Hit>, r: [f32; 4], seed: &str, glyph: Glyph, pic: Option<&Picture>) {
    k.out.push(Paint::Picture {
        picture: Picture::Title(Image::Tile {
            hue: hue_of(seed),
            glyph,
        }),
        rect: r,
        tint: 0xFFFF_FFFF,
    });
    if let Some(p) = pic {
        k.out.push(Paint::Picture {
            picture: p.clone(),
            rect: r,
            tint: 0xFFFF_FFFF,
        });
    }
    k.round(r, 4.0, 0, 0x26FF_FFFF);
}

/// Five signal bars for a ping, coloured by its quality; `None` greyed and stepping (pinging),
/// `Some(None)` crossed out (unreachable). Their width.
fn signal(k: &mut Kit<'_, Hit>, right: f32, cy: f32, ping: Option<Option<u32>>, time: f32) -> f32 {
    let bw = 2.0;
    let gap = 1.0;
    let w = 5.0 * bw + 4.0 * gap;
    let x = right - w;
    let (lit, color) = match ping {
        Some(Some(ms)) => {
            let bars = match ms {
                0..=150 => 5,
                151..=300 => 4,
                301..=600 => 3,
                601..=1000 => 2,
                _ => 1,
            };
            let color = match bars {
                5 | 4 => palette::GREEN,
                3 => 0xFFF5_B83D,
                _ => palette::DANGER,
            };
            (bars, color)
        }
        Some(None) => (0, palette::DANGER),
        None => ((time * 4.0) as usize % 6, 0xFF8A_867F),
    };
    for i in 0..5 {
        let h = 3.0 + i as f32 * 1.5;
        let on = i < lit;
        k.round(
            [x + i as f32 * (bw + gap), cy + 4.5 - h, bw, h],
            0.5,
            if on { color } else { 0x33FF_FFFF },
            0,
        );
    }
    if ping == Some(None) {
        // A small cross over the bars.
        let c = [x + w / 2.0, cy];
        for (a, b) in [([-3.0, -3.0], [3.0, 3.0]), ([-3.0, 3.0], [3.0, -3.0])] {
            let (p, q) = ([c[0] + a[0], c[1] + a[1]], [c[0] + b[0], c[1] + b[1]]);
            let t = 0.55;
            let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
            let len = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = (-dy / len * t, dx / len * t);
            k.out.push(Paint::Quad {
                points: [
                    [p[0] + nx, p[1] + ny],
                    [q[0] + nx, q[1] + ny],
                    [q[0] - nx, q[1] - ny],
                    [p[0] - nx, p[1] - ny],
                ],
                argb: palette::DANGER,
            });
        }
    }
    w
}

/// The card's own Join / Play button at its right when it is hovered or selected; its width (0
/// when hidden).
fn open_button(k: &mut Kit<'_, Hit>, r: [f32; 4], label: &str, index: usize, show: bool) -> f32 {
    if !show {
        return 0.0;
    }
    let h = 20.0;
    let w = k.button_width(label, h).max(58.0);
    let b = [r[0] + r[2] - w - 10.0, r[1] + (r[3] - h) / 2.0, w, h];
    k.button(b, label, Look::Primary, false, Hit::Open(index));
    w + 10.0
}

fn server_card(k: &mut Kit<'_, Hit>, s: &ServerRow, r: [f32; 4], v: &View<'_>, time: f32) {
    plate(k, r, s.selected);
    k.hit(r, Hit::Row(s.index));
    let pad = 6.0;
    let side = r[3] - 2.0 * pad;
    let pic = [r[0] + pad + 2.0, r[1] + pad, side, side];
    picture(
        k,
        pic,
        if s.lan { &s.address } else { &s.name },
        if s.lan { Glyph::Globe } else { Glyph::People },
        s.icon.as_ref(),
    );
    let tx = pic[0] + side + 9.0;
    let hover = k.over(r);
    // The right side: Join when hovered or selected, else the status.
    let open = open_button(k, r, v.open_label, s.index, hover || s.selected);
    let right = r[0] + r[2] - 10.0 - open;
    let mut status_w = 0.0;
    if open == 0.0 {
        let cy = r[1] + 15.0;
        match &s.reach {
            Reach::Online { ping_ms } => {
                let sw = signal(k, right, cy, Some(Some(*ping_ms)), time);
                let ms = format!("{ping_ms} ms");
                let mw = k.text(
                    right - sw - 5.0,
                    cy,
                    &ms,
                    6.5,
                    Weight::Medium,
                    0xFF8A_867F,
                    Align::Right,
                );
                status_w = sw + 5.0 + mw;
            }
            Reach::Pinging => {
                status_w = signal(k, right, cy, None, time);
            }
            Reach::Unreachable => {
                status_w = signal(k, right, cy, Some(None), time);
            }
            Reach::Incompatible { version } => {
                let (fill, ink) = Tone::Danger.colors();
                let pw = k.pill_width(version, 11.0);
                status_w = k.pill(right - pw, cy, 11.0, version, fill, ink);
            }
            Reach::Lan => {
                let (fill, ink) = Tone::Info.colors();
                let pw = k.pill_width("LAN", 11.0);
                status_w = k.pill(right - pw, cy, 11.0, "LAN", fill, ink);
            }
        }
        if let Some((online, max)) = s.players {
            let text = format!(
                "{} / {}",
                i18n::group_digits(online, v.lang),
                i18n::group_digits(max, v.lang)
            );
            k.text(
                right,
                r[1] + r[3] - 13.0,
                &text,
                7.0,
                Weight::SemiBold,
                palette::INK,
                Align::Right,
            );
            let pw = (k.measure)(&text, 7.0, Weight::SemiBold);
            // A person's head and shoulders.
            let px = right - pw - 9.0;
            let py = r[1] + r[3] - 13.0;
            k.round([px + 1.5, py - 3.5, 3.0, 3.0], 1.5, palette::MUTED, 0);
            k.round([px, py + 0.2, 6.0, 3.3], 1.5, palette::MUTED, 0);
            status_w = status_w.max(pw + 9.0);
        }
    }
    let room = (right - status_w - 10.0 - tx).max(20.0);
    // The name, and the address after it when there is room.
    let name_w = k.text(
        tx,
        r[1] + 12.0,
        &fit(&s.name, room, 8.5, Weight::SemiBold, k.measure),
        8.5,
        Weight::SemiBold,
        palette::INK,
        Align::Left,
    );
    let addr_room = room - name_w - 8.0;
    if addr_room > 30.0 && v.gui[0] >= 500.0 {
        k.text_fit(
            tx + name_w + 6.0,
            r[1] + 12.5,
            addr_room,
            &s.address,
            6.0,
            Weight::Medium,
            0xFF6E_6B67,
        );
    }
    // The MOTD (two lines), its colours kept; the state when there is none.
    let lines: Vec<&Vec<(String, u32)>> = s.motd.iter().filter(|l| !l.is_empty()).take(2).collect();
    if lines.is_empty() {
        let (text, ink) = match &s.reach {
            Reach::Pinging => (
                format!(
                    "{}{}",
                    i18n::t(T::Loading, v.lang)
                        .trim_end_matches('…')
                        .trim_end_matches("..."),
                    ".".repeat((time * 3.0) as usize % 4)
                ),
                palette::MUTED,
            ),
            Reach::Unreachable => (String::new(), palette::DANGER),
            _ => (String::new(), palette::MUTED),
        };
        if !text.is_empty() {
            k.text_fit(tx, r[1] + 27.0, room, &text, 7.0, Weight::Medium, ink);
        }
    }
    for (i, l) in lines.iter().enumerate() {
        k.runs(
            tx,
            r[1] + 24.5 + i as f32 * 9.0,
            room,
            l,
            7.0,
            Weight::Medium,
        );
    }
}

fn world_card(k: &mut Kit<'_, Hit>, w: &WorldRow, r: [f32; 4], v: &View<'_>) {
    plate(k, r, w.selected);
    k.hit(r, Hit::Row(w.index));
    let pad = 6.0;
    let side = r[3] - 2.0 * pad;
    let pic = [r[0] + pad + 2.0, r[1] + pad, side, side];
    picture(k, pic, &w.name, Glyph::Globe, w.icon.as_ref());
    let tx = pic[0] + side + 9.0;
    let hover = k.over(r);
    let open = open_button(
        k,
        r,
        v.open_label,
        w.index,
        w.playable && (hover || w.selected),
    );
    let mut right = r[0] + r[2] - 10.0 - open;
    // The badges on the right, last first, while they fit.
    if open == 0.0 {
        let cy = r[1] + r[3] / 2.0;
        let room = right - tx - 140.0;
        let mut used = 0.0;
        for (label, tone) in w.badges.iter().rev() {
            let pw = k.pill_width(label, 12.0);
            if used + pw > room {
                break;
            }
            let (fill, ink) = tone.colors();
            k.pill(right - pw, cy, 12.0, label, fill, ink);
            right -= pw + 4.0;
            used += pw + 4.0;
        }
    }
    let room = (right - 8.0 - tx).max(20.0);
    k.text_fit(
        tx,
        r[1] + 15.0,
        room,
        &w.name,
        8.5,
        Weight::SemiBold,
        palette::INK,
    );
    k.text_fit(
        tx,
        r[1] + 30.0,
        room,
        &w.detail,
        6.5,
        Weight::Medium,
        palette::MUTED,
    );
}

/// An empty or no-result state in a dashed-looking plate; its height.
#[allow(clippy::too_many_arguments)]
fn empty(
    k: &mut Kit<'_, Hit>,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    body: &str,
    buttons: &[(&str, Hit, Look)],
) -> f32 {
    let h = if buttons.is_empty() { 54.0 } else { 80.0 };
    k.round([x, y, w, h], 6.0, 0x8018_191D, 0x1FFF_FFFF);
    k.text(
        x + w / 2.0,
        y + 17.0,
        title,
        9.0,
        Weight::SemiBold,
        palette::INK,
        Align::Center,
    );
    let body = fit(body, w - 24.0, 7.0, Weight::Medium, k.measure);
    k.text(
        x + w / 2.0,
        y + 31.0,
        &body,
        7.0,
        Weight::Medium,
        palette::MUTED,
        Align::Center,
    );
    if !buttons.is_empty() {
        let bh = 20.0;
        let widths: Vec<f32> = buttons
            .iter()
            .map(|(l, _, _)| k.button_width(l, bh).max(70.0))
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + 6.0 * (widths.len() - 1) as f32;
        let mut bx = x + (w - total) / 2.0;
        for ((label, hit, look), bw) in buttons.iter().zip(widths) {
            k.button([bx, y + 46.0, bw, bh], label, *look, false, hit.clone());
            bx += bw + 6.0;
        }
    }
    h
}

/// The page.
pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let mut k: Kit<'_, Hit> = Kit::new(v.measure, v.mouse);
    // Over the whole screen: nothing of the game's list shows or takes a press.
    k.fill([0.0, 0.0, gw, gh], 0xFF0B_0C0F);
    k.hit([0.0, 0.0, gw, gh], Hit::Nothing);
    let (x, w) = column(gw);
    let mut focus_at = 0usize;
    // The header: the switch or the title, then the search row.
    match v.title {
        Some(t) => title(&mut k, gw, t),
        None => {
            crate::discover::switch_into(&mut k, gw, v.tab, v.lang, &Hit::Tab);
            focus_at += 2;
        }
    }
    let sy = 36.0;
    let sw = w.min(260.0);
    let search_focused = v.search_focused || v.focus == Some(focus_at);
    k.field(
        [x, sy, sw, 16.0],
        v.query,
        v.placeholder,
        search_focused,
        v.caret && v.search_focused,
        Hit::Search,
    );
    focus_at += 1;
    // The cards.
    let area = [x - 2.0, HEADER + 2.0, w + 6.0, gh - HEADER - FOOTER - 4.0];
    let scroll = v.scroll.max(0.0);
    let mut cards = Vec::new();
    k.out.push(Paint::ClipBegin(area));
    let top = area[1] + 4.0 - scroll;
    let mut y = top;
    let q = fold(v.query.trim());
    match &v.rows {
        Rows::Servers { saved, lan, time } => {
            let shown: Vec<&ServerRow> = saved
                .iter()
                .filter(|s| q.is_empty() || matches_server(s, &q))
                .collect();
            y += heading(&mut k, x, y, w, i18n::t(T::SavedServers, v.lang), None);
            if saved.is_empty() {
                let mut buttons: Vec<(&str, Hit, Look)> = Vec::new();
                if let Some(add) = v.buttons.iter().find(|b| b.act == Act::Create) {
                    buttons.push((&add.label, Hit::Act(Act::Create), Look::Primary));
                }
                if let Some(d) = v.discover_label {
                    buttons.push((d, Hit::Discover, Look::Secondary));
                }
                y += empty(
                    &mut k,
                    x,
                    y,
                    w,
                    i18n::t(T::NoServersTitle, v.lang),
                    i18n::t(T::NoServersBody, v.lang),
                    &buttons,
                ) + GAP;
            } else if shown.is_empty() {
                y += empty(
                    &mut k,
                    x,
                    y,
                    w,
                    i18n::t(T::NothingFound, v.lang),
                    v.query,
                    &[],
                ) + GAP;
            }
            for s in shown {
                let r = [x, y, w, CARD];
                server_card(&mut k, s, r, v, *time);
                cards.push((r, s.index));
                y += CARD + GAP;
            }
            y += 8.0;
            y += heading(
                &mut k,
                x,
                y,
                w,
                i18n::t(T::LocalNetwork, v.lang),
                Some((v.scanning, *time * 0.5)),
            );
            let lan_shown: Vec<&ServerRow> = lan
                .iter()
                .filter(|s| q.is_empty() || matches_server(s, &q))
                .collect();
            if lan_shown.is_empty() {
                k.text_fit(
                    x + 2.0,
                    y + 6.0,
                    w - 4.0,
                    i18n::t(T::LanEmpty, v.lang),
                    6.5,
                    Weight::Medium,
                    0xFF6E_6B67,
                );
                y += 16.0;
            }
            for s in lan_shown {
                let r = [x, y, w, CARD];
                server_card(&mut k, s, r, v, *time);
                cards.push((r, s.index));
                y += CARD + GAP;
            }
        }
        Rows::Worlds(worlds) => {
            let shown: Vec<&WorldRow> = worlds
                .iter()
                .filter(|wr| {
                    q.is_empty() || fold(&wr.name).contains(&q) || fold(&wr.detail).contains(&q)
                })
                .collect();
            if worlds.is_empty() {
                let mut buttons: Vec<(&str, Hit, Look)> = Vec::new();
                if let Some(c) = v.buttons.iter().find(|b| b.act == Act::Create) {
                    buttons.push((&c.label, Hit::Act(Act::Create), Look::Primary));
                }
                y += empty(
                    &mut k,
                    x,
                    y,
                    w,
                    i18n::t(T::NoWorldsTitle, v.lang),
                    i18n::t(T::NoWorldsBody, v.lang),
                    &buttons,
                ) + GAP;
            } else if shown.is_empty() {
                y += empty(
                    &mut k,
                    x,
                    y,
                    w,
                    i18n::t(T::NothingFound, v.lang),
                    v.query,
                    &[],
                ) + GAP;
            }
            for wr in shown {
                let r = [x, y, w, CARD];
                world_card(&mut k, wr, r, v);
                cards.push((r, wr.index));
                y += CARD + GAP;
            }
        }
    }
    let content = y - top + 4.0;
    k.out.push(Paint::ClipEnd);
    let max = max_scroll(content, area[3]);
    k.scrollbar(area, content, scroll.min(max));
    // The header's line and the action bar over the cards' edges.
    k.fill([0.0, HEADER, gw, 0.5], palette::RULE);
    action_bar(&mut k, v, &mut focus_at);
    Layout {
        paints: k.out,
        hits: k.hits,
        max_scroll: max,
        area,
        cards,
    }
}

fn matches_server(s: &ServerRow, q: &str) -> bool {
    fold(&s.name).contains(q)
        || fold(&s.address).contains(q)
        || s.motd
            .iter()
            .any(|l| fold(&l.iter().map(|(t, _)| t.as_str()).collect::<String>()).contains(q))
}

/// "Just now", "5 min ago", "3 h ago", "2 d ago", else the date (`2026-11-13`): `then` and `now`
/// in milliseconds since the Unix epoch.
pub fn ago(now: i64, then: i64, lang: &str) -> String {
    let secs = (now - then).max(0) / 1000;
    let n = |k: T, v: i64| i18n::t(k, lang).replace("{n}", &v.to_string());
    match secs {
        0..60 => i18n::t(T::JustNow, lang).to_owned(),
        60..3600 => n(T::MinutesAgo, secs / 60),
        3600..86_400 => n(T::HoursAgo, secs / 3600),
        86_400..2_592_000 => n(T::DaysAgo, secs / 86_400),
        _ => date(then),
    }
}

/// `ms` since the Unix epoch as a UTC date, `YYYY-MM-DD`.
pub fn date(ms: i64) -> String {
    // Howard Hinnant's days-to-civil.
    let z = ms.div_euclid(86_400_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// The row the keyboard moves to from `current` among `cards` (in their order), `down` or up;
/// the first (or last) when none is selected.
pub fn step(cards: &[([f32; 4], usize)], current: Option<usize>, down: bool) -> Option<usize> {
    if cards.is_empty() {
        return None;
    }
    let at = current.and_then(|c| cards.iter().position(|(_, i)| *i == c));
    let next = match (at, down) {
        (None, true) => 0,
        (None, false) => cards.len() - 1,
        (Some(p), true) => (p + 1).min(cards.len() - 1),
        (Some(p), false) => p.saturating_sub(1),
    };
    Some(cards[next].1)
}

/// The scroll that shows card `r` (laid out at scroll `scroll`) whole in `area`.
pub fn reveal(r: [f32; 4], area: [f32; 4], scroll: f32, max: f32) -> f32 {
    let mut s = scroll;
    if r[1] < area[1] + 4.0 {
        s -= area[1] + 4.0 - r[1];
    } else if r[1] + r[3] > area[1] + area[3] - 4.0 {
        s += r[1] + r[3] - (area[1] + area[3] - 4.0);
    }
    s.clamp(0.0, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    fn server(i: usize, name: &str, selected: bool) -> ServerRow {
        ServerRow {
            index: i,
            name: name.into(),
            address: format!("{name}.example.org"),
            lan: false,
            motd: vec![vec![
                ("A ".into(), 0xFFFF_5555),
                ("server".into(), 0xFFAA_AAAA),
            ]],
            players: Some((12, 100)),
            reach: Reach::Online { ping_ms: 40 },
            icon: None,
            selected,
        }
    }

    fn buttons() -> Vec<Button> {
        [
            (Act::Back, "Back", true),
            (Act::Refresh, "Refresh", true),
            (Act::Direct, "Direct Connection", true),
            (Act::Create, "Add Server", true),
            (Act::Edit, "Edit", false),
            (Act::Delete, "Delete", false),
            (Act::Primary, "Join Server", false),
        ]
        .into_iter()
        .map(|(act, l, enabled)| Button {
            act,
            label: l.into(),
            enabled,
        })
        .collect()
    }

    fn view<'a>(rows: Rows<'a>, buttons: &'a [Button], mouse: [f32; 2]) -> View<'a> {
        View {
            gui: [854.0, 480.0],
            mouse,
            lang: "en_us",
            measure: &test_measure,
            title: None,
            tab: Tab::MyServers,
            query: "",
            placeholder: "Search",
            search_focused: false,
            caret: false,
            rows,
            buttons,
            open_label: "Join",
            discover_label: Some("Discover"),
            scanning: "Scanning for games on your local network",
            scroll: 0.0,
            focus: None,
        }
    }

    #[test]
    fn servers_are_cards_between_the_header_and_the_action_bar() {
        let saved = vec![server(0, "alpha", false), server(1, "beta", true)];
        let lan = vec![ServerRow {
            index: 2,
            lan: true,
            reach: Reach::Lan,
            players: None,
            ..server(2, "lan", false)
        }];
        let b = buttons();
        let l = layout(&view(
            Rows::Servers {
                saved: &saved,
                lan: &lan,
                time: 0.0,
            },
            &b,
            [0.0, 0.0],
        ));
        assert_eq!(l.cards.iter().map(|c| c.1).collect::<Vec<_>>(), [0, 1, 2]);
        for (r, _) in &l.cards {
            assert!(r[1] >= HEADER && r[1] + r[3] <= 480.0 - FOOTER, "{r:?}");
        }
        // The selected card is in the accent; its Join shows.
        assert!(l.paints.iter().any(|p| matches!(
            p,
            Paint::Round {
                border: palette::ACCENT,
                ..
            }
        )));
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Open(1)));
        // A press on a card selects its row; on the action bar, its act.
        let c = l.cards[0].0;
        assert_eq!(l.hit([c[0] + 30.0, c[1] + 10.0]), Some(Hit::Row(0)));
        let refresh = l
            .hits
            .iter()
            .find(|(_, h)| *h == Hit::Act(Act::Refresh))
            .unwrap()
            .0;
        assert_eq!(l.hit(rc(refresh)), Some(Hit::Act(Act::Refresh)));
        // Off buttons take no press; Back is left of everything, the primary right.
        assert!(!l.hits.iter().any(|(_, h)| *h == Hit::Act(Act::Edit)));
        let back = l
            .hits
            .iter()
            .find(|(_, h)| *h == Hit::Act(Act::Back))
            .unwrap()
            .0;
        assert!(back[0] < refresh[0]);
        // The MOTD keeps its colours.
        assert!(l.paints.iter().any(|p| matches!(
            p,
            Paint::Text {
                argb: 0xFFFF_5555,
                ..
            }
        )));
        // The switch replaces the title: its tabs are pieces, and no text overlaps them.
        let tabs: Vec<[f32; 4]> = l
            .hits
            .iter()
            .filter(|(_, h)| matches!(h, Hit::Tab(_)))
            .map(|(r, _)| *r)
            .collect();
        assert_eq!(tabs.len(), 2);
        assert!(tabs.iter().all(|r| r[1] + r[3] < 36.0));
    }

    #[test]
    fn the_empty_list_offers_to_add_or_discover_and_search_filters() {
        let b = buttons();
        let l = layout(&view(
            Rows::Servers {
                saved: &[],
                lan: &[],
                time: 0.0,
            },
            &b,
            [0.0, 0.0],
        ));
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Discover));
        assert!(
            l.hits
                .iter()
                .any(|(r, h)| *h == Hit::Act(Act::Create) && r[1] < 480.0 - FOOTER)
        );
        let saved = vec![server(0, "alpha", false), server(1, "beta", false)];
        let mut v = view(
            Rows::Servers {
                saved: &saved,
                lan: &[],
                time: 0.0,
            },
            &b,
            [0.0, 0.0],
        );
        v.query = "BÉTA";
        let l = layout(&v);
        assert_eq!(l.cards.iter().map(|c| c.1).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn worlds_show_their_badges_and_a_title() {
        let worlds = vec![WorldRow {
            index: 0,
            name: "My world".into(),
            detail: "Played 2 h ago".into(),
            badges: vec![
                ("Survival".into(), Tone::Good),
                ("Cheats".into(), Tone::Neutral),
            ],
            icon: None,
            selected: false,
            playable: true,
        }];
        let b = vec![
            Button {
                act: Act::Back,
                label: "Back".into(),
                enabled: true,
            },
            Button {
                act: Act::Create,
                label: "Create New World".into(),
                enabled: true,
            },
            Button {
                act: Act::Primary,
                label: "Play Selected World".into(),
                enabled: false,
            },
        ];
        let mut v = view(Rows::Worlds(&worlds), &b, [0.0, 0.0]);
        v.title = Some("Singleplayer");
        v.open_label = "Play";
        let l = layout(&v);
        assert!(!l.hits.iter().any(|(_, h)| matches!(h, Hit::Tab(_))));
        for t in ["Singleplayer", "Survival", "Cheats", "My world"] {
            assert!(
                l.paints
                    .iter()
                    .any(|p| matches!(p, Paint::Text { text, .. } if text == t)),
                "{t}"
            );
        }
        // Hovered, the card shows Play in the place of its badges.
        let c = l.cards[0].0;
        let hovered = layout(&View {
            mouse: [c[0] + 50.0, c[1] + 10.0],
            ..v
        });
        assert!(hovered.hits.iter().any(|(_, h)| *h == Hit::Open(0)));
    }

    #[test]
    fn keys_step_through_the_cards_and_times_read_as_ago() {
        let cards = vec![([0.0, 10.0, 10.0, 10.0], 4), ([0.0, 30.0, 10.0, 10.0], 7)];
        assert_eq!(step(&cards, None, true), Some(4));
        assert_eq!(step(&cards, Some(4), true), Some(7));
        assert_eq!(step(&cards, Some(7), true), Some(7));
        assert_eq!(step(&cards, Some(7), false), Some(4));
        assert_eq!(step(&[], None, true), None);
        let now = 1_800_000_000_000;
        assert_eq!(ago(now, now - 5_000, "en_us"), "Just now");
        assert_eq!(ago(now, now - 300_000, "en_us"), "5 min ago");
        assert_eq!(ago(now, now - 3 * 3_600_000, "fr_fr"), "il y a 3 h");
        assert_eq!(ago(now, now - 2 * 86_400_000, "en_us"), "2 d ago");
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_763_000_000_000), "2025-11-13");
        assert_eq!(
            reveal(
                [0.0, 500.0, 10.0, 40.0],
                [0.0, 60.0, 10.0, 300.0],
                0.0,
                1000.0
            ),
            184.0
        );
    }
}
