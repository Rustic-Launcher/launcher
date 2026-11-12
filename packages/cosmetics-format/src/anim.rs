//! Motion: looping part animations and emote keyframes, sampled from time alone (no state, no
//! randomness), so every client draws the same frame for the same tick.
use crate::bake::BakedPart;
use crate::model::{Ease, Emote, LoopAnimation, LoopMotion, Track};
use std::f32::consts::TAU;

/// The rest pose of part `name` under `root`.
pub fn rest_pose(root: &BakedPart, name: &str) -> Option<[f32; 9]> {
    if root.name == name {
        return Some(root.pose);
    }
    root.children.iter().find_map(|c| rest_pose(c, name))
}

/// `rest` moved by every loop in `loops` at `ticks`.
pub fn loop_pose<'a>(
    rest: [f32; 9],
    loops: impl IntoIterator<Item = &'a LoopAnimation>,
    ticks: f32,
) -> [f32; 9] {
    let mut pose = rest;
    for a in loops {
        let turns = ticks / a.period + a.phase;
        let axis = a.axis as usize;
        match a.motion {
            LoopMotion::Swing => pose[3 + axis] += (a.amplitude * (turns * TAU).sin()).to_radians(),
            LoopMotion::Bob => pose[axis] += a.amplitude * (turns * TAU).sin(),
            LoopMotion::Spin => pose[3 + axis] += turns.rem_euclid(1.0) * TAU,
        }
    }
    pose
}

/// Calls `set(part, pose)` for every animated part found under `roots` at `ticks`.
pub fn animate(
    roots: &[&BakedPart],
    loops: &[LoopAnimation],
    ticks: f32,
    mut set: impl FnMut(&str, [f32; 9]),
) {
    let mut done: Vec<&str> = Vec::new();
    for a in loops {
        if done.contains(&a.part.as_str()) {
            continue;
        }
        done.push(&a.part);
        let Some(rest) = roots.iter().find_map(|r| rest_pose(r, &a.part)) else {
            continue;
        };
        let pose = loop_pose(rest, loops.iter().filter(|l| l.part == a.part), ticks);
        set(&a.part, pose);
    }
}

/// One sampled key: offset (pixels), rotation (degrees), scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: f32,
}

impl Sample {
    pub const REST: Sample = Sample {
        offset: [0.0; 3],
        rotation: [0.0; 3],
        scale: 1.0,
    };
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// The track's value at tick `t` of an emote `length` long.
pub fn sample_track(track: &Track, t: f32, looping: bool, length: f32) -> Sample {
    let keys = &track.keys;
    let Some(first) = keys.first() else {
        return Sample::REST;
    };
    let value = |i: usize| {
        let k = &keys[i];
        Sample {
            offset: k.offset,
            rotation: k.rotation,
            scale: k.scale.unwrap_or(1.0),
        }
    };
    if keys.len() == 1 || t <= first.t && !looping {
        return value(0);
    }
    let last = keys.len() - 1;
    // The segment [i, i + 1] holding t; a loop wraps from the last key to the first.
    let (i, j, span, local) = match keys.iter().rposition(|k| k.t <= t) {
        Some(i) if i < last => (i, i + 1, keys[i + 1].t - keys[i].t, t - keys[i].t),
        Some(_) if looping => (last, 0, length - keys[last].t + first.t, t - keys[last].t),
        Some(_) => return value(last),
        None if looping => (
            last,
            0,
            length - keys[last].t + first.t,
            t + length - keys[last].t,
        ),
        None => return value(0),
    };
    let f = if span > 0.0 {
        (local / span).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let (a, b) = (value(i), value(j));
    match keys[i].ease {
        Ease::Step => a,
        Ease::Linear => blend(a, b, f),
        Ease::InOut => blend(a, b, f * f * (3.0 - 2.0 * f)),
        Ease::Smooth => {
            let prev = if i > 0 {
                value(i - 1)
            } else if looping {
                value(last)
            } else {
                a
            };
            let next = if j < last {
                value(j + 1)
            } else if looping {
                value(0)
            } else {
                b
            };
            let c = |p: [f32; 3], q: [f32; 3], r: [f32; 3], s: [f32; 3]| -> [f32; 3] {
                std::array::from_fn(|k| catmull(p[k], q[k], r[k], s[k], f))
            };
            Sample {
                offset: c(prev.offset, a.offset, b.offset, next.offset),
                rotation: c(prev.rotation, a.rotation, b.rotation, next.rotation),
                scale: catmull(prev.scale, a.scale, b.scale, next.scale, f),
            }
        }
    }
}

fn blend(a: Sample, b: Sample, f: f32) -> Sample {
    Sample {
        offset: lerp3(a.offset, b.offset, f),
        rotation: lerp3(a.rotation, b.rotation, f),
        scale: a.scale + (b.scale - a.scale) * f,
    }
}

/// How much of the emote shows `t` ticks after it started (0 to 1): it fades in over `blend` and
/// a one-shot fades out over its last `blend` ticks. None once a one-shot has ended.
pub fn emote_weight(emote: &Emote, t: f32) -> Option<f32> {
    if t < 0.0 || (!emote.looping && t > emote.length) {
        return None;
    }
    let b = emote.blend.max(f32::EPSILON);
    let fade_in = (t / b).min(1.0);
    let fade_out = if emote.looping {
        1.0
    } else {
        ((emote.length - t) / b).min(1.0)
    };
    Some(fade_in.min(fade_out).clamp(0.0, 1.0))
}

/// The emote's local time at `t` ticks after it started (wrapped for a loop).
pub fn emote_time(emote: &Emote, t: f32) -> f32 {
    if emote.looping && emote.length > 0.0 {
        t.rem_euclid(emote.length)
    } else {
        t.min(emote.length)
    }
}

/// The six body parts' order in the game's body pose (`horizon_hooks::players::BODY_PARTS`).
pub const BODY_PARTS: [&str; 6] = [
    "head",
    "body",
    "right_arm",
    "left_arm",
    "right_leg",
    "left_leg",
];

/// Poses the body at `t` ticks after the emote started: `at_rest` holds each part before the
/// animation, `poses` as the vanilla animation left it (x, y, z, rotation in radians, scale).
/// A bone's key replaces its rotation and moves it from rest; `root` moves every part. Returns
/// false when the emote shows nothing at `t`.
pub fn pose_body(
    emote: &Emote,
    t: f32,
    at_rest: &[[f32; 9]; 6],
    poses: &mut [[f32; 9]; 6],
) -> bool {
    let Some(w) = emote_weight(emote, t) else {
        return false;
    };
    let local = emote_time(emote, t);
    let root = emote
        .tracks
        .iter()
        .find(|tr| tr.part == "root")
        .map(|tr| sample_track(tr, local, emote.looping, emote.length));
    for (i, name) in BODY_PARTS.iter().enumerate() {
        let track = emote.tracks.iter().find(|tr| tr.part == *name);
        let s = track.map(|tr| sample_track(tr, local, emote.looping, emote.length));
        let pose = &mut poses[i];
        if let Some(s) = s {
            for k in 0..3 {
                let target = at_rest[i][k] + s.offset[k];
                pose[k] += (target - pose[k]) * w;
                let rot = s.rotation[k].to_radians();
                pose[3 + k] += (rot - pose[3 + k]) * w;
            }
        }
        if let Some(r) = root {
            for (p, o) in pose.iter_mut().zip(r.offset) {
                *p += o * w;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Axis, Key};

    fn key(t: f32, rz: f32, ease: Ease) -> Key {
        Key {
            t,
            offset: [0.0; 3],
            rotation: [0.0, 0.0, rz],
            scale: None,
            ease,
        }
    }

    #[test]
    fn loops_are_a_function_of_time() {
        let swing = LoopAnimation {
            part: "p".into(),
            motion: LoopMotion::Swing,
            axis: Axis::Z,
            amplitude: 10.0,
            period: 40.0,
            phase: 0.0,
        };
        let rest = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let quarter = loop_pose(rest, [&swing], 10.0);
        assert!((quarter[5] - 10f32.to_radians()).abs() < 1e-5);
        assert_eq!(loop_pose(rest, [&swing], 10.0), quarter, "deterministic");
    }

    #[test]
    fn keys_interpolate_with_their_ease() {
        let track = Track {
            part: "right_arm".into(),
            keys: vec![
                key(0.0, 0.0, Ease::Linear),
                key(10.0, 100.0, Ease::Step),
                key(20.0, 0.0, Ease::Linear),
            ],
        };
        assert_eq!(sample_track(&track, 5.0, false, 20.0).rotation[2], 50.0);
        assert_eq!(sample_track(&track, 15.0, false, 20.0).rotation[2], 100.0);
        assert_eq!(sample_track(&track, 30.0, false, 20.0).rotation[2], 0.0);
    }

    #[test]
    fn weight_fades_in_and_out() {
        let e = Emote {
            length: 40.0,
            looping: false,
            blend: 4.0,
            tracks: vec![],
        };
        assert_eq!(emote_weight(&e, 2.0), Some(0.5));
        assert_eq!(emote_weight(&e, 20.0), Some(1.0));
        assert_eq!(emote_weight(&e, 39.0), Some(0.25));
        assert_eq!(emote_weight(&e, 41.0), None);
    }

    #[test]
    fn body_pose_moves_the_named_bone() {
        let e = Emote {
            length: 40.0,
            looping: true,
            blend: 4.0,
            tracks: vec![Track {
                part: "right_arm".into(),
                keys: vec![key(0.0, 90.0, Ease::Linear)],
            }],
        };
        let rest = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]; 6];
        let mut poses = rest;
        assert!(pose_body(&e, 10.0, &rest, &mut poses));
        assert!((poses[2][5] - 90f32.to_radians()).abs() < 1e-5);
        assert_eq!(poses[0], rest[0]);
    }
}
