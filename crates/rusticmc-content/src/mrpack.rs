//! `.mrpack` import (Modrinth's modpack format: `modrinth.index.json`, `overrides/`,
//! `client-overrides/`, `server-overrides/`), keeping only what runs on Horizon, with a report of
//! every file and why it was kept or skipped.
//!
//! - Resource packs (`resourcepacks/*.zip`) and data packs: imported (downloaded and checked
//!   against the index's SHA-512 and SHA-1, from Modrinth's allowed hosts only). Data packs go
//!   into `new-world-datapacks/`, which Horizon copies into every world it creates.
//! - Shader packs (`shaderpacks/*.zip`, Iris/OptiFine format): imported, marked partial.
//! - Java mods (`mods/*.jar`): their code can't run (Horizon has no JVM). They are reported as
//!   needing a port and, when the player has Horizon's `horizon-port-pack` tool, handed to its
//!   `port` command, which carries their data and assets into content mods
//!   (horizon/docs/modpacks.md). Rustic doesn't reimplement that carry.
//! - Configs, scripts, `options.txt` and server-only files: skipped with the reason.
use crate::download::{Downloader, FileRequest};
use crate::error::{ContentError, Result};
use crate::model::Hashes;
use crate::sources::modrinth::ALLOWED_HOSTS;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const INDEX: &str = "modrinth.index.json";
/// Import limits (as horizon-port-pack's): files and bytes of overrides, per download.
pub const MAX_OVERRIDE_FILES: usize = 65_536;
pub const MAX_OVERRIDE_BYTES: u64 = 512 << 20;
pub const MAX_DOWNLOAD: u64 = 512 << 20;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub files: Vec<MrFile>,
    #[serde(default)]
    pub dependencies: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrFile {
    pub path: String,
    pub hashes: MrHashes,
    #[serde(default)]
    pub env: Option<MrEnv>,
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MrHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MrEnv {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

/// What happened to one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Imported,
    /// Imported; runs in part (shader packs, data packs).
    ImportedPartly,
    /// A Java mod: needs horizon-port-pack (handed to it when available).
    NeedsPort,
    Skipped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportLine {
    pub path: String,
    pub action: Action,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    pub name: String,
    pub version: String,
    pub minecraft: Option<String>,
    pub loader: Option<String>,
    pub lines: Vec<ReportLine>,
    /// What horizon-port-pack said, when it ran.
    pub port: Option<String>,
}

impl ImportReport {
    pub fn count(&self, action: Action) -> usize {
        self.lines.iter().filter(|l| l.action == action).count()
    }

    fn push(&mut self, path: &str, action: Action, reason: impl Into<String>) {
        self.lines.push(ReportLine {
            path: path.to_owned(),
            action,
            reason: reason.into(),
        });
    }
}

/// Horizon's modpack porter (`horizon-port-pack port <pack> <work>`), when the player has it.
pub trait PortPack: Send + Sync {
    fn port(&self, pack: &Path, work: &Path) -> Result<String>;
}

/// Runs an installed `horizon-port-pack` executable.
pub struct ExternalPortPack {
    pub exe: PathBuf,
}

impl PortPack for ExternalPortPack {
    fn port(&self, pack: &Path, work: &Path) -> Result<String> {
        let mut cmd = std::process::Command::new(&self.exe);
        cmd.arg("port").arg(pack).arg(work);
        // No console window flashing on Windows (the launcher is a GUI app).
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd.output()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        if out.status.success() {
            Ok(text)
        } else {
            Err(ContentError::Download(format!(
                "horizon-port-pack failed ({}): {}",
                out.status,
                text.lines().last().unwrap_or("")
            )))
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImportOptions {
    /// Hosts files may come from (Modrinth's list by default).
    pub allowed_hosts: Vec<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            allowed_hosts: ALLOWED_HOSTS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

fn safe_rel(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}

fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    Some(host.split(':').next().unwrap_or(host))
}

/// Where a pack-relative path goes in a Horizon game folder, or why it doesn't.
enum Place {
    Into(String, crate::mrpack::Action),
    Java,
    Skip(&'static str),
}

fn place(path: &str) -> Place {
    let lower = path.to_lowercase();
    if lower.starts_with("mods/") && lower.ends_with(".jar") {
        Place::Java
    } else if path.starts_with("resourcepacks/") {
        Place::Into(path.to_owned(), Action::Imported)
    } else if path.starts_with("shaderpacks/") {
        if lower.ends_with(".txt") {
            // A pack's saved options: kept beside it.
            Place::Into(path.to_owned(), Action::Imported)
        } else {
            Place::Into(path.to_owned(), Action::ImportedPartly)
        }
    } else if let Some(rest) = path
        .strip_prefix("datapacks/")
        .or_else(|| path.strip_prefix("global_packs/required_data/"))
        .or_else(|| path.strip_prefix("global_packs/optional_data/"))
    {
        Place::Into(
            format!("new-world-datapacks/{rest}"),
            Action::ImportedPartly,
        )
    } else if lower == "options.txt" {
        Place::Skip("the instance keeps its own options")
    } else if lower.starts_with("config/") || lower.starts_with("defaultconfigs/") {
        Place::Skip("configuration of Java mods: no Horizon counterpart")
    } else if lower.starts_with("kubejs/") || lower.starts_with("scripts/") {
        Place::Skip("modpack scripts (KubeJS, CraftTweaker) are not run by Horizon")
    } else if lower.starts_with("mods/") {
        Place::Skip("Java mod data: no Horizon counterpart")
    } else {
        Place::Skip("not something Horizon reads")
    }
}

fn write_into(game_dir: &Path, rel: &str, bytes: &[u8]) -> Result<()> {
    let dest = game_dir.join(rel);
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(dest, bytes)?;
    Ok(())
}

/// Reads the index of an `.mrpack`.
pub fn read_index(pack: &Path) -> Result<MrIndex> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    let mut s = String::new();
    zip.by_name(INDEX)
        .map_err(|_| ContentError::Refused(format!("no {INDEX}: not an .mrpack")))?
        .read_to_string(&mut s)?;
    let index: MrIndex = serde_json::from_str(&s)?;
    if index.format_version != 1 || index.game != "minecraft" {
        return Err(ContentError::Incompatible(format!(
            ".mrpack format {} for {:?}: this launcher reads format 1",
            index.format_version, index.game
        )));
    }
    Ok(index)
}

/// Imports the Horizon-compatible part of `pack` into `game_dir` (a new instance's game folder).
pub async fn import(
    pack: &Path,
    game_dir: &Path,
    downloader: &dyn Downloader,
    port: Option<&dyn PortPack>,
    opts: &ImportOptions,
) -> Result<ImportReport> {
    let index = read_index(pack)?;
    let mut report = ImportReport {
        name: index.name.clone(),
        version: index.version_id.clone(),
        minecraft: index.dependencies.get("minecraft").cloned(),
        loader: ["neoforge", "fabric-loader", "forge", "quilt-loader"]
            .iter()
            .find(|l| index.dependencies.contains_key(**l))
            .map(|l| l.to_string()),
        ..ImportReport::default()
    };
    if report
        .minecraft
        .as_deref()
        .is_some_and(|v| v != crate::horizon_env::GAME)
    {
        report.push(
            INDEX,
            Action::Skipped,
            format!(
                "made for Minecraft {}, Horizon plays {}: older resource packs show the \
                 game's compatibility warning, data packs may need `horizon-port-pack \
                 upgrade-datapack`",
                report.minecraft.as_deref().unwrap_or("?"),
                crate::horizon_env::GAME
            ),
        );
    }
    let mut java_jars = 0usize;
    // Files listed in the index.
    for f in &index.files {
        if !safe_rel(&f.path) {
            report.push(&f.path, Action::Failed, "unsafe path: refused");
            continue;
        }
        if f.env.as_ref().and_then(|e| e.client.as_deref()) == Some("unsupported") {
            report.push(&f.path, Action::Skipped, "server-only file");
            continue;
        }
        match place(&f.path) {
            Place::Java => {
                java_jars += 1;
                report.push(
                    &f.path,
                    Action::NeedsPort,
                    "a Java mod: its code can't run on Horizon (no JVM); its data and assets \
                     can be ported with horizon-port-pack",
                );
            }
            Place::Skip(why) => report.push(&f.path, Action::Skipped, why),
            Place::Into(dest, action) => {
                let url = f.downloads.iter().find(|u| {
                    host_of(u).is_some_and(|h| opts.allowed_hosts.iter().any(|a| a == h))
                });
                let Some(url) = url else {
                    report.push(
                        &f.path,
                        Action::Failed,
                        "no download from an allowed host (Modrinth's CDN, GitHub, GitLab)",
                    );
                    continue;
                };
                if f.file_size.is_some_and(|s| s > MAX_DOWNLOAD) {
                    report.push(&f.path, Action::Failed, "over 512 MiB");
                    continue;
                }
                let req = FileRequest {
                    url: url.clone(),
                    dest: game_dir.join(&dest),
                    size: f.file_size,
                    hashes: Hashes {
                        sha1: Some(f.hashes.sha1.clone()),
                        sha256: None,
                        sha512: Some(f.hashes.sha512.clone()),
                    },
                };
                match downloader.fetch(&req).await {
                    Ok(_) => report.push(&f.path, action, reason_for(action, &dest)),
                    Err(e) => report.push(&f.path, Action::Failed, e.to_string()),
                }
            }
        }
    }
    // Overrides: common first, then the client's (which win).
    let mut zip = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    let mut total = 0u64;
    let mut count = 0usize;
    for layer in ["overrides/", "client-overrides/", "server-overrides/"] {
        for i in 0..zip.len() {
            let mut e = zip.by_index(i)?;
            let name = e.name().to_owned();
            let Some(rel) = name.strip_prefix(layer) else {
                continue;
            };
            if e.is_dir() || rel.is_empty() {
                continue;
            }
            if layer == "server-overrides/" {
                report.push(rel, Action::Skipped, "server-only override");
                continue;
            }
            if !safe_rel(rel) {
                report.push(rel, Action::Failed, "unsafe path: refused");
                continue;
            }
            match place(rel) {
                Place::Java => {
                    java_jars += 1;
                    report.push(rel, Action::NeedsPort, "a Java mod jar in the overrides");
                }
                Place::Skip(why) => report.push(rel, Action::Skipped, why),
                Place::Into(dest, action) => {
                    count += 1;
                    total += e.size();
                    if count > MAX_OVERRIDE_FILES || total > MAX_OVERRIDE_BYTES {
                        return Err(ContentError::Refused(
                            "the pack's overrides are over 65,536 files or 512 MiB".into(),
                        ));
                    }
                    let mut b = Vec::new();
                    e.read_to_end(&mut b)?;
                    write_into(game_dir, &dest, &b)?;
                    // A client override replaces a common one: report the path once.
                    report.lines.retain(|l| l.path != rel);
                    report.push(rel, action, reason_for(action, &dest));
                }
            }
        }
    }
    if java_jars > 0 {
        match port {
            Some(p) => {
                let work = game_dir.join(".rustic-port");
                std::fs::create_dir_all(&work)?;
                report.port = Some(match p.port(pack, &work) {
                    Ok(out) => format!(
                        "horizon-port-pack ran on {java_jars} Java mod(s): {}",
                        out.lines().last().unwrap_or("done")
                    ),
                    Err(e) => e.to_string(),
                });
            }
            None => {
                report.port = Some(format!(
                    "{java_jars} Java mod(s) not ported: install Horizon's horizon-port-pack to \
                     carry their data and assets over"
                ));
            }
        }
    }
    Ok(report)
}

fn reason_for(action: Action, dest: &str) -> String {
    match action {
        Action::ImportedPartly if dest.starts_with("new-world-datapacks/") => {
            "a data pack: every new world starts with it; Horizon runs part of a data pack".into()
        }
        Action::ImportedPartly => {
            "an Iris/OptiFine shader pack: Horizon's loader runs part of their features".into()
        }
        _ => format!("into {dest}"),
    }
}
