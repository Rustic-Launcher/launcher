//! The Rustic main menu, drawn over the title screen's panorama in place of the title's own
//! widgets (Horizon's `title_widgets_hidden` / `title_open` helpers, Rustic patch U13; launcher
//! ADR 0148). Pure geometry, as [`crate::overlay`]: the game crate draws the [`Draw`]s with
//! Horizon's HUD helpers (`fill`, `quad`, and `sprite` over the mod image for every picture and
//! every line of text, which the companion renders itself, smooth, in Inter) and hands clicks to
//! [`Layout::hit`] and keys to [`key`].
//!
//! The look follows the launcher and Lunar Client's organisation, not the game's widgets. Sizes
//! are given for a 1280x720 window and scale with the window ([`unit`]); the layout works in the
//! screen's own pixels and hands GUI pixels to the game. Layout:
//!
//! - top left: the account (the player's face and name; not a button);
//! - centre: the Rustic logo, Singleplayer and Multiplayer (400x40), then Realms and the Store
//!   (195x40 each, the Store in the accent);
//! - bottom centre: square icon buttons (36x36: Options, Language, Accessibility, Quit), the
//!   hovered or focused one's name above them;
//! - bottom right: the store's newest products (a card that turns every few seconds, with bars
//!   under it; hidden when there is nothing to show or no room), above the legal line ("Not
//!   affiliated with Mojang or Microsoft", always drawn: a press opens the credits, as vanilla's);
//! - bottom left: "Rustic · Horizon <version>" when it fits beside the legal line.

/// A button of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Singleplayer,
    Multiplayer,
    Realms,
    /// The centre's Store box: the website's store.
    Store,
    Options,
    Language,
    Accessibility,
    Quit,
    /// The store news card: the product it shows, on the website.
    StoreCard,
}

impl Button {
    /// The square icon buttons at the bottom.
    pub const ICONS: [Button; 4] = [
        Button::Options,
        Button::Language,
        Button::Accessibility,
        Button::Quit,
    ];

    /// The title button it presses (Horizon's `title_open` names), for the game's own ones.
    pub fn title_action(self) -> Option<&'static str> {
        Some(match self {
            Button::Singleplayer => "singleplayer",
            Button::Multiplayer => "multiplayer",
            Button::Realms => "realms",
            Button::Options => "options",
            Button::Language => "language",
            Button::Accessibility => "accessibility",
            Button::Quit => "quit",
            Button::Store | Button::StoreCard => return None,
        })
    }

    /// The game's translation key for the label of a game button and its English text when the
    /// key is missing; None for Rustic's own (the companion names them in French or English).
    pub fn label_key(self) -> Option<(&'static str, &'static str)> {
        Some(match self {
            Button::Singleplayer => ("menu.singleplayer", "Singleplayer"),
            Button::Multiplayer => ("menu.multiplayer", "Multiplayer"),
            Button::Realms => ("menu.online", "Realms"),
            Button::Options => ("menu.options", "Options"),
            Button::Language => ("options.language", "Language"),
            Button::Accessibility => ("options.accessibility", "Accessibility Settings"),
            Button::Quit => ("menu.quit", "Quit Game"),
            Button::Store | Button::StoreCard => return None,
        })
    }

    fn glyph(self) -> Glyph {
        match self {
            Button::Singleplayer => Glyph::Person,
            Button::Multiplayer => Glyph::People,
            Button::Realms => Glyph::Realms,
            Button::Store | Button::StoreCard => Glyph::Bag,
            Button::Options => Glyph::Gear,
            Button::Language => Glyph::Globe,
            Button::Accessibility => Glyph::Access,
            Button::Quit => Glyph::Power,
        }
    }
}

/// Rustic's own texts, in the game's language (French for `fr_*`, English otherwise).
pub fn rustic_text(key: RusticText, lang: &str) -> &'static str {
    let fr = lang.starts_with("fr");
    match (key, fr) {
        (RusticText::Store, true) => "Boutique",
        (RusticText::Store, false) => "Store",
        (RusticText::StoreNews, true) => "Nouveautés boutique",
        (RusticText::StoreNews, false) => "Store news",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RusticText {
    Store,
    StoreNews,
}

/// A translated label made short for a button: the game's "Options..." loses its dots and
/// "Minecraft Realms" the game's name (the service is Realms).
pub fn tidy_label(text: &str) -> String {
    let t = text.trim();
    let t = t.trim_end_matches("...").trim_end_matches('…').trim_end();
    let t = t.strip_prefix("Minecraft ").unwrap_or(t);
    t.to_owned()
}

/// An icon of the strip `mod/assets/rustic-icons.png` (128 px each, in this order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Glyph {
    Person,
    People,
    Realms,
    Bag,
    Gear,
    Globe,
    Access,
    Power,
    Switch,
    Crown,
    Sparkle,
    Paw,
    Smile,
    Tag,
    Shirt,
}

impl Glyph {
    /// Its place in the strip.
    pub fn index(self) -> u32 {
        self as u32
    }
    pub const SIDE: u32 = 128;
    pub const COUNT: u32 = 15;

    /// The icon of a cosmetic slot, as the website's store shows it (`SLOT_ICON`).
    pub fn of_slot(slot: &str) -> Self {
        match slot {
            "head" => Glyph::Crown,
            "back" => Glyph::Bag,
            "top" => Glyph::Shirt,
            "shoulder_companion" | "companion" => Glyph::Paw,
            "emote" => Glyph::Smile,
            "nametag" => Glyph::Tag,
            _ => Glyph::Sparkle,
        }
    }
}

/// The text's weight (Inter Medium or SemiBold).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Weight {
    Medium,
    SemiBold,
}

/// Inter's ascender and line height over its size: a line of `size` pixels is drawn in a box
/// `ceil(size * LINE)` high with its baseline `round(size * ASCENT)` from the top.
pub const ASCENT: f32 = 0.96875;
pub const LINE: f32 = 1.2109;

/// The height of a line of text of `size` pixels.
pub fn line_height(size: f32) -> f32 {
    (size * LINE).ceil()
}

/// A picture the menu shows, from the mod image the companion uploads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Image {
    Logo,
    /// A glyph in a colour (`0xRRGGBB`).
    Icon(Glyph, u32),
    /// A product's picture: a gradient of its hue (the website's `hueOf`) with its slot's glyph.
    Tile {
        hue: u16,
        glyph: Glyph,
    },
}

/// One thing to draw. Coordinates are GUI pixels; a text's `size` is in the screen's pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Fill {
        rect: [f32; 4],
        argb: u32,
    },
    /// A quadrilateral (a triangle when two corners are the same point).
    Quad {
        points: [[f32; 2]; 4],
        argb: u32,
    },
    /// A line of text, its box's top left at `at`.
    Text {
        at: [f32; 2],
        text: String,
        size: f32,
        weight: Weight,
        rgb: u32,
    },
    /// `image` stretched into `dst`, whose size is a whole number of the screen's pixels.
    Image {
        image: Image,
        dst: [f32; 4],
    },
}

/// What a press lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Button(Button),
    /// The legal line: vanilla's bottom notice, which opens the credits.
    Credits,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub draws: Vec<Draw>,
    /// In GUI pixels.
    pub hits: Vec<([f32; 4], Hit)>,
    /// The buttons shown, in keyboard order.
    pub order: Vec<Button>,
    /// The store card's product, when the card shows.
    pub card_item: Option<usize>,
}

impl Layout {
    /// What is under `at` (GUI pixels), if anything.
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, at))
            .map(|(_, h)| *h)
    }
}

fn inside(r: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= r[0] && at[0] < r[0] + r[2] && at[1] >= r[1] && at[1] < r[1] + r[3]
}

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// The launcher's dark theme (launcher/packages/ui/src/tokens/tokens.css).
pub mod colors {
    /// `--rl-fg`.
    pub const INK: u32 = 0xF3F1EE;
    /// `--rl-fg-muted`.
    pub const MUTED: u32 = 0xA29E98;
    /// `--rl-accent`, the logo's orange.
    pub const ACCENT: u32 = 0xFF6A00;
    /// `--rl-accent-text`.
    pub const ACCENT_TEXT: u32 = 0xFF8A3D;
    /// `--rl-danger`.
    pub const DANGER: u32 = 0xF0645A;
    /// A box: #121212 at 80 %.
    pub const FACE: u32 = 0xCC12_1212;
    /// A hovered or focused box.
    pub const FACE_HOVER: u32 = 0xE61E_1D1C;
    /// A box's border: white at 10 %.
    pub const BORDER: u32 = 0x1AFF_FFFF;
    /// The accent box (the Store): orange at 16 %, its border at 70 %.
    pub const ACCENT_FACE: u32 = 0x29FF_6A00;
    pub const ACCENT_FACE_HOVER: u32 = 0x4DFF_6A00;
    pub const ACCENT_BORDER: u32 = 0xB3FF_6A00;
    /// Over the whole panorama.
    pub const SHADE: u32 = 0x4805_0505;
}

/// The player's face, 8x8 ARGB pixels (the skin's face with its hat over it), row by row.
pub type Face = [u32; 64];

/// The face of a 64x64 (or legacy 64x32) RGBA skin: the face at (8, 8), the hat layer at (40, 8)
/// over it where the hat has a pixel.
pub fn skin_face(rgba: &[u8], width: u32, height: u32) -> Option<Face> {
    if width != 64 || (height != 64 && height != 32) || rgba.len() < (width * height * 4) as usize {
        return None;
    }
    let px = |x: u32, y: u32| {
        let i = ((y * width + x) * 4) as usize;
        let [r, g, b, a] = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
        u32::from_be_bytes([a, r, g, b])
    };
    let mut face = [0u32; 64];
    for y in 0..8 {
        for x in 0..8 {
            let base = px(8 + x, 8 + y) | 0xFF00_0000;
            let hat = px(40 + x, 8 + y);
            face[(y * 8 + x) as usize] = if hat >> 24 >= 0x80 {
                hat | 0xFF00_0000
            } else {
                base
            };
        }
    }
    Some(face)
}

/// A product of the store's catalogue, as the card shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreItem {
    pub slug: String,
    pub name: String,
    /// The API's own display string (`4,99 €`).
    pub price: String,
    /// The picture: its first item's hue and slot glyph, as the website draws products.
    pub hue: u16,
    pub glyph: Glyph,
}

/// The website's `hueOf` (apps/web/src/pages/store/lib.ts, the launcher's too): a cosmetic keeps
/// its colour everywhere.
pub fn hue_of(s: &str) -> u16 {
    let mut h: u32 = 0;
    for c in s.encode_utf16() {
        h = (h * 31 + u32::from(c)) % 360;
    }
    h as u16
}

/// The newest products of a `GET /v1/store/catalog` answer (Rustic Cloud), at most `max`: the
/// ones with a price, newest first. Slugs go into the store page's URL: letters, digits, `-`, `_`.
pub fn store_items(catalog: &serde_json::Value, max: usize) -> Vec<StoreItem> {
    let Some(products) = catalog.get("products").and_then(|p| p.as_array()) else {
        return Vec::new();
    };
    let mut items: Vec<(&str, StoreItem)> = products
        .iter()
        .filter_map(|p| {
            let slug = p.get("slug")?.as_str()?;
            let name = p.get("name")?.as_str()?;
            if slug.is_empty()
                || !slug
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return None;
            }
            let price = p.get("price")?.get("display")?.as_str()?.to_owned();
            let first = p
                .get("items")
                .and_then(|i| i.as_array())
                .and_then(|i| i.first());
            let item_slug = first
                .and_then(|i| i.get("slug"))
                .and_then(|s| s.as_str())
                .unwrap_or(slug);
            let slot = first
                .and_then(|i| i.get("slot"))
                .and_then(|s| s.as_str())
                .unwrap_or("");
            Some((
                p.get("created_at").and_then(|c| c.as_str()).unwrap_or(""),
                StoreItem {
                    slug: slug.to_owned(),
                    name: name.to_owned(),
                    price,
                    hue: hue_of(item_slug),
                    glyph: Glyph::of_slot(slot),
                },
            ))
        })
        .collect();
    // RFC 3339 times in UTC sort as text.
    items.sort_by(|a, b| b.0.cmp(a.0));
    items.into_iter().take(max).map(|(_, i)| i).collect()
}

/// A line's width: `(text, size in screen pixels, weight) -> screen pixels`.
pub type Measure<'a> = &'a dyn Fn(&str, f32, Weight) -> f32;

/// What the menu is drawn from.
pub struct View<'a> {
    /// GUI size.
    pub gui: [f32; 2],
    /// GUI scale (screen pixels per GUI pixel).
    pub scale: f32,
    /// The pointer, GUI pixels.
    pub mouse: [f32; 2],
    /// The button the keyboard is on.
    pub focus: Option<Button>,
    /// A button's label in the game's language.
    pub label: &'a dyn Fn(Button) -> String,
    pub measure: Measure<'a>,
    /// The signed-in player's name and face.
    pub player: Option<(&'a str, Option<&'a Face>)>,
    /// The store's newest products, and the card's title.
    pub store: &'a [StoreItem],
    pub store_title: &'a str,
    /// Seconds since the menu came up (the card turns every [`CARD_TURN`] seconds).
    pub time: f32,
    /// The logo's width over its height.
    pub logo_aspect: f32,
    /// "Rustic · Horizon 26.3".
    pub version: &'a str,
    /// "Not affiliated with Mojang or Microsoft", translated.
    pub legal: &'a str,
}

/// Seconds a product stays on the store card.
pub const CARD_TURN: f32 = 5.0;

/// The size of a 1280x720 design's pixel in this window's pixels: the window's smaller ratio to
/// 1280x720, kept between 0.6 and 2 (larger windows keep the 1440p sizes, which the mod image
/// holds).
pub fn unit(screen: [f32; 2]) -> f32 {
    (screen[0] / 1280.0).min(screen[1] / 720.0).clamp(0.6, 2.0)
}

/// How a box looks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Plain,
    Lit,
    Accent,
    AccentLit,
}

impl Look {
    fn of(accent: bool, lit: bool) -> Self {
        match (accent, lit) {
            (false, false) => Look::Plain,
            (false, true) => Look::Lit,
            (true, false) => Look::Accent,
            (true, true) => Look::AccentLit,
        }
    }
    fn colors(self) -> (u32, u32) {
        match self {
            Look::Plain => (colors::FACE, colors::BORDER),
            Look::Lit => (colors::FACE_HOVER, 0xFF00_0000 | colors::ACCENT),
            Look::Accent => (colors::ACCENT_FACE, colors::ACCENT_BORDER),
            Look::AccentLit => (colors::ACCENT_FACE_HOVER, 0xFF00_0000 | colors::ACCENT),
        }
    }
}

/// Points of a quarter circle around `c` from angle `a0` (radians, y down) over 90 degrees.
fn arc(c: [f32; 2], r: f32, a0: f32, steps: usize) -> Vec<[f32; 2]> {
    (0..=steps)
        .map(|i| {
            let a = a0 + std::f32::consts::FRAC_PI_2 * i as f32 / steps as f32;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// A box with round corners of radius `r` and a border `t` thick (screen pixels): the inside,
/// then the border around it, so that no pixel is covered twice (an even translucent colour).
/// The corners are fans of thin triangles, smooth at the screen's own pixels.
fn rounded_box(
    out: &mut Vec<Draw>,
    [x, y, w, h]: [f32; 4],
    r: f32,
    t: f32,
    fill: u32,
    border: u32,
) {
    let r = r.min(w / 2.0).min(h / 2.0).max(t);
    let ri = r - t;
    let steps = ((r / 2.0).ceil() as usize).clamp(3, 10);
    for rect in [
        [x + r, y + t, w - 2.0 * r, h - 2.0 * t],
        [x + t, y + r, r - t, h - 2.0 * r],
        [x + w - r, y + r, r - t, h - 2.0 * r],
    ] {
        if rect[2] > 0.0 && rect[3] > 0.0 {
            out.push(Draw::Fill { rect, argb: fill });
        }
    }
    // Corner centres with the angle each quarter starts at (y down).
    let corners = [
        ([x + r, y + r], std::f32::consts::PI),
        ([x + w - r, y + r], 1.5 * std::f32::consts::PI),
        ([x + w - r, y + h - r], 0.0),
        ([x + r, y + h - r], 0.5 * std::f32::consts::PI),
    ];
    for (c, a0) in corners {
        let inner = arc(c, ri, a0, steps);
        let outer = arc(c, r, a0, steps);
        for i in 0..steps {
            if ri > 0.0 {
                out.push(Draw::Quad {
                    points: [c, inner[i], inner[i + 1], inner[i + 1]],
                    argb: fill,
                });
            }
            out.push(Draw::Quad {
                points: [outer[i], outer[i + 1], inner[i + 1], inner[i]],
                argb: border,
            });
        }
    }
    for rect in [
        [x + r, y, w - 2.0 * r, t],
        [x + r, y + h - t, w - 2.0 * r, t],
        [x, y + r, t, h - 2.0 * r],
        [x + w - t, y + r, t, h - 2.0 * r],
    ] {
        if rect[2] > 0.0 && rect[3] > 0.0 {
            out.push(Draw::Fill { rect, argb: border });
        }
    }
}

/// `text` cut to `width` screen pixels with an ellipsis.
fn fit(text: &str, width: f32, size: f32, weight: Weight, measure: Measure<'_>) -> String {
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

/// Builds the menu in screen pixels; [`layout`] turns it into GUI pixels.
struct Builder<'a> {
    out: Vec<Draw>,
    hits: Vec<([f32; 4], Hit)>,
    measure: Measure<'a>,
}

impl Builder<'_> {
    fn text(&mut self, at: [f32; 2], text: &str, size: f32, weight: Weight, rgb: u32) -> f32 {
        let size = size.round().max(6.0);
        self.out.push(Draw::Text {
            at: [at[0].round(), at[1].round()],
            text: text.to_owned(),
            size,
            weight,
            rgb,
        });
        (self.measure)(text, size, weight)
    }
    /// A line of text centred vertically on `cy`.
    fn text_mid(&mut self, x: f32, cy: f32, text: &str, size: f32, weight: Weight, rgb: u32) {
        let size = size.round().max(6.0);
        let y = cy - line_height(size) / 2.0;
        self.text([x, y], text, size, weight, rgb);
    }
    fn image(&mut self, image: Image, rect: [f32; 4]) {
        self.out.push(Draw::Image {
            image,
            dst: [
                rect[0].round(),
                rect[1].round(),
                rect[2].round().max(1.0),
                rect[3].round().max(1.0),
            ],
        });
    }
    /// An icon `side` pixels square centred on `c`.
    fn icon(&mut self, glyph: Glyph, rgb: u32, c: [f32; 2], side: f32) {
        let side = side.round();
        self.image(
            Image::Icon(glyph, rgb),
            [c[0] - side / 2.0, c[1] - side / 2.0, side, side],
        );
    }
}

/// Lays the menu out for `v`.
pub fn layout(v: &View<'_>) -> Layout {
    let s = v.scale.max(1.0);
    let (w, h) = (v.gui[0] * s, v.gui[1] * s);
    let u = unit([w, h]);
    let mouse = [v.mouse[0] * s, v.mouse[1] * s];
    let px = |n: f32| (n * u).round();
    let mut b = Builder {
        out: Vec::new(),
        hits: Vec::new(),
        measure: v.measure,
    };
    let mut order = Vec::new();
    let lit = |button: Button, r: [f32; 4]| v.focus == Some(button) || inside(r, mouse);
    let (radius, t) = (px(8.0), 1.0);
    let margin = px(20.0);

    // The panorama, darker, and darker still towards the top and the bottom (a soft vignette,
    // one screen pixel a step).
    b.out.push(Draw::Fill {
        rect: [0.0, 0.0, w, h],
        argb: colors::SHADE,
    });
    let band = (h / 4.0).floor().max(1.0) as u32;
    for i in (0..band).step_by(2) {
        let alpha = (0x58 as f32 * (1.0 - i as f32 / band as f32)).round() as u32;
        if alpha == 0 {
            continue;
        }
        let y = i as f32;
        for rect in [[0.0, y, w, 2.0], [0.0, h - y - 2.0, w, 2.0]] {
            b.out.push(Draw::Fill {
                rect,
                argb: alpha << 24,
            });
        }
    }

    // Top left: the account (not a button).
    let top_h = px(36.0);
    if let Some((name, face)) = v.player {
        let size = px(14.0);
        let tw = (v.measure)(name, size, Weight::SemiBold).ceil();
        let head = px(20.0);
        let card = [
            margin,
            margin,
            px(10.0) + head + px(10.0) + tw + px(14.0),
            top_h,
        ];
        rounded_box(&mut b.out, card, radius, t, colors::FACE, colors::BORDER);
        // The face at a whole number of screen pixels per skin pixel.
        let p = (head / 8.0).floor().max(1.0);
        let (fx, fy) = (
            card[0] + px(10.0),
            (card[1] + (top_h - 8.0 * p) / 2.0).round(),
        );
        match face {
            Some(face) => {
                for (i, argb) in face.iter().enumerate() {
                    let (cx, cy) = ((i % 8) as f32, (i / 8) as f32);
                    b.out.push(Draw::Fill {
                        rect: [fx + cx * p, fy + cy * p, p, p],
                        argb: *argb,
                    });
                }
            }
            None => b.out.push(Draw::Fill {
                rect: [fx, fy, 8.0 * p, 8.0 * p],
                argb: 0xFF2A_2724,
            }),
        }
        b.text_mid(
            fx + 8.0 * p + px(10.0),
            card[1] + top_h / 2.0,
            name,
            size,
            Weight::SemiBold,
            colors::INK,
        );
    }

    // The footer: the legal line always, the edition's name when both fit.
    let small = px(12.0);
    let legal_w = (v.measure)(v.legal, small, Weight::Medium).ceil();
    let line = line_height(small.round().max(6.0));
    let fy = h - px(14.0) - line;
    let legal = [w - margin - legal_w, fy, legal_w, line];
    let over_legal = inside(legal, mouse);
    b.text(
        [legal[0], fy],
        v.legal,
        small,
        Weight::Medium,
        if over_legal {
            colors::INK
        } else {
            colors::MUTED
        },
    );
    if over_legal {
        b.out.push(Draw::Fill {
            rect: [legal[0], fy + line, legal_w, t],
            argb: 0xFF00_0000 | colors::INK,
        });
    }
    let version_w = (v.measure)(v.version, small, Weight::Medium).ceil();
    if margin + version_w + px(24.0) < legal[0] {
        b.text(
            [margin, fy],
            v.version,
            small,
            Weight::Medium,
            colors::MUTED,
        );
    }

    // The bottom row of square icon buttons.
    let square = px(36.0);
    let gap = px(10.0);
    let n = Button::ICONS.len() as f32;
    let row_w = n * square + (n - 1.0) * gap;
    let cx = (w / 2.0).round();
    let row_y = (h - px(24.0) - square).min(fy - px(12.0) - square);
    let row_x = (cx - row_w / 2.0).round();

    // The centre block: the logo, then the buttons, in the middle of the room between the top
    // row and the bottom row.
    let logo_h = px(56.0);
    let button_h = px(40.0);
    let bgap = px(8.0);
    let column_w = px(400.0).min(w - 2.0 * margin);
    let half = ((column_w - bgap) / 2.0).floor();
    let block_h = logo_h + px(32.0) + 3.0 * button_h + 2.0 * bgap;
    let top = margin + top_h + px(16.0);
    let bottom = row_y - px(16.0);
    let mut y = (top + (bottom - top - block_h).max(0.0) * 0.45).round();
    let logo_w = (logo_h * v.logo_aspect).round();
    let logo = [(cx - logo_w / 2.0).round(), y, logo_w, logo_h];
    b.image(Image::Logo, logo);
    y += logo_h + px(32.0);
    let x0 = (cx - column_w / 2.0).round();
    let label_size = px(15.0);
    let icon_side = px(18.0);
    let wide = [
        (Button::Singleplayer, [x0, y, column_w, button_h], false),
        (
            Button::Multiplayer,
            [x0, y + button_h + bgap, column_w, button_h],
            false,
        ),
        (
            Button::Realms,
            [x0, y + 2.0 * (button_h + bgap), half, button_h],
            false,
        ),
        (
            Button::Store,
            [
                x0 + column_w - half,
                y + 2.0 * (button_h + bgap),
                half,
                button_h,
            ],
            true,
        ),
    ];
    for (button, r, accent) in wide {
        let on = lit(button, r);
        let (fill, border) = Look::of(accent, on).colors();
        rounded_box(&mut b.out, r, radius, t, fill, border);
        let label = (v.label)(button);
        let tw = (v.measure)(&label, label_size.round(), Weight::SemiBold);
        let tint = if accent || on {
            colors::ACCENT
        } else {
            colors::INK
        };
        // Icon and label centred together.
        let group = icon_side + px(10.0) + tw;
        let gx = (r[0] + (r[2] - group) / 2.0).round();
        b.icon(
            button.glyph(),
            tint,
            [gx + icon_side / 2.0, r[1] + r[3] / 2.0],
            icon_side,
        );
        b.text_mid(
            gx + icon_side + px(10.0),
            r[1] + r[3] / 2.0,
            &label,
            label_size,
            Weight::SemiBold,
            if accent {
                colors::ACCENT_TEXT
            } else {
                colors::INK
            },
        );
        b.hits.push((r, Hit::Button(button)));
        order.push(button);
    }

    let mut tooltip = None;
    for (i, button) in Button::ICONS.into_iter().enumerate() {
        let r = [row_x + i as f32 * (square + gap), row_y, square, square];
        let on = lit(button, r);
        let (fill, border) = Look::of(false, on).colors();
        rounded_box(&mut b.out, r, radius, t, fill, border);
        let tint = match (on, button) {
            (true, _) => colors::ACCENT,
            (false, Button::Quit) => colors::DANGER,
            _ => colors::INK,
        };
        b.icon(
            button.glyph(),
            tint,
            [r[0] + square / 2.0, r[1] + square / 2.0],
            px(18.0),
        );
        if on && (inside(r, mouse) || tooltip.is_none()) {
            tooltip = Some((button, r[0] + square / 2.0, r[1]));
        }
        b.hits.push((r, Hit::Button(button)));
        order.push(button);
    }

    // The store card, above the legal line, when there is a product and room for it: the
    // product's picture, its name and price, and a bar per product under it.
    let mut card_item = None;
    if !v.store.is_empty() {
        let card_w = px(240.0).min((w * 0.3).floor());
        let card_h = px(84.0);
        let bars_h = px(3.0);
        let card = [
            w - margin - card_w,
            fy - px(12.0) - bars_h - px(8.0) - card_h,
            card_w,
            card_h,
        ];
        let bars_y = card[1] + card_h + px(8.0);
        let room = card_w >= px(180.0)
            && !overlaps(card, logo)
            && b.hits.iter().all(|(r, _)| !overlaps(card, *r))
            && card[1] > top;
        if room {
            let i = (v.time.max(0.0) / CARD_TURN) as usize % v.store.len();
            let item = &v.store[i];
            let on = lit(Button::StoreCard, card);
            let (fill, border) = Look::of(false, on).colors();
            rounded_box(&mut b.out, card, radius + px(2.0), t, fill, border);
            let pad = px(10.0);
            let pic = card_h - 2.0 * pad;
            b.image(
                Image::Tile {
                    hue: item.hue,
                    glyph: item.glyph,
                },
                [card[0] + pad, card[1] + pad, pic, pic],
            );
            let tx = card[0] + pad + pic + px(12.0);
            let inner = card[0] + card_w - pad - tx;
            let title_size = px(11.0);
            let name_size = px(15.0);
            let price_size = px(14.0);
            b.text(
                [tx, card[1] + pad],
                &fit(
                    v.store_title,
                    inner,
                    title_size.round(),
                    Weight::Medium,
                    v.measure,
                ),
                title_size,
                Weight::Medium,
                colors::MUTED,
            );
            b.text_mid(
                tx,
                card[1] + card_h / 2.0 + px(1.0),
                &fit(
                    &item.name,
                    inner,
                    name_size.round(),
                    Weight::SemiBold,
                    v.measure,
                ),
                name_size,
                Weight::SemiBold,
                colors::INK,
            );
            b.text(
                [
                    tx,
                    card[1] + card_h - pad - line_height(price_size.round().max(6.0)),
                ],
                &fit(
                    &item.price,
                    inner,
                    price_size.round(),
                    Weight::SemiBold,
                    v.measure,
                ),
                price_size,
                Weight::SemiBold,
                colors::ACCENT_TEXT,
            );
            // The carousel's bars, the shown one in the accent.
            let n = v.store.len().min(6);
            let bar_w = px(18.0);
            let bar_gap = px(5.0);
            let bars_w = n as f32 * bar_w + (n as f32 - 1.0) * bar_gap;
            let bx0 = (card[0] + (card_w - bars_w) / 2.0).round();
            for k in 0..n {
                let argb = if k == i % n {
                    0xFF00_0000 | colors::ACCENT
                } else {
                    0x59FF_FFFF
                };
                rounded_box(
                    &mut b.out,
                    [bx0 + k as f32 * (bar_w + bar_gap), bars_y, bar_w, bars_h],
                    bars_h / 2.0,
                    t,
                    argb,
                    argb,
                );
            }
            b.hits.push((card, Hit::Button(Button::StoreCard)));
            order.push(Button::StoreCard);
            card_item = Some(i);
        }
    }
    b.hits.push((legal, Hit::Credits));

    // The hovered or focused icon button's name above it, kept inside the window.
    if let Some((button, centre, above)) = tooltip {
        let text = (v.label)(button);
        let size = px(12.0);
        let tw = (v.measure)(&text, size.round(), Weight::Medium).ceil();
        let (bw, bh) = (tw + px(16.0), line_height(size.round()) + px(10.0));
        let bx = (centre - bw / 2.0)
            .round()
            .clamp(2.0, (w - bw - 2.0).max(2.0));
        let by = above - px(8.0) - bh;
        rounded_box(
            &mut b.out,
            [bx, by, bw, bh],
            px(6.0),
            t,
            0xF012_1212,
            colors::BORDER,
        );
        b.text_mid(
            bx + px(8.0),
            by + bh / 2.0,
            &text,
            size,
            Weight::Medium,
            colors::INK,
        );
    }

    // Screen pixels to GUI pixels.
    let g = |p: f32| p / s;
    let gr = |r: [f32; 4]| [g(r[0]), g(r[1]), g(r[2]), g(r[3])];
    let draws = b
        .out
        .into_iter()
        .map(|d| match d {
            Draw::Fill { rect, argb } => Draw::Fill {
                rect: gr(rect),
                argb,
            },
            Draw::Quad { points, argb } => Draw::Quad {
                points: points.map(|[x, y]| [g(x), g(y)]),
                argb,
            },
            Draw::Text {
                at,
                text,
                size,
                weight,
                rgb,
            } => Draw::Text {
                at: [g(at[0]), g(at[1])],
                text,
                size,
                weight,
                rgb,
            },
            Draw::Image { image, dst } => Draw::Image {
                image,
                dst: gr(dst),
            },
        })
        .collect();
    Layout {
        draws,
        hits: b.hits.into_iter().map(|(r, hit)| (gr(r), hit)).collect(),
        order,
        card_item,
    }
}

/// What a key does on the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyOutcome {
    /// The keyboard moves to this button.
    Focus(Button),
    /// This button is pressed.
    Press(Button),
    /// Not the menu's key: the game handles it (Escape does nothing on the title, as vanilla).
    Ignore,
}

/// A key press (`key.keyboard.*`, `shift` held) with the keyboard on `focus`, over the buttons
/// shown (`order`): Tab and the arrows walk them (wrapping), Enter and Space press the focused
/// one.
pub fn key(order: &[Button], focus: Option<Button>, name: &str, shift: bool) -> KeyOutcome {
    if order.is_empty() {
        return KeyOutcome::Ignore;
    }
    let at = focus.and_then(|f| order.iter().position(|b| *b == f));
    let step = |forward: bool| {
        let n = order.len();
        let i = match (at, forward) {
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
            (None, true) => 0,
            (None, false) => n - 1,
        };
        KeyOutcome::Focus(order[i])
    };
    match name {
        "key.keyboard.tab" => step(!shift),
        "key.keyboard.down" | "key.keyboard.right" => step(true),
        "key.keyboard.up" | "key.keyboard.left" => step(false),
        "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.space" => {
            match focus.filter(|f| order.contains(f)) {
                Some(b) => KeyOutcome::Press(b),
                None => KeyOutcome::Ignore,
            }
        }
        _ => KeyOutcome::Ignore,
    }
}

/// An RGBA image (not premultiplied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32) -> Self {
        Rgba {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }
    /// The part `[x, y, w, h]` of it.
    pub fn crop(&self, [x, y, w, h]: [u32; 4]) -> Rgba {
        let w = w.min(self.width.saturating_sub(x));
        let h = h.min(self.height.saturating_sub(y));
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for row in y..y + h {
            let start = ((row * self.width + x) * 4) as usize;
            pixels.extend_from_slice(&self.pixels[start..start + (w * 4) as usize]);
        }
        Rgba {
            width: w,
            height: h,
            pixels,
        }
    }
    /// Every pixel's colour replaced by `rgb` (`0xRRGGBB`), its alpha kept: a white glyph in a
    /// colour.
    pub fn tinted(mut self, rgb: u32) -> Rgba {
        let [_, r, g, b] = rgb.to_be_bytes();
        for p in self.pixels.as_chunks_mut::<4>().0 {
            p[0] = r;
            p[1] = g;
            p[2] = b;
        }
        self
    }
    /// `src` drawn over this image at `(x, y)` ("over" blending).
    pub fn draw_over(&mut self, src: &Rgba, x: u32, y: u32) {
        for sy in 0..src.height {
            for sx in 0..src.width {
                let (dx, dy) = (x + sx, y + sy);
                if dx >= self.width || dy >= self.height {
                    continue;
                }
                let si = ((sy * src.width + sx) * 4) as usize;
                let di = ((dy * self.width + dx) * 4) as usize;
                let sa = src.pixels[si + 3] as f32 / 255.0;
                let da = self.pixels[di + 3] as f32 / 255.0;
                let oa = sa + da * (1.0 - sa);
                if oa <= 0.0 {
                    continue;
                }
                for c in 0..3 {
                    let sc = src.pixels[si + c] as f32;
                    let dc = self.pixels[di + c] as f32;
                    self.pixels[di + c] = ((sc * sa + dc * da * (1.0 - sa)) / oa).round() as u8;
                }
                self.pixels[di + 3] = (oa * 255.0).round() as u8;
            }
        }
    }
}

/// HSL (degrees, 0-1, 0-1) to `[r, g, b]`.
fn hsl(h: f32, s: f32, l: f32) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

/// A product's picture, `side` pixels square: the website's colours for its hue
/// (`hsl(hue 55% 55%)` to `hsl(hue+40 60% 40%)`, corner to corner) in a box with round corners of
/// `radius`, its glyph (`glyph`, a white image) in the middle.
pub fn tile(hue: u16, side: u32, radius: f32, glyph: &Rgba) -> Rgba {
    let mut im = Rgba::new(side, side);
    let a = hsl(f32::from(hue), 0.55, 0.55);
    let b = hsl(f32::from(hue) + 40.0, 0.60, 0.40);
    let n = side as f32;
    for y in 0..side {
        for x in 0..side {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            // Coverage of the rounded square, anti-aliased over a pixel.
            let qx = (fx - n / 2.0).abs() - (n / 2.0 - radius);
            let qy = (fy - n / 2.0).abs() - (n / 2.0 - radius);
            let dist = (qx.max(0.0).hypot(qy.max(0.0))) + qx.max(qy).min(0.0) - radius;
            let cover = (0.5 - dist).clamp(0.0, 1.0);
            let k = (fx + fy) / (2.0 * n);
            let i = ((y * side + x) * 4) as usize;
            for c in 0..3 {
                im.pixels[i + c] = (a[c] + (b[c] - a[c]) * k).round().clamp(0.0, 255.0) as u8;
            }
            im.pixels[i + 3] = (cover * 255.0).round() as u8;
        }
    }
    let g = (side as f32 * 0.5).round() as u32;
    let icon = resample(glyph, g, g);
    im.draw_over(&icon, (side - g) / 2, (side - g) / 2);
    im
}

/// `src` scaled to `width` x `height` by area averaging (premultiplied, so transparent pixels
/// do not darken the edges): sharp at any size, as the logo and icons must be at each scale.
pub fn resample(src: &Rgba, width: u32, height: u32) -> Rgba {
    let (width, height) = (width.max(1), height.max(1));
    let (sw, sh) = (src.width as f64, src.height as f64);
    let (fx, fy) = (sw / width as f64, sh / height as f64);
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for oy in 0..height {
        let (y0, y1) = (oy as f64 * fy, (oy + 1) as f64 * fy);
        for ox in 0..width {
            let (x0, x1) = (ox as f64 * fx, (ox + 1) as f64 * fx);
            let mut acc = [0.0f64; 4];
            let mut area = 0.0;
            let mut sy = y0.floor() as u32;
            while (sy as f64) < y1 && sy < src.height {
                let cy = (y1.min(sy as f64 + 1.0) - y0.max(sy as f64)).max(0.0);
                let mut sx = x0.floor() as u32;
                while (sx as f64) < x1 && sx < src.width {
                    let cx = (x1.min(sx as f64 + 1.0) - x0.max(sx as f64)).max(0.0);
                    let wgt = cx * cy;
                    let i = ((sy * src.width + sx) * 4) as usize;
                    let a = src.pixels[i + 3] as f64 / 255.0;
                    for (sum, channel) in acc.iter_mut().zip(&src.pixels[i..i + 3]) {
                        *sum += *channel as f64 * a * wgt;
                    }
                    acc[3] += a * wgt;
                    area += wgt;
                    sx += 1;
                }
                sy += 1;
            }
            let o = ((oy * width + ox) * 4) as usize;
            if area > 0.0 && acc[3] > 0.0 {
                let a = acc[3] / area;
                for (out, sum) in pixels[o..o + 3].iter_mut().zip(&acc) {
                    *out = (sum / acc[3]).round().clamp(0.0, 255.0) as u8;
                }
                pixels[o + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Rgba {
        width,
        height,
        pixels,
    }
}

/// Images packed into one picture no wider than `max_w` (shelves, left to right, tallest first
/// for a tight fit): the picture and where each one went (`[x, y, w, h]`, in the given order),
/// None when they do not fit in `max_w` x `max_h`.
pub fn pack(images: &[Rgba], max_w: u32, max_h: u32) -> Option<(Rgba, Vec<[u32; 4]>)> {
    let mut by_height: Vec<usize> = (0..images.len()).collect();
    by_height.sort_by_key(|&i| std::cmp::Reverse(images[i].height));
    let mut places = vec![[0u32; 4]; images.len()];
    let (mut x, mut y, mut shelf, mut width) = (0u32, 0u32, 0u32, 0u32);
    for &i in &by_height {
        let im = &images[i];
        if im.width > max_w {
            return None;
        }
        if x + im.width > max_w {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        places[i] = [x, y, im.width, im.height];
        x += im.width;
        width = width.max(x);
        shelf = shelf.max(im.height);
    }
    let height = y + shelf;
    if height > max_h || width == 0 || height == 0 {
        return None;
    }
    let mut picture = Rgba::new(width, height);
    for (im, [px, py, w, _]) in images.iter().zip(&places) {
        for row in 0..im.height {
            let src = (row * im.width * 4) as usize;
            let dst = (((py + row) * width + px) * 4) as usize;
            picture.pixels[dst..dst + (w * 4) as usize]
                .copy_from_slice(&im.pixels[src..src + (im.width * 4) as usize]);
        }
    }
    Some((picture, places))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(b: Button) -> String {
        match b.label_key() {
            Some((_, en)) => en.to_owned(),
            None => "Store".into(),
        }
    }
    /// Inter is about 0.55 em a character.
    fn measure(t: &str, size: f32, _: Weight) -> f32 {
        t.chars().count() as f32 * size * 0.55
    }
    fn items() -> Vec<StoreItem> {
        (0..3)
            .map(|i| StoreItem {
                slug: format!("item-{i}"),
                name: format!("Ember Wings {i}"),
                price: "4,99 €".into(),
                hue: 20,
                glyph: Glyph::Sparkle,
            })
            .collect()
    }
    fn view<'a>(window: [f32; 2], scale: f32, store: &'a [StoreItem]) -> View<'a> {
        View {
            gui: [window[0] / scale, window[1] / scale],
            scale,
            mouse: [-1.0, -1.0],
            focus: None,
            label: &label,
            measure: &measure,
            player: Some(("Steve", None)),
            store,
            store_title: "Store news",
            time: 0.0,
            logo_aspect: 1000.0 / 256.0,
            version: "Rustic · Horizon 26.3",
            legal: "Not affiliated with Mojang or Microsoft",
        }
    }
    fn hit_rect(l: &Layout, b: Button) -> Option<[f32; 4]> {
        l.hits
            .iter()
            .find(|(_, h)| *h == Hit::Button(b))
            .map(|(r, _)| *r)
    }
    /// Windows and GUI scales the game picks (854x480 at 2, 1280x720 at 3, 1920x1080 at 4...).
    const WINDOWS: [([f32; 2], f32); 7] = [
        ([854.0, 480.0], 2.0),
        ([1280.0, 720.0], 3.0),
        ([1280.0, 720.0], 2.0),
        ([1366.0, 768.0], 3.0),
        ([1920.0, 1080.0], 4.0),
        ([1920.0, 1009.0], 4.0),
        ([2560.0, 1440.0], 4.0),
    ];

    #[test]
    fn every_button_is_reachable_and_nothing_overlaps_from_small_to_large_windows() {
        let store = items();
        for (window, scale) in WINDOWS {
            let l = layout(&view(window, scale, &store));
            let gui = [window[0] / scale, window[1] / scale];
            for b in [
                Button::Singleplayer,
                Button::Multiplayer,
                Button::Realms,
                Button::Store,
                Button::Options,
                Button::Language,
                Button::Accessibility,
                Button::Quit,
            ] {
                let r = hit_rect(&l, b).unwrap_or_else(|| panic!("{b:?} in {window:?}"));
                assert!(r[0] >= 0.0 && r[1] >= 0.0, "{b:?} at {r:?} in {window:?}");
                assert!(r[0] + r[2] <= gui[0] && r[1] + r[3] <= gui[1]);
                assert_eq!(l.hit([r[0] + 1.0, r[1] + 1.0]), Some(Hit::Button(b)));
                assert!(l.order.contains(&b));
            }
            for (i, (a, _)) in l.hits.iter().enumerate() {
                for (b, _) in &l.hits[i + 1..] {
                    assert!(!overlaps(*a, *b), "{a:?} {b:?} in {window:?}");
                }
            }
            let logo = l
                .draws
                .iter()
                .find_map(|d| match d {
                    Draw::Image {
                        image: Image::Logo,
                        dst,
                    } => Some(*dst),
                    _ => None,
                })
                .unwrap();
            for (r, _) in &l.hits {
                assert!(!overlaps(logo, *r), "logo over {r:?} in {window:?}");
            }
            let legal = l.hits.iter().find(|(_, h)| *h == Hit::Credits).unwrap().0;
            assert!(legal[0] >= 0.0 && legal[1] + legal[3] <= gui[1] + 0.01);
        }
    }

    #[test]
    fn buttons_have_the_reference_sizes_at_1280x720() {
        let l = layout(&view([1280.0, 720.0], 2.0, &[]));
        let px = |r: [f32; 4]| [r[2] * 2.0, r[3] * 2.0];
        assert_eq!(
            px(hit_rect(&l, Button::Singleplayer).unwrap()),
            [400.0, 40.0]
        );
        assert_eq!(px(hit_rect(&l, Button::Store).unwrap()), [196.0, 40.0]);
        assert_eq!(px(hit_rect(&l, Button::Quit).unwrap()), [36.0, 36.0]);
        // And proportionally smaller in 854x480.
        let small = layout(&view([854.0, 480.0], 2.0, &[]));
        let w = hit_rect(&small, Button::Singleplayer).unwrap()[2] * 2.0;
        assert!((w - 400.0 * 480.0 / 720.0).abs() <= 1.0, "{w}");
    }

    #[test]
    fn the_store_card_shows_when_there_is_room_and_turns() {
        let store = items();
        let l = layout(&view([1280.0, 720.0], 3.0, &store));
        assert_eq!(l.card_item, Some(0));
        assert!(hit_rect(&l, Button::StoreCard).is_some());
        assert!(l.draws.iter().any(|d| matches!(
            d,
            Draw::Image {
                image: Image::Tile { .. },
                ..
            }
        )));
        let mut v = view([1280.0, 720.0], 3.0, &store);
        v.time = CARD_TURN * 4.0 + 0.1;
        assert_eq!(layout(&v).card_item, Some(1));
        assert_eq!(layout(&view([1280.0, 720.0], 3.0, &[])).card_item, None);
        // A window too narrow for it: no card (and nothing overlaps, checked above).
        assert_eq!(layout(&view([340.0, 400.0], 1.0, &store)).card_item, None);
    }

    #[test]
    fn the_edition_name_gives_way_to_the_legal_line_on_narrow_windows() {
        let has_version = |window| {
            layout(&view(window, 2.0, &[]))
                .draws
                .iter()
                .any(|d| matches!(d, Draw::Text { text, .. } if text.starts_with("Rustic ·")))
        };
        assert!(has_version([1280.0, 720.0]));
        assert!(!has_version([260.0, 400.0]));
    }

    #[test]
    fn hover_and_focus_light_a_button_in_the_accent() {
        let accent_ring = |l: &Layout| {
            l.draws.iter().any(
                |d| matches!(d, Draw::Fill { argb, .. } if *argb == 0xFF00_0000 | colors::ACCENT),
            )
        };
        let base = layout(&view([1280.0, 720.0], 3.0, &[]));
        assert!(!accent_ring(&base));
        let quit = hit_rect(&base, Button::Quit).unwrap();
        let mut v = view([1280.0, 720.0], 3.0, &[]);
        v.mouse = [quit[0] + 2.0, quit[1] + 2.0];
        let hovered = layout(&v);
        assert!(accent_ring(&hovered));
        // The hovered icon's name shows above it.
        assert!(hovered.draws.iter().any(
            |d| matches!(d, Draw::Text { text, at, .. } if text == "Quit Game" && at[1] < quit[1])
        ));
        let mut v = view([1280.0, 720.0], 3.0, &[]);
        v.focus = Some(Button::Singleplayer);
        assert!(accent_ring(&layout(&v)));
    }

    #[test]
    fn rounded_boxes_cover_their_corners_smoothly_without_overlap() {
        let mut out = Vec::new();
        rounded_box(&mut out, [10.0, 10.0, 80.0, 40.0], 8.0, 1.0, 1, 2);
        let rects: Vec<[f32; 4]> = out
            .iter()
            .filter_map(|d| match d {
                Draw::Fill { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                assert!(!overlaps(*a, *b), "{a:?} {b:?}");
            }
        }
        for d in &out {
            if let Draw::Quad { points, .. } = d {
                for [x, y] in points {
                    assert!((10.0..=90.0).contains(x) && (10.0..=50.0).contains(y));
                }
            }
        }
    }

    #[test]
    fn pictures_and_text_sit_on_whole_screen_pixels() {
        let store = items();
        for (window, scale) in WINDOWS {
            for d in layout(&view(window, scale, &store)).draws {
                let coords: Vec<f32> = match &d {
                    Draw::Image { dst, .. } => dst.to_vec(),
                    Draw::Text { at, .. } => at.to_vec(),
                    _ => continue,
                };
                for c in coords {
                    let px = c * scale;
                    assert!((px - px.round()).abs() < 1e-2, "{d:?} at {scale}");
                }
            }
        }
    }

    #[test]
    fn keys_walk_the_buttons_shown_and_press_the_focused_one() {
        let order = layout(&view([1280.0, 720.0], 3.0, &[])).order;
        assert_eq!(order[0], Button::Singleplayer);
        assert_eq!(
            key(&order, None, "key.keyboard.tab", false),
            KeyOutcome::Focus(Button::Singleplayer)
        );
        assert_eq!(
            key(&order, None, "key.keyboard.tab", true),
            KeyOutcome::Focus(Button::Quit)
        );
        assert_eq!(
            key(&order, Some(Button::Quit), "key.keyboard.tab", false),
            KeyOutcome::Focus(Button::Singleplayer)
        );
        assert_eq!(
            key(&order, Some(Button::Realms), "key.keyboard.down", false),
            KeyOutcome::Focus(Button::Store)
        );
        assert_eq!(
            key(&order, Some(Button::Language), "key.keyboard.enter", false),
            KeyOutcome::Press(Button::Language)
        );
        assert_eq!(
            key(&order, None, "key.keyboard.enter", false),
            KeyOutcome::Ignore
        );
        // A button no longer shown (the card went away) is not pressed.
        assert_eq!(
            key(&order, Some(Button::StoreCard), "key.keyboard.enter", false),
            KeyOutcome::Ignore
        );
        assert_eq!(
            key(&order, Some(Button::Quit), "key.keyboard.escape", false),
            KeyOutcome::Ignore
        );
    }

    #[test]
    fn labels_are_tidied_for_buttons() {
        assert_eq!(tidy_label("Options..."), "Options");
        assert_eq!(tidy_label("Langue…"), "Langue");
        assert_eq!(tidy_label("Minecraft Realms"), "Realms");
        assert_eq!(tidy_label("Solo"), "Solo");
        assert_eq!(rustic_text(RusticText::Store, "fr_fr"), "Boutique");
        assert_eq!(rustic_text(RusticText::Store, "en_us"), "Store");
    }

    #[test]
    fn the_store_card_takes_the_newest_priced_products_with_safe_slugs() {
        let catalog = serde_json::json!({"products": [
            {"slug": "old", "name": "Old", "created_at": "2026-10-01T00:00:00Z", "price": {"display": "1,99 €"},
             "items": [{"slug": "copper-top-hat", "slot": "head"}]},
            {"slug": "new", "name": "New", "created_at": "2026-11-10T00:00:00Z", "price": {"display": "4,99 €"}, "items": []},
            {"slug": "free", "name": "No price", "created_at": "2026-11-11T00:00:00Z", "price": null},
            {"slug": "../evil", "name": "Bad", "created_at": "2026-11-12T00:00:00Z", "price": {"display": "1 €"}}
        ]});
        let items = store_items(&catalog, 5);
        let slugs: Vec<_> = items.iter().map(|i| i.slug.as_str()).collect();
        assert_eq!(slugs, ["new", "old"]);
        assert_eq!(items[0].price, "4,99 €");
        assert_eq!(items[1].glyph, Glyph::Crown);
        assert_eq!(items[1].hue, hue_of("copper-top-hat"));
        assert!(store_items(&serde_json::json!({}), 5).is_empty());
        assert_eq!(store_items(&catalog, 1).len(), 1);
    }

    #[test]
    fn hues_match_the_websites() {
        // hueOf('ab') = ((0*31 + 97) % 360 * 31 + 98) % 360 = 3105 % 360 = 225.
        assert_eq!(hue_of("ab"), 225);
        assert_eq!(hue_of(""), 0);
    }

    #[test]
    fn long_texts_are_cut_to_fit() {
        assert_eq!(fit("Short", 60.0, 10.0, Weight::Medium, &measure), "Short");
        let cut = fit(
            "A very long product name",
            60.0,
            10.0,
            Weight::Medium,
            &measure,
        );
        assert!(
            measure(&cut, 10.0, Weight::Medium) <= 60.0 && cut.ends_with('…'),
            "{cut}"
        );
    }

    #[test]
    fn a_skins_face_takes_its_hat_over_it() {
        let mut skin = vec![0u8; 64 * 64 * 4];
        let set = |s: &mut Vec<u8>, x: usize, y: usize, c: [u8; 4]| {
            let i = (y * 64 + x) * 4;
            s[i..i + 4].copy_from_slice(&c);
        };
        set(&mut skin, 8, 8, [10, 20, 30, 255]);
        set(&mut skin, 9, 8, [10, 20, 30, 255]);
        set(&mut skin, 41, 8, [200, 100, 50, 255]);
        let face = skin_face(&skin, 64, 64).unwrap();
        assert_eq!(face[0], 0xFF0A_141E);
        assert_eq!(face[1], 0xFFC8_6432);
        assert_eq!(skin_face(&skin, 32, 32), None);
        assert!(skin_face(&skin[..64 * 32 * 4], 64, 32).is_some());
    }

    #[test]
    fn resampling_averages_areas_and_keeps_edges_clean() {
        let mut px = Vec::new();
        for _ in 0..2 {
            for x in 0..4 {
                px.extend_from_slice(if x < 2 {
                    &[255, 0, 0, 255]
                } else {
                    &[0, 0, 0, 0]
                });
            }
        }
        let src = Rgba {
            width: 4,
            height: 2,
            pixels: px,
        };
        assert_eq!(resample(&src, 1, 1).pixels, vec![255, 0, 0, 128]);
        assert_eq!(
            resample(&src, 2, 1).pixels,
            vec![255, 0, 0, 255, 0, 0, 0, 0]
        );
        assert_eq!(resample(&src, 4, 2), src);
        assert_eq!(src.crop([2, 0, 2, 2]).pixels, vec![0; 16]);
        assert_eq!(
            src.crop([0, 0, 1, 1]).tinted(0x00FF00).pixels,
            vec![0, 255, 0, 255]
        );
    }

    #[test]
    fn a_tile_is_rounded_coloured_and_carries_its_glyph() {
        let glyph = Rgba {
            width: 2,
            height: 2,
            pixels: vec![255; 16],
        };
        let t = tile(0, 40, 8.0, &glyph);
        let at = |x: u32, y: u32| {
            let i = ((y * 40 + x) * 4) as usize;
            [
                t.pixels[i],
                t.pixels[i + 1],
                t.pixels[i + 2],
                t.pixels[i + 3],
            ]
        };
        assert_eq!(at(0, 0)[3], 0, "the corner is cut");
        assert_eq!(at(20, 2)[3], 255);
        assert_eq!(at(20, 20), [255, 255, 255, 255], "the glyph in the middle");
        // Hue 0: reddish at the top left.
        let c = at(4, 10);
        assert!(c[0] > c[1] && c[0] > c[2], "{c:?}");
    }

    #[test]
    fn images_pack_on_shelves() {
        let im = |w, h| Rgba {
            width: w,
            height: h,
            pixels: vec![7; (w * h * 4) as usize],
        };
        let (picture, places) = pack(&[im(5, 2), im(6, 4), im(3, 3)], 12, 12).unwrap();
        // Tallest first: the 6x4, the 3x3 beside it, the 5x2 on the next shelf.
        assert_eq!(places, vec![[0, 4, 5, 2], [0, 0, 6, 4], [6, 0, 3, 3]]);
        assert_eq!((picture.width, picture.height), (9, 6));
        assert!(pack(&[im(20, 2)], 12, 12).is_none());
        assert!(pack(&[im(10, 10), im(10, 10)], 12, 12).is_none());
    }
}
