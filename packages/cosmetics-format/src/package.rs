//! A cosmetic package: `cosmetic.json` plus the files it names (the PNG texture, an optional
//! glTF mesh), as an author's folder or a zip's entries; and the self-contained bundle the CDN
//! serves (texture embedded, triangles inline).
use crate::model::{Cosmetic, Model, TextureSource};
use crate::texture::{Texture, decode_base64, encode_base64};
use crate::validate::{self, Report};
use std::collections::BTreeMap;
use std::path::Path;

/// The manifest's file name in a package.
pub const MANIFEST: &str = "cosmetic.json";

#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("{MANIFEST}: {0}")]
    Manifest(String),
    #[error("missing file {0:?}")]
    Missing(String),
    #[error("unsafe path {0:?}: relative, `/`-separated, no `..`")]
    UnsafePath(String),
    #[error("texture: {0}")]
    Texture(String),
    #[error("mesh: {0}")]
    Mesh(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// A loaded package: the cosmetic, its texture's PNG bytes and decoded pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub cosmetic: Cosmetic,
    pub png: Option<Vec<u8>>,
    pub texture: Option<Texture>,
    /// Files the manifest never names (left out of a bundle).
    pub unused: Vec<String>,
}

/// A path inside a package: relative, `/`-separated, no empty, `.` or `..` part.
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}

impl Package {
    /// Reads a package from its files by path (a folder's or a zip's entries).
    pub fn from_files(files: &BTreeMap<String, Vec<u8>>) -> Result<Package, PackageError> {
        for name in files.keys() {
            if !safe_path(name) {
                return Err(PackageError::UnsafePath(name.clone()));
            }
        }
        let manifest = files
            .get(MANIFEST)
            .ok_or_else(|| PackageError::Missing(MANIFEST.into()))?;
        let mut cosmetic: Cosmetic =
            serde_json::from_slice(manifest).map_err(|e| PackageError::Manifest(e.to_string()))?;
        let mut used = vec![MANIFEST.to_owned()];
        let png = match &cosmetic.texture {
            Some(TextureSource {
                png: Some(b64),
                path: None,
            }) => Some(decode_base64(b64).map_err(|e| PackageError::Texture(e.to_string()))?),
            Some(TextureSource {
                path: Some(path),
                png: None,
            }) => {
                if !safe_path(path) {
                    return Err(PackageError::UnsafePath(path.clone()));
                }
                used.push(path.clone());
                Some(
                    files
                        .get(path)
                        .cloned()
                        .ok_or_else(|| PackageError::Missing(path.clone()))?,
                )
            }
            Some(_) => {
                return Err(PackageError::Texture(
                    "give the texture's `path` or its `png`, not both".into(),
                ));
            }
            None => None,
        };
        let texture = match &png {
            Some(bytes) => {
                Some(Texture::decode_png(bytes).map_err(|e| PackageError::Texture(e.to_string()))?)
            }
            None => None,
        };
        if let Some(Model::Mesh(mesh)) = &mut cosmetic.model
            && let Some(source) = mesh.source.clone()
        {
            if !safe_path(&source) {
                return Err(PackageError::UnsafePath(source));
            }
            let bytes = files
                .get(&source)
                .ok_or_else(|| PackageError::Missing(source.clone()))?;
            used.push(source.clone());
            if mesh.groups.is_empty() {
                mesh.groups = import_mesh(bytes, mesh.units_to_pixels)?;
            }
        }
        let unused = files
            .keys()
            .filter(|k| !used.contains(k))
            .cloned()
            .collect();
        Ok(Package {
            cosmetic,
            png,
            texture,
            unused,
        })
    }

    /// Reads a package folder (every file under it).
    pub fn from_dir(dir: &Path) -> Result<Package, PackageError> {
        let mut files = BTreeMap::new();
        read_dir(dir, dir, &mut files)?;
        Package::from_files(&files)
    }

    /// Reads a self-contained bundle (what the CDN serves).
    pub fn from_bundle(bytes: &[u8]) -> Result<Package, PackageError> {
        let mut files = BTreeMap::new();
        files.insert(MANIFEST.to_owned(), bytes.to_vec());
        Package::from_files(&files)
    }

    /// Validates the package.
    pub fn validate(&self) -> Report {
        validate::validate(
            &self.cosmetic,
            self.texture.as_ref(),
            self.png.as_ref().map_or(0, Vec::len),
        )
    }

    /// The cosmetic with its texture embedded and its mesh inline: one file, nothing to resolve.
    pub fn bundle(&self) -> Cosmetic {
        let mut c = self.cosmetic.clone();
        c.texture = self.png.as_ref().map(|png| TextureSource {
            path: None,
            png: Some(encode_base64(png)),
        });
        if let Some(Model::Mesh(m)) = &mut c.model {
            m.source = None;
        }
        c
    }

    /// The bundle as canonical JSON bytes (stable key order and formatting): what is hashed.
    pub fn bundle_bytes(&self) -> Vec<u8> {
        canonical_json(&self.bundle())
    }
}

/// `value` as compact JSON with object keys in a stable order.
pub fn canonical_json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    // serde_json's Value keeps keys sorted (BTreeMap) without the preserve_order feature.
    let v = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    serde_json::to_vec(&v).unwrap_or_default()
}

fn read_dir(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            read_dir(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(rel, std::fs::read(&path)?);
        }
    }
    Ok(())
}

/// Reads a glTF 2.0 file (`.glb`, or `.gltf` with `data:` buffers) into mesh groups: one group
/// per node with a mesh, its triangles in pixels (y flipped to point down, as model space).
#[cfg(feature = "gltf")]
pub fn import_mesh(
    bytes: &[u8],
    units_to_pixels: f32,
) -> Result<Vec<crate::model::MeshGroup>, PackageError> {
    use crate::model::MeshGroup;
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| PackageError::Mesh(e.to_string()))?;
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    for buffer in gltf.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => gltf
                .blob
                .clone()
                .ok_or_else(|| PackageError::Mesh("a .glb without its binary chunk".into()))?,
            gltf::buffer::Source::Uri(uri) => {
                let b64 = uri
                    .strip_prefix("data:")
                    .and_then(|rest| rest.split_once(";base64,"))
                    .map(|(_, data)| data)
                    .ok_or_else(|| {
                        PackageError::Mesh(format!(
                            "external buffer {uri:?}: export with embedded buffers or as .glb"
                        ))
                    })?;
                decode_base64(b64).map_err(|e| PackageError::Mesh(e.to_string()))?
            }
        };
        buffers.push(data);
    }
    // Every node of every scene with its world matrix (parents first).
    let mut stack: Vec<(gltf::Node<'_>, crate::math::Mat4)> = gltf
        .scenes()
        .flat_map(|s| s.nodes())
        .map(|n| (n, crate::math::IDENTITY))
        .collect();
    let mut placed = Vec::new();
    while let Some((node, parent)) = stack.pop() {
        let world = crate::math::mul(&parent, &node.transform().matrix());
        for child in node.children() {
            stack.push((child, world));
        }
        placed.push((node, world));
    }
    placed.sort_by_key(|(n, _)| n.index());
    let mut groups = Vec::new();
    for (node, matrix) in placed {
        let Some(mesh) = node.mesh() else { continue };
        let name = node
            .name()
            .or(mesh.name())
            .map(|n| n.to_owned())
            .unwrap_or_else(|| format!("mesh_{}", node.index()));
        let mut triangles = Vec::new();
        for prim in mesh.primitives() {
            if prim.mode() != gltf::mesh::Mode::Triangles {
                return Err(PackageError::Mesh(format!(
                    "{name}: only triangle primitives"
                )));
            }
            let reader = prim.reader(|b| buffers.get(b.index()).map(|v| v.as_slice()));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| PackageError::Mesh(format!("{name}: no positions")))?
                .collect();
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|t| t.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
            let indices: Vec<u32> = match reader.read_indices() {
                Some(i) => i.into_u32().collect(),
                None => (0..positions.len() as u32).collect(),
            };
            let vertex = |i: u32| -> Result<[f32; 5], PackageError> {
                let p = positions
                    .get(i as usize)
                    .ok_or_else(|| PackageError::Mesh(format!("{name}: index out of range")))?;
                let uv = uvs.get(i as usize).copied().unwrap_or([0.0, 0.0]);
                let w = transform(&matrix, *p);
                Ok([
                    w[0] * units_to_pixels,
                    -w[1] * units_to_pixels,
                    w[2] * units_to_pixels,
                    uv[0],
                    uv[1],
                ])
            };
            for tri in indices.as_chunks::<3>().0 {
                // Flipping y mirrors the mesh: swap two corners to keep its faces outward.
                triangles.push([vertex(tri[0])?, vertex(tri[2])?, vertex(tri[1])?]);
            }
        }
        groups.push(MeshGroup {
            name,
            pivot: [0.0; 3],
            triangles,
        });
    }
    if groups.is_empty() {
        return Err(PackageError::Mesh("no mesh in the file".into()));
    }
    Ok(groups)
}

#[cfg(feature = "gltf")]
fn transform(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    crate::math::transform_point(m, p)
}

/// Without the `gltf` feature only self-contained bundles (triangles inline) load.
#[cfg(not(feature = "gltf"))]
pub fn import_mesh(
    _bytes: &[u8],
    _units_to_pixels: f32,
) -> Result<Vec<crate::model::MeshGroup>, PackageError> {
    Err(PackageError::Mesh(
        "glTF import is not built in (feature `gltf`)".into(),
    ))
}
