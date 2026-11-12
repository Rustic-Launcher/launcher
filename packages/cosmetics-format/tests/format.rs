use rusticmc_cosmetics_format::horizon::{self, HorizonItem};
use rusticmc_cosmetics_format::texture::{Texture, encode_base64};
use rusticmc_cosmetics_format::validate::Code;
use rusticmc_cosmetics_format::*;
use std::collections::BTreeMap;

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut rgba = Vec::new();
    for i in 0..w * h {
        rgba.extend([(i % 251) as u8, 80, 200, 255]);
    }
    Texture {
        width: w,
        height: h,
        rgba,
    }
    .encode_png()
}

/// Horizon's documented example hat (docs/cosmetics.md), with a real 64x64 PNG.
fn horizon_hat() -> String {
    format!(
        r#"{{
  "format": 1, "id": "example_hat", "slot": "head", "bone": "head",
  "attach": {{ "offset": [0, -8, 0], "rotation": [0, 0, 0], "scale": 1 }},
  "texture": {{ "png": "{}" }},
  "parts": [
    {{ "name": "brim", "pivot": [0, 0, 0], "rotation": [0, 0, 0],
      "cubes": [{{ "origin": [-4, -2, -4], "size": [8, 2, 8], "uv": [0, 0], "inflate": 0, "mirror": false }}],
      "children": [] }}
  ],
  "animations": [
    {{ "part": "brim", "motion": "swing", "axis": "z", "amplitude": 10, "period": 40, "phase": 0 }}
  ]
}}"#,
        encode_base64(&png(64, 64))
    )
}

fn files(manifest: &str, extra: &[(&str, Vec<u8>)]) -> BTreeMap<String, Vec<u8>> {
    let mut f = BTreeMap::new();
    f.insert("cosmetic.json".to_owned(), manifest.as_bytes().to_vec());
    for (k, v) in extra {
        f.insert((*k).to_owned(), v.clone());
    }
    f
}

const RUSTIC_HAT: &str = r#"{
  "format": 1, "id": "top_hat", "version": "1.2.0", "name": "Top Hat",
  "authors": ["Rustic"], "category": "head",
  "attachment": { "point": "head", "offset": [0, -8, 0] },
  "texture": { "path": "textures/top_hat.png" },
  "model": { "kind": "cubes", "parts": [
    { "name": "brim", "cubes": [{ "origin": [-5, -1, -5], "size": [10, 1, 10], "uv": [0, 0] }] },
    { "name": "crown", "pivot": [0, -1, 0], "cubes": [{ "origin": [-3, -6, -3], "size": [6, 6, 6], "uv": [0, 11] }] }
  ] },
  "loops": [{ "part": "crown", "motion": "bob", "axis": "y", "amplitude": 0.5, "period": 60 }]
}"#;

#[test]
fn a_good_hat_validates_and_bakes() {
    let pkg = Package::from_files(&files(
        RUSTIC_HAT,
        &[
            ("textures/top_hat.png", png(64, 32)),
            ("notes.txt", b"x".to_vec()),
        ],
    ))
    .unwrap();
    let report = pkg.validate();
    assert!(report.ok(), "{:#?}", report.errors);
    assert_eq!(report.stats.vertices, 48);
    assert_eq!(report.stats.parts, 2);
    assert_eq!(report.stats.texture, Some([64, 32]));
    assert!(report.stats.reach > 8.0 && report.stats.reach < 24.0);
    assert_eq!(pkg.unused, vec!["notes.txt".to_owned()]);
    // The bundle embeds the texture and reloads to the same package.
    let bundle = pkg.bundle_bytes();
    let again = Package::from_bundle(&bundle).unwrap();
    assert_eq!(again.texture, pkg.texture);
    assert_eq!(again.bundle_bytes(), bundle, "canonical bytes are stable");
}

#[test]
fn horizon_round_trip_is_lossless() {
    let original: HorizonItem = horizon::parse_horizon(horizon_hat().as_bytes()).unwrap();
    let rustic = horizon::from_horizon(&original).unwrap();
    assert_eq!(rustic.category, Category::Head);
    assert_eq!(rustic.attachment.as_ref().unwrap().offset, [0.0, -8.0, 0.0]);
    let back = horizon::to_horizon(&rustic, None).unwrap();
    assert_eq!(
        horizon::normalized(&back.item),
        horizon::normalized(&original)
    );
    // And through JSON text again.
    let text = serde_json::to_vec(&back.item).unwrap();
    let reread = horizon::parse_horizon(&text).unwrap();
    assert_eq!(horizon::normalized(&reread), horizon::normalized(&original));
    // The converted Rustic item is valid in Rustic too (once named and versioned).
    let pkg = Package::from_bundle(&serde_json::to_vec(&rustic).unwrap()).unwrap();
    assert!(pkg.validate().ok(), "{:#?}", pkg.validate().errors);
}

#[test]
fn rustic_to_horizon_reports_what_it_drops() {
    let mut c: Cosmetic = serde_json::from_str(RUSTIC_HAT).unwrap();
    c.attachment = Some(Attachment {
        point: AttachmentPoint::Back,
        offset: [0.0, 1.0, 0.0],
        rotation: [0.0; 3],
        scale: 1.0,
    });
    c.category = Category::Back;
    let out = horizon::to_horizon(&c, Some(&png(64, 32))).unwrap();
    assert_eq!(out.item.slot, horizon::Slot::Back);
    assert_eq!(out.item.bone, Some(Bone::Body));
    assert_eq!(out.item.attach.as_ref().unwrap().offset, [0.0, 1.0, 2.0]);
    assert!(out.dropped.iter().any(|d| d.contains("version 1.2.0")));
    assert!(out.dropped.iter().any(|d| d.contains("Back")));
    // A path texture with no bytes given can't be embedded.
    assert_eq!(
        horizon::to_horizon(&c, None).unwrap_err(),
        horizon::ConvertError::NoTexture
    );
}

#[test]
fn every_horizon_slot_maps_both_ways() {
    for c in Category::ALL {
        assert_eq!(horizon::Slot::of(c).category(), c);
    }
}

#[test]
fn capes_are_refused_by_name_and_by_shape() {
    let named = RUSTIC_HAT
        .replace("\"top_hat\"", "\"hero_cape\"")
        .replace("\"head\"", "\"back\"");
    let pkg =
        Package::from_files(&files(&named, &[("textures/top_hat.png", png(64, 32))])).unwrap();
    assert!(
        pkg.validate()
            .errors
            .iter()
            .any(|e| e.code == Code::CapeLike)
    );

    let panel = r#"{
      "format": 1, "id": "banner", "version": "1.0.0", "name": "Banner", "category": "back",
      "attachment": { "point": "back" },
      "texture": { "path": "t.png" },
      "model": { "kind": "cubes", "parts": [
        { "name": "panel", "cubes": [{ "origin": [-5, 0, 0], "size": [10, 16, 1], "uv": [0, 0] }] }
      ] }
    }"#;
    let pkg = Package::from_files(&files(panel, &[("t.png", png(64, 32))])).unwrap();
    let report = pkg.validate();
    assert!(
        report.errors.iter().any(|e| e.code == Code::CapeLike),
        "{:#?}",
        report.errors
    );
}

#[test]
fn budgets_are_enforced() {
    // 30 boxes: 720 vertices, past 512; and a texture that is not a power of two.
    let cubes: Vec<String> = (0..30)
        .map(|i| {
            format!(
                r#"{{ "origin": [{}, -2, -1], "size": [1, 1, 1], "uv": [0, 0] }}"#,
                (i % 8) as f32 - 4.0
            )
        })
        .collect();
    let manifest = format!(
        r#"{{ "format": 1, "id": "heavy", "version": "1.0.0", "name": "Heavy", "category": "head",
        "attachment": {{ "point": "head" }}, "texture": {{ "path": "t.png" }},
        "model": {{ "kind": "cubes", "parts": [{{ "name": "p", "cubes": [{}] }}] }} }}"#,
        cubes.join(",")
    );
    let pkg = Package::from_files(&files(&manifest, &[("t.png", png(48, 32))])).unwrap();
    let report = pkg.validate();
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.code == Code::Budget && e.message.contains("vertices"))
    );
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.code == Code::Texture && e.message.contains("power of two"))
    );

    // Too far from the bone.
    let far = RUSTIC_HAT.replace("[0, -8, 0]", "[0, -30, 0]");
    let pkg = Package::from_files(&files(&far, &[("textures/top_hat.png", png(64, 32))])).unwrap();
    assert!(pkg.validate().errors.iter().any(|e| e.code == Code::Reach));
}

#[test]
fn categories_need_their_blocks() {
    let tag = r##"{ "format": 1, "id": "violet_tag", "version": "1.0.0", "name": "Violet",
        "category": "nametag", "texture": { "path": "b.png" }, "nametag": { "line": "#a98fff" } }"##;
    let ok = Package::from_files(&files(tag, &[("b.png", png(8, 8))])).unwrap();
    assert!(ok.validate().ok(), "{:#?}", ok.validate().errors);
    let wide = Package::from_files(&files(tag, &[("b.png", png(16, 8))])).unwrap();
    assert!(
        wide.validate()
            .errors
            .iter()
            .any(|e| e.code == Code::Nametag)
    );

    let trail = r#"{ "format": 1, "id": "embers", "version": "1.0.0", "name": "Embers",
        "category": "trail", "texture": { "path": "t.png" },
        "trail": { "sprites": 4, "rate": 2, "lifetime": 30, "size": [1.5, 3], "gravity": -0.01, "fade": true } }"#;
    let ok = Package::from_files(&files(trail, &[("t.png", png(32, 8))])).unwrap();
    assert!(ok.validate().ok(), "{:#?}", ok.validate().errors);
    let fast = trail.replace("\"rate\": 2", "\"rate\": 9");
    let bad = Package::from_files(&files(&fast, &[("t.png", png(32, 8))])).unwrap();
    assert!(bad.validate().errors.iter().any(|e| e.code == Code::Fx));

    let wave = r#"{ "format": 1, "id": "wave", "version": "1.0.0", "name": "Wave", "category": "emote",
        "emote": { "length": 40, "tracks": [ { "part": "right_arm", "keys": [
          { "t": 0 }, { "t": 8, "rotation": [0, 0, 150] }, { "t": 40 } ] } ] } }"#;
    let ok = Package::from_bundle(wave.as_bytes()).unwrap();
    assert!(ok.validate().ok(), "{:#?}", ok.validate().errors);
    let backwards = wave.replace("\"t\": 8", "\"t\": 0");
    let bad = Package::from_bundle(backwards.as_bytes()).unwrap();
    assert!(bad.validate().errors.iter().any(|e| e.code == Code::Emote));

    // A head item without its attachment, a trail with a model.
    let no_attach = RUSTIC_HAT.replace(
        r#""attachment": { "point": "head", "offset": [0, -8, 0] },"#,
        "",
    );
    let pkg =
        Package::from_files(&files(&no_attach, &[("textures/top_hat.png", png(64, 32))])).unwrap();
    assert!(
        pkg.validate()
            .errors
            .iter()
            .any(|e| e.code == Code::Category)
    );
}

#[test]
fn companion_keeps_clear_of_the_head() {
    let pet = r#"{ "format": 1, "id": "wisp", "version": "1.0.0", "name": "Wisp", "category": "companion",
        "texture": { "path": "t.png" },
        "model": { "kind": "cubes", "parts": [ { "name": "body", "cubes": [{ "origin": [-2, -2, -2], "size": [4, 4, 4], "uv": [0, 0] }] } ] },
        "loops": [{ "part": "body", "motion": "bob", "axis": "y", "amplitude": 1, "period": 40 }],
        "companion": { "anchor": [-14, -6, 8], "lag": 8, "look": "body" } }"#;
    let ok = Package::from_files(&files(pet, &[("t.png", png(16, 16))])).unwrap();
    assert!(ok.validate().ok(), "{:#?}", ok.validate().errors);
    let in_face = pet.replace("[-14, -6, 8]", "[0, -4, -3]");
    let bad = Package::from_files(&files(&in_face, &[("t.png", png(16, 16))])).unwrap();
    assert!(bad.validate().errors.iter().any(|e| e.code == Code::Fx));
}

#[test]
fn unknown_fields_and_unsafe_paths_are_refused() {
    let extra = RUSTIC_HAT.replace("\"format\": 1,", "\"format\": 1, \"damage\": 5,");
    assert!(Package::from_files(&files(&extra, &[])).is_err());
    let escape = RUSTIC_HAT.replace("textures/top_hat.png", "../secret.png");
    assert!(matches!(
        Package::from_files(&files(&escape, &[])),
        Err(package::PackageError::UnsafePath(_))
    ));
}

#[test]
fn gltf_triangles_import_as_a_mesh() {
    // One triangle, positions in blocks, an embedded buffer (as Blockbench's glTF export).
    let mut buf = Vec::new();
    for v in [[0.0f32, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]] {
        for x in v {
            buf.extend(x.to_le_bytes());
        }
    }
    let gltf = format!(
        r#"{{ "asset": {{ "version": "2.0" }}, "scene": 0, "scenes": [{{ "nodes": [0] }}],
        "nodes": [{{ "mesh": 0, "name": "fin" }}],
        "meshes": [{{ "primitives": [{{ "attributes": {{ "POSITION": 0 }} }}] }}],
        "accessors": [{{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                        "min": [0, 0, 0], "max": [0.5, 0.5, 0] }}],
        "bufferViews": [{{ "buffer": 0, "byteLength": 36 }}],
        "buffers": [{{ "byteLength": 36, "uri": "data:application/octet-stream;base64,{}" }}] }}"#,
        encode_base64(&buf)
    );
    let manifest = r#"{ "format": 1, "id": "fin", "version": "1.0.0", "name": "Fin", "category": "back",
        "attachment": { "point": "back" }, "texture": { "path": "t.png" },
        "model": { "kind": "mesh", "source": "fin.gltf" } }"#;
    let pkg = Package::from_files(&files(
        manifest,
        &[("t.png", png(16, 16)), ("fin.gltf", gltf.into_bytes())],
    ))
    .unwrap();
    let Some(Model::Mesh(mesh)) = &pkg.cosmetic.model else {
        panic!("a mesh")
    };
    assert_eq!(mesh.groups.len(), 1);
    assert_eq!(mesh.groups[0].name, "fin");
    assert_eq!(mesh.groups[0].triangles.len(), 1);
    // Blocks to pixels, y flipped to point down.
    assert!(mesh.groups[0].triangles[0].iter().any(|v| v[1] == -8.0));
    let report = pkg.validate();
    assert!(report.ok(), "{:#?}", report.errors);
    assert_eq!(report.stats.triangles, 1);
    // No Horizon form for meshes.
    assert!(matches!(
        horizon::to_horizon(&pkg.bundle(), None),
        Err(horizon::ConvertError::NotRepresentable(_))
    ));
    // The bundle carries the triangles inline and drops the source.
    let bundle = pkg.bundle();
    let Some(Model::Mesh(m)) = &bundle.model else {
        panic!()
    };
    assert!(m.source.is_none() && !m.groups.is_empty());
}

#[test]
fn json_schema_is_exported_and_current() {
    let schema = json_schema();
    let text = serde_json::to_string_pretty(&schema).unwrap() + "\n";
    assert!(text.contains("\"shoulder_companion\""));
    assert!(!text.to_lowercase().contains("cape\""), "no cape category");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/cosmetic.schema.json");
    if std::env::var_os("UPDATE_SCHEMA").is_some() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        on_disk, text,
        "run with UPDATE_SCHEMA=1 to refresh schema/cosmetic.schema.json"
    );
    assert!(report_schema().is_object());
}
