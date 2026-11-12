//! Drawing Rustic cosmetics on players, through Horizon's platform player hooks
//! (`horizon_hooks::Hooks`: `frame` for the atlas sprites, `tick` for the models in view and the
//! emote body poses, `player_layers` for the meshes and trail particles, `name_decoration` for
//! name tag styles). These hooks are not part of the trait mod API: they are registered by the
//! Rustic edition's executable before the game starts (`horizon_hooks::register`), which a
//! trait mod built by Horizon's launcher could not do (ADR 0070).
//!
//! Horizon's own cosmetics keep drawing beside these (the user's decision): both register player
//! hooks. Rustic's sprite ids live under a high prefix so the two never share an atlas slot.
use crate::state::{OwnEmote, guard, guard_or, lock};
use horizon_hooks::players::{
    Badge, BodyPose, Bone as HBone, Frame, Gear, Kind, Light, Look, Mesh, NameDecoration, Particle,
    Pass, PlayerDraw, PlayerModel, PlayerModels, Quad, SpriteStore, Sprites,
};
use rusticmc_companion_core::trail;
use rusticmc_cosmetics_format::bake::BakedPart;
use rusticmc_cosmetics_format::math;
use rusticmc_cosmetics_format::{Bone, Category, Emote, FxAnchor, anim, parse_colour};
use std::sync::Arc;
use std::time::Instant;

pub struct Players;

/// What the executable registers.
pub static PLAYERS: Players = Players;

/// An emote laid over a model's body, from its start (the model's age in ticks).
#[derive(Debug)]
struct RusticPose {
    emote: Emote,
    start: f32,
}

impl BodyPose for RusticPose {
    fn pose(&self, age: f32, at_rest: &[[f32; 9]; 6], poses: &mut [[f32; 9]; 6]) -> bool {
        anim::pose_body(&self.emote, age - self.start, at_rest, poses)
    }
}

fn hbone(b: Bone) -> HBone {
    match b {
        Bone::Head => HBone::Head,
        Bone::Body => HBone::Body,
        Bone::LeftArm => HBone::LeftArm,
        Bone::RightArm => HBone::RightArm,
        Bone::LeftLeg => HBone::LeftLeg,
        Bone::RightLeg => HBone::RightLeg,
    }
}

fn mesh_of(p: &BakedPart) -> Mesh {
    Mesh::new(
        p.name.clone(),
        p.pose,
        p.faces
            .iter()
            .map(|f| Quad::new(f.normal, f.vertices))
            .collect(),
        p.children.iter().map(mesh_of).collect(),
    )
}

/// Gear covers a slot: a helmet a head item, a chestplate a top, an elytra or cape a back item.
fn covered(gear: Gear, c: Category) -> bool {
    match c {
        Category::Head => gear.head,
        Category::Top => gear.chest,
        Category::Back => gear.back,
        _ => false,
    }
}

fn ours(pose: &Arc<dyn BodyPose>) -> bool {
    let any: &dyn std::any::Any = &**pose;
    any.is::<RusticPose>()
}

impl horizon_hooks::Hooks for Players {
    fn frame(&self, _frame: &Frame, sprites: &mut dyn SpriteStore) {
        guard("frame", || {
            let mut st = lock();
            crate::badge::frame(&mut st, sprites);
            let missing = st.store.sprites_missing(|id| sprites.has(id));
            drop(st);
            for l in missing {
                if let Some(slot) = &l.slot {
                    sprites.put(l.sprite, slot.clone());
                }
            }
        });
    }

    fn tick(&self, models: &mut dyn PlayerModels) {
        guard("tick", || {
            let mut list: Vec<PlayerModel> = Vec::new();
            models.for_each(&mut |m| list.push(m));
            let now = Instant::now();
            let mut st = lock();
            if models.connection() != st.connection {
                st.connection = models.connection();
                st.trails.clear();
                st.others.clear();
            }
            st.in_view = list.iter().filter_map(|m| m.uuid).collect();
            // The window's own emote: laid over its model until it ends or the player moves.
            if let Some(own) = list.iter().find(|m| m.kind == Kind::Own) {
                let mut end = false;
                let st = &mut *st;
                if let Some(OwnEmote { item, pose }) = &mut st.own_emote {
                    // Until its bundle arrives, nothing is posed.
                    if let Some(emote) = st
                        .store
                        .get(&item.hash)
                        .and_then(|l| l.cosmetic.emote.clone())
                    {
                        let p = pose.get_or_insert_with(|| {
                            Arc::new(RusticPose {
                                emote: emote.clone(),
                                start: own.age as f32,
                            }) as Arc<dyn BodyPose>
                        });
                        let t = own.age as f32 - p_start(p).unwrap_or(own.age as f32);
                        if own.moved || anim::emote_weight(&emote, t).is_none() {
                            end = true;
                        } else if !models
                            .body_pose(own.id)
                            .is_some_and(|cur| Arc::ptr_eq(cur, p))
                        {
                            models.set_body_pose(own.id, Some(p.clone()));
                        }
                    }
                }
                if end {
                    st.own_emote = None;
                    if models.body_pose(own.id).is_some_and(ours) {
                        models.set_body_pose(own.id, None);
                    }
                    if let Some(n) = crate::state::net() {
                        n.send(crate::net::Job::Playing(None));
                    }
                }
            }
            // Others' emotes, as the cloud says they play them.
            for m in list.iter().filter(|m| m.kind == Kind::Player) {
                let Some(uuid) = m.uuid else { continue };
                let playing = st
                    .worn
                    .playing(uuid, now)
                    .map(|(i, ms)| (i.hash.clone(), ms));
                let loaded = playing
                    .as_ref()
                    .and_then(|(h, _)| st.store.get(h))
                    .and_then(|l| l.cosmetic.emote.clone());
                match (playing, loaded) {
                    (Some((hash, ms)), Some(emote)) if !m.moved => {
                        let known = st.others.get(&uuid).filter(|(h, _)| *h == hash);
                        let pose = match known {
                            Some((_, p)) => p.clone(),
                            None => {
                                let p: Arc<dyn BodyPose> = Arc::new(RusticPose {
                                    emote,
                                    start: m.age as f32 - ms as f32 / 50.0,
                                });
                                st.others.insert(uuid, (hash, p.clone()));
                                p
                            }
                        };
                        if !models
                            .body_pose(m.id)
                            .is_some_and(|cur| Arc::ptr_eq(cur, &pose))
                        {
                            models.set_body_pose(m.id, Some(pose));
                        }
                    }
                    _ => {
                        if st.others.remove(&uuid).is_some()
                            && models.body_pose(m.id).is_some_and(ours)
                        {
                            models.set_body_pose(m.id, None);
                        }
                    }
                }
            }
        });
    }

    /// The Rustic badge in the place of Horizon's mark, for Rustic users (player list and name
    /// tags; Rustic's Horizon patch U14).
    fn name_picture(&self, uuid: u128) -> Option<u64> {
        guard_or("name_picture", None, || {
            crate::badge::picture(&lock(), uuid)
        })
    }

    fn player_layers(&self, model: &PlayerModel, draw: &mut dyn PlayerDraw) {
        guard("player_layers", || draw_player(model, draw));
    }

    fn name_decoration(
        &self,
        model: &PlayerModel,
        sprites: &dyn Sprites,
    ) -> Option<NameDecoration> {
        guard_or("name_decoration", None, || {
            let uuid = model.uuid?;
            let st = lock();
            if model.kind != Kind::Own && !st.cfg.show_others {
                return None;
            }
            let l = st
                .worn
                .worn(uuid)
                .iter()
                .filter_map(|w| st.store.get(&w.hash))
                .find(|l| l.category() == Category::Nametag)?
                .clone();
            drop(st);
            let [r, g, b] = parse_colour(&l.cosmetic.nametag.as_ref()?.line)?;
            let uv = l.uv(sprites.get(l.sprite)?);
            Some(NameDecoration::new(
                None,
                Some(Badge::new(uv, u32::from_be_bytes([0, r, g, b]))),
                None,
            ))
        })
    }
}

/// The start tick of one of our poses.
fn p_start(p: &Arc<dyn BodyPose>) -> Option<f32> {
    let any: &dyn std::any::Any = &**p;
    any.downcast_ref::<RusticPose>().map(|r| r.start)
}

fn draw_player(m: &PlayerModel, draw: &mut dyn PlayerDraw) {
    if m.invisible {
        return;
    }
    let Some(uuid) = m.uuid else {
        return;
    };
    let own = m.kind == Kind::Own;
    let mut st = lock();
    if !own && !st.cfg.show_others {
        return;
    }
    let hide_fx = !own && st.cfg.hide_effects;
    let items: Vec<_> = st
        .worn
        .worn(uuid)
        .iter()
        .filter_map(|w| st.store.get(&w.hash).cloned())
        .collect();
    let ticks = draw.ticks();
    for l in items {
        let cat = l.category();
        if covered(m.gear, cat) || matches!(cat, Category::Nametag | Category::Emote) {
            continue;
        }
        let Some(slot) = draw.sprite(l.sprite) else {
            continue; // in the atlas from the next frame
        };
        let uv = l.uv(slot);
        if cat == Category::Trail {
            if hide_fx || draw.pass() != Pass::World {
                continue;
            }
            let Some(fx) = l.cosmetic.trail.clone() else {
                continue;
            };
            let motion = draw.motion();
            let view = draw.view();
            let emitter = st.trails.entry(uuid).or_default();
            let walking = !motion.crouching && !motion.asleep && !motion.flying;
            emitter.step(motion.feet, ticks, &fx, walking);
            let n = fx.sprites.max(1) as f32;
            let particles: Vec<Particle> = emitter
                .particles
                .iter()
                .map(|p| {
                    let (pos, size, alpha, sprite) = trail::place(p, ticks, &fx);
                    let rel = [
                        (pos[0] - view.camera[0]) as f32,
                        (pos[1] - view.camera[1]) as f32,
                        (pos[2] - view.camera[2]) as f32,
                    ];
                    let h = size / 32.0;
                    let (r, u) = (view.right, view.up);
                    let c = |sx: f32, sy: f32| {
                        [
                            rel[0] + (r[0] * sx + u[0] * sy) * h,
                            rel[1] + (r[1] * sx + u[1] * sy) * h,
                            rel[2] + (r[2] * sx + u[2] * sy) * h,
                        ]
                    };
                    let (u0, u1) = (sprite as f32 / n, (sprite + 1) as f32 / n);
                    Particle::new(
                        [c(-1.0, 1.0), c(-1.0, -1.0), c(1.0, -1.0), c(1.0, 1.0)],
                        [[u0, 0.0], [u0, 1.0], [u1, 1.0], [u1, 0.0]],
                        alpha,
                    )
                })
                .collect();
            let light = if fx.glow { Light::Full } else { Light::Model };
            draw.particles(uv, &particles, light);
            continue;
        }
        if cat == Category::Aura && hide_fx {
            continue;
        }
        let look = match (&l.cosmetic.aura, &l.cosmetic.companion) {
            (Some(a), _) => {
                let lk = Look::new(uv).see_through(a.alpha);
                if a.glow { lk.lit(Light::Full) } else { lk }
            }
            (_, Some(c)) if c.glow => Look::new(uv).lit(Light::Full),
            _ => Look::new(uv),
        };
        let roots: Vec<(Bone, &BakedPart)> = l.baked.roots().collect();
        let parts: Vec<&BakedPart> = roots.iter().map(|(_, p)| *p).collect();
        for (i, (bone, part)) in roots.iter().enumerate() {
            let matrix = match (&l.cosmetic.companion, &l.cosmetic.aura) {
                // Floats beside the player, in the body's space from the neck.
                (Some(c), _) => math::mul(&draw.base(), &math::translation(c.anchor)),
                // At the middle of the feet: 24 pixels under the neck.
                (_, Some(a)) => {
                    let at = match a.at {
                        FxAnchor::Feet | FxAnchor::Body => [0.0, 24.0, 0.0],
                    };
                    math::mul(&draw.base(), &math::translation(at))
                }
                _ => match draw.bone(hbone(*bone)) {
                    Some(m) => m,
                    None => continue,
                },
            };
            let mesh = st
                .meshes
                .entry((l.sprite, i))
                .or_insert_with(|| Arc::new(mesh_of(part)))
                .clone();
            let loops = &l.cosmetic.loops;
            draw.draw(&mesh, matrix, look, &mut |poses| {
                anim::animate(&parts, loops, ticks, |name, pose| *poses.pose(name) = pose);
            });
        }
    }
}
