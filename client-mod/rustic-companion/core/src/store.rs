//! Cosmetic bundles by content hash: fetched once (they never change for a hash), checked
//! (SHA-256, the format's validation: budgets, no capes), baked once, given a sprite id for the
//! game's atlas. A bundle that fails is not asked for again for a while.
use crate::backoff::Backoff;
use rusticmc_cosmetics_format::bake::{self, Baked};
use rusticmc_cosmetics_format::{Category, Cosmetic, Package};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Rustic's sprite ids in the game's shared `hooks/<hex>` atlas names: a high prefix, so they
/// never meet Horizon's own cosmetics' small sequential ids.
pub const SPRITE_BASE: u64 = 0x5255_5354_0000_0000;

/// A cosmetic ready to draw.
#[derive(Debug)]
pub struct Loaded {
    pub cosmetic: Cosmetic,
    pub baked: Baked,
    pub sprite: u64,
    /// The texture in a 64x64 RGBA slot (what the atlas takes), when it has one.
    pub slot: Option<Vec<u8>>,
    /// The texture's share of the slot (`width / 64`, `height / 64`).
    pub uv_scale: [f32; 2],
}

impl Loaded {
    pub fn category(&self) -> Category {
        self.cosmetic.category
    }

    /// The sprite part of the slot this texture covers, given the slot's atlas rectangle.
    pub fn uv(&self, slot: [f32; 4]) -> [f32; 4] {
        let [u0, v0, u1, v1] = slot;
        [
            u0,
            v0,
            u0 + (u1 - u0) * self.uv_scale[0],
            v0 + (v1 - v0) * self.uv_scale[1],
        ]
    }
}

#[derive(Debug)]
enum State {
    Fetching,
    Ready(Arc<Loaded>),
    Failed(Backoff),
}

#[derive(Debug, Default)]
pub struct CosmeticStore {
    by_hash: HashMap<String, State>,
    next_sprite: u64,
}

/// Checks and bakes a fetched bundle.
pub fn load(hash: &str, bytes: &[u8], sprite: u64) -> Result<Loaded, String> {
    let got = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if !got.eq_ignore_ascii_case(hash) {
        return Err(format!("sha256 {got} is not the worn hash {hash}"));
    }
    if bytes.len() > rusticmc_cosmetics_format::validate::budget::MAX_BUNDLE_BYTES {
        return Err("bundle over budget".into());
    }
    let pkg = Package::from_bundle(bytes).map_err(|e| e.to_string())?;
    let report = pkg.validate();
    if !report.ok() {
        return Err(format!(
            "refused: {}",
            report
                .errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let slot = pkg.texture.as_ref().and_then(|t| t.slot_64());
    let uv_scale = pkg.texture.as_ref().map_or([1.0, 1.0], |t| {
        [t.width as f32 / 64.0, t.height as f32 / 64.0]
    });
    let baked = bake::bake(&pkg.cosmetic, pkg.texture.clone());
    Ok(Loaded {
        cosmetic: pkg.cosmetic,
        baked,
        sprite,
        slot,
        uv_scale,
    })
}

impl CosmeticStore {
    /// The cosmetic of `hash`, if ready.
    pub fn get(&self, hash: &str) -> Option<&Arc<Loaded>> {
        match self.by_hash.get(hash) {
            Some(State::Ready(l)) => Some(l),
            _ => None,
        }
    }

    /// Whether `hash` should be fetched now (unknown, or failed long enough ago); marks it.
    pub fn want(&mut self, hash: &str, now: Instant) -> bool {
        match self.by_hash.get(hash) {
            None => {
                self.by_hash.insert(hash.to_owned(), State::Fetching);
                true
            }
            Some(State::Failed(b)) if b.ready(now) => {
                self.by_hash.insert(hash.to_owned(), State::Fetching);
                true
            }
            _ => false,
        }
    }

    /// A fetched bundle arrived.
    pub fn arrived(&mut self, hash: &str, bytes: &[u8], now: Instant) -> Result<(), String> {
        let sprite = SPRITE_BASE | self.next_sprite;
        match load(hash, bytes, sprite) {
            Ok(l) => {
                self.next_sprite += 1;
                self.by_hash
                    .insert(hash.to_owned(), State::Ready(Arc::new(l)));
                Ok(())
            }
            Err(e) => {
                self.failed(hash, now);
                Err(e)
            }
        }
    }

    pub fn failed(&mut self, hash: &str, now: Instant) {
        let mut b = match self.by_hash.remove(hash) {
            Some(State::Failed(b)) => b,
            _ => Backoff::default(),
        };
        b.failed(now);
        self.by_hash.insert(hash.to_owned(), State::Failed(b));
    }

    /// Every ready cosmetic whose sprite isn't in the atlas yet, per `has`.
    pub fn sprites_missing(&self, has: impl Fn(u64) -> bool) -> Vec<Arc<Loaded>> {
        self.by_hash
            .values()
            .filter_map(|s| match s {
                State::Ready(l) if l.slot.is_some() && !has(l.sprite) => Some(l.clone()),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusticmc_cosmetics_format::texture::{Texture, encode_base64};

    pub fn bundle() -> Vec<u8> {
        let png = Texture {
            width: 64,
            height: 32,
            rgba: vec![200; 64 * 32 * 4],
        }
        .encode_png();
        format!(
            r#"{{"format":1,"id":"top_hat","version":"1.0.0","name":"Top Hat","category":"head",
            "attachment":{{"point":"head","offset":[0,-8,0]}},"texture":{{"png":"{}"}},
            "model":{{"kind":"cubes","parts":[{{"name":"brim","cubes":[{{"origin":[-5,-1,-5],"size":[10,1,10],"uv":[0,0]}}]}}]}}}}"#,
            encode_base64(&png)
        )
        .into_bytes()
    }

    fn sha(b: &[u8]) -> String {
        Sha256::digest(b)
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect()
    }

    #[test]
    fn verified_baked_and_given_a_sprite() {
        let t = Instant::now();
        let b = bundle();
        let h = sha(&b);
        let mut s = CosmeticStore::default();
        assert!(s.want(&h, t));
        assert!(!s.want(&h, t), "fetched once");
        s.arrived(&h, &b, t).unwrap();
        let l = s.get(&h).unwrap();
        assert_eq!(l.sprite & SPRITE_BASE, SPRITE_BASE);
        assert_eq!(l.baked.vertices, 24);
        assert_eq!(l.uv([0.0, 0.0, 1.0, 1.0]), [0.0, 0.0, 1.0, 0.5]);
        assert_eq!(s.sprites_missing(|_| false).len(), 1);
        assert!(s.sprites_missing(|_| true).is_empty());
    }

    #[test]
    fn tampered_or_invalid_bundles_are_refused() {
        let t = Instant::now();
        let mut s = CosmeticStore::default();
        let b = bundle();
        let wrong = "0".repeat(64);
        s.want(&wrong, t);
        assert!(s.arrived(&wrong, &b, t).is_err());
        assert!(!s.want(&wrong, t), "backs off");
        let cape = String::from_utf8(bundle())
            .unwrap()
            .replace("\"Top Hat\"", "\"Hero Cape\"")
            .into_bytes();
        let h = sha(&cape);
        s.want(&h, t);
        assert!(s.arrived(&h, &cape, t).unwrap_err().contains("cape"));
    }
}
