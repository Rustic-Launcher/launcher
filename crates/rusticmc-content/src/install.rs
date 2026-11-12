//! Installing into an instance, a world, the packs new worlds start with, or a server; turning
//! content on and off; uninstalling. Files go where Horizon reads them
//! (horizon/docs/api/content.md), downloaded to a staging folder, checked, then moved in place:
//! a file that fails a check never reaches the game folder.
use crate::compat;
use crate::download::{Downloader, FileRequest};
use crate::error::{ContentError, Result};
use crate::fairplay::FairPlayPolicy;
use crate::horizon_env::HorizonEnv;
use crate::inspect;
use crate::ledger::{Entry, InstalledFile, Ledger};
use crate::model::*;
use crate::sources::horizon::Index;
use crate::switches;
use std::io::Read;
use std::path::{Path, PathBuf};

const STAGING: &str = ".rustic-staging";

#[derive(Clone, Debug, Default)]
pub struct InstallOptions {
    /// Turn what is installed on (default for a fresh install).
    pub enable: bool,
    /// Copy native mod files into `horizon-native-mods/` even though only Horizon's launcher can
    /// build them (an expert's choice; off by default).
    pub allow_native: bool,
}

/// What an install did, per version.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub project_id: String,
    pub version: String,
    pub path: PathBuf,
    pub compatibility: Compatibility,
    pub dependency_of: Option<String>,
}

pub struct Installer<'a> {
    pub downloader: &'a dyn Downloader,
    pub env: &'a HorizonEnv,
    pub fairplay: &'a FairPlayPolicy,
    /// Horizon's index, for registry files (their checks) and `.hpack` mods.
    pub index: Option<&'a Index>,
}

fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', ':']) && name != ".."
}

/// The folder a kind goes to under a target, relative to the target's root.
pub fn folder_for(kind: ContentKind, target: &Target) -> Result<PathBuf> {
    Ok(match (kind, target) {
        (ContentKind::Mod, Target::Instance { .. } | Target::Server { .. }) => {
            "horizon-mods".into()
        }
        (ContentKind::NativeMod, Target::Instance { .. }) => "horizon-native-mods".into(),
        (ContentKind::ResourcePack, Target::Instance { .. }) => "resourcepacks".into(),
        (ContentKind::Shader | ContentKind::ShaderPack, Target::Instance { .. }) => {
            "shaderpacks".into()
        }
        (ContentKind::DataPack, Target::World { world, .. }) => {
            if !plain_name(world) {
                return Err(ContentError::Refused(format!("world name {world:?}")));
            }
            PathBuf::from("saves").join(world).join("datapacks")
        }
        (ContentKind::DataPack, Target::NewWorlds { .. }) => "new-world-datapacks".into(),
        (ContentKind::Plugin, Target::Server { .. }) => "plugins".into(),
        (kind, t) => {
            return Err(ContentError::Incompatible(format!(
                "a {} doesn't go into {}",
                kind.as_str(),
                t.label()
            )));
        }
    })
}

fn rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

impl Installer<'_> {
    /// Installs `versions` (dependencies first, the asked-for project last, as
    /// `resolve::resolve` orders them) into `target`.
    pub async fn install(
        &self,
        versions: &[ContentVersion],
        target: &Target,
        opts: &InstallOptions,
    ) -> Result<Vec<Installed>> {
        let root_project = versions.last().map(|v| v.project_id.clone());
        let mut done = Vec::new();
        for v in versions {
            let dependency_of = root_project.clone().filter(|r| r != &v.project_id);
            done.extend(self.install_one(v, target, opts, dependency_of).await?);
        }
        Ok(done)
    }

    async fn install_one(
        &self,
        v: &ContentVersion,
        target: &Target,
        opts: &InstallOptions,
        dependency_of: Option<String>,
    ) -> Result<Vec<Installed>> {
        let summary_name = v.name.clone();
        let standing = self.fairplay.judge(&ProjectSummary {
            source: v.source,
            id: v.project_id.clone(),
            slug: v.project_id.clone(),
            kind: v.kind,
            name: summary_name,
            summary: String::new(),
            authors: vec![],
            icon_url: None,
            downloads: 0,
            categories: vec![],
            tags: vec![],
            license: None,
            loaders: vec![],
            game_versions: vec![],
            fair_play: FairPlay::Clean,
        });
        if let FairPlay::Blocked { reason } = standing {
            return Err(ContentError::Refused(format!(
                "{} is not installed by Rustic: {reason}",
                v.name
            )));
        }
        let compatibility = compat::check(v, Some(target), self.env);
        let native_ok = compatibility.verdict == Verdict::NeedsBuild && opts.allow_native;
        if !compatibility.installable() && !native_ok {
            return Err(ContentError::Incompatible(format!(
                "{} {}: {}",
                v.name,
                v.version,
                compatibility.reasons.join("; ")
            )));
        }
        if v.kind == ContentKind::Modpack && v.source != SourceKind::Horizon {
            return Err(ContentError::Incompatible(
                "import .mrpack files and Rustic packs with `mrpack::import` / `rustic_pack::import`"
                    .into(),
            ));
        }
        let file = v
            .primary_file()
            .ok_or_else(|| ContentError::NotFound(format!("{} has no file", v.name)))?;
        if !plain_name(&file.filename) {
            return Err(ContentError::Refused(format!(
                "file name {:?}",
                file.filename
            )));
        }
        check_origin(v.source, &file.url, &file.filename)?;
        let root = target.root().clone();
        let staging = root.join(STAGING);
        std::fs::create_dir_all(&staging)?;
        let staged = staging.join(&file.filename);
        let fetched = self
            .downloader
            .fetch(&FileRequest {
                url: file.url.clone(),
                dest: staged.clone(),
                size: file.size,
                hashes: file.hashes.clone(),
            })
            .await;
        let result = match fetched {
            Ok(path) => {
                self.place(v, file, &path, target, opts, compatibility, dependency_of)
                    .await
            }
            Err(e) => Err(e),
        };
        let _ = std::fs::remove_file(&staged);
        let _ = std::fs::remove_dir(&staging);
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn place(
        &self,
        v: &ContentVersion,
        file: &ContentFile,
        staged: &Path,
        target: &Target,
        opts: &InstallOptions,
        compatibility: Compatibility,
        dependency_of: Option<String>,
    ) -> Result<Vec<Installed>> {
        let root = target.root().clone();
        // The source's own checks, before anything reaches the game folder.
        match v.source {
            SourceKind::Horizon => self
                .index
                .ok_or_else(|| ContentError::NotConfigured("Horizon's index".into()))?
                .check_file(v, staged)?,
            _ if matches!(v.kind, ContentKind::ResourcePack | ContentKind::DataPack) => {
                inspect::check_matches(v, staged)?
            }
            _ => {}
        }
        if v.kind == ContentKind::Plugin {
            let Target::Server { server_dir } = target else {
                return Err(ContentError::Incompatible(
                    "plugins go into a server".into(),
                ));
            };
            let done = crate::plugins::install(server_dir, staged)?;
            let mut ledger = Ledger::load(&root)?;
            ledger.upsert(entry(
                v,
                file,
                target,
                &rel(&done.path, &root),
                Some(done.id.clone()),
                true,
                dependency_of.clone(),
            ));
            ledger.save(&root)?;
            return Ok(vec![Installed {
                project_id: v.project_id.clone(),
                version: v.version.clone(),
                path: done.path,
                compatibility,
                dependency_of,
            }]);
        }
        if v.kind == ContentKind::Modpack {
            return self.install_hpack(v, staged, target, opts).await;
        }
        let folder = root.join(folder_for(v.kind, target)?);
        std::fs::create_dir_all(&folder)?;
        let dest = folder.join(&file.filename);
        let game_id = match v.kind {
            ContentKind::Mod => Some(inspect::mod_manifest(staged)?.id),
            ContentKind::NativeMod => Some(inspect::native_manifest(staged)?.id),
            _ => Some(format!("file/{}", file.filename)),
        };
        // A newer version of the same project replaces the old file.
        let mut ledger = Ledger::load(&root)?;
        if let Some(old) = ledger.entries.iter().find(|e| {
            e.source == v.source && e.project_id == v.project_id && e.target == target.label()
        }) {
            for f in &old.files {
                let p = root.join(&f.path);
                if p != dest {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        std::fs::copy(staged, &dest)?;
        let enabled = opts.enable;
        apply_switch(v.kind, target, game_id.as_deref(), &file.filename, enabled)?;
        ledger.upsert(entry(
            v,
            file,
            target,
            &rel(&dest, &root),
            game_id,
            enabled,
            dependency_of.clone(),
        ));
        ledger.save(&root)?;
        Ok(vec![Installed {
            project_id: v.project_id.clone(),
            version: v.version.clone(),
            path: dest,
            compatibility,
            dependency_of,
        }])
    }

    /// A Horizon `.hpack`: each listed mod from the index (pinned by SHA-256), then the allowed
    /// `overrides/` (horizon/docs/api/mod-index.md "`.hpack`").
    async fn install_hpack(
        &self,
        v: &ContentVersion,
        staged: &Path,
        target: &Target,
        opts: &InstallOptions,
    ) -> Result<Vec<Installed>> {
        let index = self
            .index
            .ok_or_else(|| ContentError::NotConfigured("Horizon's index".into()))?;
        let Target::Instance { game_dir } = target else {
            return Err(ContentError::Incompatible(
                "a modpack becomes an instance".into(),
            ));
        };
        let toml_bytes = inspect::read_entry(staged, "horizon-modpack.toml")?
            .ok_or_else(|| ContentError::Refused("no horizon-modpack.toml".into()))?;
        let pack: HPack = toml::from_str(&String::from_utf8_lossy(&toml_bytes))
            .map_err(|e| ContentError::Refused(format!("horizon-modpack.toml: {e}")))?;
        if pack.format != self.env.modpack_format {
            return Err(ContentError::Incompatible(format!(
                ".hpack format {}, this Horizon reads {}",
                pack.format, self.env.modpack_format
            )));
        }
        let listed: Vec<String> = pack.mods.iter().map(|m| m.id.clone()).collect();
        // Check the overrides before installing anything.
        let overrides = read_overrides(staged, &listed)?;
        let mut out = Vec::new();
        for m in &pack.mods {
            let mv = index
                .versions(&m.id)
                .into_iter()
                .find(|x| x.version == m.version)
                .ok_or_else(|| {
                    ContentError::NotFound(format!("{} {} in Horizon's index", m.id, m.version))
                })?;
            let sha = mv
                .primary_file()
                .and_then(|f| f.hashes.sha256.clone())
                .unwrap_or_default();
            if !sha.eq_ignore_ascii_case(&m.sha256) {
                return Err(ContentError::Refused(format!(
                    "{} {}: the pack pins another file than the index lists",
                    m.id, m.version
                )));
            }
            let mod_opts = InstallOptions {
                enable: m.on,
                ..opts.clone()
            };
            out.extend(
                Box::pin(self.install_one(&mv, target, &mod_opts, Some(v.project_id.clone())))
                    .await?,
            );
        }
        for (path, bytes) in overrides {
            let dest = game_dir.join(&path);
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(dest, bytes)?;
        }
        // The pack itself: its mods are pinned by it (no update one by one).
        let mut ledger = Ledger::load(game_dir)?;
        let file = v.primary_file();
        ledger.upsert(Entry {
            source: v.source,
            project_id: v.project_id.clone(),
            version_id: v.id.clone(),
            version: v.version.clone(),
            name: v.name.clone(),
            kind: ContentKind::Modpack,
            target: target.label(),
            files: Vec::new(),
            enabled: true,
            game_id: None,
            needed_by: Vec::new(),
            url: file.map(|f| f.url.clone()),
        });
        ledger.save(game_dir)?;
        Ok(out)
    }

    /// Turns installed content on or off (the switch files only; nothing is deleted).
    pub fn set_enabled(
        &self,
        target: &Target,
        source: SourceKind,
        project: &str,
        on: bool,
    ) -> Result<()> {
        let root = target.root().clone();
        let mut ledger = Ledger::load(&root)?;
        let e = ledger
            .find_mut(source, project, &target.label())
            .ok_or_else(|| ContentError::NotFound(project.into()))?;
        let file = e
            .files
            .first()
            .map(|f| f.path.rsplit('/').next().unwrap_or("").to_owned())
            .unwrap_or_default();
        apply_switch(e.kind, target, e.game_id.as_deref(), &file, on)?;
        e.enabled = on;
        ledger.save(&root)
    }

    /// Removes installed content and its switch. Refused while something installed needs it.
    pub fn uninstall(&self, target: &Target, source: SourceKind, project: &str) -> Result<()> {
        let root = target.root().clone();
        let mut ledger = Ledger::load(&root)?;
        let label = target.label();
        let Some(pos) = ledger
            .entries
            .iter()
            .position(|e| e.source == source && e.project_id == project && e.target == label)
        else {
            return Err(ContentError::NotFound(project.into()));
        };
        let e = ledger.entries[pos].clone();
        let needed: Vec<&String> = e
            .needed_by
            .iter()
            .filter(|p| ledger.entries.iter().any(|x| &x.project_id == *p))
            .collect();
        if !needed.is_empty() {
            return Err(ContentError::Dependency(format!(
                "{} is needed by {}",
                e.name,
                needed
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        let file = e
            .files
            .first()
            .map(|f| f.path.rsplit('/').next().unwrap_or("").to_owned())
            .unwrap_or_default();
        if e.kind == ContentKind::Plugin {
            if let (Target::Server { server_dir }, Some(id)) = (target, &e.game_id) {
                crate::plugins::uninstall(server_dir, id)?;
            }
        } else {
            apply_switch(e.kind, target, e.game_id.as_deref(), &file, false)?;
            for f in &e.files {
                let p = root.join(&f.path);
                if p.exists() {
                    std::fs::remove_file(p)?;
                }
            }
        }
        // Mods' own `mod.<id>` line is left as false (as Horizon's Remove does).
        ledger.entries.remove(pos);
        ledger.save(&root)
    }
}

/// Where a source's files may come from: Horizon's two release shapes, Modrinth's hosts, https
/// for Rustic's CDN.
pub fn check_origin(source: SourceKind, url: &str, filename: &str) -> Result<()> {
    match source {
        SourceKind::Horizon => crate::sources::horizon::check_url(url, filename),
        SourceKind::Modrinth => {
            let host = url
                .strip_prefix("https://")
                .and_then(|r| r.split(['/', '?', '#']).next())
                .unwrap_or("");
            if crate::sources::modrinth::ALLOWED_HOSTS.contains(&host) {
                Ok(())
            } else {
                Err(ContentError::Refused(format!(
                    "{url}: not one of Modrinth's file hosts"
                )))
            }
        }
        SourceKind::Rustic if url.starts_with("https://") => Ok(()),
        SourceKind::Rustic => Err(ContentError::Refused(format!("{url}: not https"))),
        SourceKind::Local => Ok(()),
    }
}

fn entry(
    v: &ContentVersion,
    file: &ContentFile,
    target: &Target,
    path: &str,
    game_id: Option<String>,
    enabled: bool,
    dependency_of: Option<String>,
) -> Entry {
    Entry {
        source: v.source,
        project_id: v.project_id.clone(),
        version_id: v.id.clone(),
        version: v.version.clone(),
        name: v.name.clone(),
        kind: v.kind,
        target: target.label(),
        files: vec![InstalledFile {
            path: path.to_owned(),
            hashes: file.hashes.clone(),
        }],
        enabled,
        game_id,
        needed_by: dependency_of.into_iter().collect(),
        url: Some(file.url.clone()),
    }
}

/// Sets the switch Horizon reads for one piece of content.
pub fn apply_switch(
    kind: ContentKind,
    target: &Target,
    game_id: Option<&str>,
    filename: &str,
    on: bool,
) -> Result<()> {
    match (kind, target) {
        (
            ContentKind::Mod | ContentKind::NativeMod,
            Target::Instance { game_dir }
            | Target::Server {
                server_dir: game_dir,
            },
        ) => {
            if let Some(id) = game_id {
                switches::set_mod(game_dir, id, on)?;
            }
        }
        (ContentKind::ResourcePack, Target::Instance { game_dir }) => {
            switches::set_resource_pack(game_dir, filename, on)?
        }
        (ContentKind::Shader | ContentKind::ShaderPack, Target::Instance { game_dir }) => {
            if on {
                switches::set_shader_pack(game_dir, Some(filename))?;
            } else if switches::shader_pack(game_dir).as_deref() == Some(filename) {
                switches::set_shader_pack(game_dir, None)?;
            }
        }
        (ContentKind::DataPack, Target::World { game_dir, world }) => {
            switches::set_data_pack(&game_dir.join("saves").join(world), filename, on)?
        }
        // New worlds take every pack in the folder; plugins load when present.
        _ => {}
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HPack {
    format: u32,
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: String,
    #[serde(default)]
    #[allow(dead_code)]
    authors: Vec<String>,
    #[allow(dead_code)]
    game: String,
    #[serde(default)]
    mods: Vec<HPackMod>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HPackMod {
    id: String,
    version: String,
    sha256: String,
    #[serde(default = "yes")]
    on: bool,
}

fn yes() -> bool {
    true
}

/// `overrides/` of an `.hpack`: only `options.txt`, `horizon-graphics.txt`,
/// `horizon-extras.txt`, `resourcepacks/<name>.zip` and
/// `horizon-mods/.horizon-mod-data/<a listed mod>/...`, each at most 64 MiB and 256 MiB in
/// all; anything else refuses the pack.
fn read_overrides(path: &Path, listed: &[String]) -> Result<Vec<(String, Vec<u8>)>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let mut out = Vec::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut e = zip.by_index(i)?;
        let name = e.name().to_owned();
        let Some(rel) = name.strip_prefix("overrides/") else {
            continue;
        };
        if e.is_dir() || rel.is_empty() {
            continue;
        }
        if rel
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
            || rel.contains('\\')
        {
            return Err(ContentError::Refused(format!("override {name:?}")));
        }
        let allowed = matches!(
            rel,
            "options.txt" | "horizon-graphics.txt" | "horizon-extras.txt"
        ) || rel
            .strip_prefix("resourcepacks/")
            .is_some_and(|n| !n.contains('/') && n.ends_with(".zip"))
            || rel
                .strip_prefix("horizon-mods/.horizon-mod-data/")
                .and_then(|r| r.split('/').next())
                .is_some_and(|id| listed.iter().any(|l| l == id));
        if !allowed {
            return Err(ContentError::Refused(format!(
                "the pack's override {rel:?} is not one a modpack may hold"
            )));
        }
        if e.size() > 64 << 20 {
            return Err(ContentError::Refused(format!(
                "override {rel} is over 64 MiB"
            )));
        }
        total += e.size();
        if total > 256 << 20 {
            return Err(ContentError::Refused("overrides are over 256 MiB".into()));
        }
        let mut b = Vec::new();
        e.read_to_end(&mut b)?;
        out.push((rel.to_owned(), b));
    }
    Ok(out)
}
