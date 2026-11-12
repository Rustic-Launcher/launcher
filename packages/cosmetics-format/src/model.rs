//! The Rustic cosmetic file (`cosmetic.json`), as authors write it and as the CDN serves it.
//!
//! Coordinates are model pixels (1/16 block) in the player model's space, y pointing down, as in
//! vanilla's `ModelPart`s, Blockbench's Java "Modded Entity" models and Horizon's cosmetics
//! (`horizon/crates/horizon-cosmetics`, `horizon/docs/cosmetics.md`). A part's `pivot` is its
//! `PartPose` offset and its cubes are relative to it; rotations are degrees applied Z, then Y,
//! then X (vanilla's `rotationZYX`). See `launcher/docs/COSMETICS_FORMAT.md`.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The only version of this format.
pub const FORMAT: u32 = 1;

/// One cosmetic: metadata, where it sits on the player, its model, texture and motion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cosmetic {
    /// Always [`FORMAT`] (1). A reader refuses a higher number.
    pub format: u32,
    /// `[a-z0-9_]{1,64}`: the catalogue id, also the Horizon item id when converted.
    pub id: String,
    /// Semantic version of this cosmetic's file (`1.0.0`). Every published change bumps it.
    pub version: String,
    /// Display name (English); translations live in the catalogue, not here.
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    pub category: Category,
    /// Where it sits on the player: required for categories worn on a bone (head, back, top,
    /// shoulder companion), optional for an emote's props, absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Attachment>,
    /// The one texture (PNG), by file path inside the package or embedded. An emote without
    /// props has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<TextureSource>,
    /// The geometry. Absent for a trail and a name tag style (no model) and an emote without
    /// props.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    /// Looping motions of named parts on top of their rest pose.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loops: Vec<LoopAnimation>,
    /// Category `emote` only: keyframed body animation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emote: Option<Emote>,
    /// Category `companion` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion: Option<CompanionFx>,
    /// Category `trail` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trail: Option<TrailFx>,
    /// Category `aura` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aura: Option<AuraFx>,
    /// Category `nametag` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nametag: Option<NametagStyle>,
}

/// What kind of cosmetic it is; a player wears at most one per category (and plays emotes).
/// There is no cape category, on purpose: capes are Mojang's (launcher/docs/LEGAL.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// A hat, a crown, ears.
    Head,
    /// A backpack, wings that never act like an elytra. Never a cape.
    Back,
    /// Over the body: a shirt, its sleeves may ride the arms.
    Top,
    /// A small pet sitting on a shoulder.
    ShoulderCompanion,
    /// A small creature floating beside the player, on no bone.
    Companion,
    /// Particles left behind while walking.
    Trail,
    /// A see-through model around the feet or body.
    Aura,
    /// A short body animation, played from the emote wheel.
    Emote,
    /// A badge after the name and a coloured line under it.
    Nametag,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Head,
        Category::Back,
        Category::Top,
        Category::ShoulderCompanion,
        Category::Companion,
        Category::Trail,
        Category::Aura,
        Category::Emote,
        Category::Nametag,
    ];

    /// Worn on a bone of the player model (needs an [`Attachment`]).
    pub fn on_bone(self) -> bool {
        matches!(
            self,
            Category::Head | Category::Back | Category::Top | Category::ShoulderCompanion
        )
    }

    /// Drawn relative to a bone at all (an emote's props too); companions, trails and auras
    /// ride none (Horizon's `on_bone`).
    pub fn rides_bone(self) -> bool {
        !matches!(self, Category::Companion | Category::Trail | Category::Aura)
    }

    /// Its name in files.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Head => "head",
            Category::Back => "back",
            Category::Top => "top",
            Category::ShoulderCompanion => "shoulder_companion",
            Category::Companion => "companion",
            Category::Trail => "trail",
            Category::Aura => "aura",
            Category::Emote => "emote",
            Category::Nametag => "nametag",
        }
    }
}

/// A bone of the player model (vanilla `PlayerModel`'s parts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Bone {
    #[default]
    Head,
    Body,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl Bone {
    pub const ALL: [Bone; 6] = [
        Bone::Head,
        Bone::Body,
        Bone::RightArm,
        Bone::LeftArm,
        Bone::RightLeg,
        Bone::LeftLeg,
    ];

    /// The part's name in the player model.
    pub fn part(self) -> &'static str {
        match self {
            Bone::Head => "head",
            Bone::Body => "body",
            Bone::LeftArm => "left_arm",
            Bone::RightArm => "right_arm",
            Bone::LeftLeg => "left_leg",
            Bone::RightLeg => "right_leg",
        }
    }

    pub fn from_part(name: &str) -> Option<Bone> {
        Bone::ALL.into_iter().find(|b| b.part() == name)
    }
}

/// A named place on the player skeleton. Each is a bone plus a base offset in pixels: the
/// cosmetic's own `offset` is added to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentPoint {
    /// The head's pivot (the neck), turning with the head.
    Head,
    /// The body's pivot (the neck), swaying with the body.
    Body,
    /// The middle of the back's top edge: body + (0, 0, 2).
    Back,
    /// On top of the left shoulder: body + (6, 0, 0).
    LeftShoulder,
    /// On top of the right shoulder: body + (-6, 0, 0).
    RightShoulder,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl AttachmentPoint {
    pub const ALL: [AttachmentPoint; 9] = [
        AttachmentPoint::Head,
        AttachmentPoint::Body,
        AttachmentPoint::Back,
        AttachmentPoint::LeftShoulder,
        AttachmentPoint::RightShoulder,
        AttachmentPoint::LeftArm,
        AttachmentPoint::RightArm,
        AttachmentPoint::LeftLeg,
        AttachmentPoint::RightLeg,
    ];

    /// The bone it follows and its offset from that bone's pivot, in pixels.
    pub fn bone_and_base(self) -> (Bone, [f32; 3]) {
        match self {
            AttachmentPoint::Head => (Bone::Head, [0.0; 3]),
            AttachmentPoint::Body => (Bone::Body, [0.0; 3]),
            AttachmentPoint::Back => (Bone::Body, [0.0, 0.0, 2.0]),
            AttachmentPoint::LeftShoulder => (Bone::Body, [6.0, 0.0, 0.0]),
            AttachmentPoint::RightShoulder => (Bone::Body, [-6.0, 0.0, 0.0]),
            AttachmentPoint::LeftArm => (Bone::LeftArm, [0.0; 3]),
            AttachmentPoint::RightArm => (Bone::RightArm, [0.0; 3]),
            AttachmentPoint::LeftLeg => (Bone::LeftLeg, [0.0; 3]),
            AttachmentPoint::RightLeg => (Bone::RightLeg, [0.0; 3]),
        }
    }

    /// The point with no base offset on `bone`.
    pub fn of_bone(bone: Bone) -> AttachmentPoint {
        match bone {
            Bone::Head => AttachmentPoint::Head,
            Bone::Body => AttachmentPoint::Body,
            Bone::LeftArm => AttachmentPoint::LeftArm,
            Bone::RightArm => AttachmentPoint::RightArm,
            Bone::LeftLeg => AttachmentPoint::LeftLeg,
            Bone::RightLeg => AttachmentPoint::RightLeg,
        }
    }
}

/// Where a worn cosmetic sits: the point, then an offset, rotation and uniform scale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attachment {
    pub point: AttachmentPoint,
    /// Pixels from the point.
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub offset: [f32; 3],
    /// Degrees, Z then Y then X.
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub rotation: [f32; 3],
    /// Uniform scale in (0, 2].
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f32,
}

impl Attachment {
    pub fn at(point: AttachmentPoint) -> Self {
        Self {
            point,
            offset: [0.0; 3],
            rotation: [0.0; 3],
            scale: 1.0,
        }
    }

    /// The bone and the offset from its pivot (base + own offset).
    pub fn resolved(&self) -> (Bone, [f32; 3]) {
        let (bone, base) = self.point.bone_and_base();
        (
            bone,
            [
                base[0] + self.offset[0],
                base[1] + self.offset[1],
                base[2] + self.offset[2],
            ],
        )
    }
}

/// The texture: a PNG file in the package (`path`) or embedded as base64 (`png`). A
/// self-contained bundle (what the CDN serves) always embeds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextureSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Base64 (standard alphabet) PNG bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub png: Option<String>,
}

/// The geometry: boxes (Blockbench's Java models; convertible to Horizon) or a triangle mesh
/// (glTF 2.0; drawn by Rustic's companion, not convertible to Horizon's box format).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Model {
    Cubes { parts: Vec<Part> },
    Mesh(MeshModel),
}

/// A part of a box model: a pivot, a rotation, boxes and children.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// Unique in the cosmetic; never starts with `cosmetic`.
    pub name: String,
    /// Top-level parts only: ride this bone instead of the attachment's (a shirt's sleeves).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bone: Option<Bone>,
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub pivot: [f32; 3],
    /// Degrees, Z then Y then X.
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub rotation: [f32; 3],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cubes: Vec<Cube>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Part>,
}

/// A box with Minecraft's box UV (`ModelPart.Cube`): the unfolded box spans `2(d + w)` by
/// `d + h` texels from `uv`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cube {
    pub origin: [f32; 3],
    pub size: [f32; 3],
    pub uv: [f32; 2],
    /// Grows the box on every side, 0 to 4 pixels, over the same texels.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub inflate: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mirror: bool,
}

/// A triangle mesh: authored as glTF 2.0 (`source`), served with its triangles inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshModel {
    /// A `.glb` or `.gltf` (embedded buffers) inside the package; the pipeline reads it into
    /// `triangles` and drops it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Glb units to pixels (Blockbench exports 1 unit = 1 block: 16).
    #[serde(default = "sixteen", skip_serializing_if = "is_sixteen")]
    pub units_to_pixels: f32,
    /// Named groups of triangles; each may be animated by name like a part.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<MeshGroup>,
}

/// Triangles of a mesh that move together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshGroup {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub pivot: [f32; 3],
    /// Each vertex: x, y, z (pixels, y down), u, v (0 to 1 across the texture).
    pub triangles: Vec<[[f32; 5]; 3]>,
}

/// A looping motion of one part on top of its rest pose: a sine of `period` ticks started
/// `phase` periods in. Deterministic: it depends on the model's age only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoopAnimation {
    pub part: String,
    pub motion: LoopMotion,
    pub axis: Axis,
    /// Degrees for `swing` (at most 45), pixels for `bob` (at most 4); unused by `spin`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub amplitude: f32,
    /// Ticks, 4 to 1200.
    pub period: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub phase: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LoopMotion {
    Swing,
    Bob,
    Spin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    Y,
    Z,
}

/// A keyframed body animation (Horizon's emote format).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Emote {
    /// Ticks, 4 to 200.
    pub length: f32,
    #[serde(default, rename = "loop", skip_serializing_if = "is_false")]
    pub looping: bool,
    /// Fade-in/out ticks, at most half the length.
    #[serde(default = "four", skip_serializing_if = "is_four")]
    pub blend: f32,
    pub tracks: Vec<Track>,
}

/// The keys of one bone (`head`, `body`, `right_arm`, `left_arm`, `right_leg`, `left_leg`), of
/// `root` (the whole body about the feet) or of a prop part.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub part: String,
    pub keys: Vec<Key>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub t: f32,
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub offset: [f32; 3],
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub rotation: [f32; 3],
    /// Props only, 0 to 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    #[serde(default, skip_serializing_if = "is_smooth")]
    pub ease: Ease,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    #[default]
    Smooth,
    Linear,
    InOut,
    Step,
}

/// A companion's float: where it rests beside the player and how it follows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompanionFx {
    /// Pixels from the neck in the wearer's body space: y down, +x the wearer's left, +z behind.
    pub anchor: [f32; 3],
    /// Follow time constant in ticks, 1 to 40.
    #[serde(default = "six", skip_serializing_if = "is_six")]
    pub lag: f32,
    /// A part that turns to look where the wearer looks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub glow: bool,
}

/// Where a trail starts or an aura sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FxAnchor {
    #[default]
    Feet,
    Body,
}

/// A trail: particles spawned by distance walked (never by time or chance), each one a sprite of
/// the texture's row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrailFx {
    #[serde(default = "one_u32", skip_serializing_if = "is_one_u32")]
    pub sprites: u32,
    /// Particles per block moved, at most 4.
    pub rate: f32,
    /// Ticks, at most 40.
    pub lifetime: u32,
    /// Smallest and largest side in pixels, at most 4.
    pub size: [f32; 2],
    #[serde(default, skip_serializing_if = "is_zero")]
    pub gravity: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub drift: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub spin: f32,
    #[serde(default, skip_serializing_if = "is_feet")]
    pub from: FxAnchor,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fade: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub glow: bool,
}

/// An aura: a see-through model at the feet or around the body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuraFx {
    #[serde(default, skip_serializing_if = "is_feet")]
    pub at: FxAnchor,
    /// At most 0.6 at the feet, 0.35 around the body.
    pub alpha: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub glow: bool,
}

/// A name tag style: the texture is a square badge (at most 16x16) after the name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NametagStyle {
    /// `#rrggbb`: the line under the name.
    pub line: String,
}

pub(crate) fn is_zero3(v: &[f32; 3]) -> bool {
    *v == [0.0; 3]
}
pub(crate) fn is_zero(v: &f32) -> bool {
    *v == 0.0
}
pub(crate) fn is_false(v: &bool) -> bool {
    !*v
}
pub(crate) fn one() -> f32 {
    1.0
}
pub(crate) fn is_one(v: &f32) -> bool {
    *v == 1.0
}
fn four() -> f32 {
    4.0
}
fn is_four(v: &f32) -> bool {
    *v == 4.0
}
fn six() -> f32 {
    6.0
}
fn is_six(v: &f32) -> bool {
    *v == 6.0
}
fn sixteen() -> f32 {
    16.0
}
fn is_sixteen(v: &f32) -> bool {
    *v == 16.0
}
fn one_u32() -> u32 {
    1
}
fn is_one_u32(v: &u32) -> bool {
    *v == 1
}
fn is_smooth(e: &Ease) -> bool {
    *e == Ease::Smooth
}
fn is_feet(a: &FxAnchor) -> bool {
    *a == FxAnchor::Feet
}

/// `#rrggbb` as bytes.
pub fn parse_colour(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#').filter(|h| h.len() == 6)?;
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// `[a-z0-9_]{1,64}`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
