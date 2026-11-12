//! Rustic packs (`.rusticpack`): an instance's content shared as one file
//! (launcher/docs/CONTENT.md, "Rustic pack format").
//!
//! A zip holding `rustic-pack.json` and, optionally, `overrides/`. Content from a source
//! (Modrinth, Horizon's registry, Rustic's) is listed by address and hashes, never copied in, so
//! a pack carries no one else's files; the player's own files (added by hand) are bundled under
//! `overrides/` only when the exporter asks for it. Importing installs each listed file through
//! the normal install path (every check of its source), then copies the allowed overrides.
use crate::download::Downloader;
use crate::error::{ContentError, Result};
use crate::fairplay::FairPlayPolicy;
use crate::horizon_env::HorizonEnv;
use crate::install::{InstallOptions, Installer};
use crate::ledger::Ledger;
use crate::model::*;
use crate::sources::horizon::Index;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

pub const MANIFEST: &str = "rustic-pack.json";
pub const FORMAT: u32 = 1;
pub const EXTENSION: &str = "rusticpack";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RusticPack {
    pub format: u32,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The game version (`26.3`).
    pub game: String,
    /// What the exporting instance's Horizon loaded, for the importer's verdicts.
    pub horizon: PackHorizon,
    pub content: Vec<PackContent>,
    /// Paths under `overrides/`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bundled: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackHorizon {
    pub mod_api: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_hooks: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackContent {
    pub source: SourceKind,
    pub project_id: String,
    pub version_id: String,
    pub version: String,
    pub name: String,
    pub kind: ContentKind,
    /// `instance` or `new_worlds`.
    pub target: String,
    pub enabled: bool,
    pub file: PackFile,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackFile {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub hashes: Hashes,
}

/// What may be bundled under `overrides/` (and is copied back on import).
pub fn allowed_override(rel: &str) -> bool {
    let plain = |p: &str| !p.is_empty() && !p.contains('/') && !p.starts_with('.');
    if rel.contains('\\')
        || rel
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return false;
    }
    matches!(
        rel,
        "options.txt" | "horizon-graphics.txt" | "horizon-extras.txt"
    ) || ["resourcepacks/", "shaderpacks/", "new-world-datapacks/"]
        .iter()
        .any(|dir| rel.strip_prefix(dir).is_some_and(plain))
}

#[derive(Clone, Debug, Default)]
pub struct ExportOptions {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Bundle the instance's `options.txt`, `horizon-graphics.txt`, `horizon-extras.txt`.
    pub settings: bool,
    /// Bundle files the player added by hand (their licence is the player's responsibility).
    pub local_files: bool,
}

/// What the export left out and why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExportReport {
    pub listed: usize,
    pub bundled: Vec<String>,
    pub left_out: Vec<(String, String)>,
}

/// Exports `game_dir`'s installed content (its `rustic-content.json`) as a Rustic pack at `out`.
pub fn export(
    game_dir: &Path,
    env: &HorizonEnv,
    opts: &ExportOptions,
    out: &Path,
) -> Result<ExportReport> {
    let ledger = Ledger::load(game_dir)?;
    let mut report = ExportReport::default();
    let mut pack = RusticPack {
        format: FORMAT,
        name: opts.name.clone(),
        version: opts.version.clone(),
        description: opts.description.clone(),
        game: env.game.clone(),
        horizon: PackHorizon {
            mod_api: env.mod_api.clone(),
            native_hooks: env.native_hooks,
        },
        content: Vec::new(),
        bundled: Vec::new(),
    };
    let mut bundle: Vec<(String, Vec<u8>)> = Vec::new();
    for e in &ledger.entries {
        if e.target.starts_with("world:") {
            report
                .left_out
                .push((e.name.clone(), "installed in one world only".into()));
            continue;
        }
        let file = e.files.first();
        match (e.source, &e.url, file) {
            (SourceKind::Local, _, Some(f)) => {
                if opts.local_files && allowed_override(&f.path) {
                    bundle.push((f.path.clone(), std::fs::read(game_dir.join(&f.path))?));
                } else {
                    report
                        .left_out
                        .push((e.name.clone(), "added by hand: not bundled".into()));
                }
            }
            (_, Some(url), Some(f)) => {
                pack.content.push(PackContent {
                    source: e.source,
                    project_id: e.project_id.clone(),
                    version_id: e.version_id.clone(),
                    version: e.version.clone(),
                    name: e.name.clone(),
                    kind: e.kind,
                    target: e.target.clone(),
                    enabled: e.enabled,
                    file: PackFile {
                        name: f.path.rsplit('/').next().unwrap_or("").to_owned(),
                        url: url.clone(),
                        size: std::fs::metadata(game_dir.join(&f.path))
                            .ok()
                            .map(|m| m.len()),
                        hashes: f.hashes.clone(),
                    },
                });
            }
            _ => report
                .left_out
                .push((e.name.clone(), "no address to download it from".into())),
        }
    }
    if opts.settings {
        for name in ["options.txt", "horizon-graphics.txt", "horizon-extras.txt"] {
            if let Ok(b) = std::fs::read(game_dir.join(name)) {
                bundle.push((name.to_owned(), b));
            }
        }
    }
    pack.bundled = bundle.iter().map(|(p, _)| p.clone()).collect();
    report.listed = pack.content.len();
    report.bundled = pack.bundled.clone();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(out)?);
    let o = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(MANIFEST, o)?;
    zip.write_all(&serde_json::to_vec_pretty(&pack)?)?;
    for (path, bytes) in &bundle {
        zip.start_file(format!("overrides/{path}"), o)?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;
    Ok(report)
}

/// Reads a pack's manifest.
pub fn read(pack: &Path) -> Result<RusticPack> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    let mut s = String::new();
    zip.by_name(MANIFEST)
        .map_err(|_| ContentError::Refused(format!("no {MANIFEST}: not a Rustic pack")))?
        .read_to_string(&mut s)?;
    let p: RusticPack = serde_json::from_str(&s)?;
    if p.format > FORMAT {
        return Err(ContentError::Incompatible(format!(
            "Rustic pack format {}, this launcher reads {FORMAT}: update it",
            p.format
        )));
    }
    Ok(p)
}

/// One line of an import.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportLine {
    pub name: String,
    pub ok: bool,
    pub reason: String,
}

/// Imports a pack into `game_dir` (a new instance): each listed file through the installer (its
/// source's checks, fair play, compatibility), then the allowed overrides.
pub async fn import(
    pack: &Path,
    game_dir: &Path,
    downloader: &dyn Downloader,
    env: &HorizonEnv,
    index: Option<&Index>,
) -> Result<Vec<ImportLine>> {
    let manifest = read(pack)?;
    let fairplay = FairPlayPolicy::default();
    let installer = Installer {
        downloader,
        env,
        fairplay: &fairplay,
        index,
    };
    let mut lines = Vec::new();
    for c in &manifest.content {
        let target = match c.target.as_str() {
            "new_worlds" => Target::NewWorlds {
                game_dir: game_dir.to_path_buf(),
            },
            _ => Target::Instance {
                game_dir: game_dir.to_path_buf(),
            },
        };
        let version = ContentVersion {
            source: c.source,
            project_id: c.project_id.clone(),
            id: c.version_id.clone(),
            name: c.name.clone(),
            version: c.version.clone(),
            kind: c.kind,
            game_versions: vec![manifest.game.clone()],
            loaders: Vec::new(),
            files: vec![ContentFile {
                url: c.file.url.clone(),
                filename: c.file.name.clone(),
                size: c.file.size,
                hashes: c.file.hashes.clone(),
                primary: true,
                signature: None,
            }],
            dependencies: Vec::new(),
            published: String::new(),
            horizon: index.and_then(|i| {
                i.versions(&c.project_id)
                    .into_iter()
                    .find(|v| v.version == c.version)
                    .and_then(|v| v.horizon)
            }),
        };
        // Registry files keep their registry signature: take the index's own entry.
        let version = match (c.source, index) {
            (SourceKind::Horizon, Some(i)) => i
                .versions(&c.project_id)
                .into_iter()
                .find(|v| v.version == c.version)
                .unwrap_or(version),
            _ => version,
        };
        let opts = InstallOptions {
            enable: c.enabled,
            allow_native: false,
        };
        match installer.install(&[version], &target, &opts).await {
            Ok(_) => lines.push(ImportLine {
                name: c.name.clone(),
                ok: true,
                reason: "installed".into(),
            }),
            Err(e) => lines.push(ImportLine {
                name: c.name.clone(),
                ok: false,
                reason: e.to_string(),
            }),
        }
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    for i in 0..zip.len() {
        let mut e = zip.by_index(i)?;
        let name = e.name().to_owned();
        let Some(rel) = name.strip_prefix("overrides/") else {
            continue;
        };
        if e.is_dir() || rel.is_empty() {
            continue;
        }
        if !allowed_override(rel) || e.size() > 256 << 20 {
            lines.push(ImportLine {
                name: rel.to_owned(),
                ok: false,
                reason: "not a file a Rustic pack may carry: skipped".into(),
            });
            continue;
        }
        let mut b = Vec::new();
        e.read_to_end(&mut b)?;
        let dest = game_dir.join(rel);
        if let Some(d) = dest.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(dest, b)?;
        lines.push(ImportLine {
            name: rel.to_owned(),
            ok: true,
            reason: "copied from the pack".into(),
        });
    }
    Ok(lines)
}
