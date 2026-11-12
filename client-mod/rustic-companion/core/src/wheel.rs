//! The emote wheel: up to 8 owned emotes around the middle of the screen; the slice under the
//! pointer (or the number key 1 to 8) picks one. Geometry only: the game crate draws it with
//! `quad`/`text` on its mod screen.
use std::f32::consts::TAU;

pub const MAX_SLICES: usize = 8;
pub const INNER: f32 = 20.0;
pub const OUTER: f32 = 70.0;

/// The slice under `pointer` for a wheel of `n` slices centred at `centre`; none in the middle
/// or outside. Slice 0 is at the top, going clockwise.
pub fn pick(centre: [f32; 2], pointer: [f32; 2], n: usize) -> Option<usize> {
    let n = n.min(MAX_SLICES);
    if n == 0 {
        return None;
    }
    let (dx, dy) = (pointer[0] - centre[0], pointer[1] - centre[1]);
    let r = (dx * dx + dy * dy).sqrt();
    if !(INNER..=OUTER * 1.5).contains(&r) {
        return None;
    }
    // Angle from the top, clockwise (GUI y points down).
    let a = (dx.atan2(-dy)).rem_euclid(TAU);
    let slice = TAU / n as f32;
    Some((((a + slice / 2.0) / slice) as usize) % n)
}

/// The four corners of slice `i` of `n` (an annular sector drawn as one quad) and where its
/// label goes.
pub fn slice_quad(centre: [f32; 2], i: usize, n: usize) -> ([[f32; 2]; 4], [f32; 2]) {
    let n = n.clamp(1, MAX_SLICES);
    let slice = TAU / n as f32;
    let gap = 0.04;
    let a0 = i as f32 * slice - slice / 2.0 + gap;
    let a1 = i as f32 * slice + slice / 2.0 - gap;
    let at = |a: f32, r: f32| [centre[0] + r * a.sin(), centre[1] - r * a.cos()];
    let mid = i as f32 * slice;
    (
        [at(a0, INNER), at(a0, OUTER), at(a1, OUTER), at(a1, INNER)],
        at(mid, (INNER + OUTER) / 2.0),
    )
}

/// The slice a number key picks (`key.keyboard.1` .. `key.keyboard.8`).
pub fn number_key(key: &str, n: usize) -> Option<usize> {
    let d: usize = key.strip_prefix("key.keyboard.")?.parse().ok()?;
    (1..=n.min(MAX_SLICES)).contains(&d).then(|| d - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_by_direction() {
        let c = [100.0, 100.0];
        assert_eq!(pick(c, [100.0, 50.0], 8), Some(0), "top");
        assert_eq!(pick(c, [150.0, 100.0], 8), Some(2), "right");
        assert_eq!(pick(c, [100.0, 150.0], 8), Some(4), "bottom");
        assert_eq!(pick(c, [50.0, 100.0], 4), Some(3), "left of four");
        assert_eq!(pick(c, [101.0, 101.0], 8), None, "the middle");
        assert_eq!(pick(c, [100.0, 50.0], 0), None);
        assert_eq!(number_key("key.keyboard.3", 8), Some(2));
        assert_eq!(number_key("key.keyboard.9", 8), None);
        assert_eq!(number_key("key.keyboard.3", 2), None);
        let (q, label) = slice_quad(c, 0, 8);
        assert!(q.iter().all(|p| p[1] <= 100.0));
        assert!(label[1] < 100.0);
    }
}
