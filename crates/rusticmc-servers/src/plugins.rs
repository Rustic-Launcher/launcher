//! Rustic Server plugins (`<server>/plugins/*.rplugin`): read and validate `rustic.plugin.toml`
//! per the contract in launcher/AGENTS.md (mirroring the rules of the server's own reader,
//! `crates/rusticmc-core/src/manifest.rs` on `main`: unknown keys refused, ids `[a-z0-9_-]{1,64}`
//! and not reserved, semver version, api `MAJOR.MINOR` compatible with 0.1, entry a relative
//! `.wasm` path present in the archive, network `false` or a domain list, filesystem `none` or
//! `data-folder`), enable/disable by moving files to and from `plugins/.disabled/`, dependency
//! checks, installs and updates by version.

use crate::error::{Result, ServerError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "rustic.plugin.toml";
pub const EXTENSION: &str = "rplugin";
pub const DISABLED_DIR: &str = ".disabled";
/// The plugin API the bundled Rustic Server implements.
pub const HOST_API: (u64, u64) = (0, 1);
const RESERVED_IDS: [&str; 3] = ["rustic", "minecraft", "horizon"];
const MANIFEST_BYTES: u64 = 64 * 1024;
const CONFIG_FILE: &str = "config.toml";
const CONFIG_BYTES: u64 = 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;

/// Whether `version` matches `req` as Rustic Server does (docs/PLUGIN_FORMAT.md): `*` matches
/// every version, pre-releases included; other requirements follow semver.
pub fn requirement_matches(req: &str, version: &semver::Version) -> bool {
    if req.trim() == "*" {
        return true;
    }
    semver::VersionReq::parse(req).is_ok_and(|r| r.matches(version))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    plugin: RawPlugin,
    #[serde(default)]
    dependencies: RawDependencies,
    #[serde(default)]
    capabilities: RawCapabilities,
    #[serde(default)]
    limits: Option<RawLimits>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlugin {
    id: String,
    name: String,
    version: String,
    api: String,
    authors: Vec<String>,
    license: String,
    entry: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDependencies {
    #[serde(default)]
    required: BTreeMap<String, String>,
    #[serde(default)]
    optional: BTreeMap<String, String>,
    #[serde(default)]
    load_before: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawNetwork {
    Flag(bool),
    Domains(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilities {
    #[serde(default = "no_network")]
    network: RawNetwork,
    #[serde(default = "no_fs")]
    filesystem: String,
}

impl Default for RawCapabilities {
    fn default() -> Self {
        Self {
            network: no_network(),
            filesystem: no_fs(),
        }
    }
}

fn no_network() -> RawNetwork {
    RawNetwork::Flag(false)
}
fn no_fs() -> String {
    "none".into()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimits {
    #[serde(default)]
    fuel_per_call: Option<u64>,
    #[serde(default)]
    memory_mb: Option<u32>,
}

/// A validated manifest, as the UI shows it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub api: String,
    pub authors: Vec<String>,
    pub license: String,
    pub entry: String,
    pub description: Option<String>,
    pub required: BTreeMap<String, String>,
    pub optional: BTreeMap<String, String>,
    pub load_before: Vec<String>,
    /// `None`: no network; else the hosts it may reach.
    pub network: Option<Vec<String>>,
    /// `none` or `data-folder`.
    pub filesystem: String,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
}

fn valid_domain(d: &str) -> bool {
    let body = d.strip_prefix("*.").unwrap_or(d);
    !body.is_empty()
        && body.len() <= 253
        && body.contains('.')
        && body.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

fn valid_entry(entry: &str) -> std::result::Result<(), String> {
    if entry.is_empty()
        || entry.starts_with('/')
        || entry.contains('\\')
        || entry.contains(':')
        || entry
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(format!(
            "entry `{entry}` must be a relative path with forward slashes"
        ));
    }
    if !entry.ends_with(".wasm") {
        return Err(format!("entry `{entry}` must be a `.wasm` file"));
    }
    Ok(())
}

fn api_compatible(api: &str) -> std::result::Result<(), String> {
    let parts: Vec<&str> = api.split('.').collect();
    let num = |p: &str| {
        (!p.is_empty()
            && p.bytes().all(|b| b.is_ascii_digit())
            && !(p.len() > 1 && p.starts_with('0')))
        .then(|| p.parse::<u64>().ok())
        .flatten()
    };
    let [a, b] = parts.as_slice() else {
        return Err(format!("api `{api}` must be \"MAJOR.MINOR\""));
    };
    let (Some(major), Some(minor)) = (num(a), num(b)) else {
        return Err(format!("api `{api}` must be \"MAJOR.MINOR\""));
    };
    let (hm, hn) = HOST_API;
    let ok = major == hm && if major == 0 { minor == hn } else { minor <= hn };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "api {api} is not compatible with this server's plugin API {hm}.{hn}"
        ))
    }
}

impl PluginManifest {
    /// Parses and validates; every problem found is reported.
    pub fn parse(text: &str) -> Result<Self> {
        let raw: RawManifest = toml::from_str(text).map_err(|e| {
            ServerError::Plugin(format!("{MANIFEST_FILE}: {}", e.to_string().trim()))
        })?;
        let mut errors = Vec::new();
        let p = raw.plugin;
        if !valid_id(&p.id) {
            errors.push(format!("id `{}` must be 1-64 of a-z 0-9 _ -", p.id));
        } else if RESERVED_IDS.contains(&p.id.as_str()) {
            errors.push(format!("id `{}` is reserved", p.id));
        }
        let name = p.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 64 {
            errors.push("name must be 1 to 64 characters".into());
        }
        let version = semver::Version::parse(&p.version)
            .map_err(|e| errors.push(format!("version `{}` is not semver: {e}", p.version)))
            .ok();
        if let Err(e) = api_compatible(&p.api) {
            errors.push(e);
        }
        if p.authors.is_empty() || p.authors.iter().any(|a| a.trim().is_empty()) {
            errors.push("authors must list at least one non-empty name".into());
        }
        if p.license.trim().is_empty() {
            errors.push("license must not be empty".into());
        }
        if let Err(e) = valid_entry(&p.entry) {
            errors.push(e);
        }
        if p.description
            .as_ref()
            .is_some_and(|d| d.chars().count() > 1024)
        {
            errors.push("description must be at most 1024 characters".into());
        }
        let deps = raw.dependencies;
        for (kind, map) in [("required", &deps.required), ("optional", &deps.optional)] {
            for (k, req) in map {
                if !valid_id(k) {
                    errors.push(format!("{kind} dependency `{k}` is not a valid plugin id"));
                } else if *k == p.id {
                    errors.push(format!(
                        "{kind} dependency `{k}`: a plugin cannot depend on itself"
                    ));
                }
                if let Err(e) = semver::VersionReq::parse(req) {
                    errors.push(format!(
                        "{kind} dependency `{k}`: bad version requirement `{req}`: {e}"
                    ));
                }
            }
        }
        for k in deps.optional.keys() {
            if deps.required.contains_key(k) {
                errors.push(format!(
                    "`{k}` is both a required and an optional dependency"
                ));
            }
        }
        for k in &deps.load_before {
            if !valid_id(k) || *k == p.id {
                errors.push(format!("load_before `{k}` is not a valid other plugin id"));
            } else if deps.required.contains_key(k) || deps.optional.contains_key(k) {
                errors.push(format!("`{k}` is both a dependency and in load_before"));
            }
        }
        let network = match raw.capabilities.network {
            RawNetwork::Flag(false) => None,
            RawNetwork::Flag(true) => {
                errors.push("capabilities.network = true is not allowed: list the domains".into());
                None
            }
            RawNetwork::Domains(list) => {
                let mut out: Vec<String> = Vec::new();
                for d in list {
                    let d = d.trim().trim_end_matches('.').to_ascii_lowercase();
                    if valid_domain(&d) {
                        if !out.contains(&d) {
                            out.push(d);
                        }
                    } else {
                        errors.push(format!("capabilities.network: `{d}` is not a host name"));
                    }
                }
                (!out.is_empty()).then_some(out)
            }
        };
        let filesystem = raw.capabilities.filesystem;
        if filesystem != "none" && filesystem != "data-folder" {
            errors.push(format!(
                "capabilities.filesystem must be \"none\" or \"data-folder\", not `{filesystem}`"
            ));
        }
        if let Some(l) = &raw.limits {
            if l.memory_mb.is_some_and(|m| m == 0 || m > 4096) {
                errors.push("limits.memory_mb must be 1 to 4096".into());
            }
            if l.fuel_per_call == Some(0) {
                errors.push("limits.fuel_per_call must be positive".into());
            }
        }
        match (errors.is_empty(), version) {
            (true, Some(version)) => Ok(Self {
                id: p.id,
                name,
                version,
                api: p.api,
                authors: p.authors,
                license: p.license,
                entry: p.entry,
                description: p.description,
                required: deps.required,
                optional: deps.optional,
                load_before: deps.load_before,
                network,
                filesystem,
            }),
            _ => Err(ServerError::Plugin(format!(
                "{MANIFEST_FILE}: {}",
                errors.join("; ")
            ))),
        }
    }

    /// Reads and validates an `.rplugin` archive (the manifest at its root, the entry present).
    pub fn read_archive(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(|e| ServerError::io(path, e))?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| {
            ServerError::Plugin(format!("{}: not a .rplugin (zip): {e}", path.display()))
        })?;
        let text = {
            let entry = zip.by_name(MANIFEST_FILE).map_err(|_| {
                ServerError::Plugin(format!(
                    "{}: missing {MANIFEST_FILE} at the root",
                    path.display()
                ))
            })?;
            let mut text = String::new();
            entry
                .take(MANIFEST_BYTES + 1)
                .read_to_string(&mut text)
                .map_err(|e| ServerError::Plugin(format!("{MANIFEST_FILE}: {e}")))?;
            if text.len() as u64 > MANIFEST_BYTES {
                return Err(ServerError::Plugin(format!("{MANIFEST_FILE} is too large")));
            }
            text
        };
        let manifest = Self::parse(&text)?;
        if zip.by_name(&manifest.entry).is_err() {
            return Err(ServerError::Plugin(format!(
                "entry `{}` named in {MANIFEST_FILE} is not in the archive",
                manifest.entry
            )));
        }
        if zip.len() > MAX_ENTRIES {
            return Err(ServerError::Plugin(format!(
                "more than {MAX_ENTRIES} entries"
            )));
        }
        for i in 0..zip.len() {
            let entry = zip
                .by_index_raw(i)
                .map_err(|e| ServerError::Plugin(e.to_string()))?;
            if entry.enclosed_name().is_none() || entry.name().contains('\\') {
                return Err(ServerError::Plugin(format!(
                    "unsafe entry name `{}`",
                    entry.name()
                )));
            }
        }
        if let Ok(config) = zip.by_name(CONFIG_FILE) {
            let mut text = String::new();
            config
                .take(CONFIG_BYTES + 1)
                .read_to_string(&mut text)
                .map_err(|e| ServerError::Plugin(format!("{CONFIG_FILE}: {e}")))?;
            if text.len() as u64 > CONFIG_BYTES {
                return Err(ServerError::Plugin(format!(
                    "{CONFIG_FILE} is larger than 1 MiB"
                )));
            }
            toml::from_str::<toml::Table>(&text).map_err(|e| {
                ServerError::Plugin(format!(
                    "{CONFIG_FILE} is not valid TOML: {}",
                    e.to_string().trim()
                ))
            })?;
        }
        Ok(manifest)
    }

    /// Reads a dev-mode plugin directory (the manifest at its root, the entry present).
    pub fn read_dir(dir: &Path) -> Result<Self> {
        let path = dir.join(MANIFEST_FILE);
        let meta = std::fs::metadata(&path).map_err(|e| ServerError::io(&path, e))?;
        if meta.len() > MANIFEST_BYTES {
            return Err(ServerError::Plugin(format!("{MANIFEST_FILE} is too large")));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| ServerError::io(&path, e))?;
        let manifest = Self::parse(&text)?;
        if !dir.join(&manifest.entry).is_file() {
            return Err(ServerError::Plugin(format!(
                "entry `{}` named in {MANIFEST_FILE} does not exist",
                manifest.entry
            )));
        }
        Ok(manifest)
    }
}

/// An installed plugin.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    /// The `.rplugin` file, or the dev-mode directory.
    pub file: PathBuf,
    pub enabled: bool,
    /// An unpacked dev-mode directory rather than an archive.
    #[serde(default)]
    pub dev: bool,
}

/// A problem a dependency check found.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DependencyProblem {
    Missing {
        plugin: String,
        dependency: String,
        requirement: String,
    },
    Disabled {
        plugin: String,
        dependency: String,
    },
    WrongVersion {
        plugin: String,
        dependency: String,
        requirement: String,
        found: String,
    },
    /// Two enabled packages with the same id: the server loads none of them.
    Duplicate {
        plugin: String,
    },
    /// Required dependencies in a cycle: all its members are refused.
    Cycle {
        plugins: Vec<String>,
    },
}

/// Cycles among enabled plugins' required dependencies (each reported once, members sorted).
fn required_cycles(plugins: &[InstalledPlugin]) -> Vec<DependencyProblem> {
    let enabled: BTreeMap<&str, &PluginManifest> = plugins
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (p.manifest.id.as_str(), &p.manifest))
        .collect();
    let mut found: Vec<Vec<String>> = Vec::new();
    for start in enabled.keys() {
        let mut stack: Vec<(&str, Vec<&str>)> = vec![(start, vec![*start])];
        while let Some((node, path)) = stack.pop() {
            let Some(m) = enabled.get(node) else { continue };
            for dep in m.required.keys() {
                if dep == start {
                    let mut members: Vec<String> = path.iter().map(|s| (*s).to_owned()).collect();
                    members.sort();
                    if !found.contains(&members) {
                        found.push(members);
                    }
                } else if !path.contains(&dep.as_str()) && enabled.contains_key(dep.as_str()) {
                    let mut next = path.clone();
                    next.push(dep);
                    stack.push((dep, next));
                }
            }
        }
    }
    found
        .into_iter()
        .map(|plugins| DependencyProblem::Cycle { plugins })
        .collect()
}

/// A file that isn't a valid plugin.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvalidPlugin {
    pub file: PathBuf,
    pub error: String,
}

/// A server's plugins folder.
pub struct PluginManager {
    dir: PathBuf,
}

impl PluginManager {
    /// `server_dir/plugins`.
    pub fn new(server_dir: &Path) -> Self {
        Self {
            dir: server_dir.join("plugins"),
        }
    }

    fn disabled_dir(&self) -> PathBuf {
        self.dir.join(DISABLED_DIR)
    }

    fn scan(
        dir: &Path,
        enabled: bool,
        out: &mut Vec<InstalledPlugin>,
        bad: &mut Vec<InvalidPlugin>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                (p.extension().is_some_and(|e| e == EXTENSION) && p.is_file())
                    // Dev-mode directories have a manifest; data folders don't.
                    || (p.is_dir() && p.join(MANIFEST_FILE).is_file())
            })
            .collect();
        files.sort();
        for file in files {
            let dev = file.is_dir();
            let read = if dev {
                PluginManifest::read_dir(&file)
            } else {
                PluginManifest::read_archive(&file)
            };
            match read {
                Ok(manifest) => out.push(InstalledPlugin {
                    manifest,
                    file,
                    enabled,
                    dev,
                }),
                Err(e) => bad.push(InvalidPlugin {
                    file,
                    error: e.to_string(),
                }),
            }
        }
    }

    /// Every plugin (enabled and disabled) and every invalid file.
    pub fn list(&self) -> (Vec<InstalledPlugin>, Vec<InvalidPlugin>) {
        let mut out = Vec::new();
        let mut bad = Vec::new();
        Self::scan(&self.dir, true, &mut out, &mut bad);
        Self::scan(&self.disabled_dir(), false, &mut out, &mut bad);
        out.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
        (out, bad)
    }

    fn find(&self, id: &str) -> Result<InstalledPlugin> {
        self.list()
            .0
            .into_iter()
            .find(|p| p.manifest.id == id)
            .ok_or_else(|| ServerError::Plugin(format!("no plugin {id}")))
    }

    /// Installs (or updates) from an `.rplugin` file: validated first; an installed plugin with the
    /// same id is replaced only by a newer version (or the same one with `allow_same`), keeping its
    /// enabled state. Returns the installed plugin.
    pub fn install(&self, archive: &Path, allow_downgrade: bool) -> Result<InstalledPlugin> {
        let manifest = PluginManifest::read_archive(archive)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| ServerError::io(&self.dir, e))?;
        let existing = self.find(&manifest.id).ok();
        if let Some(old) = &existing
            && manifest.version < old.manifest.version
            && !allow_downgrade
        {
            return Err(ServerError::Plugin(format!(
                "{} {} is older than the installed {}",
                manifest.id, manifest.version, old.manifest.version
            )));
        }
        let enabled = existing.as_ref().is_none_or(|o| o.enabled);
        let folder = if enabled {
            self.dir.clone()
        } else {
            self.disabled_dir()
        };
        std::fs::create_dir_all(&folder).map_err(|e| ServerError::io(&folder, e))?;
        let target = folder.join(format!("{}-{}.{EXTENSION}", manifest.id, manifest.version));
        let tmp = folder.join(format!(".{}.installing", manifest.id));
        std::fs::copy(archive, &tmp).map_err(|e| ServerError::io(&tmp, e))?;
        if let Some(old) = &existing {
            if old.dev {
                let _ = std::fs::remove_file(&tmp);
                return Err(ServerError::Plugin(format!(
                    "{} is a dev-mode directory ({}): remove it before installing an archive",
                    manifest.id,
                    old.file.display()
                )));
            }
            if old.file != target {
                std::fs::remove_file(&old.file).map_err(|e| ServerError::io(&old.file, e))?;
            }
        }
        std::fs::rename(&tmp, &target).map_err(|e| ServerError::io(&target, e))?;
        Ok(InstalledPlugin {
            manifest,
            file: target,
            enabled,
            dev: false,
        })
    }

    /// Enables or disables a plugin by moving its file (the server reads `plugins/*.rplugin` only).
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<InstalledPlugin> {
        let mut plugin = self.find(id)?;
        if plugin.enabled == enabled {
            return Ok(plugin);
        }
        let folder = if enabled {
            self.dir.clone()
        } else {
            self.disabled_dir()
        };
        std::fs::create_dir_all(&folder).map_err(|e| ServerError::io(&folder, e))?;
        let target = folder.join(plugin.file.file_name().unwrap_or_default());
        std::fs::rename(&plugin.file, &target).map_err(|e| ServerError::io(&target, e))?;
        plugin.file = target;
        plugin.enabled = enabled;
        Ok(plugin)
    }

    /// Removes a plugin's package (its data folder `plugins/<id>/` stays).
    pub fn remove(&self, id: &str) -> Result<()> {
        let plugin = self.find(id)?;
        if plugin.dev {
            std::fs::remove_dir_all(&plugin.file)
        } else {
            std::fs::remove_file(&plugin.file)
        }
        .map_err(|e| ServerError::io(&plugin.file, e))
    }

    /// Problems with the enabled plugins' dependencies, as Rustic Server's resolution would find
    /// them (docs/PLUGIN_FORMAT.md section 3): duplicate ids (none of them loads), a missing,
    /// disabled or non-matching required dependency, required-dependency cycles, and optional
    /// dependencies present at a non-matching version (ignored by the server, with a warning).
    pub fn check_dependencies(&self) -> Vec<DependencyProblem> {
        let (plugins, _) = self.list();
        let mut problems = Vec::new();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for p in plugins.iter().filter(|p| p.enabled) {
            *counts.entry(&p.manifest.id).or_default() += 1;
        }
        for (id, n) in &counts {
            if *n > 1 {
                problems.push(DependencyProblem::Duplicate {
                    plugin: (*id).to_owned(),
                });
            }
        }
        for p in plugins.iter().filter(|p| p.enabled) {
            for (dep, req) in &p.manifest.required {
                match plugins.iter().find(|o| &o.manifest.id == dep) {
                    None => problems.push(DependencyProblem::Missing {
                        plugin: p.manifest.id.clone(),
                        dependency: dep.clone(),
                        requirement: req.clone(),
                    }),
                    Some(o) if !o.enabled => problems.push(DependencyProblem::Disabled {
                        plugin: p.manifest.id.clone(),
                        dependency: dep.clone(),
                    }),
                    Some(o) if !requirement_matches(req, &o.manifest.version) => {
                        problems.push(DependencyProblem::WrongVersion {
                            plugin: p.manifest.id.clone(),
                            dependency: dep.clone(),
                            requirement: req.clone(),
                            found: o.manifest.version.to_string(),
                        })
                    }
                    Some(_) => {}
                }
            }
            for (dep, req) in &p.manifest.optional {
                if let Some(o) = plugins.iter().find(|o| &o.manifest.id == dep && o.enabled)
                    && !requirement_matches(req, &o.manifest.version)
                {
                    problems.push(DependencyProblem::WrongVersion {
                        plugin: p.manifest.id.clone(),
                        dependency: dep.clone(),
                        requirement: req.clone(),
                        found: o.manifest.version.to_string(),
                    });
                }
            }
        }
        problems.extend(required_cycles(&plugins));
        problems
    }

    /// Which plugins depend on `id` (refuse disabling it without saying so).
    pub fn dependents(&self, id: &str) -> Vec<String> {
        self.list()
            .0
            .into_iter()
            .filter(|p| p.enabled && p.manifest.required.contains_key(id))
            .map(|p| p.manifest.id)
            .collect()
    }
}

/// Writes an `.rplugin` (tests and the plugin tooling's docs).
pub fn write_test_archive(path: &Path, manifest: &str, entry: Option<&str>) {
    use std::io::Write;
    let file = std::fs::File::create(path).expect("create");
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file(MANIFEST_FILE, opts).expect("zip");
    zip.write_all(manifest.as_bytes()).expect("zip");
    if let Some(entry) = entry {
        zip.start_file(entry, opts).expect("zip");
        zip.write_all(b"\0asm\x01\0\0\0").expect("zip");
    }
    zip.finish().expect("zip");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str, version: &str, deps: &str) -> String {
        format!(
            "[plugin]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\napi = \"0.1\"\nauthors = [\"Sam\"]\nlicense = \"MIT\"\nentry = \"plugin.wasm\"\n{deps}"
        )
    }

    fn archive(dir: &Path, id: &str, version: &str, deps: &str) -> PathBuf {
        let p = dir.join(format!("{id}-{version}-src.{EXTENSION}"));
        write_test_archive(&p, &manifest(id, version, deps), Some("plugin.wasm"));
        p
    }

    #[test]
    fn manifests_are_validated_strictly() {
        let ok = PluginManifest::parse(&format!(
            "{}\n[capabilities]\nnetwork = [\"api.example.com\", \"*.Example.org.\"]\nfilesystem = \"data-folder\"\n",
            manifest("chat-format", "1.2.0", "")
        ))
        .unwrap();
        assert_eq!(
            ok.network.as_deref(),
            Some(&["api.example.com".to_owned(), "*.example.org".to_owned()][..])
        );
        let bad = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
        assert!(bad(&manifest("Bad Id", "1.0.0", "")).contains("id"));
        assert!(bad(&manifest("rustic", "1.0.0", "")).contains("reserved"));
        assert!(bad(&manifest("x", "1.0", "")).contains("semver"));
        assert!(bad(&manifest("x", "1.0.0", "").replace("0.1", "0.2")).contains("not compatible"));
        assert!(
            bad(&manifest("x", "1.0.0", "").replace("plugin.wasm", "../a.wasm")).contains("entry")
        );
        assert!(
            bad(&format!(
                "{}\n[capabilities]\nnetwork = true\n",
                manifest("x", "1.0.0", "")
            ))
            .contains("network")
        );
        assert!(
            bad(&format!("{}\nsurprise = 1\n", manifest("x", "1.0.0", ""))).contains("unknown")
        );
        assert!(
            bad(&manifest(
                "x",
                "1.0.0",
                "[dependencies]\nrequired = { x = \"1\" }"
            ))
            .contains("itself")
        );
    }

    #[test]
    fn install_enable_disable_update() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let pm = PluginManager::new(&t.path().join("server"));
        let core = pm
            .install(&archive(&src, "perms", "1.0.0", ""), false)
            .unwrap();
        assert!(core.enabled && core.file.ends_with("perms-1.0.0.rplugin"));
        pm.install(
            &archive(
                &src,
                "essentials-lite",
                "0.3.0",
                "[dependencies]\nrequired = { perms = \">=1.1\" }",
            ),
            false,
        )
        .unwrap();
        // perms 1.0.0 doesn't satisfy >=1.1.
        assert!(matches!(
            pm.check_dependencies().as_slice(),
            [DependencyProblem::WrongVersion { dependency, .. }] if dependency == "perms"
        ));
        // Update by version.
        let updated = pm
            .install(&archive(&src, "perms", "1.2.0", ""), false)
            .unwrap();
        assert!(updated.file.ends_with("perms-1.2.0.rplugin"));
        assert!(!t.path().join("server/plugins/perms-1.0.0.rplugin").exists());
        assert!(pm.check_dependencies().is_empty());
        assert!(
            pm.install(&archive(&src, "perms", "1.1.0", ""), false)
                .is_err(),
            "no silent downgrade"
        );
        // Disable: moved into .disabled, dependents reported.
        assert_eq!(pm.dependents("perms"), ["essentials-lite"]);
        let off = pm.set_enabled("perms", false).unwrap();
        assert!(
            off.file
                .starts_with(t.path().join("server/plugins/.disabled"))
        );
        assert!(matches!(
            pm.check_dependencies().as_slice(),
            [DependencyProblem::Disabled { .. }]
        ));
        // An update keeps it disabled.
        let again = pm
            .install(&archive(&src, "perms", "1.3.0", ""), false)
            .unwrap();
        assert!(!again.enabled);
        pm.set_enabled("perms", true).unwrap();
        assert_eq!(pm.list().0.iter().filter(|p| p.enabled).count(), 2);
        pm.remove("essentials-lite").unwrap();
        assert_eq!(pm.list().0.len(), 1);
    }

    #[test]
    fn broken_archives_are_listed_apart() {
        let t = tempfile::tempdir().unwrap();
        let server = t.path().join("server");
        std::fs::create_dir_all(server.join("plugins")).unwrap();
        write_test_archive(
            &server.join("plugins/no-entry.rplugin"),
            &manifest("no-entry", "1.0.0", ""),
            None,
        );
        std::fs::write(server.join("plugins/garbage.rplugin"), b"not a zip").unwrap();
        let (ok, bad) = PluginManager::new(&server).list();
        assert!(ok.is_empty());
        assert_eq!(bad.len(), 2);
        assert!(bad.iter().any(|b| b.error.contains("not in the archive")));
    }

    /// Conformance with Rustic Server's docs/PLUGIN_FORMAT.md (main): its example manifest, verbatim.
    #[test]
    fn the_spec_example_manifest_is_valid() {
        let spec = r#"
[plugin]
id = "homes"                    # required, [a-z0-9_-]{1,64}, unique on the server
name = "Homes"                  # required, 1-64 characters, shown to admins
version = "1.2.0"               # required, semantic version (pre-release/build allowed)
api = "0.1"                     # required, plugin API "MAJOR.MINOR" the plugin targets
authors = ["Alice", "Bob"]      # required, at least one non-empty name
license = "MIT"                 # required, an SPDX expression
entry = "homes.wasm"            # required, the component, relative path inside the package
description = "Set and teleport to homes"   # optional, at most 1024 characters

[dependencies]                  # optional table
required = { economy = "^1.0" } # must be installed, enabled first, version must match
optional = { placeholders = "*" }   # enabled first when installed (and matching)
load_before = ["chat"]          # these plugins are loaded after this one when installed

[capabilities]                  # optional table; everything is denied by default
network = false                 # or a list of host names: ["api.example.com", "*.example.org"]
filesystem = "data-folder"      # "none" (default) or "data-folder"

[limits]                        # optional overrides, capped by the server's rustic.toml
fuel_per_call = 50000000        # fuel (WebAssembly instructions, roughly) per call
memory_mb = 64                  # linear memory limit
"#;
        let m = PluginManifest::parse(spec).unwrap();
        assert_eq!(m.id, "homes");
        assert_eq!(m.required["economy"], "^1.0");
        assert_eq!(m.load_before, ["chat"]);
        assert_eq!(m.filesystem, "data-folder");
        assert!(m.network.is_none());
        // The table's rules.
        let bad = |text: String| PluginManifest::parse(&text).is_err();
        assert!(bad(spec.replace("memory_mb = 64", "memory_mb = 5000")));
        assert!(bad(
            spec.replace("fuel_per_call = 50000000", "fuel_per_call = 0")
        ));
        assert!(bad(spec.replace(
            "load_before = [\"chat\"]",
            "load_before = [\"economy\"]"
        )));
        assert!(bad(
            spec.replace("optional = { placeholders", "optional = { economy")
        ));
        assert!(bad(
            spec.replace("\"data-folder\"      #", "\"everything\"      #")
        ));
        assert!(bad(spec.replace("api = \"0.1\"", "api = \"0.01\"")));
        assert!(bad(spec.replace("homes.wasm", "homes.WASM")));
    }

    #[test]
    fn star_matches_pre_releases_like_the_server() {
        let pre = semver::Version::parse("1.1.0-beta").unwrap();
        assert!(requirement_matches("*", &pre));
        assert!(!requirement_matches("^1.0", &pre));
        assert!(requirement_matches(
            ">=1.2, <2",
            &semver::Version::parse("1.4.0").unwrap()
        ));
    }

    #[test]
    fn archives_and_dev_directories() {
        let t = tempfile::tempdir().unwrap();
        let server = t.path().join("server");
        let plugins = server.join("plugins");
        std::fs::create_dir_all(plugins.join("homes-dev")).unwrap();
        std::fs::write(
            plugins.join("homes-dev").join(MANIFEST_FILE),
            manifest("homes", "0.1.0", ""),
        )
        .unwrap();
        std::fs::write(plugins.join("homes-dev/plugin.wasm"), b"\0asm").unwrap();
        // A data folder (no manifest) is not a plugin.
        std::fs::create_dir_all(plugins.join("homes/.rustic")).unwrap();
        // An archive whose config.toml is not TOML is refused.
        {
            use std::io::Write;
            let f = std::fs::File::create(plugins.join("badcfg.rplugin")).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            let o = zip::write::SimpleFileOptions::default();
            zip.start_file(MANIFEST_FILE, o).unwrap();
            zip.write_all(manifest("badcfg", "1.0.0", "").as_bytes())
                .unwrap();
            zip.start_file("plugin.wasm", o).unwrap();
            zip.write_all(b"\0asm").unwrap();
            zip.start_file("config.toml", o).unwrap();
            zip.write_all(b"not = = toml").unwrap();
            zip.finish().unwrap();
        }
        let pm = PluginManager::new(&server);
        let (ok, bad) = pm.list();
        assert_eq!(ok.len(), 1);
        assert!(ok[0].dev && ok[0].manifest.id == "homes");
        assert_eq!(bad.len(), 1);
        assert!(bad[0].error.contains("config.toml"));
        // Disabling a dev directory moves it too.
        let off = pm.set_enabled("homes", false).unwrap();
        assert!(off.file.is_dir() && !off.enabled);
    }

    #[test]
    fn required_cycles_are_reported() {
        let t = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(&t.path().join("server"));
        pm.install(
            &archive(
                t.path(),
                "a",
                "1.0.0",
                "[dependencies]\nrequired = { b = \"*\" }",
            ),
            false,
        )
        .unwrap();
        pm.install(
            &archive(
                t.path(),
                "b",
                "1.0.0",
                "[dependencies]\nrequired = { a = \"*\" }",
            ),
            false,
        )
        .unwrap();
        assert_eq!(
            pm.check_dependencies(),
            [DependencyProblem::Cycle {
                plugins: vec!["a".into(), "b".into()]
            }]
        );
    }

    #[test]
    fn a_missing_dependency_is_reported() {
        let t = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(&t.path().join("server"));
        pm.install(
            &archive(
                t.path(),
                "menus",
                "1.0.0",
                "[dependencies]\nrequired = { perms = \"^1\" }",
            ),
            false,
        )
        .unwrap();
        assert!(matches!(
            pm.check_dependencies().as_slice(),
            [DependencyProblem::Missing { dependency, .. }] if dependency == "perms"
        ));
    }
}
