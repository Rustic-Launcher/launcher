//! Conversion to and from Horizon's cosmetic item JSON (`format: 1`, read by
//! `horizon_cosmetics::Item::parse`; `horizon/docs/cosmetics.md`).
//!
//! Horizon -> Rustic -> Horizon is lossless (the same item, field for field, once Horizon's own
//! defaults are spelled out). Rustic -> Horizon keeps everything Horizon can draw and reports
//! what it drops: the metadata Horizon's item file has no field for (version, name, description,
//! authors) and the named attachment point (folded into the bone's offset). Mesh models have no
//! Horizon form (Horizon items are boxes only) and are refused.
use crate::model::*;
use crate::texture::{decode_base64, encode_base64};
use serde::{Deserialize, Serialize};

/// Horizon's slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Head,
    Back,
    Shoulder,
    Torso,
    NameTag,
    Emote,
    Companion,
    Trail,
    Aura,
}

impl Slot {
    pub fn of(category: Category) -> Slot {
        match category {
            Category::Head => Slot::Head,
            Category::Back => Slot::Back,
            Category::Top => Slot::Torso,
            Category::ShoulderCompanion => Slot::Shoulder,
            Category::Companion => Slot::Companion,
            Category::Trail => Slot::Trail,
            Category::Aura => Slot::Aura,
            Category::Emote => Slot::Emote,
            Category::Nametag => Slot::NameTag,
        }
    }

    pub fn category(self) -> Category {
        match self {
            Slot::Head => Category::Head,
            Slot::Back => Category::Back,
            Slot::Torso => Category::Top,
            Slot::Shoulder => Category::ShoulderCompanion,
            Slot::Companion => Category::Companion,
            Slot::Trail => Category::Trail,
            Slot::Aura => Category::Aura,
            Slot::Emote => Category::Emote,
            Slot::NameTag => Category::Nametag,
        }
    }

    fn on_bone(self) -> bool {
        !matches!(self, Slot::Companion | Slot::Trail | Slot::Aura)
    }
}

/// Horizon's `attach`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attach {
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub offset: [f32; 3],
    #[serde(default, skip_serializing_if = "is_zero3")]
    pub rotation: [f32; 3],
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HorizonTexture {
    /// Base64 PNG.
    pub png: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub line: String,
}

/// One Horizon item file. Parts, cubes, animations, the emote, companion, trail and aura blocks
/// have the same JSON shape in both formats, so they share types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HorizonItem {
    pub format: u32,
    pub id: String,
    pub slot: Slot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bone: Option<Bone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<Attach>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<HorizonTexture>,
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<LoopAnimation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<Tag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emote: Option<Emote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion: Option<CompanionFx>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trail: Option<TrailFx>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aura: Option<AuraFx>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConvertError {
    #[error("not representable in Horizon's format: {0}")]
    NotRepresentable(String),
    #[error("the texture is missing: embed it or load the package first")]
    NoTexture,
    #[error("not a Horizon item: {0}")]
    Json(String),
}

/// What [`to_horizon`] produced and what it dropped.
#[derive(Clone, Debug, PartialEq)]
pub struct Converted {
    pub item: HorizonItem,
    /// Fields of the Rustic file with no Horizon counterpart.
    pub dropped: Vec<String>,
}

fn is_zero3(v: &[f32; 3]) -> bool {
    *v == [0.0; 3]
}
fn one() -> f32 {
    1.0
}
fn is_one(v: &f32) -> bool {
    *v == 1.0
}

/// Converts `c` to Horizon's item, with the PNG bytes of its texture when it is not embedded.
pub fn to_horizon(c: &Cosmetic, png: Option<&[u8]>) -> Result<Converted, ConvertError> {
    let parts = match &c.model {
        Some(Model::Cubes { parts }) => parts.clone(),
        Some(Model::Mesh(_)) => {
            return Err(ConvertError::NotRepresentable(
                "a mesh model (Horizon items are boxes only)".into(),
            ));
        }
        None => Vec::new(),
    };
    let mut dropped = Vec::new();
    if c.version != "1.0.0" {
        dropped.push(format!("version {}", c.version));
    }
    dropped.push("name".into());
    if !c.description.is_empty() {
        dropped.push("description".into());
    }
    if !c.authors.is_empty() {
        dropped.push("authors".into());
    }
    let (bone, attach) = match &c.attachment {
        Some(a) => {
            let (bone, offset) = a.resolved();
            if a.point != AttachmentPoint::of_bone(bone) {
                dropped.push(format!(
                    "attachment point {:?} (kept as its bone and offset)",
                    a.point
                ));
            }
            let attach = Attach {
                offset,
                rotation: a.rotation,
                scale: a.scale,
            };
            let default =
                attach.offset == [0.0; 3] && attach.rotation == [0.0; 3] && attach.scale == 1.0;
            (
                (bone != Bone::Head).then_some(bone),
                (!default).then_some(attach),
            )
        }
        None => (None, None),
    };
    let texture = match (&c.texture, png) {
        (_, Some(bytes)) => Some(HorizonTexture {
            png: encode_base64(bytes),
        }),
        (Some(TextureSource { png: Some(b64), .. }), None) => {
            Some(HorizonTexture { png: b64.clone() })
        }
        (Some(_), None) => return Err(ConvertError::NoTexture),
        (None, None) => None,
    };
    Ok(Converted {
        item: HorizonItem {
            format: 1,
            id: c.id.clone(),
            slot: Slot::of(c.category),
            bone,
            attach,
            texture,
            parts,
            animations: c.loops.clone(),
            tag: c.nametag.as_ref().map(|n| Tag {
                line: n.line.clone(),
            }),
            emote: c.emote.clone(),
            companion: c.companion.clone(),
            trail: c.trail.clone(),
            aura: c.aura.clone(),
        },
        dropped,
    })
}

/// Reads a Horizon item file.
pub fn parse_horizon(bytes: &[u8]) -> Result<HorizonItem, ConvertError> {
    serde_json::from_slice(bytes).map_err(|e| ConvertError::Json(e.to_string()))
}

/// Converts a Horizon item to a Rustic cosmetic (texture embedded, version `1.0.0`, the id as
/// its name until the catalogue names it).
pub fn from_horizon(item: &HorizonItem) -> Result<Cosmetic, ConvertError> {
    if item.format != 1 {
        return Err(ConvertError::NotRepresentable(format!(
            "Horizon item format {}",
            item.format
        )));
    }
    let category = item.slot.category();
    let attach = item.attach.clone().unwrap_or(Attach {
        offset: [0.0; 3],
        rotation: [0.0; 3],
        scale: 1.0,
    });
    let has_attach = item.attach.is_some() || item.bone.is_some();
    let attachment = if category.on_bone() || (item.slot.on_bone() && has_attach) {
        Some(Attachment {
            point: AttachmentPoint::of_bone(item.bone.unwrap_or_default()),
            offset: attach.offset,
            rotation: attach.rotation,
            scale: attach.scale,
        })
    } else if has_attach {
        return Err(ConvertError::NotRepresentable(format!(
            "an attachment on a {:?} item",
            item.slot
        )));
    } else {
        None
    };
    if let Some(t) = &item.texture {
        decode_base64(&t.png).map_err(|e| ConvertError::Json(e.to_string()))?;
    }
    let model = if item.parts.is_empty() {
        None
    } else {
        Some(Model::Cubes {
            parts: item.parts.clone(),
        })
    };
    Ok(Cosmetic {
        format: FORMAT,
        id: item.id.clone(),
        version: "1.0.0".into(),
        name: item.id.clone(),
        description: String::new(),
        authors: Vec::new(),
        category,
        attachment,
        texture: item.texture.as_ref().map(|t| TextureSource {
            path: None,
            png: Some(t.png.clone()),
        }),
        model,
        loops: item.animations.clone(),
        emote: item.emote.clone(),
        companion: item.companion.clone(),
        trail: item.trail.clone(),
        aura: item.aura.clone(),
        nametag: item.tag.as_ref().map(|t| NametagStyle {
            line: t.line.clone(),
        }),
    })
}

/// `item` with Horizon's defaults spelled the one way [`to_horizon`] writes them (a head bone
/// and a neutral attachment left out), for comparing items.
pub fn normalized(item: &HorizonItem) -> HorizonItem {
    let mut out = item.clone();
    if out.bone == Some(Bone::Head) {
        out.bone = None;
    }
    if out
        .attach
        .as_ref()
        .is_some_and(|a| a.offset == [0.0; 3] && a.rotation == [0.0; 3] && a.scale == 1.0)
    {
        out.attach = None;
    }
    out
}
