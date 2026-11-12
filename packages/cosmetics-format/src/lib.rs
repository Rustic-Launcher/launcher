//! The Rustic cosmetic format (`launcher/docs/COSMETICS_FORMAT.md`).
//!
//! - [`model`]: the file (`cosmetic.json`): categories, attachment points on the player
//!   skeleton, box or mesh models, texture, loops, emotes, companions, trails, auras, name tags.
//! - [`validate`]: structure, budgets (vertices, triangles, parts, texture, reach), the rules
//!   (no capes, nothing random, no gameplay: the format has no field that could carry any).
//! - [`bake`]: box UV faces, ready to draw (the in-game companion draws from it).
//! - [`anim`]: loops and emote keyframes sampled from time alone.
//! - [`horizon`]: conversion to and from Horizon's cosmetics JSON.
//! - [`package`]: author folders, zips' entries and self-contained bundles; glTF import.
//! - [`json_schema`]: the JSON Schema of the file, for the TypeScript side.
#![forbid(unsafe_code)]

pub mod anim;
pub mod bake;
pub mod horizon;
pub mod math;
pub mod model;
pub mod package;
pub mod texture;
pub mod validate;

pub use model::*;
pub use package::Package;
pub use validate::{Report, validate};

/// The JSON Schema (draft 2020-12) of `cosmetic.json`.
pub fn json_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(model::Cosmetic)).unwrap_or_default()
}

/// The JSON Schema of a validation [`Report`], for tools that show one.
pub fn report_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(validate::Report)).unwrap_or_default()
}
