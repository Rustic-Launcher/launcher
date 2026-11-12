//! Rustic Server plugins (`.rplugin`), per the final format reference on `main`
//! (`docs/PLUGIN_FORMAT.md`, implemented by `rusticmc-core`'s `manifest`/`archive`): a zip with
//! `rustic.plugin.toml` at its root, the WebAssembly component named by `plugin.entry`, an
//! optional `config.toml` and `resources/`; anything else is ignored. Installed into
//! `<server>/plugins/`, only for servers whose engine is Rustic Server (`horizon-server`'s
//! `[engine] path` pointing at the `rustic-server` executable).
//!
//! The launcher re-implements the checks rather than linking `rusticmc-core` (the launcher
//! worktree has no stable path to it, and it brings the runtime's dependencies): the rules of
//! PLUGIN_FORMAT.md sections 1 to 3 are mirrored here and tested (ADR 0064).
use crate::error::{ContentError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The plugin API the servers Rustic installs for implement.
pub const SERVER_API: (u32, u32) = (0, 1);
pub const MANIFEST: &str = "rustic.plugin.toml";
/// Caps while decompressing (PLUGIN_FORMAT.md section 2).
pub const MAX_MANIFEST: u64 = 64 << 10;
pub const MAX_CONFIG: u64 = 1 << 20;
pub const MAX_COMPONENT: u64 = 256 << 20;
pub const MAX_RESOURCE: u64 = 64 << 20;
pub const MAX_ENTRIES: usize = 10_000;
pub const RESERVED_IDS: &[&str] = &["rustic", "minecraft", "horizon"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub plugin: PluginInfo,
    #[serde(default)]
    pub dependencies: PluginDependencies,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: String,
    pub authors: Vec<String>,
    pub license: String,
    pub entry: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginDependencies {
    #[serde(default)]
    pub required: BTreeMap<String, String>,
    #[serde(default)]
    pub optional: BTreeMap<String, String>,
    #[serde(default)]
    pub load_before: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Network {
    /// Only `false` is valid: `true` is refused (list the domains).
    Allowed(bool),
    Domains(Vec<String>),
}

impl Default for Network {
    fn default() -> Self {
        Network::Allowed(false)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Filesystem {
    #[default]
    None,
    DataFolder,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default)]
    pub network: Network,
    #[serde(default)]
    pub filesystem: Filesystem,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default)]
    pub fuel_per_call: Option<u64>,
    #[serde(default)]
    pub memory_mb: Option<u32>,
}

/// A read and checked `.rplugin`.
#[derive(Clone, Debug, PartialEq)]
pub struct RPlugin {
    pub manifest: PluginManifest,
    pub version: semver::Version,
    pub sha256: String,
}

/// `[a-z0-9_-]{1,64}`, not `rustic`, `minecraft` or `horizon`.
pub fn valid_plugin_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && !RESERVED_IDS.contains(&id)
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// A host name: lowercase letters, digits and `-`, at least one dot; a leading `*.` matches any
/// subdomain. No scheme, port or path.
pub fn valid_domain(d: &str) -> bool {
    let d = d.strip_prefix("*.").unwrap_or(d);
    d.contains('.')
        && d.len() <= 253
        && d.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// Relative, forward slashes, no `.`/`..`/empty segment, ends in `.wasm`.
pub fn valid_entry(entry: &str) -> bool {
    entry.ends_with(".wasm")
        && !entry.starts_with('/')
        && !entry.contains('\\')
        && !entry.contains(':')
        && entry
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}

/// Whether a plugin targeting `api` runs on a server implementing `server`: while the major is
/// 0 the minor must match; from 1.0 the major must match and the minor be no newer.
pub fn api_compatible(api: &str, server: (u32, u32)) -> Option<bool> {
    let (maj, min) = api.split_once('.')?;
    if maj.is_empty()
        || min.is_empty()
        || !maj.bytes().chain(min.bytes()).all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let (maj, min): (u32, u32) = (maj.parse().ok()?, min.parse().ok()?);
    Some(if maj == 0 {
        server.0 == 0 && server.1 == min
    } else {
        server.0 == maj && min <= server.1
    })
}

fn bad(msg: impl Into<String>) -> ContentError {
    ContentError::Plugin(msg.into())
}

/// Every problem of a manifest, at once (PLUGIN_FORMAT.md section 1).
pub fn validate_manifest(m: &PluginManifest) -> Vec<String> {
    let mut e = Vec::new();
    let p = &m.plugin;
    if !valid_plugin_id(&p.id) {
        e.push(format!(
            "id {:?}: [a-z0-9_-]{{1,64}}, not rustic, minecraft or horizon",
            p.id
        ));
    }
    if p.name.trim().is_empty() || p.name.chars().count() > 64 {
        e.push("name: 1 to 64 characters".into());
    }
    if semver::Version::parse(&p.version).is_err() {
        e.push(format!("version {:?} is not semver", p.version));
    }
    match api_compatible(&p.api, SERVER_API) {
        None => e.push(format!("api {:?} is not MAJOR.MINOR", p.api)),
        Some(false) => e.push(format!(
            "plugin API {} doesn't run on API {}.{} servers",
            p.api, SERVER_API.0, SERVER_API.1
        )),
        Some(true) => {}
    }
    if p.authors.is_empty() || p.authors.iter().any(|a| a.trim().is_empty()) {
        e.push("authors: at least one non-empty name".into());
    }
    if p.license.trim().is_empty() {
        e.push("license: an SPDX expression".into());
    }
    if !valid_entry(&p.entry) {
        e.push(format!(
            "entry {:?}: a relative path ending in .wasm",
            p.entry
        ));
    }
    if p.description
        .as_ref()
        .is_some_and(|d| d.chars().count() > 1024)
    {
        e.push("description: at most 1024 characters".into());
    }
    let d = &m.dependencies;
    for (id, req) in d.required.iter().chain(&d.optional) {
        if !valid_plugin_id(id) || id == &p.id {
            e.push(format!("dependency {id:?} is not another valid plugin id"));
        }
        if semver::VersionReq::parse(req).is_err() {
            e.push(format!(
                "dependency {id} = {req:?}: not a version requirement"
            ));
        }
    }
    for id in d.required.keys() {
        if d.optional.contains_key(id) {
            e.push(format!("{id} is both required and optional"));
        }
    }
    for id in &d.load_before {
        if !valid_plugin_id(id) || id == &p.id {
            e.push(format!("load_before {id:?} is not another valid plugin id"));
        }
        if d.required.contains_key(id) || d.optional.contains_key(id) {
            e.push(format!("{id} is both a dependency and in load_before"));
        }
    }
    match &m.capabilities.network {
        Network::Allowed(true) => e.push("network = true is refused: list the domains".into()),
        Network::Allowed(false) => {}
        Network::Domains(ds) => {
            for dom in ds {
                if !valid_domain(dom) {
                    e.push(format!("network domain {dom:?} is not a host name"));
                }
            }
        }
    }
    if let Some(l) = &m.limits {
        if l.fuel_per_call == Some(0) {
            e.push("limits.fuel_per_call is positive".into());
        }
        if l.memory_mb.is_some_and(|mb| !(1..=4096).contains(&mb)) {
            e.push("limits.memory_mb is 1 to 4096".into());
        }
    }
    e
}

type Archive = zip::ZipArchive<std::io::Cursor<Vec<u8>>>;

/// Reads entry `name`, refusing it past `cap` bytes; None when absent.
fn read_capped(zip: &mut Archive, name: &str, cap: u64) -> Result<Option<Vec<u8>>> {
    let e = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    if e.size() > cap {
        return Err(bad(format!("{name} is over {cap} bytes")));
    }
    let mut b = Vec::new();
    e.take(cap + 1).read_to_end(&mut b)?;
    if b.len() as u64 > cap {
        return Err(bad(format!("{name} is over {cap} bytes")));
    }
    Ok(Some(b))
}

/// Reads and validates an `.rplugin` without running anything.
pub fn read_rplugin(path: &Path) -> Result<RPlugin> {
    let bytes = std::fs::read(path)?;
    let sha256 = crate::download::digest(&bytes).sha256.unwrap_or_default();
    let mut zip: Archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    if zip.len() > MAX_ENTRIES {
        return Err(bad(format!("more than {MAX_ENTRIES} entries")));
    }
    let names: Vec<String> = zip.file_names().map(str::to_owned).collect();
    for n in &names {
        let drive = n.as_bytes().get(1) == Some(&b':');
        if n.starts_with('/') || n.contains('\\') || drive || n.split('/').any(|p| p == "..") {
            return Err(bad(format!("unsafe path {n:?} in the archive")));
        }
    }
    let manifest_bytes = read_capped(&mut zip, MANIFEST, MAX_MANIFEST)?
        .ok_or_else(|| bad(format!("no {MANIFEST} at the root")))?;
    let manifest: PluginManifest = toml::from_str(&String::from_utf8_lossy(&manifest_bytes))
        .map_err(|e| bad(format!("{MANIFEST}: {e}")))?;
    let problems = validate_manifest(&manifest);
    if !problems.is_empty() {
        return Err(bad(problems.join("; ")));
    }
    let entry = read_capped(&mut zip, &manifest.plugin.entry, MAX_COMPONENT)?
        .ok_or_else(|| bad(format!("entry {:?} is missing", manifest.plugin.entry)))?;
    if !entry.starts_with(b"\0asm") {
        return Err(bad("the entry is not a WebAssembly component"));
    }
    if let Some(cfg) = read_capped(&mut zip, "config.toml", MAX_CONFIG)? {
        toml::from_str::<toml::Table>(&String::from_utf8_lossy(&cfg))
            .map_err(|e| bad(format!("config.toml: {e}")))?;
    }
    for i in 0..zip.len() {
        let e = zip.by_index(i)?;
        if e.name().starts_with("resources/") && e.size() > MAX_RESOURCE {
            return Err(bad(format!("{} is over 64 MiB", e.name())));
        }
    }
    let version =
        semver::Version::parse(&manifest.plugin.version).map_err(|e| bad(e.to_string()))?;
    Ok(RPlugin {
        manifest,
        version,
        sha256,
    })
}

/// What a server folder runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerInfo {
    /// `[engine] path` of `server.toml`, when set.
    pub engine: Option<PathBuf>,
    /// The engine is the `rustic-server` executable: Rustic Server.
    pub rustic: bool,
}

impl ServerInfo {
    pub fn detect(server_dir: &Path) -> ServerInfo {
        let engine = std::fs::read_to_string(server_dir.join("server.toml"))
            .ok()
            .and_then(|t| toml::from_str::<toml::Table>(&t).ok())
            .and_then(|t| {
                t.get("engine")?
                    .get("path")?
                    .as_str()
                    .filter(|p| !p.is_empty())
                    .map(PathBuf::from)
            });
        let rustic = engine.as_ref().is_some_and(|p| {
            // Spelled for any OS: `.../rustic-server`, `...\rustic-server.exe`.
            let name = p
                .to_string_lossy()
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("")
                .to_lowercase();
            name == "rustic-server" || name == "rustic-server.exe"
        });
        ServerInfo { engine, rustic }
    }
}

/// A load order and who is refused (PLUGIN_FORMAT.md section 3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolution {
    pub order: Vec<String>,
    /// Plugins the server would not load, with why.
    pub refused: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

fn matches(req: &str, version: &str) -> bool {
    match (
        semver::VersionReq::parse(req),
        semver::Version::parse(version),
    ) {
        // `*` matches every version, pre-releases included.
        (_, Ok(_)) if req.trim() == "*" => true,
        (Ok(r), Ok(v)) => r.matches(&v),
        _ => false,
    }
}

/// Resolves `set` as the server does: duplicate ids refuse every copy; a missing, mismatched or
/// refused required dependency refuses (transitively); a required cycle refuses its members;
/// optional dependencies and `load_before` only order (a mismatched optional one is ignored
/// with a warning, a soft cycle broken with one); ties by id.
pub fn resolve(set: &[PluginManifest]) -> Resolution {
    let mut res = Resolution::default();
    let mut by_id: BTreeMap<&str, &PluginManifest> = BTreeMap::new();
    let mut dup: Vec<&str> = Vec::new();
    for m in set {
        if by_id.insert(m.plugin.id.as_str(), m).is_some() && !dup.contains(&m.plugin.id.as_str()) {
            dup.push(m.plugin.id.as_str());
        }
    }
    for d in &dup {
        by_id.remove(d);
        res.refused.push((
            (*d).to_owned(),
            "two packages declare this id: none is loaded".into(),
        ));
    }
    // Required dependencies, refused transitively until nothing changes.
    let mut alive: BTreeMap<&str, &PluginManifest> = by_id.clone();
    loop {
        let mut out: Vec<(String, String)> = Vec::new();
        for (id, m) in &alive {
            let why = m.dependencies.required.iter().find_map(|(dep, req)| {
                match alive.get(dep.as_str()) {
                    None if by_id.contains_key(dep.as_str()) || dup.contains(&dep.as_str()) => {
                        Some(format!("required dependency '{dep}' is refused"))
                    }
                    None => Some(format!(
                        "required dependency '{dep}' ({req}) is not installed"
                    )),
                    Some(d) if !matches(req, &d.plugin.version) => Some(format!(
                        "required dependency '{dep}' {req}: {} is installed",
                        d.plugin.version
                    )),
                    Some(_) => None,
                }
            });
            if let Some(why) = why {
                out.push(((*id).to_owned(), why));
            }
        }
        if out.is_empty() {
            break;
        }
        for (id, why) in out {
            alive.remove(id.as_str());
            res.refused.push((id, why));
        }
    }
    // Required cycles: members never placed by required edges alone.
    let mut placed: Vec<&str> = Vec::new();
    loop {
        let next: Vec<&str> = alive
            .iter()
            .filter(|(id, m)| {
                !placed.contains(id)
                    && m.dependencies
                        .required
                        .keys()
                        .all(|d| placed.contains(&d.as_str()))
            })
            .map(|(id, _)| *id)
            .collect();
        if next.is_empty() {
            break;
        }
        placed.extend(next);
    }
    let cyclic: Vec<&str> = alive
        .keys()
        .filter(|id| !placed.contains(id))
        .copied()
        .collect();
    if !cyclic.is_empty() {
        let names = cyclic.join(", ");
        for id in &cyclic {
            alive.remove(id);
            res.refused.push((
                (*id).to_owned(),
                format!("required dependency cycle between {names}"),
            ));
        }
    }
    // Edges (a, b, hard): a loads before b.
    let mut edges: Vec<(&str, &str, bool)> = Vec::new();
    for (id, m) in &alive {
        for dep in m.dependencies.required.keys() {
            edges.push((dep.as_str(), id, true));
        }
        for (dep, req) in &m.dependencies.optional {
            if let Some(d) = alive.get(dep.as_str()) {
                if matches(req, &d.plugin.version) {
                    edges.push((dep.as_str(), id, false));
                } else {
                    res.warnings.push(format!(
                        "{id}: optional dependency '{dep}' {req} doesn't match {}: ignored",
                        d.plugin.version
                    ));
                }
            }
        }
        for later in &m.dependencies.load_before {
            if alive.contains_key(later.as_str()) {
                edges.push((id, later.as_str(), false));
            }
        }
    }
    // Kahn's algorithm, ties by id; a soft cycle drops the soft edges with a warning.
    let mut remaining: Vec<&str> = alive.keys().copied().collect();
    while !remaining.is_empty() {
        let ready = remaining.iter().copied().find(|n| {
            !edges
                .iter()
                .any(|(a, b, _)| b == n && remaining.contains(a))
        });
        match ready {
            Some(n) => {
                res.order.push(n.to_owned());
                remaining.retain(|x| *x != n);
            }
            None => {
                res.warnings.push(format!(
                    "an optional/load_before cycle between {}: broken",
                    remaining.join(", ")
                ));
                edges.retain(|(_, _, hard)| *hard);
            }
        }
    }
    res.refused.sort();
    res
}

/// The plugins installed in `<server>/plugins/`.
pub fn installed(server_dir: &Path) -> Result<Vec<(PathBuf, RPlugin)>> {
    let dir = server_dir.join("plugins");
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    for e in read {
        let path = e?.path();
        if path.extension().is_some_and(|x| x == "rplugin") {
            // A broken package already there doesn't block installing others.
            match read_rplugin(&path) {
                Ok(p) => out.push((path.clone(), p)),
                Err(e) => tracing::warn!("{}: {e}", path.display()),
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// What [`install`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginInstall {
    pub id: String,
    pub path: PathBuf,
    pub replaced: Option<String>,
    pub load_order: Vec<String>,
    pub warnings: Vec<String>,
}

/// Installs `file` into a Rustic Server's `plugins/` after checking it, the server and the
/// dependencies of every plugin there with it (a newer version replaces the old one).
pub fn install(server_dir: &Path, file: &Path) -> Result<PluginInstall> {
    let info = ServerInfo::detect(server_dir);
    if !info.rustic {
        return Err(ContentError::Incompatible(
            "this server runs vanilla Horizon: set its engine to Rustic Server first".into(),
        ));
    }
    let new = read_rplugin(file)?;
    let id = new.manifest.plugin.id.clone();
    let present = installed(server_dir)?;
    let replaced = present
        .iter()
        .find(|(_, p)| p.manifest.plugin.id == id)
        .map(|(path, p)| (path.clone(), p.manifest.plugin.version.clone()));
    let mut set: Vec<PluginManifest> = present
        .iter()
        .filter(|(_, p)| p.manifest.plugin.id != id)
        .map(|(_, p)| p.manifest.clone())
        .collect();
    set.push(new.manifest.clone());
    let resolution = resolve(&set);
    if let Some((_, why)) = resolution.refused.iter().find(|(r, _)| r == &id) {
        return Err(ContentError::Dependency(format!(
            "{id} would not load: {why}"
        )));
    }
    let load_order = resolution.order;
    let dir = server_dir.join("plugins");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{id}.rplugin"));
    if let Some((old, _)) = &replaced
        && old != &dest
    {
        std::fs::remove_file(old)?;
    }
    let tmp = dir.join(format!(".{id}.rplugin.part"));
    std::fs::copy(file, &tmp)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(PluginInstall {
        id,
        path: dest,
        replaced: replaced.map(|(_, v)| v),
        load_order,
        warnings: resolution
            .warnings
            .into_iter()
            .chain(
                resolution
                    .refused
                    .into_iter()
                    .map(|(p, why)| format!("{p} will not load: {why}")),
            )
            .collect(),
    })
}

/// Removes plugin `id` unless another installed plugin requires it.
pub fn uninstall(server_dir: &Path, id: &str) -> Result<()> {
    let present = installed(server_dir)?;
    if let Some((_, needs)) = present
        .iter()
        .find(|(_, p)| p.manifest.dependencies.required.contains_key(id))
    {
        return Err(ContentError::Dependency(format!(
            "{} requires {id}: remove it first",
            needs.manifest.plugin.id
        )));
    }
    let (path, _) = present
        .iter()
        .find(|(_, p)| p.manifest.plugin.id == id)
        .ok_or_else(|| ContentError::NotFound(format!("plugin {id}")))?;
    std::fs::remove_file(path)?;
    Ok(())
}
