//! Trail particles: spawned by distance walked (never by time or chance) and placed from their
//! age alone, so two clients that see the same walk draw the same trail. Each particle's
//! variation (sprite, size, sway) comes from its index, not from a random generator.
use rusticmc_cosmetics_format::TrailFx;

/// Particles per player at most, and the distance at which a jump (a teleport) spawns nothing.
pub const MAX_PER_PLAYER: usize = 24;
pub const TELEPORT: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// Where it was born (blocks, world space).
    pub origin: [f64; 3],
    /// The tick it was born.
    pub born: f32,
    /// Its index since the emitter started: what its variation is derived from.
    pub index: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Emitter {
    last: Option<[f64; 3]>,
    /// Distance walked not yet spent on a particle (blocks).
    carry: f64,
    count: u64,
    pub particles: Vec<Particle>,
}

/// A fixed spread in [0, 1) from an index (a hash, not a random draw).
pub fn spread(index: u64, salt: u64) -> f32 {
    let mut x =
        index.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

impl Emitter {
    /// The wearer's feet at tick `now`; spawns along the path walked since the last call.
    pub fn step(&mut self, feet: [f64; 3], now: f32, fx: &TrailFx, moving: bool) {
        let life = fx.lifetime as f32;
        self.particles.retain(|p| now - p.born < life);
        let Some(last) = self.last.replace(feet) else {
            return;
        };
        let d = [feet[0] - last[0], feet[1] - last[1], feet[2] - last[2]];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        // Drawn more than once a frame (passes): the same place spawns nothing and keeps the
        // distance carried over.
        if dist == 0.0 {
            return;
        }
        if !moving || dist >= TELEPORT {
            self.carry = 0.0;
            return;
        }
        let every = 1.0 / f64::from(fx.rate.max(0.01));
        self.carry += dist;
        while self.carry >= every {
            self.carry -= every;
            // Placed back along the step by what is left over.
            let back = self.carry / dist;
            let origin = [
                feet[0] - d[0] * back,
                feet[1] - d[1] * back,
                feet[2] - d[2] * back,
            ];
            if self.particles.len() < MAX_PER_PLAYER {
                self.particles.push(Particle {
                    origin,
                    born: now,
                    index: self.count,
                });
            }
            self.count += 1;
        }
    }
}

/// Where particle `p` is at tick `now` (blocks), its side (pixels), alpha and sprite index.
pub fn place(p: &Particle, now: f32, fx: &TrailFx) -> ([f64; 3], f32, f32, u32) {
    let age = (now - p.born).max(0.0);
    let life = fx.lifetime.max(1) as f32;
    let sway = (spread(p.index, 1) - 0.5) * 2.0 * fx.drift / 16.0;
    let phase = spread(p.index, 2) * std::f32::consts::TAU;
    let x = p.origin[0] + f64::from(sway * (age * 0.2 + phase).sin());
    let y = p.origin[1] - f64::from(0.5 * fx.gravity * age * age);
    let z = p.origin[2] + f64::from(sway * (age * 0.2 + phase).cos());
    let size = fx.size[0] + (fx.size[1] - fx.size[0]) * spread(p.index, 3);
    let alpha = if fx.fade && age > life * 2.0 / 3.0 {
        ((life - age) / (life / 3.0)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let sprite = (spread(p.index, 4) * fx.sprites.max(1) as f32) as u32 % fx.sprites.max(1);
    ([x, y, z], size, alpha, sprite)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusticmc_cosmetics_format::FxAnchor;

    fn fx() -> TrailFx {
        TrailFx {
            sprites: 4,
            rate: 2.0,
            lifetime: 30,
            size: [1.5, 3.0],
            gravity: 0.01,
            drift: 1.5,
            spin: 0.0,
            from: FxAnchor::Feet,
            fade: true,
            glow: false,
        }
    }

    #[test]
    fn spawns_by_distance_only_and_identically() {
        let f = fx();
        let mut a = Emitter::default();
        let mut b = Emitter::default();
        for e in [&mut a, &mut b] {
            e.step([0.0, 64.0, 0.0], 0.0, &f, true);
            // Standing still for ten ticks: nothing.
            for t in 1..10 {
                e.step([0.0, 64.0, 0.0], t as f32, &f, true);
            }
            assert!(e.particles.is_empty());
            // Two blocks walked: four particles at 2 per block.
            e.step([1.0, 64.0, 0.0], 10.0, &f, true);
            e.step([2.0, 64.0, 0.0], 11.0, &f, true);
        }
        assert_eq!(a.particles.len(), 4);
        assert_eq!(a.particles, b.particles);
        let (p1, s1, _, sp1) = place(&a.particles[0], 20.0, &f);
        let (p2, s2, _, sp2) = place(&b.particles[0], 20.0, &f);
        assert_eq!((p1, s1, sp1), (p2, s2, sp2));
        assert!((1.5..=3.0).contains(&s1));
        // A teleport spawns nothing; old particles die after their lifetime.
        a.step([50.0, 64.0, 0.0], 12.0, &f, true);
        assert_eq!(a.particles.len(), 4);
        a.step([50.0, 64.0, 0.0], 45.0, &f, true);
        assert!(a.particles.is_empty());
    }
}
