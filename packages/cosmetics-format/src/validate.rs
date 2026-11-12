//! Validation: structure, budgets and the fair-play rules (no capes, no gameplay, nothing
//! random). Returns every problem found, not just the first, so an author fixes them in one go.
use crate::anim;
use crate::bake::{self, Baked, BakedPart};
use crate::math::{self, Mat4};
use crate::model::*;
use crate::texture::Texture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The budgets every cosmetic stays within. The box-model ones are Horizon's (its cosmetics
/// are drawn through the skin atlas's 64x64 slots), so a box cosmetic that passes here also
/// passes Horizon's `Item::parse`.
pub mod budget {
    /// Vertices of a box model (4 per face, 24 per box): Horizon's limit.
    pub const MAX_VERTICES: usize = 512;
    /// Triangles of a mesh model (each drawn as a quad).
    pub const MAX_TRIANGLES: usize = 512;
    pub const MAX_PARTS: usize = 16;
    /// Largest texture side; sides are powers of two.
    pub const MAX_TEXTURE: u32 = 64;
    /// A name tag badge is square, at most this side.
    pub const MAX_BADGE: u32 = 16;
    /// The PNG file itself.
    pub const MAX_PNG_BYTES: usize = 64 * 1024;
    /// A whole self-contained bundle (JSON with the texture and triangles inline).
    pub const MAX_BUNDLE_BYTES: usize = 256 * 1024;
    /// Pixels from the bone's pivot, animations included.
    pub const MAX_REACH: f32 = 24.0;
    pub const MAX_COMPANION_REACH: f32 = 8.0;
    pub const MAX_COMPANION_ANCHOR: f32 = 28.0;
    /// The head's centre in body space and the clearance a companion keeps from it.
    pub const HEAD: [f32; 3] = [0.0, -4.0, 0.0];
    pub const HEAD_CLEARANCE: f32 = 11.0;
    pub const MAX_LAG: f32 = 40.0;
    pub const MAX_TRAIL_RATE: f32 = 4.0;
    pub const MAX_TRAIL_LIFETIME: u32 = 40;
    pub const MAX_PARTICLE_SIZE: f32 = 4.0;
    pub const MAX_TRAIL_SPRITES: u32 = 8;
    pub const MAX_SPRITE: u32 = 16;
    pub const MAX_AURA_RADIUS: f32 = 14.0;
    pub const MAX_EMOTE_LENGTH: f32 = 200.0;
    pub const MAX_TRACKS: usize = 16;
    pub const MAX_KEYS: usize = 32;
    pub const MAX_ALL_KEYS: usize = 192;
    pub const MAX_TURN: f32 = 360.0;
    pub const MAX_ROOT_TURN: f32 = 720.0;
    pub const MAX_MOVE: f32 = 8.0;
    pub const MAX_ROOT_MOVE: f32 = 16.0;
}

/// What a problem is about, for tools and the TS side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    Format,
    Id,
    Version,
    Name,
    Category,
    Attachment,
    Texture,
    Model,
    Budget,
    Reach,
    Animation,
    Emote,
    Fx,
    Nametag,
    /// Looks or acts like a cape: refused (capes are Mojang's).
    CapeLike,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Issue {
    pub code: Code,
    /// A JSON-pointer-like place (`model.parts[2].cubes[0]`).
    pub at: String,
    pub message: String,
}

/// What validation found, with the numbers a budget panel shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub stats: Stats,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Stats {
    pub vertices: usize,
    pub triangles: usize,
    pub parts: usize,
    pub texture: Option<[u32; 2]>,
    /// Pixels from the bone's pivot (or the origin), animations included.
    pub reach: f32,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    fn error(&mut self, code: Code, at: impl Into<String>, message: impl Into<String>) {
        self.errors.push(Issue {
            code,
            at: at.into(),
            message: message.into(),
        });
    }

    fn warn(&mut self, code: Code, at: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(Issue {
            code,
            at: at.into(),
            message: message.into(),
        });
    }
}

fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Validates `cosmetic` with its decoded texture (when it has one) and the PNG's size in bytes.
pub fn validate(cosmetic: &Cosmetic, texture: Option<&Texture>, png_bytes: usize) -> Report {
    let mut r = Report::default();
    let c = cosmetic;
    if c.format != FORMAT {
        r.error(
            Code::Format,
            "format",
            format!("format {}, this reader knows {FORMAT}", c.format),
        );
    }
    if !valid_id(&c.id) {
        r.error(Code::Id, "id", "the id is 1 to 64 of a-z, 0-9 and _");
    }
    if !semver_like(&c.version) {
        r.error(
            Code::Version,
            "version",
            "the version is MAJOR.MINOR.PATCH (semver)",
        );
    }
    if c.name.trim().is_empty() || c.name.chars().count() > 64 {
        r.error(Code::Name, "name", "the name is 1 to 64 characters");
    }
    cape_words(c, &mut r);
    // Which blocks each category needs or forbids.
    let cat = c.category;
    let has = |b: bool, what: &str, r: &mut Report, need: bool| {
        if b != need {
            let verb = if need { "needs" } else { "has no" };
            r.error(
                Code::Category,
                what,
                format!("a {} cosmetic {verb} `{what}`", cat.as_str()),
            );
        }
    };
    has(c.emote.is_some(), "emote", &mut r, cat == Category::Emote);
    has(
        c.companion.is_some(),
        "companion",
        &mut r,
        cat == Category::Companion,
    );
    has(c.trail.is_some(), "trail", &mut r, cat == Category::Trail);
    has(c.aura.is_some(), "aura", &mut r, cat == Category::Aura);
    has(
        c.nametag.is_some(),
        "nametag",
        &mut r,
        cat == Category::Nametag,
    );
    if cat.on_bone() {
        has(c.attachment.is_some(), "attachment", &mut r, true);
    } else if !cat.rides_bone() && c.attachment.is_some() {
        r.error(
            Code::Attachment,
            "attachment",
            "a companion, a trail or an aura rides no bone",
        );
    } else if cat == Category::Nametag && c.attachment.is_some() {
        r.error(Code::Attachment, "attachment", "a name tag rides no bone");
    }
    let needs_model = matches!(
        cat,
        Category::Head
            | Category::Back
            | Category::Top
            | Category::ShoulderCompanion
            | Category::Companion
            | Category::Aura
    );
    let no_model = matches!(cat, Category::Trail | Category::Nametag);
    if needs_model && c.model.is_none() {
        r.error(Code::Model, "model", "this category needs a model");
    }
    if no_model && c.model.is_some() {
        r.error(Code::Model, "model", "a trail or a name tag has no model");
    }
    if no_model && !c.loops.is_empty() {
        r.error(
            Code::Animation,
            "loops",
            "a trail or a name tag has no animations",
        );
    }
    if let Some(a) = &c.attachment {
        if !finite(&a.offset) || !finite(&a.rotation) {
            r.error(Code::Attachment, "attachment", "not a number");
        }
        if !(a.scale > 0.0 && a.scale <= 2.0) {
            r.error(
                Code::Attachment,
                "attachment.scale",
                "the scale is above 0 and at most 2",
            );
        }
    }
    // Texture.
    let emote_without_props = cat == Category::Emote && c.model.is_none();
    match (texture, &c.texture) {
        (None, Some(_)) => r.error(Code::Texture, "texture", "the texture could not be read"),
        (None, None) if !emote_without_props => {
            r.error(Code::Texture, "texture", "this cosmetic needs a texture")
        }
        (Some(t), _) => {
            r.stats.texture = Some([t.width, t.height]);
            if !t.width.is_power_of_two() || !t.height.is_power_of_two() {
                r.error(
                    Code::Texture,
                    "texture",
                    format!("{}x{}: each side is a power of two", t.width, t.height),
                );
            }
            if t.width > budget::MAX_TEXTURE || t.height > budget::MAX_TEXTURE {
                r.error(
                    Code::Budget,
                    "texture",
                    format!(
                        "{}x{}, at most {m}x{m}",
                        t.width,
                        t.height,
                        m = budget::MAX_TEXTURE
                    ),
                );
            }
            if png_bytes > budget::MAX_PNG_BYTES {
                r.error(
                    Code::Budget,
                    "texture",
                    format!(
                        "the PNG is {png_bytes} bytes, at most {}",
                        budget::MAX_PNG_BYTES
                    ),
                );
            }
            if t.used() == 0 {
                r.warn(Code::Texture, "texture", "the texture is fully transparent");
            }
            if cat == Category::Nametag && (t.width != t.height || t.width > budget::MAX_BADGE) {
                r.error(
                    Code::Nametag,
                    "texture",
                    format!(
                        "a name tag's badge is square, at most {m}x{m}",
                        m = budget::MAX_BADGE
                    ),
                );
            }
        }
        _ => {}
    }
    // Model structure.
    let mut names: Vec<String> = Vec::new();
    let size = texture.map_or([64.0, 64.0], |t| [t.width as f32, t.height as f32]);
    match &c.model {
        Some(Model::Cubes { parts }) => {
            for (i, p) in parts.iter().enumerate() {
                check_part(
                    p,
                    &format!("model.parts[{i}]"),
                    size,
                    true,
                    cat,
                    &mut names,
                    &mut r,
                );
            }
        }
        Some(Model::Mesh(m)) => {
            if m.groups.is_empty() {
                r.error(
                    Code::Model,
                    "model.groups",
                    "a mesh has no triangles yet: run the pipeline on its `source`",
                );
            }
            let tris: usize = m.groups.iter().map(|g| g.triangles.len()).sum();
            r.stats.triangles = tris;
            if tris > budget::MAX_TRIANGLES {
                r.error(
                    Code::Budget,
                    "model",
                    format!("{tris} triangles, at most {}", budget::MAX_TRIANGLES),
                );
            }
            for (i, g) in m.groups.iter().enumerate() {
                if g.name.is_empty() || g.name.starts_with("cosmetic") || names.contains(&g.name) {
                    r.error(
                        Code::Model,
                        format!("model.groups[{i}].name"),
                        "group names are unique and never start with `cosmetic`",
                    );
                }
                names.push(g.name.clone());
                if !g.triangles.iter().flatten().all(|v| finite(v)) {
                    r.error(Code::Model, format!("model.groups[{i}]"), "not a number");
                }
            }
        }
        None => {}
    }
    r.stats.parts = names.len();
    if names.len() > budget::MAX_PARTS {
        r.error(
            Code::Budget,
            "model",
            format!("{} parts, at most {}", names.len(), budget::MAX_PARTS),
        );
    }
    // Loops.
    for (i, a) in c.loops.iter().enumerate() {
        let at = format!("loops[{i}]");
        if !finite(&[a.amplitude, a.period, a.phase]) {
            r.error(Code::Animation, &at, "not a number");
            continue;
        }
        if !names.contains(&a.part) {
            r.error(Code::Animation, &at, format!("moves no part {:?}", a.part));
        }
        let limit = match a.motion {
            LoopMotion::Swing => 45.0,
            LoopMotion::Bob => 4.0,
            LoopMotion::Spin => 0.0,
        };
        if a.amplitude.abs() > limit || !(4.0..=1200.0).contains(&a.period) {
            r.error(
                Code::Animation,
                &at,
                "swing at most 45 degrees, bob at most 4 pixels, a period of 4 to 1200 ticks",
            );
        }
    }
    if let Some(e) = &c.emote {
        check_emote(e, &names, &mut r);
    }
    if let Some(t) = &c.trail {
        check_trail(t, texture, &mut r);
    }
    if let Some(n) = &c.nametag
        && parse_colour(&n.line).is_none()
    {
        r.error(Code::Nametag, "nametag.line", "the line colour is #rrggbb");
    }
    // Geometry: budgets and reach need a baked model.
    if r.errors.is_empty() || c.model.is_some() {
        let baked = bake::bake(c, texture.cloned());
        if matches!(c.model, Some(Model::Cubes { .. })) {
            r.stats.vertices = baked.vertices;
            if baked.vertices > budget::MAX_VERTICES {
                r.error(
                    Code::Budget,
                    "model",
                    format!(
                        "{} vertices, at most {}",
                        baked.vertices,
                        budget::MAX_VERTICES
                    ),
                );
            }
        } else {
            r.stats.vertices = baked.vertices;
        }
        let ext = extent(&baked, c);
        r.stats.reach = ext.reach;
        match cat {
            Category::Companion => {
                if ext.reach > budget::MAX_COMPANION_REACH {
                    r.error(
                        Code::Reach,
                        "model",
                        format!(
                            "a companion reaches {:.1} pixels from its origin, at most {}",
                            ext.reach,
                            budget::MAX_COMPANION_REACH
                        ),
                    );
                }
                if let Some(cf) = &c.companion {
                    check_companion(cf, &names, ext.reach, &mut r);
                }
            }
            Category::Aura => {
                if let Some(a) = &c.aura {
                    check_aura(a, &ext, &mut r);
                }
            }
            Category::Trail | Category::Nametag => {}
            _ if ext.reach > budget::MAX_REACH => r.error(
                Code::Reach,
                "model",
                format!(
                    "reaches {:.1} pixels from its bone, at most {}",
                    ext.reach,
                    budget::MAX_REACH
                ),
            ),
            _ => {}
        }
        if cat == Category::Back {
            cape_shape(&baked, &mut r);
        }
    }
    r
}

fn semver_like(v: &str) -> bool {
    let core = v.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

#[allow(clippy::too_many_arguments)]
fn check_part(
    p: &Part,
    at: &str,
    size: [f32; 2],
    top: bool,
    cat: Category,
    names: &mut Vec<String>,
    r: &mut Report,
) {
    if p.name.is_empty() || p.name.starts_with("cosmetic") || names.contains(&p.name) {
        r.error(
            Code::Model,
            format!("{at}.name"),
            format!(
                "part names are unique and never start with `cosmetic`: {:?}",
                p.name
            ),
        );
    }
    names.push(p.name.clone());
    if p.bone.is_some() && (!top || !cat.rides_bone()) {
        r.error(
            Code::Model,
            format!("{at}.bone"),
            "only a top-level part of a cosmetic worn on a bone rides its own bone",
        );
    }
    if !finite(&p.pivot) || !finite(&p.rotation) {
        r.error(Code::Model, at, "not a number");
    }
    for (i, cube) in p.cubes.iter().enumerate() {
        let cat_at = format!("{at}.cubes[{i}]");
        if !finite(&cube.origin) || !finite(&cube.size) || !finite(&cube.uv) {
            r.error(Code::Model, &cat_at, "not a number");
            continue;
        }
        if cube.size.iter().any(|s| *s < 0.0) || !(0.0..=4.0).contains(&cube.inflate) {
            r.error(
                Code::Model,
                &cat_at,
                "sizes are not negative and inflate is 0 to 4",
            );
        }
        let [w, h, d] = cube.size;
        if cube.uv[0] < 0.0
            || cube.uv[1] < 0.0
            || cube.uv[0] + 2.0 * (d + w) > size[0]
            || cube.uv[1] + d + h > size[1]
        {
            r.error(Code::Model, &cat_at, "the box's UV leaves the texture");
        }
    }
    for (i, child) in p.children.iter().enumerate() {
        check_part(
            child,
            &format!("{at}.children[{i}]"),
            size,
            false,
            cat,
            names,
            r,
        );
    }
}

fn check_emote(e: &Emote, props: &[String], r: &mut Report) {
    use budget::*;
    if !finite(&[e.length, e.blend]) {
        r.error(Code::Emote, "emote", "not a number");
        return;
    }
    if !(4.0..=MAX_EMOTE_LENGTH).contains(&e.length) {
        r.error(
            Code::Emote,
            "emote.length",
            format!("an emote lasts 4 to {MAX_EMOTE_LENGTH} ticks"),
        );
    }
    if !(0.0..=e.length / 2.0).contains(&e.blend) {
        r.error(
            Code::Emote,
            "emote.blend",
            "the blend is at most half the length",
        );
    }
    if e.tracks.is_empty() || e.tracks.len() > MAX_TRACKS {
        r.error(
            Code::Emote,
            "emote.tracks",
            format!("1 to {MAX_TRACKS} tracks"),
        );
    }
    let all: usize = e.tracks.iter().map(|t| t.keys.len()).sum();
    if all > MAX_ALL_KEYS {
        r.error(
            Code::Budget,
            "emote.tracks",
            format!("{all} keys, at most {MAX_ALL_KEYS}"),
        );
    }
    let mut seen: Vec<&str> = Vec::new();
    for (i, t) in e.tracks.iter().enumerate() {
        let at = format!("emote.tracks[{i}]");
        let is_root = t.part == "root";
        let is_bone = Bone::from_part(&t.part).is_some();
        let is_prop = props.contains(&t.part);
        if !(is_root || is_bone || is_prop) {
            r.error(Code::Emote, &at, format!("moves no part {:?}", t.part));
        }
        if seen.contains(&t.part.as_str()) {
            r.error(Code::Emote, &at, format!("two tracks move {:?}", t.part));
        }
        seen.push(&t.part);
        if t.keys.is_empty() || t.keys.len() > MAX_KEYS {
            r.error(Code::Emote, &at, format!("1 to {MAX_KEYS} keys"));
        }
        let (turn, reach) = if is_root {
            (MAX_ROOT_TURN, MAX_ROOT_MOVE)
        } else {
            (MAX_TURN, MAX_MOVE)
        };
        let mut last: Option<f32> = None;
        for (j, k) in t.keys.iter().enumerate() {
            let kat = format!("{at}.keys[{j}]");
            let scale = k.scale.unwrap_or(1.0);
            let mut values = vec![k.t, scale];
            values.extend(k.offset);
            values.extend(k.rotation);
            if !finite(&values) {
                r.error(Code::Emote, &kat, "not a number");
                continue;
            }
            if last.is_some_and(|l| k.t <= l) || !(0.0..=e.length).contains(&k.t) {
                r.error(
                    Code::Emote,
                    &kat,
                    "keys go forward in time within the emote",
                );
            }
            last = Some(k.t);
            if k.rotation.iter().any(|v| v.abs() > turn) || k.offset.iter().any(|v| v.abs() > reach)
            {
                r.error(Code::Emote, &kat, "turns or moves too far");
            }
            if (k.scale.is_some() && !is_prop) || !(0.0..=2.0).contains(&scale) {
                r.error(Code::Emote, &kat, "only a prop's key scales it, 0 to 2");
            }
        }
    }
}

fn check_trail(t: &TrailFx, texture: Option<&Texture>, r: &mut Report) {
    use budget::*;
    if !finite(&[t.rate, t.size[0], t.size[1], t.gravity, t.drift, t.spin]) {
        r.error(Code::Fx, "trail", "not a number");
        return;
    }
    if let Some(tex) = texture
        && (!(1..=MAX_TRAIL_SPRITES).contains(&t.sprites)
            || tex.height > MAX_SPRITE
            || tex.width != t.sprites * tex.height)
    {
        r.error(
            Code::Fx,
            "trail.sprites",
            format!(
                "the texture is a row of 1 to {MAX_TRAIL_SPRITES} square sprites of at most \
                 {MAX_SPRITE} texels"
            ),
        );
    }
    if !(t.rate > 0.0 && t.rate <= MAX_TRAIL_RATE) {
        r.error(
            Code::Fx,
            "trail.rate",
            format!("above 0 and at most {MAX_TRAIL_RATE} per block"),
        );
    }
    if !(1..=MAX_TRAIL_LIFETIME).contains(&t.lifetime) {
        r.error(
            Code::Fx,
            "trail.lifetime",
            format!("1 to {MAX_TRAIL_LIFETIME} ticks"),
        );
    }
    if !(t.size[0] > 0.0 && t.size[0] <= t.size[1] && t.size[1] <= MAX_PARTICLE_SIZE) {
        r.error(
            Code::Fx,
            "trail.size",
            format!("[smallest, largest], above 0 and at most {MAX_PARTICLE_SIZE}"),
        );
    }
    if t.gravity.abs() > 0.04 || !(0.0..=4.0).contains(&t.drift) || !(0.0..=20.0).contains(&t.spin)
    {
        r.error(
            Code::Fx,
            "trail",
            "gravity within 0.04, drift 0 to 4 pixels, spin 0 to 20 degrees a tick",
        );
    }
}

fn check_companion(c: &CompanionFx, names: &[String], reach: f32, r: &mut Report) {
    use budget::*;
    if !finite(&c.anchor) || !c.lag.is_finite() {
        r.error(Code::Fx, "companion", "not a number");
        return;
    }
    let anchor_len = math::length(c.anchor);
    if anchor_len > MAX_COMPANION_ANCHOR {
        r.error(
            Code::Fx,
            "companion.anchor",
            format!("at most {MAX_COMPANION_ANCHOR} pixels from the neck"),
        );
    }
    if math::length(math::sub(c.anchor, HEAD)) < HEAD_CLEARANCE + reach {
        r.error(
            Code::Fx,
            "companion.anchor",
            format!(
                "{:.1} pixels or more from the head's centre",
                HEAD_CLEARANCE + reach
            ),
        );
    }
    if !(1.0..=MAX_LAG).contains(&c.lag) {
        r.error(Code::Fx, "companion.lag", format!("1 to {MAX_LAG} ticks"));
    }
    if let Some(look) = &c.look
        && !names.contains(look)
    {
        r.error(
            Code::Fx,
            "companion.look",
            format!("names no part {look:?}"),
        );
    }
}

fn check_aura(a: &AuraFx, ext: &Extent, r: &mut Report) {
    let (max_alpha, up, down) = match a.at {
        FxAnchor::Feet => (0.6, 6.0, 1.0),
        FxAnchor::Body => (0.35, 30.0, 1.0),
    };
    if !(a.alpha > 0.0 && a.alpha <= max_alpha) {
        r.error(
            Code::Fx,
            "aura.alpha",
            format!("above 0 and at most {max_alpha}"),
        );
    }
    let radius = [ext.low[0], ext.high[0], ext.low[2], ext.high[2]]
        .iter()
        .fold(0.0_f32, |m, v| m.max(v.abs()));
    if radius > budget::MAX_AURA_RADIUS || -ext.low[1] > up || ext.high[1] > down {
        r.error(
            Code::Reach,
            "model",
            format!(
                "an aura stays within {} pixels of the feet's middle, {up} above them and {down} below",
                budget::MAX_AURA_RADIUS
            ),
        );
    }
}

/// Capes are Mojang's: a back cosmetic's name may not say cape, cloak or mantle.
fn cape_words(c: &Cosmetic, r: &mut Report) {
    let text = format!("{} {}", c.id, c.name).to_lowercase();
    for word in ["cape", "cloak", "mantle", "elytra"] {
        if text.contains(word) {
            r.error(
                Code::CapeLike,
                "name",
                format!("no capes or cape-like items (`{word}`)"),
            );
            return;
        }
    }
}

/// A flat panel hanging down the back is a cape whatever it is called: a back item whose
/// geometry is at most 2 pixels deep, at least 6 wide and reaches 10 or more below the neck.
fn cape_shape(baked: &Baked, r: &mut Report) {
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    walk(&baked.root, &math::IDENTITY, &HashMap::new(), &mut |p| {
        for k in 0..3 {
            low[k] = low[k].min(p[k]);
            high[k] = high[k].max(p[k]);
        }
    });
    if low[0] > high[0] {
        return;
    }
    let (w, h, d) = (high[0] - low[0], high[1] - low[1], high[2] - low[2]);
    if d <= 2.0 && w >= 6.0 && h >= 10.0 && high[1] >= 10.0 {
        r.error(
            Code::CapeLike,
            "model",
            format!("a flat panel {w:.0}x{h:.0}x{d:.0} hanging down the back looks like a cape"),
        );
    }
}

struct Extent {
    reach: f32,
    low: [f32; 3],
    high: [f32; 3],
}

fn walk(
    part: &BakedPart,
    parent: &Mat4,
    poses: &HashMap<String, [f32; 9]>,
    each: &mut impl FnMut([f32; 3]),
) {
    let pose = poses.get(&part.name).unwrap_or(&part.pose);
    let m = math::mul(parent, &math::pose_matrix(pose));
    for face in &part.faces {
        for v in face.vertices {
            each(math::transform_point(&m, [v[0], v[1], v[2]]));
        }
    }
    for c in &part.children {
        walk(c, &m, poses, each);
    }
}

/// How far the vertices get from their bone's pivot (or the origin), sampled over every loop's
/// period and the emote's props, as Horizon's `Item::extent`.
fn extent(baked: &Baked, c: &Cosmetic) -> Extent {
    let mut times = vec![0.0_f32];
    for a in &c.loops {
        if a.period > 0.0 && a.period.is_finite() {
            times.extend((0..32).map(|k| a.period * k as f32 / 32.0));
        }
    }
    let emote_len = c.emote.as_ref().map(|e| e.length).unwrap_or(0.0);
    if emote_len > 0.0 && emote_len.is_finite() {
        times.extend((0..=32).map(|k| emote_len * k as f32 / 32.0));
    }
    let mut far = 0.0_f32;
    let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
    let mut poses = HashMap::new();
    let roots: Vec<&BakedPart> = baked.roots().map(|(_, p)| p).collect();
    for t in times {
        poses.clear();
        anim::animate(&roots, &c.loops, t, |name, pose| {
            poses.insert(name.to_owned(), pose);
        });
        if let Some(e) = &c.emote {
            for track in &e.tracks {
                let Some(rest) = baked
                    .roots()
                    .find_map(|(_, p)| anim::rest_pose(p, &track.part))
                else {
                    continue;
                };
                let s = anim::sample_track(track, t, e.looping, e.length);
                let mut pose = rest;
                for k in 0..3 {
                    pose[k] += s.offset[k];
                    pose[3 + k] += s.rotation[k].to_radians();
                    pose[6 + k] *= s.scale;
                }
                poses.insert(track.part.clone(), pose);
            }
        }
        for (_, root) in baked.roots() {
            walk(root, &math::IDENTITY, &poses, &mut |p| {
                far = far.max(math::length(p));
                for k in 0..3 {
                    low[k] = low[k].min(p[k]);
                    high[k] = high[k].max(p[k]);
                }
            });
        }
    }
    if low[0] > high[0] {
        (low, high) = ([0.0; 3], [0.0; 3]);
    }
    Extent {
        reach: far,
        low,
        high,
    }
}
