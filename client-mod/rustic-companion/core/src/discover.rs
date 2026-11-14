//! The in-game Discover screen (the multiplayer screen's "Discover" tab, also the title's Discover
//! button): Rustic Cloud's server directory (`GET /v1/server-listings`, ADR 0158) as a featured
//! hero, trending servers, the ones played recently, categories and every server, each with its
//! icon, name, players online, tags and verified badge, Join (connects at once) and Add to list.
//! Pure parts: the API's JSON, the queries, the "recently played" match, and the layout in GUI
//! pixels; the game crate fetches, decodes pictures, draws and joins.
use crate::i18n::{self, T};
use crate::kit::{Kit, Look, line, max_scroll};
use crate::paint::{Align, Measure, Paint, Picture, wrap};
use crate::skin::palette;
use crate::title::{Glyph, Image, Weight, hue_of};
use serde_json::Value;

/// A server of the directory, as the lists give it (`ServerListing`, without the description).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    pub id: String,
    pub slug: String,
    pub name: String,
    /// `host` or `host:port`.
    pub address: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub verified: bool,
    pub featured: bool,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    /// None until the cloud pinged it.
    pub online: Option<bool>,
    pub players: Option<i64>,
    pub max_players: Option<i64>,
    pub requires_rustic_edition: bool,
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

/// Only addresses a player may join: letters, digits, dots, dashes, one optional port.
fn safe_address(a: &str) -> bool {
    let (host, port) = match a.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (a, None),
    };
    !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        && port.is_none_or(|p| p.parse::<u16>().is_ok_and(|n| n > 0))
}

/// A picture's URL, kept only when it is https (or this computer's development cloud).
fn picture_url(v: &Value, k: &str) -> Option<String> {
    let url = s(v, k)?;
    let ok = url.starts_with("https://")
        || ["http://127.0.0.1:", "http://localhost:"]
            .iter()
            .any(|p| url.starts_with(p));
    (ok && url.len() < 512).then_some(url)
}

/// One listing of the API's JSON; None for one that lacks what the screen needs.
pub fn parse_listing(v: &Value) -> Option<Listing> {
    let address = s(v, "address")?.trim().to_ascii_lowercase();
    if !safe_address(&address) {
        return None;
    }
    let status = v.get("status").filter(|s| s.is_object());
    let list = |k: &str| -> Vec<String> {
        v.get(k)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(Listing {
        id: s(v, "id")?,
        slug: s(v, "slug").unwrap_or_default(),
        name: s(v, "name")?.chars().take(64).collect(),
        address,
        summary: s(v, "summary")
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect(),
        tags: list("tags"),
        verified: v.get("verified").and_then(Value::as_bool).unwrap_or(false),
        featured: v.get("featured").and_then(Value::as_bool).unwrap_or(false),
        icon_url: picture_url(v, "icon_url"),
        banner_url: picture_url(v, "banner_url"),
        online: status
            .and_then(|s| s.get("online"))
            .and_then(Value::as_bool),
        players: status
            .and_then(|s| s.get("players_online"))
            .and_then(Value::as_i64),
        max_players: status
            .and_then(|s| s.get("players_max"))
            .and_then(Value::as_i64),
        requires_rustic_edition: v
            .get("requires_rustic_edition")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

/// A `ServerListingPage`'s listings.
pub fn parse_page(v: &Value) -> Option<Vec<Listing>> {
    Some(
        v.get("items")?
            .as_array()?
            .iter()
            .filter_map(parse_listing)
            .collect(),
    )
}

/// The catalog's tags (`ServerListingCatalog.tags`).
pub fn parse_catalog(v: &Value) -> Vec<String> {
    v.get("tags")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|t| t.len() <= 24)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// A list the screen asks for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Query {
    Featured,
    Trending,
    /// Every server, most players first.
    All,
    /// A search and/or a category.
    Search {
        q: String,
        tag: Option<String>,
    },
}

fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl Query {
    /// Its URL on the API at `api`.
    pub fn url(&self, api: &str) -> String {
        let base = format!("{api}/v1/server-listings");
        match self {
            Query::Featured => format!("{base}?featured=true&limit=6"),
            Query::Trending => format!("{base}?sort=trending&limit=24"),
            Query::All => format!("{base}?sort=players&limit=48"),
            Query::Search { q, tag } => {
                let mut url = format!("{base}?sort=trending&limit=48");
                let q = q.trim();
                if !q.is_empty() {
                    url.push_str("&q=");
                    url.push_str(&encode(&q.chars().take(100).collect::<String>()));
                }
                if let Some(t) = tag {
                    url.push_str("&tag=");
                    url.push_str(&encode(t));
                }
                url
            }
        }
    }
}

/// The catalog's URL.
pub fn catalog_url(api: &str) -> String {
    format!("{api}/v1/server-listings/catalog")
}

/// A join's anonymous count (`POST`, ADR 0158).
pub fn plays_url(api: &str, id: &str) -> String {
    format!("{api}/v1/server-listings/{}/plays", encode(id))
}

/// An address as the server list compares them: lower case, the default port left out.
pub fn same_address(a: &str) -> String {
    let a = a.trim().to_ascii_lowercase();
    a.strip_suffix(":25565").map(str::to_owned).unwrap_or(a)
}

/// The listings among `pool` the player has played: the addresses of the game's own server list
/// (`recent`, the ones joined by address first, newest first), at most `max`.
pub fn recently_played<'a>(
    pool: &[&'a Listing],
    recent: &[String],
    max: usize,
) -> Vec<&'a Listing> {
    let mut out: Vec<&Listing> = Vec::new();
    for address in recent {
        let a = same_address(address);
        if let Some(l) = pool.iter().find(|l| same_address(&l.address) == a)
            && !out.iter().any(|o| o.id == l.id)
        {
            out.push(l);
        }
        if out.len() == max {
            break;
        }
    }
    out
}

/// HSL (degrees, 0-1, 0-1) to an opaque ARGB colour.
pub fn hue_color(hue: f32, sat: f32, light: f32) -> u32 {
    let c = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let hp = hue.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = light - c / 2.0;
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    0xFF00_0000 | (to(r) << 16) | (to(g) << 8) | to(b)
}

/// The multiplayer screen's two tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    MyServers,
    Discover,
}

/// How the lists stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Loading,
    Ready,
    Failed,
}

/// What a press lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Tab(Tab),
    Search,
    Category(Option<String>),
    Join(String),
    Add(String),
    Retry,
    /// The "Publish yours" link (the website's publish page).
    Publish,
    /// The hero's dots.
    Hero(usize),
    /// Anything else inside the screen (eaten, so the list under it gets nothing).
    Nothing,
}

/// The tab switch at the top of the multiplayer screen ("My servers | Discover").
pub fn switch(kit: &mut Kit<'_, Hit>, gui_w: f32, tab: Tab, lang: &str) {
    switch_into(kit, gui_w, tab, lang, &Hit::Tab);
}

/// [`switch`] in a screen whose presses are `H` (the server list's), `hit` naming a tab's.
pub fn switch_into<H: Clone>(
    kit: &mut Kit<'_, H>,
    gui_w: f32,
    tab: Tab,
    lang: &str,
    hit: &dyn Fn(Tab) -> H,
) {
    let labels = [
        (Tab::MyServers, i18n::t(T::MyServers, lang)),
        (Tab::Discover, i18n::t(T::Discover, lang)),
    ];
    let h = 18.0;
    let seg: f32 = labels
        .iter()
        .map(|(_, l)| (kit.measure)(l, 7.5, Weight::SemiBold) + 24.0)
        .fold(0.0, f32::max)
        .max(70.0)
        .ceil();
    let w = seg * 2.0 + 4.0;
    let r = [((gui_w - w) / 2.0).round(), 7.5, w, h + 4.0];
    kit.round(r, (h + 4.0) / 2.0, 0xFF0F_1013, palette::BORDER);
    for (i, (t, label)) in labels.iter().enumerate() {
        let s = [r[0] + 2.0 + i as f32 * seg, r[1] + 2.0, seg, h];
        let on = *t == tab;
        let lit = kit.over(s);
        let (fill, ink) = match (on, lit) {
            (true, _) => (palette::ACCENT, 0xFFFF_FFFF),
            (false, true) => (0x1AFF_FFFF, palette::INK),
            (false, false) => (0, palette::MUTED),
        };
        if fill != 0 {
            kit.round(s, h / 2.0, fill, 0);
        }
        kit.text(
            s[0] + seg / 2.0,
            s[1] + h / 2.0,
            label,
            7.5,
            Weight::SemiBold,
            ink,
            Align::Center,
        );
        kit.hit(s, hit(*t));
    }
}

/// What the Discover screen is drawn from.
pub struct View<'a> {
    pub gui: [f32; 2],
    pub mouse: [f32; 2],
    pub lang: &'a str,
    pub measure: Measure<'a>,
    pub query: &'a str,
    pub search_focused: bool,
    pub caret: bool,
    pub tags: &'a [String],
    pub tag: Option<&'a str>,
    pub featured: &'a [Listing],
    pub trending: &'a [Listing],
    pub all: &'a [Listing],
    pub recent: &'a [&'a Listing],
    /// The search's or category's results (shown instead of the sections while searching).
    pub results: Option<&'a [Listing]>,
    pub status: Status,
    /// Whether the player's list has this address already.
    pub listed: &'a dyn Fn(&str) -> bool,
    pub scroll: f32,
    /// Seconds since the screen opened (the hero turns).
    pub time: f32,
    /// The button the keyboard is on (an index into the layout's focusable hits).
    pub focus: Option<usize>,
}

/// The website's page to publish a server, in the game's language.
pub fn publish_url(lang: &str) -> String {
    i18n::website("/servers/publish", lang)
}

pub struct Layout {
    pub paints: Vec<Paint>,
    pub hits: Vec<([f32; 4], Hit)>,
    pub max_scroll: f32,
    /// The scrolled area.
    pub area: [f32; 4],
    /// The featured listing the hero shows.
    pub hero: Option<usize>,
}

impl Layout {
    /// What a press at `at` lands on: inside the scrolled area, what is there; outside it, the
    /// search field or nothing.
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        let in_area = crate::paint::inside(self.area, at);
        self.hits
            .iter()
            .rev()
            .filter(|(_, h)| in_area || matches!(h, Hit::Nothing | Hit::Search))
            .find(|(r, _)| crate::paint::inside(*r, at))
            .map(|(_, h)| h.clone())
    }
    /// The pieces the keyboard walks (Join, Add, categories), in order, with their boxes.
    pub fn focusable(&self) -> Vec<([f32; 4], Hit)> {
        self.hits
            .iter()
            .filter(|(_, h)| {
                matches!(
                    h,
                    Hit::Join(_) | Hit::Add(_) | Hit::Category(_) | Hit::Publish
                )
            })
            .cloned()
            .collect()
    }
}

/// Seconds a featured server stays on the hero.
pub const HERO_TURN: f32 = 7.0;

/// The verified badge: an accent disc with a check, `d` across, centred on `c`.
fn verified_badge(k: &mut Kit<'_, Hit>, c: [f32; 2], d: f32) {
    k.round(
        [c[0] - d / 2.0, c[1] - d / 2.0, d, d],
        d / 2.0,
        palette::ACCENT,
        0,
    );
    let t = (d * 0.09).max(0.45);
    for (a, b) in [
        ([c[0] - d * 0.22, c[1]], [c[0] - d * 0.05, c[1] + d * 0.18]),
        (
            [c[0] - d * 0.07, c[1] + d * 0.18],
            [c[0] + d * 0.24, c[1] - d * 0.17],
        ),
    ] {
        k.out.push(Paint::Quad {
            points: [
                [a[0], a[1] - t],
                [b[0], b[1] - t],
                [b[0], b[1] + t],
                [a[0], a[1] + t],
            ],
            argb: 0xFFFF_FFFF,
        });
    }
}

/// A server's icon (its own picture over a coloured tile while it loads, or for none).
fn icon(k: &mut Kit<'_, Hit>, l: &Listing, r: [f32; 4]) {
    k.out.push(Paint::Picture {
        picture: Picture::Title(Image::Tile {
            hue: hue_of(&l.slug),
            glyph: Glyph::People,
        }),
        rect: r,
        tint: 0xFFFF_FFFF,
    });
    if let Some(url) = &l.icon_url {
        k.out.push(Paint::Picture {
            picture: Picture::Remote(url.clone()),
            rect: r,
            tint: 0xFFFF_FFFF,
        });
    }
}

/// "123 online" with a green dot, or "Offline"; its width.
fn players_line(k: &mut Kit<'_, Hit>, l: &Listing, x: f32, cy: f32, size: f32, lang: &str) -> f32 {
    let (text, dot) = match (l.online, l.players) {
        (Some(false), _) => (i18n::t(T::Offline, lang).to_owned(), palette::MUTED),
        (_, Some(n)) => (i18n::players_online(n, lang), palette::GREEN),
        (Some(true), None) => (i18n::players_online(0, lang), palette::GREEN),
        (None, None) => (String::new(), 0),
    };
    if text.is_empty() {
        return 0.0;
    }
    let d = size * 0.6;
    k.round([x, cy - d / 2.0, d, d], d / 2.0, dot, 0);
    d + 3.0
        + k.text(
            x + d + 3.0,
            cy,
            &text,
            size,
            Weight::Medium,
            palette::MUTED,
            Align::Left,
        )
}

/// Small tag pills from `x`, at most `room` wide; the width used.
fn tag_pills(k: &mut Kit<'_, Hit>, tags: &[String], x: f32, cy: f32, room: f32, lang: &str) -> f32 {
    let mut used = 0.0;
    for t in tags.iter().take(3) {
        let name = i18n::tag_name(t, lang);
        let w = (k.measure)(&name, 5.5, Weight::SemiBold) + 8.0;
        if used + w > room {
            break;
        }
        k.round([x + used, cy - 5.0, w, 10.0], 5.0, 0x1AFF_FFFF, 0);
        k.text(
            x + used + w / 2.0,
            cy,
            &name,
            5.5,
            Weight::SemiBold,
            palette::MUTED,
            Align::Center,
        );
        used += w + 3.0;
    }
    used
}

/// Join and Add (or "in your list") at the bottom right of `r`.
fn actions(
    k: &mut Kit<'_, Hit>,
    l: &Listing,
    right: f32,
    cy: f32,
    h: f32,
    v: &View<'_>,
    focus: &mut Focus,
) {
    let join_label = i18n::t(T::Join, v.lang);
    let jw = ((k.measure)(join_label, 7.0, Weight::SemiBold) + 18.0).max(44.0);
    let join = [right - jw, cy - h / 2.0, jw, h];
    let f = focus.next(Hit::Join(l.id.clone()));
    k.button(join, join_label, Look::Primary, f, Hit::Join(l.id.clone()));
    let add = [join[0] - h - 4.0, join[1], h, h];
    if (v.listed)(&l.address) {
        // Already in the list: a muted check, nothing to press.
        k.round(add, 4.0, 0x0FFF_FFFF, palette::BORDER);
        let c = [add[0] + h / 2.0, add[1] + h / 2.0];
        let t = 0.6;
        for (a, b) in [
            ([c[0] - 3.0, c[1]], [c[0] - 1.0, c[1] + 2.2]),
            ([c[0] - 1.2, c[1] + 2.2], [c[0] + 3.0, c[1] - 2.4]),
        ] {
            k.out.push(Paint::Quad {
                points: [
                    [a[0], a[1] - t],
                    [b[0], b[1] - t],
                    [b[0], b[1] + t],
                    [a[0], a[1] + t],
                ],
                argb: palette::GREEN,
            });
        }
        k.hit(add, Hit::Nothing);
    } else {
        let f = focus.next(Hit::Add(l.id.clone()));
        k.button(add, "+", Look::Secondary, f, Hit::Add(l.id.clone()));
    }
}

/// Counts the focusable pieces as they are laid out, to light the keyboard's one.
struct Focus {
    at: usize,
    want: Option<usize>,
}

impl Focus {
    fn next(&mut self, _h: Hit) -> bool {
        let lit = self.want == Some(self.at);
        self.at += 1;
        lit
    }
}

const CARD_H: f32 = 56.0;

/// A server's card.
fn card(k: &mut Kit<'_, Hit>, l: &Listing, r: [f32; 4], v: &View<'_>, focus: &mut Focus) {
    let lit = k.over(r);
    k.round(
        r,
        5.0,
        if lit {
            palette::PANEL_RAISED
        } else {
            0xE617_181C
        },
        if lit { 0x33FF_FFFF } else { palette::BORDER },
    );
    let pad = 7.0;
    let side = 24.0;
    icon(k, l, [r[0] + pad, r[1] + pad, side, side]);
    let tx = r[0] + pad + side + 7.0;
    let room = r[0] + r[2] - pad - tx;
    let name_w = (k.measure)(&l.name, 8.0, Weight::SemiBold);
    let badge = if l.verified { 10.0 } else { 0.0 };
    k.text_fit(
        tx,
        r[1] + pad + 5.0,
        room - badge,
        &l.name,
        8.0,
        Weight::SemiBold,
        palette::INK,
    );
    if l.verified {
        let bx = tx + name_w.min(room - badge) + 6.0;
        verified_badge(k, [bx, r[1] + pad + 5.0], 7.0);
    }
    let summary = crate::paint::fit(&l.summary, room, 6.0, Weight::Medium, k.measure);
    k.text(
        tx,
        r[1] + pad + 15.0,
        &summary,
        6.0,
        Weight::Medium,
        palette::MUTED,
        Align::Left,
    );
    let by = r[1] + r[3] - pad - 7.0;
    let used = players_line(k, l, r[0] + pad, by, 6.0, v.lang);
    let button_h = 14.0;
    let actions_w = 44.0 + button_h + 8.0;
    tag_pills(
        k,
        &l.tags,
        r[0] + pad + used + 6.0,
        by,
        r[2] - 2.0 * pad - used - 6.0 - actions_w,
        v.lang,
    );
    actions(k, l, r[0] + r[2] - pad, by, button_h, v, focus);
}

/// A section's title.
fn heading(k: &mut Kit<'_, Hit>, x: f32, y: f32, text: &str) -> f32 {
    k.text(
        x,
        y + 6.0,
        text,
        9.0,
        Weight::SemiBold,
        palette::INK,
        Align::Left,
    );
    16.0
}

/// A grid of cards from `y`; its height.
fn grid(
    k: &mut Kit<'_, Hit>,
    items: &[&Listing],
    x: f32,
    y: f32,
    w: f32,
    v: &View<'_>,
    focus: &mut Focus,
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    let gap = 6.0;
    let columns = ((w + gap) / (190.0 + gap)).floor().clamp(1.0, 3.0) as usize;
    let cw = ((w - gap * (columns as f32 - 1.0)) / columns as f32).floor();
    for (i, l) in items.iter().enumerate() {
        let (row, col) = (i / columns, i % columns);
        let r = [
            x + col as f32 * (cw + gap),
            y + row as f32 * (CARD_H + gap),
            cw,
            CARD_H,
        ];
        card(k, l, r, v, focus);
    }
    let rows = items.len().div_ceil(columns);
    rows as f32 * (CARD_H + gap) - gap
}

/// The featured hero; its height.
fn hero(
    k: &mut Kit<'_, Hit>,
    items: &[Listing],
    x: f32,
    y: f32,
    w: f32,
    v: &View<'_>,
    focus: &mut Focus,
) -> (f32, Option<usize>) {
    if items.is_empty() {
        return (0.0, None);
    }
    let i = (v.time.max(0.0) / HERO_TURN) as usize % items.len();
    let l = &items[i];
    let h = 92.0;
    let r = [x, y, w, h];
    let hue = f32::from(hue_of(&l.slug));
    k.round(r, 6.0, hue_color(hue, 0.45, 0.22), palette::BORDER);
    if let Some(url) = &l.banner_url {
        k.out.push(Paint::Picture {
            picture: Picture::Remote(url.clone()),
            rect: [x + 1.0, y + 1.0, w - 2.0, h - 2.0],
            tint: 0xFFFF_FFFF,
        });
    }
    // A shade from the left so the text reads over any banner.
    k.out.push(Paint::Gradient {
        rect: [x + 1.0, y + 1.0, w - 2.0, h - 2.0],
        left: 0xF20B_0C0F,
        right: 0x260B_0C0F,
    });
    let pad = 12.0;
    let label = i18n::t(T::Featured, v.lang).to_uppercase();
    let lw = (k.measure)(&label, 5.5, Weight::SemiBold) + 10.0;
    k.round([x + pad, y + pad, lw, 10.0], 5.0, palette::ACCENT, 0);
    k.text(
        x + pad + lw / 2.0,
        y + pad + 5.0,
        &label,
        5.5,
        Weight::SemiBold,
        0xFFFF_FFFF,
        Align::Center,
    );
    let side = 30.0;
    let iy = y + pad + 16.0;
    icon(k, l, [x + pad, iy, side, side]);
    let tx = x + pad + side + 9.0;
    let room = (w * 0.62).min(w - tx + x - pad);
    let name_w = (k.measure)(&l.name, 12.0, Weight::SemiBold).min(room - 12.0);
    k.text_fit(
        tx,
        iy + 7.0,
        room - 12.0,
        &l.name,
        12.0,
        Weight::SemiBold,
        0xFFFF_FFFF,
    );
    if l.verified {
        verified_badge(k, [tx + name_w + 7.0, iy + 7.0], 8.0);
    }
    for (n, text) in wrap(&l.summary, room, 6.5, Weight::Medium, 2, k.measure)
        .iter()
        .enumerate()
    {
        k.text(
            tx,
            iy + 19.0 + n as f32 * line(6.5),
            text,
            6.5,
            Weight::Medium,
            0xFFD6_D3CF,
            Align::Left,
        );
    }
    let by = y + h - pad - 6.0;
    let used = players_line(k, l, x + pad, by, 6.5, v.lang);
    tag_pills(k, &l.tags, x + pad + used + 6.0, by, room, v.lang);
    actions(k, l, x + w - pad, by, 16.0, v, focus);
    // The dots, one per featured server.
    if items.len() > 1 {
        let n = items.len();
        let dw = 10.0;
        let total = n as f32 * dw + (n as f32 - 1.0) * 3.0;
        let dx = x + w - pad - total;
        for d in 0..n {
            let rr = [dx + d as f32 * (dw + 3.0), y + pad + 3.0, dw, 3.0];
            k.round(
                rr,
                1.5,
                if d == i { palette::ACCENT } else { 0x59FF_FFFF },
                0,
            );
            k.hit([rr[0], rr[1] - 4.0, rr[2], rr[3] + 8.0], Hit::Hero(d));
        }
    }
    (h, Some(i))
}

/// Category chips wrapping over lines; their height.
fn categories(
    k: &mut Kit<'_, Hit>,
    v: &View<'_>,
    x: f32,
    y: f32,
    w: f32,
    focus: &mut Focus,
) -> f32 {
    let h = 14.0;
    let (mut cx, mut cy) = (x, y);
    let mut chips: Vec<(Option<String>, String)> = vec![(None, i18n::t(T::All, v.lang).to_owned())];
    chips.extend(
        v.tags
            .iter()
            .map(|t| (Some(t.clone()), i18n::tag_name(t, v.lang))),
    );
    for (tag, label) in chips {
        let cw = k.chip_width(&label, h);
        if cx + cw > x + w && cx > x {
            cx = x;
            cy += h + 4.0;
        }
        let selected = tag.as_deref() == v.tag;
        let f = focus.next(Hit::Category(tag.clone()));
        let r = [cx, cy, cw, h];
        k.chip(r, &label, selected || f, Hit::Category(tag));
        cx += cw + 4.0;
    }
    cy + h - y
}

/// A centred message (empty, loading, failed); its height.
fn message(
    k: &mut Kit<'_, Hit>,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    body: &str,
    retry: Option<&str>,
) -> f32 {
    let h = if retry.is_some() { 74.0 } else { 54.0 };
    let r = [x, y, w, h];
    k.round(r, 6.0, 0x99121317, palette::BORDER);
    k.text(
        x + w / 2.0,
        y + 16.0,
        title,
        9.0,
        Weight::SemiBold,
        palette::INK,
        Align::Center,
    );
    k.text(
        x + w / 2.0,
        y + 30.0,
        body,
        6.5,
        Weight::Medium,
        palette::MUTED,
        Align::Center,
    );
    if let Some(label) = retry {
        k.button(
            [x + w / 2.0 - 40.0, y + 46.0, 80.0, 16.0],
            label,
            Look::Secondary,
            false,
            Hit::Retry,
        );
    }
    h
}

/// Lays the Discover screen out under the multiplayer screen's header (the switch).
pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let mut k: Kit<'_, Hit> = Kit::new(v.measure, v.mouse);
    // Over the whole multiplayer screen: nothing of the list shows or takes a press.
    k.fill([0.0, 0.0, gw, gh], 0xFF0B_0C0F);
    k.hit([0.0, 0.0, gw, gh], Hit::Nothing);
    let margin = 12.0;
    let w = (gw - 2.0 * margin).min(620.0);
    let x = ((gw - w) / 2.0).round();
    // The search row under the switch.
    let sy = 36.0;
    let sw = w.min(260.0);
    k.field(
        [x, sy, sw, 16.0],
        v.query,
        i18n::t(T::Search, v.lang),
        v.search_focused,
        v.caret,
        Hit::Search,
    );
    let hint = i18n::t(T::PublishHint, v.lang);
    let hw = (k.measure)(hint, 6.0, Weight::Medium);
    // The link is the keyboard's first stop when it shows.
    let link = w - sw > hw + 16.0;
    if link {
        // A link: the accent and an underline when pointed at or focused.
        let r = [x + w - hw - 2.0, sy + 1.0, hw + 4.0, 14.0];
        let lit = k.over(r) || v.focus == Some(0);
        let ink = if lit {
            palette::ACCENT_TEXT
        } else {
            palette::MUTED
        };
        k.text(
            x + w,
            sy + 8.0,
            hint,
            6.0,
            Weight::Medium,
            ink,
            Align::Right,
        );
        if lit {
            k.fill([x + w - hw, sy + 12.0, hw, 0.6], ink);
        }
        k.hit(r, Hit::Publish);
    }
    let area = [x - 2.0, sy + 24.0, w + 6.0, gh - sy - 24.0 - 6.0];
    let mut focus = Focus {
        at: usize::from(link),
        want: v.focus,
    };
    let scroll = v.scroll.max(0.0);
    k.out.push(Paint::ClipBegin(area));
    let top = area[1] + 2.0 - scroll;
    let mut y = top;
    let mut hero_index = None;
    let gap = 12.0;
    match (v.status, v.results) {
        (_, Some(results)) => {
            y += categories(&mut k, v, x, y, w, &mut focus) + gap;
            if results.is_empty() {
                y += message(
                    &mut k,
                    x,
                    y,
                    w,
                    i18n::t(T::NoResults, v.lang),
                    i18n::t(T::PublishHint, v.lang),
                    None,
                );
            } else {
                let items: Vec<&Listing> = results.iter().collect();
                y += grid(&mut k, &items, x, y, w, v, &mut focus);
            }
        }
        (Status::Loading, None) if v.trending.is_empty() && v.all.is_empty() => {
            let dots = ".".repeat((v.time * 3.0) as usize % 4);
            y += message(
                &mut k,
                x,
                y,
                w,
                &format!(
                    "{}{dots}",
                    i18n::t(T::Loading, v.lang).trim_end_matches('…')
                ),
                i18n::t(T::RusticEdition, v.lang),
                None,
            );
        }
        (Status::Failed, None) if v.trending.is_empty() && v.all.is_empty() => {
            y += message(
                &mut k,
                x,
                y,
                w,
                i18n::t(T::ErrorTitle, v.lang),
                "api.rustic-launcher.com",
                Some(i18n::t(T::Retry, v.lang)),
            );
        }
        _ => {
            if v.featured.is_empty() && v.trending.is_empty() && v.all.is_empty() {
                y += message(
                    &mut k,
                    x,
                    y,
                    w,
                    i18n::t(T::EmptyTitle, v.lang),
                    i18n::t(T::EmptyBody, v.lang),
                    None,
                ) + gap;
            }
            let (hh, hi) = hero(&mut k, v.featured, x, y, w, v, &mut focus);
            hero_index = hi;
            if hh > 0.0 {
                y += hh + gap;
            }
            if !v.recent.is_empty() {
                y += heading(&mut k, x, y, i18n::t(T::RecentlyPlayed, v.lang));
                y += grid(&mut k, v.recent, x, y, w, v, &mut focus) + gap;
            }
            if !v.trending.is_empty() {
                y += heading(&mut k, x, y, i18n::t(T::Trending, v.lang));
                let items: Vec<&Listing> = v.trending.iter().take(6).collect();
                y += grid(&mut k, &items, x, y, w, v, &mut focus) + gap;
            }
            if !v.tags.is_empty() {
                y += heading(&mut k, x, y, i18n::t(T::Categories, v.lang));
                y += categories(&mut k, v, x, y, w, &mut focus) + gap;
            }
            if !v.all.is_empty() {
                y += heading(&mut k, x, y, i18n::t(T::AllServers, v.lang));
                let items: Vec<&Listing> = v.all.iter().collect();
                y += grid(&mut k, &items, x, y, w, v, &mut focus);
            }
        }
    }
    k.out.push(Paint::ClipEnd);
    let content = y - top + 4.0;
    let max = max_scroll(content, area[3]);
    k.scrollbar([area[0], area[1], area[2] + 2.0, area[3]], content, scroll);
    let hits = k.hits;
    Layout {
        paints: k.out,
        hits,
        max_scroll: max,
        area,
        hero: hero_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;
    use serde_json::json;

    fn listing(id: &str, name: &str, address: &str) -> Value {
        json!({
            "id": id, "slug": name.to_lowercase().replace(' ', "-"), "name": name,
            "address": address, "summary": "A friendly survival server with events every week",
            "description": null, "tags": ["survival", "smp"], "languages": ["en", "fr"],
            "engine": "rustic", "game_version": "26.3", "requires_rustic_edition": false,
            "website": null, "discord": null,
            "icon_url": format!("https://cdn.rustic-launcher.com/marketplace/servers/{id}/icon.png"),
            "banner_url": null, "visibility": "public", "verified": true, "featured": false,
            "owner": null,
            "status": {"online": true, "players_online": 42, "players_max": 100, "motd": "hi",
                       "version": "26.3", "checked_at": "2026-11-13T10:00:00Z"},
            "plays_7d": 12, "created_at": "2026-11-13T10:00:00Z", "updated_at": "2026-11-13T10:00:00Z"
        })
    }

    #[test]
    fn the_apis_listings_parse_and_unsafe_ones_are_dropped() {
        let page = json!({
            "items": [
                listing("a1", "Ember SMP", "Play.Ember.gg"),
                listing("a2", "Bad", "evil host; rm -rf"),
                {"id": "a3", "name": "No address"},
                listing("a4", "Port", "mc.example.org:25570"),
            ],
            "has_more": false, "total": 3
        });
        let items = parse_page(&page).unwrap();
        assert_eq!(items.len(), 2);
        let a = &items[0];
        assert_eq!(a.address, "play.ember.gg");
        assert_eq!(a.players, Some(42));
        assert_eq!(a.online, Some(true));
        assert!(a.verified);
        assert_eq!(a.tags, ["survival", "smp"]);
        assert!(a.icon_url.as_deref().unwrap().starts_with("https://cdn."));
        assert_eq!(items[1].address, "mc.example.org:25570");
        // A plain http picture from elsewhere is ignored.
        let mut v = listing("x", "X", "x.org");
        v["icon_url"] = json!("http://evil.example/icon.png");
        assert_eq!(parse_listing(&v).unwrap().icon_url, None);
        assert_eq!(
            parse_catalog(&json!({"tags": ["survival", "pvp"], "languages": []})),
            ["survival", "pvp"]
        );
    }

    #[test]
    fn queries_have_the_apis_parameters() {
        let api = "https://api.rustic-launcher.com";
        assert_eq!(
            Query::Trending.url(api),
            "https://api.rustic-launcher.com/v1/server-listings?sort=trending&limit=24"
        );
        assert!(Query::Featured.url(api).ends_with("?featured=true&limit=6"));
        assert_eq!(
            Query::Search {
                q: "créa & pvp".into(),
                tag: Some("pvp".into())
            }
            .url(api),
            "https://api.rustic-launcher.com/v1/server-listings?sort=trending&limit=48&q=cr%C3%A9a%20%26%20pvp&tag=pvp"
        );
        assert_eq!(
            plays_url(api, "a1"),
            "https://api.rustic-launcher.com/v1/server-listings/a1/plays"
        );
        assert_eq!(
            catalog_url(api),
            format!("{api}/v1/server-listings/catalog")
        );
    }

    #[test]
    fn recently_played_follows_the_games_list() {
        let page = json!({"items": [listing("a", "A", "a.org"), listing("b", "B", "b.org:25565"),
            listing("c", "C", "c.org")]});
        let items = parse_page(&page).unwrap();
        let pool: Vec<&Listing> = items.iter().collect();
        let recent = vec!["B.org".to_owned(), "zzz.org".into(), "a.org:25565".into()];
        let got: Vec<&str> = recently_played(&pool, &recent, 6)
            .iter()
            .map(|l| l.id.as_str())
            .collect();
        assert_eq!(got, ["b", "a"]);
        assert_eq!(recently_played(&pool, &recent, 1).len(), 1);
    }

    fn view<'a>(
        featured: &'a [Listing],
        trending: &'a [Listing],
        recent: &'a [&'a Listing],
        status: Status,
        results: Option<&'a [Listing]>,
    ) -> View<'a> {
        View {
            gui: [640.0, 360.0],
            mouse: [-1.0, -1.0],
            lang: "fr_fr",
            measure: &test_measure,
            query: "",
            search_focused: false,
            caret: false,
            tags: &[],
            tag: None,
            featured,
            trending,
            all: trending,
            recent,
            results,
            status,
            listed: &|a| a == "b.org",
            scroll: 0.0,
            time: 0.0,
            focus: None,
        }
    }

    #[test]
    fn the_screen_has_its_sections_and_join_buttons() {
        let page = json!({"items": [listing("a", "A", "a.org"), listing("b", "B", "b.org")]});
        let items = parse_page(&page).unwrap();
        let recent: Vec<&Listing> = vec![&items[1]];
        let l = layout(&view(&items[..1], &items, &recent, Status::Ready, None));
        let texts: Vec<String> = l
            .paints
            .iter()
            .filter_map(|p| match p {
                Paint::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        for want in [
            "À LA UNE",
            "Tendances",
            "Joués récemment",
            "Tous les serveurs",
            "42 en ligne",
        ] {
            assert!(texts.iter().any(|t| t == want), "{want} in {texts:?}");
        }
        assert_eq!(l.hero, Some(0));
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Join("a".into())));
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Add("a".into())));
        // b is in the player's list already: no Add for it.
        assert!(!l.hits.iter().any(|(_, h)| *h == Hit::Add("b".into())));
        // Icons: a picture over a tile for every card.
        assert!(l.paints.iter().any(|p| matches!(
            p,
            Paint::Picture { picture: Picture::Remote(u), .. } if u.contains("/a/icon.png")
        )));
        assert!(l.max_scroll > 0.0);
    }

    #[test]
    fn empty_loading_failed_and_no_results_have_their_message() {
        let has = |l: &Layout, s: &str| {
            l.paints
                .iter()
                .any(|p| matches!(p, Paint::Text { text, .. } if text.starts_with(s)))
        };
        let l = layout(&view(&[], &[], &[], Status::Ready, None));
        assert!(has(&l, "Aucun serveur pour l'instant"));
        let l = layout(&view(&[], &[], &[], Status::Loading, None));
        assert!(has(&l, "Chargement des serveurs"));
        let l = layout(&view(&[], &[], &[], Status::Failed, None));
        assert!(has(&l, "L'annuaire des serveurs est injoignable"));
        assert!(l.hits.iter().any(|(_, h)| *h == Hit::Retry));
        let l = layout(&view(&[], &[], &[], Status::Ready, Some(&[])));
        assert!(has(&l, "Aucun serveur ne correspond"));
        // The whole screen eats presses.
        assert_eq!(l.hit([600.0, 350.0]), Some(Hit::Nothing));
    }

    #[test]
    fn the_switch_names_both_tabs() {
        let mut k: Kit<'_, Hit> = Kit::new(&test_measure, [-1.0, -1.0]);
        switch(&mut k, 640.0, Tab::Discover, "en_us");
        assert_eq!(k.hits.len(), 2);
        assert_eq!(k.hits[1].1, Hit::Tab(Tab::Discover));
        assert!(
            k.out
                .iter()
                .any(|p| matches!(p, Paint::Text { text, .. } if text == "My servers"))
        );
        let mut k: Kit<'_, Hit> = Kit::new(&test_measure, [-1.0, -1.0]);
        switch(&mut k, 640.0, Tab::MyServers, "fr_fr");
        assert!(
            k.out
                .iter()
                .any(|p| matches!(p, Paint::Text { text, .. } if text == "Découvrir"))
        );
        assert_eq!(hue_color(0.0, 1.0, 0.5), 0xFFFF_0000);
        assert_eq!(same_address("Play.Example.org:25565"), "play.example.org");
    }
}
