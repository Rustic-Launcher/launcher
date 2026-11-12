//! Small 4x4 matrix helpers, column-major (`m[column][row]`), as the game's hooks take them.

pub type Mat4 = [[f32; 4]; 4];

pub const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for (c, col) in out.iter_mut().enumerate() {
        for (r, cell) in col.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    out
}

pub fn translation(t: [f32; 3]) -> Mat4 {
    let mut m = IDENTITY;
    m[3][0] = t[0];
    m[3][1] = t[1];
    m[3][2] = t[2];
    m
}

pub fn scale(s: [f32; 3]) -> Mat4 {
    let mut m = IDENTITY;
    m[0][0] = s[0];
    m[1][1] = s[1];
    m[2][2] = s[2];
    m
}

fn rot_x(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, c, s, 0.0],
        [0.0, -s, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn rot_y(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    [
        [c, 0.0, -s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn rot_z(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    [
        [c, s, 0.0, 0.0],
        [-s, c, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Rotation by radians `r`, Z then Y then X (vanilla's `rotationZYX`): `Rz * Ry * Rx`.
pub fn rotation_zyx(r: [f32; 3]) -> Mat4 {
    mul(&mul(&rot_z(r[2]), &rot_y(r[1])), &rot_x(r[0]))
}

/// A part pose `[x, y, z, rx, ry, rz (radians), sx, sy, sz]` as a matrix: translate, rotate,
/// scale (`ModelPart.translateAndRotate`).
pub fn pose_matrix(p: &[f32; 9]) -> Mat4 {
    mul(
        &mul(
            &translation([p[0], p[1], p[2]]),
            &rotation_zyx([p[3], p[4], p[5]]),
        ),
        &scale([p[6], p[7], p[8]]),
    )
}

pub fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (r, o) in out.iter_mut().enumerate() {
        *o = m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r];
    }
    out
}

pub fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

pub fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    #[test]
    fn pose_translates_rotates_scales() {
        let m = pose_matrix(&[
            1.0,
            2.0,
            3.0,
            0.0,
            std::f32::consts::FRAC_PI_2,
            0.0,
            2.0,
            2.0,
            2.0,
        ]);
        // (1, 0, 0) scaled to (2, 0, 0), turned 90 degrees about y to (0, 0, -2), then moved.
        assert!(close(transform_point(&m, [1.0, 0.0, 0.0]), [1.0, 2.0, 1.0]));
        assert!(close(
            transform_point(&IDENTITY, [4.0, 5.0, 6.0]),
            [4.0, 5.0, 6.0]
        ));
    }
}
