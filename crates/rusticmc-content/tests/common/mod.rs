#![allow(dead_code)]
use rusticmc_content::download::digest;
use rusticmc_content::model::*;
use std::io::Write;
use std::path::{Path, PathBuf};

/// A zip of `(name, bytes)` entries.
pub fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut out);
        let o = zip::write::SimpleFileOptions::default();
        for (name, bytes) in entries {
            z.start_file(*name, o).unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
    }
    out.into_inner()
}

pub fn resource_pack(format: i64) -> Vec<u8> {
    zip(&[
        (
            "pack.mcmeta",
            format!(r#"{{"pack": {{"pack_format": {format}, "description": "test"}}}}"#).as_bytes(),
        ),
        ("assets/minecraft/textures/block/stone.png", b"png"),
    ])
}

pub fn data_pack() -> Vec<u8> {
    zip(&[
        (
            "pack.mcmeta",
            br#"{"pack": {"pack_format": 121, "description": "dp"}}"#,
        ),
        ("data/demo/function/hello.mcfunction", b"say hi"),
    ])
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

pub fn file(url: &str, name: &str, bytes: &[u8]) -> ContentFile {
    let d = digest(bytes);
    ContentFile {
        url: url.into(),
        filename: name.into(),
        size: Some(bytes.len() as u64),
        hashes: Hashes {
            sha1: d.sha1,
            sha256: None,
            sha512: d.sha512,
        },
        primary: true,
        signature: None,
    }
}

pub fn modrinth_version(
    project: &str,
    kind: ContentKind,
    loaders: &[&str],
    file: ContentFile,
) -> ContentVersion {
    ContentVersion {
        source: SourceKind::Modrinth,
        project_id: project.into(),
        id: format!("{project}-v1"),
        name: project.into(),
        version: "1.0.0".into(),
        kind,
        game_versions: vec!["26.3".into()],
        loaders: loaders.iter().map(|s| s.to_string()).collect(),
        files: vec![file],
        dependencies: vec![],
        published: "2026-11-01T00:00:00Z".into(),
        horizon: None,
    }
}

pub fn instance(dir: &Path) -> Target {
    Target::Instance {
        game_dir: dir.to_path_buf(),
    }
}
