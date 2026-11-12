//! Baking: a cosmetic's boxes (or mesh groups) turned into textured quads in pixels, as the
//! game's `ModelPart.Cube` makes them, ready for a renderer. Mirrors Horizon's own baking
//! (`horizon_cosmetics::item`'s box faces), so a converted item draws the same in both.
use crate::model::{Bone, Category, Cosmetic, Cube, Model, Part};
use crate::texture::Texture;

/// One quad: its normal, then four corners `x, y, z` (pixels) and `u, v` (0 to 1 across the
/// texture), in the game's `Polygon` order.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub normal: [f32; 3],
    pub vertices: [[f32; 5]; 4],
}

/// A posed part: `[x, y, z, rx, ry, rz (radians), sx, sy, sz]`, its faces and children.
#[derive(Clone, Debug, PartialEq)]
pub struct BakedPart {
    pub name: String,
    pub pose: [f32; 9],
    pub faces: Vec<Face>,
    pub children: Vec<BakedPart>,
}

/// A cosmetic ready to draw: roots per bone, the texture and the motion data.
#[derive(Clone, Debug, PartialEq)]
pub struct Baked {
    pub id: String,
    pub version: String,
    pub category: Category,
    /// The bone `root` rides (head when the category rides none).
    pub bone: Bone,
    /// Named `cosmetic`, posed by the attachment.
    pub root: BakedPart,
    /// Top-level parts riding another bone, under a copy of the attachment pose.
    pub limbs: Vec<(Bone, BakedPart)>,
    pub texture: Option<Texture>,
    pub vertices: usize,
}

impl Baked {
    /// Every root with its bone: `root` first, then the limbs.
    pub fn roots(&self) -> impl Iterator<Item = (Bone, &BakedPart)> {
        std::iter::once((self.bone, &self.root)).chain(self.limbs.iter().map(|(b, p)| (*b, p)))
    }
}

/// Bakes `cosmetic` with its decoded `texture` (None for an emote without props). Validation
/// (`crate::validate`) is separate: baking only needs a well-formed model.
pub fn bake(cosmetic: &Cosmetic, texture: Option<Texture>) -> Baked {
    let (bone, offset, rotation, scale) = match &cosmetic.attachment {
        Some(a) => {
            let (bone, offset) = a.resolved();
            (bone, offset, a.rotation, a.scale)
        }
        None => (Bone::Head, [0.0; 3], [0.0; 3], 1.0),
    };
    let [rx, ry, rz] = rotation.map(f32::to_radians);
    let pose = [
        offset[0], offset[1], offset[2], rx, ry, rz, scale, scale, scale,
    ];
    let size = texture
        .as_ref()
        .map_or([64.0, 64.0], |t| [t.width as f32, t.height as f32]);
    let mut children = Vec::new();
    let mut limbs: Vec<(Bone, BakedPart)> = Vec::new();
    match &cosmetic.model {
        Some(Model::Cubes { parts }) => {
            for p in parts {
                let baked = bake_part(p, size);
                match p
                    .bone
                    .filter(|b| *b != bone && cosmetic.category.rides_bone())
                {
                    None => children.push(baked),
                    Some(other) => match limbs.iter_mut().find(|(b, _)| *b == other) {
                        Some((_, limb)) => limb.children.push(baked),
                        None => limbs.push((
                            other,
                            BakedPart {
                                name: format!("cosmetic_{}", other.part()),
                                pose,
                                faces: Vec::new(),
                                children: vec![baked],
                            },
                        )),
                    },
                }
            }
        }
        Some(Model::Mesh(mesh)) => {
            for g in &mesh.groups {
                let faces = g
                    .triangles
                    .iter()
                    .map(|t| {
                        let [a, b, c] = *t;
                        let normal = triangle_normal(a, b, c);
                        // A triangle as a quad whose last corner repeats its third.
                        Face {
                            normal,
                            vertices: [
                                rel(a, g.pivot),
                                rel(b, g.pivot),
                                rel(c, g.pivot),
                                rel(c, g.pivot),
                            ],
                        }
                    })
                    .collect();
                children.push(BakedPart {
                    name: g.name.clone(),
                    pose: [
                        g.pivot[0], g.pivot[1], g.pivot[2], 0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
                    ],
                    faces,
                    children: Vec::new(),
                });
            }
        }
        None => {}
    }
    let root = BakedPart {
        name: "cosmetic".into(),
        pose,
        faces: Vec::new(),
        children,
    };
    let vertices = count(&root) + limbs.iter().map(|(_, l)| count(l)).sum::<usize>();
    Baked {
        id: cosmetic.id.clone(),
        version: cosmetic.version.clone(),
        category: cosmetic.category,
        bone,
        root,
        limbs,
        texture,
        vertices,
    }
}

fn rel(v: [f32; 5], pivot: [f32; 3]) -> [f32; 5] {
    [
        v[0] - pivot[0],
        v[1] - pivot[1],
        v[2] - pivot[2],
        v[3],
        v[4],
    ]
}

fn triangle_normal(a: [f32; 5], b: [f32; 5], c: [f32; 5]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let l = crate::math::length(n);
    if l > 0.0 {
        n.map(|x| x / l)
    } else {
        [0.0, -1.0, 0.0]
    }
}

/// Vertices drawn: four per face.
pub fn count(p: &BakedPart) -> usize {
    p.faces.len() * 4 + p.children.iter().map(count).sum::<usize>()
}

fn bake_part(p: &Part, size: [f32; 2]) -> BakedPart {
    let [rx, ry, rz] = p.rotation.map(f32::to_radians);
    let [x, y, z] = p.pivot;
    BakedPart {
        name: p.name.clone(),
        pose: [x, y, z, rx, ry, rz, 1.0, 1.0, 1.0],
        faces: p.cubes.iter().flat_map(|c| box_faces(c, size)).collect(),
        children: p.children.iter().map(|c| bake_part(c, size)).collect(),
    }
}

/// `ModelPart.Cube`: six faces with box UV, in vanilla's order (down, up, west, north, east,
/// south), grown by `inflate`, mirrored in x on request.
pub fn box_faces(cube: &Cube, [tw, th]: [f32; 2]) -> Vec<Face> {
    let [w, h, d] = cube.size;
    let g = cube.inflate;
    let (mut x0, y0, z0) = (cube.origin[0] - g, cube.origin[1] - g, cube.origin[2] - g);
    let (mut x1, y1, z1) = (
        cube.origin[0] + w + g,
        cube.origin[1] + h + g,
        cube.origin[2] + d + g,
    );
    if cube.mirror {
        std::mem::swap(&mut x0, &mut x1);
    }
    let t0 = [x0, y0, z0];
    let t1 = [x1, y0, z0];
    let t2 = [x1, y1, z0];
    let t3 = [x0, y1, z0];
    let l0 = [x0, y0, z1];
    let l1 = [x1, y0, z1];
    let l2 = [x1, y1, z1];
    let l3 = [x0, y1, z1];
    let [u, v] = cube.uv;
    let (u1, u2, u22) = (u + d, u + d + w, u + d + w + w);
    let (u3, u4) = (u + d + w + d, u + d + w + d + w);
    let (v1, v2) = (v + d, v + d + h);
    type FaceSpec = ([[f32; 3]; 4], [f32; 4], [f32; 3]);
    let faces: [FaceSpec; 6] = [
        ([l1, l0, t0, t1], [u1, v, u2, v1], [0.0, -1.0, 0.0]),
        ([t2, t3, l3, l2], [u2, v1, u22, v], [0.0, 1.0, 0.0]),
        ([t0, l0, l3, t3], [u, v1, u1, v2], [-1.0, 0.0, 0.0]),
        ([t1, t0, t3, t2], [u1, v1, u2, v2], [0.0, 0.0, -1.0]),
        ([l1, t1, t2, l2], [u2, v1, u3, v2], [1.0, 0.0, 0.0]),
        ([l0, l1, l2, l3], [u3, v1, u4, v2], [0.0, 0.0, 1.0]),
    ];
    faces
        .into_iter()
        .map(|(corners, [a, b, c, e], mut normal)| {
            let uvs = [[c, b], [a, b], [a, e], [c, e]];
            let mut vertices: [[f32; 5]; 4] = std::array::from_fn(|i| {
                let [x, y, z] = corners[i];
                [x, y, z, uvs[i][0] / tw, uvs[i][1] / th]
            });
            if cube.mirror {
                vertices.reverse();
                normal[0] = -normal[0];
            }
            Face { normal, vertices }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn hat() -> Cosmetic {
        Cosmetic {
            format: 1,
            id: "hat".into(),
            version: "1.0.0".into(),
            name: "Hat".into(),
            description: String::new(),
            authors: vec![],
            category: Category::Top,
            attachment: Some(Attachment {
                point: AttachmentPoint::Back,
                offset: [0.0, 1.0, 0.0],
                rotation: [0.0; 3],
                scale: 1.0,
            }),
            texture: None,
            model: Some(Model::Cubes {
                parts: vec![
                    Part {
                        name: "a".into(),
                        bone: None,
                        pivot: [0.0; 3],
                        rotation: [0.0; 3],
                        cubes: vec![Cube {
                            origin: [-4.0, 0.0, -2.0],
                            size: [8.0, 12.0, 4.0],
                            uv: [0.0, 0.0],
                            inflate: 0.25,
                            mirror: false,
                        }],
                        children: vec![],
                    },
                    Part {
                        name: "sleeve".into(),
                        bone: Some(Bone::RightArm),
                        pivot: [0.0; 3],
                        rotation: [0.0; 3],
                        cubes: vec![Cube {
                            origin: [-3.0, -2.0, -2.0],
                            size: [4.0, 4.0, 4.0],
                            uv: [24.0, 0.0],
                            inflate: 0.0,
                            mirror: true,
                        }],
                        children: vec![],
                    },
                ],
            }),
            loops: vec![],
            emote: None,
            companion: None,
            trail: None,
            aura: None,
            nametag: None,
        }
    }

    #[test]
    fn boxes_bake_to_six_faces_and_limbs_split() {
        let b = bake(&hat(), None);
        assert_eq!(b.bone, Bone::Body);
        // Back's base (0, 0, 2) plus the offset.
        assert_eq!(&b.root.pose[0..3], &[0.0, 1.0, 2.0]);
        assert_eq!(b.root.children.len(), 1);
        assert_eq!(b.limbs.len(), 1);
        assert_eq!(b.limbs[0].0, Bone::RightArm);
        assert_eq!(b.limbs[0].1.name, "cosmetic_right_arm");
        assert_eq!(b.vertices, 48);
        let up = &b.root.children[0].faces[1];
        assert_eq!(up.normal, [0.0, 1.0, 0.0]);
        // Inflate grows the box on every side.
        assert!(
            b.root.children[0].faces[0]
                .vertices
                .iter()
                .any(|v| v[0] == -4.25)
        );
        // A mirrored box flips its normals in x.
        assert_eq!(b.limbs[0].1.children[0].faces[2].normal, [1.0, 0.0, 0.0]);
    }
}
