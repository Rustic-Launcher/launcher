//! One native build, end to end (see the crate's documentation for the steps). One build runs at
//! a time in the launcher; a second one gets [`NativeError::Busy`].
//!
//! A mod that doesn't fit the edition's Horizon doesn't stop the others (Horizon's launcher does
//! the same): when horizon-mixins refuses it (a function it hooks or a hook point this Horizon
//! doesn't have, a clash with another mod's module) or the compiler's errors are inside what it
//! brought, the build runs again without it, at most [`RETRIES`] more times. The record names the
//! mods left out and why; the build stays up to date for the same mods turned on.
use crate::bundle::Bundle;
use crate::compile::{self, Compile, Outcome};
use crate::error::{IoContext, NativeError, Result};
use crate::instance::{self, BuildRecord, Consent, LeftOut, ModRef, NativeModFile};
use crate::paths::NativeRoot;
use crate::progress::{Phase, Progress};
use crate::source::{self, SourceSpec};
use crate::toolchain::Toolchain;
use crate::{cache, disk, mixins, stage};
use rusticmc_launcher_core::{CancellationToken, DownloadManager};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// How many times a build runs again after leaving mods out.
pub const RETRIES: usize = 3;

/// The launcher's native builder.
#[derive(Clone)]
pub struct Builder {
    pub root: NativeRoot,
    pub downloads: DownloadManager,
    /// The compile cache limit in GiB (0: never cleared automatically).
    pub cache_limit_gib: u64,
}

/// What to build.
#[derive(Clone, Debug)]
pub struct BuildRequest {
    /// `<data>/instances/<id>`.
    pub instance_dir: PathBuf,
    /// Its game folder (the native mods are in `horizon-native-mods/`).
    pub game_dir: PathBuf,
    /// The edition build the instance plays (its version name and folder).
    pub base_build: String,
    pub base_build_dir: PathBuf,
    pub source: SourceSpec,
    pub toolchain: Toolchain,
}

fn busy() -> &'static tokio::sync::Mutex<()> {
    static BUSY: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    BUSY.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Whether a build is running now.
pub fn running() -> bool {
    busy().try_lock().is_err()
}

/// The build log: every line of every step, replaced by each build.
struct Log(Mutex<Option<std::io::BufWriter<std::fs::File>>>);

impl Log {
    fn open(path: &Path) -> Self {
        let file = path
            .parent()
            .map(std::fs::create_dir_all)
            .and_then(|r| r.ok())
            .and_then(|()| std::fs::File::create(path).ok())
            .map(std::io::BufWriter::new);
        Self(Mutex::new(file))
    }

    fn line(&self, text: &str) {
        if let Some(w) = self.0.lock().expect("log").as_mut() {
            let _ = writeln!(w, "{}", text.trim_end());
        }
    }

    fn flush(&self) {
        if let Some(w) = self.0.lock().expect("log").as_mut() {
            let _ = w.flush();
        }
    }
}

impl Builder {
    pub fn new(root: NativeRoot, downloads: DownloadManager) -> Self {
        Self {
            root,
            downloads,
            cache_limit_gib: cache::DEFAULT_LIMIT_GIB,
        }
    }

    /// Builds the instance's client (see the module). `on` hears progress.
    pub async fn build(
        &self,
        req: &BuildRequest,
        on: &(dyn Fn(Progress) + Send + Sync),
        cancel: &CancellationToken,
    ) -> Result<BuildRecord> {
        let _guard = busy().try_lock().map_err(|_| NativeError::Busy)?;
        let started = Instant::now();
        if Consent::load(&req.instance_dir).is_none() {
            return Err(NativeError::ConsentRequired);
        }
        let mods = instance::enabled(&req.game_dir)?;
        if mods.is_empty() {
            return Err(NativeError::NoMods);
        }
        let log = Log::open(&self.root.log());
        log.line(&format!(
            "Rustic native build of {} on {} ({} mods: {})",
            req.instance_dir.display(),
            req.base_build,
            mods.len(),
            mods.iter()
                .map(|(m, _)| format!("{} {}", m.id, m.version))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        let result = self.run(req, &mods, &log, on, cancel, started).await;
        match &result {
            Ok(r) => log.line(&format!(
                "Built in {:.0} s ({} crates compiled)",
                r.seconds, r.compiled
            )),
            Err(e) => log.line(&format!("Failed: {e}")),
        }
        log.flush();
        result
    }

    async fn run(
        &self,
        req: &BuildRequest,
        mods: &[(NativeModFile, Bundle)],
        log: &Log,
        on: &(dyn Fn(Progress) + Send + Sync),
        cancel: &CancellationToken,
        started: Instant,
    ) -> Result<BuildRecord> {
        let root = &self.root;
        let rust = &req.toolchain;
        on(Progress::new(
            Phase::Toolchain,
            1.0,
            format!("Rust {} ({:?})", rust.version, rust.kind),
        ));
        log.line(&format!(
            "Rust {} {} at {}",
            rust.version,
            rust.host,
            rust.bin.display()
        ));
        let first = cache::size(&root.target()) == 0;
        let needed = if first {
            disk::FIRST_BUILD_GIB
        } else {
            disk::REBUILD_GIB
        };
        std::fs::create_dir_all(root.root()).at(root.root())?;
        disk::check(root.root(), needed)?;

        // The source.
        on(Progress::new(
            Phase::Source,
            0.0,
            "Preparing the Rustic edition's source",
        ));
        let prepared = source::prepare(root, &req.source, &self.downloads, cancel).await?;
        let pristine = prepared.horizon();
        stage::write_if_changed(
            &pristine.join(".horizon-revision"),
            prepared.horizon_commit.as_bytes(),
        )?;
        log.line(&format!(
            "Source {} (Horizon {})",
            &prepared.id[..prepared.id.len().min(16)],
            prepared.horizon_commit
        ));
        on(Progress::new(Phase::Source, 1.0, "Source ready"));

        // horizon-mixins.
        on(Progress::new(Phase::Tool, 0.0, "Preparing horizon-mixins"));
        let tool = mixins::ensure(rust, root, cancel, &mut |l| log.line(l)).await?;
        on(Progress::new(Phase::Tool, 1.0, "horizon-mixins ready"));

        let mut left_out: Vec<LeftOut> = Vec::new();
        // Mods the edition can't build yet are left out from the start.
        for (m, b) in mods {
            if let Some(what) = b.needs_unsupported() {
                left_out.push(LeftOut {
                    id: m.id.clone(),
                    version: m.version.clone(),
                    why: "unsupported".into(),
                    detail: format!("it brings {what}, which Rustic can't build yet"),
                });
            }
        }
        let work = root.work();
        let mods_dir = work.join("mods");
        let applied = work.join("applied");
        let stage_root = root.stage();
        let mut attempt = 0;
        let (program, compiled, built) = loop {
            let candidates: Vec<&(NativeModFile, Bundle)> = mods
                .iter()
                .filter(|(m, _)| !left_out.iter().any(|l| l.id == m.id))
                .collect();
            if candidates.is_empty() {
                return Err(NativeError::NothingToBuild(
                    left_out
                        .iter()
                        .map(|l| format!("{}: {}", l.id, l.detail))
                        .collect::<Vec<_>>()
                        .join("; "),
                ));
            }
            // The bundles, re-pinned to the edition's Horizon.
            on(Progress::new(
                Phase::Mixins,
                0.0,
                format!(
                    "Applying {}",
                    candidates
                        .iter()
                        .map(|(m, _)| m.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
            let _ = std::fs::remove_dir_all(&mods_dir);
            std::fs::create_dir_all(&mods_dir).at(&mods_dir)?;
            for (_, b) in &candidates {
                b.unpack_for(&mods_dir, &prepared.horizon_commit)?;
            }
            let refused = mixins::apply(
                &tool,
                rust,
                root,
                &pristine,
                &mods_dir,
                &applied,
                cancel,
                &mut |l| log.line(l),
            )
            .await?;
            match refused {
                Ok(report) => log.line(&format!(
                    "horizon-mixins: {} injections, {} modules",
                    report.injections, report.modules
                )),
                Err(refusal) => {
                    let culprit = refusal
                        .mod_id
                        .as_ref()
                        .and_then(|id| candidates.iter().find(|(m, _)| &m.id == id));
                    match culprit {
                        Some((m, _)) if attempt < RETRIES => {
                            log.line(&format!("{} left out: {}", m.id, refusal.line()));
                            left_out.push(LeftOut {
                                id: m.id.clone(),
                                version: m.version.clone(),
                                why: "other_version".into(),
                                detail: refusal.line(),
                            });
                            attempt += 1;
                            continue;
                        }
                        _ => {
                            return Err(NativeError::Mixins {
                                message: refusal.line(),
                                mod_id: refusal.mod_id,
                            });
                        }
                    }
                }
            }
            on(Progress::new(
                Phase::Mixins,
                0.7,
                "Staging the changed files",
            ));
            let horizon_stage = stage_root.join(source::HORIZON);
            let synced = stage::sync(&applied, &horizon_stage, &[])?;
            for part in source::LAUNCHER_PARTS
                .iter()
                .chain([&source::LAUNCHER_ROOT])
            {
                let from = prepared.dir.join(source::LAUNCHER).join(part);
                let to = stage_root.join(source::LAUNCHER).join(part);
                if from.is_file() {
                    let bytes = std::fs::read(&from).at(&from)?;
                    stage::write_if_changed(&to, &bytes)?;
                } else {
                    stage::sync(&from, &to, &[])?;
                }
            }
            let _ = std::fs::remove_dir_all(&applied);
            log.line(&format!(
                "Stage: {} files written, {} removed, {} unchanged",
                synced.written, synced.removed, synced.unchanged
            ));
            on(Progress::new(Phase::Mixins, 1.0, "Mods applied"));

            // The compile.
            on(Progress::new(
                Phase::Compile,
                0.0,
                if first {
                    "First build: the whole game (10 to 15 minutes)".to_owned()
                } else {
                    "Compiling what changed".to_owned()
                },
            ));
            let compile = Compile {
                rust,
                root,
                manifest: compile::manifest(&stage_root),
                horizon: horizon_stage.clone(),
                pristine: pristine.clone(),
            };
            let units = |done: u32, total: u32, names: &str| {
                let inner = if total == 0 {
                    0.0
                } else {
                    f64::from(done) / f64::from(total)
                };
                let mut p = Progress::new(
                    Phase::Compile,
                    inner,
                    if names.is_empty() {
                        "Compiling".to_owned()
                    } else {
                        format!("Compiling {names}")
                    },
                );
                p.units = Some((done, total));
                on(p);
            };
            match compile.run(cancel, &units, &mut |l| log.line(l)).await? {
                Outcome::Built { program, compiled } => {
                    let built: Vec<ModRef> =
                        candidates.iter().map(|(m, _)| ModRef::from(m)).collect();
                    break (program, compiled, built);
                }
                Outcome::Failed {
                    errors,
                    owners,
                    tail,
                } => {
                    let blamed: Vec<&&(NativeModFile, Bundle)> = candidates
                        .iter()
                        .filter(|(m, _)| owners.contains(&m.id))
                        .collect();
                    let excerpt = if errors.is_empty() {
                        tail.join("\n")
                    } else {
                        errors.join("\n")
                    };
                    if !blamed.is_empty() && attempt < RETRIES {
                        for (m, _) in blamed {
                            log.line(&format!("{} left out: it doesn't compile here", m.id));
                            left_out.push(LeftOut {
                                id: m.id.clone(),
                                version: m.version.clone(),
                                why: "other_version".into(),
                                detail: first_line(&excerpt),
                            });
                        }
                        attempt += 1;
                        continue;
                    }
                    return Err(NativeError::Compile {
                        mods: owners.into_iter().collect(),
                        excerpt: shorten(&excerpt, 4000),
                    });
                }
            }
        };

        // The client into the instance.
        on(Progress::new(
            Phase::Install,
            0.0,
            "Installing the built client",
        ));
        let (client, client_bytes) = install_client(
            &req.instance_dir,
            &program,
            &req.base_build_dir,
            &stage_root.join(source::HORIZON),
            &built,
            &req.base_build,
            &prepared.horizon_commit,
        )?;
        let requested: Vec<ModRef> = mods.iter().map(|(m, _)| ModRef::from(m)).collect();
        let record = BuildRecord {
            format: 1,
            key: instance::key(&req.base_build, &requested),
            base_build: req.base_build.clone(),
            source_id: prepared.id.clone(),
            horizon_commit: prepared.horizon_commit.clone(),
            mixins_rev: mixins::REV.into(),
            rust_version: rust.version.clone(),
            profile: crate::PROFILE.into(),
            requested,
            built,
            left_out,
            built_at: instance::now(),
            seconds: started.elapsed().as_secs_f64(),
            compiled,
            client,
            client_bytes,
        };
        record.save(&req.instance_dir)?;
        on(Progress::new(Phase::Install, 1.0, "Installed"));

        // The cache limit.
        on(Progress::new(
            Phase::Cleanup,
            0.0,
            "Checking the compile cache",
        ));
        match cache::enforce(root, self.cache_limit_gib) {
            Ok(0) => {}
            Ok(freed) => log.line(&format!(
                "Compile cache over {} GiB: cleared {} MiB",
                self.cache_limit_gib,
                freed >> 20
            )),
            Err(e) => log.line(&format!("Could not clear the compile cache: {e}")),
        }
        on(Progress::new(Phase::Done, 1.0, "Done"));
        Ok(record)
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_owned()
}

fn shorten(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// What of the edition bundle a client reads beside itself: Horizon's `mods/` folder (the release's
/// own content) and its licence.
const BESIDE: &[&str] = &["mods", "LICENSE"];

/// Puts the built `program` into `<instance>/native/bin/` with what it reads beside itself, then
/// swaps it in place of the previous build. Returns the client's path (relative to
/// `<instance>/native/`) and size.
pub fn install_client(
    instance_dir: &Path,
    program: &Path,
    base_build_dir: &Path,
    stage_horizon: &Path,
    built: &[ModRef],
    base_build: &str,
    horizon_commit: &str,
) -> Result<(String, u64)> {
    let native = instance::dir(instance_dir);
    let next = native.join("bin.new");
    let _ = std::fs::remove_dir_all(&next);
    std::fs::create_dir_all(&next).at(&next)?;
    let exe = crate::exe(crate::EDITION_BIN);
    let to = next.join(&exe);
    std::fs::copy(program, &to).at(&to)?;
    let size = std::fs::metadata(&to).at(&to)?.len();
    for name in BESIDE {
        let from = base_build_dir.join(name);
        if from.is_dir() {
            copy_dir(&from, &next.join(name))?;
        } else if from.is_file() {
            std::fs::copy(&from, next.join(name)).at(&from)?;
        }
    }
    let data = stage_horizon.join("native-data");
    if data.is_dir() {
        copy_dir(&data, &next.join("native-data"))?;
    }
    let info = serde_json::json!({
        "mods": built.iter().map(|m| serde_json::json!({"id": m.id, "version": m.version})).collect::<Vec<_>>(),
        "base": base_build,
        "horizon": horizon_commit,
        "builder": "rustic-launcher",
    });
    std::fs::write(
        next.join(instance::BUILD_INFO),
        serde_json::to_vec_pretty(&info).expect("json"),
    )
    .at(&next)?;
    let bin = native.join("bin");
    let old = native.join("bin.old");
    let _ = std::fs::remove_dir_all(&old);
    if bin.exists() {
        std::fs::rename(&bin, &old).at(&bin)?;
    }
    if let Err(e) = std::fs::rename(&next, &bin) {
        // Put the previous build back.
        let _ = std::fs::rename(&old, &bin);
        return Err(NativeError::io(&bin, e));
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok((format!("bin/{exe}"), size))
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for (rel, path) in stage::walk(from, &[])? {
        let dest = to.join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).at(parent)?;
        }
        std::fs::copy(&path, &dest).at(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_is_installed_beside_what_it_reads_and_replaces_the_last() {
        let t = tempfile::tempdir().unwrap();
        let inst = t.path().join("instances/a");
        let program = t.path().join("prog");
        std::fs::write(&program, "client-1").unwrap();
        let base = t.path().join("builds/rustic-edition-x");
        std::fs::create_dir_all(base.join("mods/horizon")).unwrap();
        std::fs::write(base.join("mods/horizon/horizon-mod.toml"), "m").unwrap();
        std::fs::write(base.join("LICENSE"), "MIT").unwrap();
        std::fs::write(base.join("horizon-server.exe"), "big").unwrap();
        let stage = t.path().join("stage");
        std::fs::create_dir_all(stage.join("native-data/zoom")).unwrap();
        std::fs::write(stage.join("native-data/zoom/a.bin"), "d").unwrap();
        let built = vec![ModRef {
            id: "zoom".into(),
            version: "2.0.0".into(),
            sha256: "x".into(),
        }];
        let (rel, size) = install_client(
            &inst,
            &program,
            &base,
            &stage,
            &built,
            "rustic-edition-x",
            "abc",
        )
        .unwrap();
        let bin = instance::dir(&inst).join("bin");
        assert_eq!(rel, format!("bin/{}", crate::exe("rustic-client")));
        assert_eq!(size, 8);
        assert!(bin.join("mods/horizon/horizon-mod.toml").is_file());
        assert!(bin.join("LICENSE").is_file());
        assert!(!bin.join("horizon-server.exe").exists());
        assert!(bin.join("native-data/zoom/a.bin").is_file());
        let info: serde_json::Value =
            serde_json::from_slice(&std::fs::read(bin.join(instance::BUILD_INFO)).unwrap())
                .unwrap();
        assert_eq!(info["mods"][0]["id"], "zoom");
        std::fs::write(&program, "client-2").unwrap();
        install_client(
            &inst,
            &program,
            &base,
            &stage,
            &built,
            "rustic-edition-x",
            "abc",
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(bin.join(crate::exe("rustic-client"))).unwrap(),
            "client-2"
        );
        assert!(!instance::dir(&inst).join("bin.old").exists());
        assert!(!instance::dir(&inst).join("bin.new").exists());
    }

    #[test]
    fn excerpts_are_bounded() {
        assert_eq!(shorten("abc", 10), "abc");
        assert_eq!(shorten("ééé", 3), "é…");
        assert_eq!(first_line("\n  error: x\nmore"), "error: x");
    }

    #[tokio::test]
    async fn nothing_is_built_without_consent_or_mods() {
        let t = tempfile::tempdir().unwrap();
        let root = NativeRoot::at(t.path().join("native"));
        let dl = DownloadManager::new(t.path().join("cache"), 1, 0).unwrap();
        let b = Builder::new(root, dl);
        let inst = t.path().join("instances/a");
        let game = inst.join("game");
        std::fs::create_dir_all(&game).unwrap();
        let req = BuildRequest {
            instance_dir: inst.clone(),
            game_dir: game.clone(),
            base_build: "rustic-edition-x".into(),
            base_build_dir: t.path().join("builds/x"),
            source: SourceSpec::Local(source::LocalSource {
                horizon: t.path().join("h"),
                launcher: t.path().join("l"),
                horizon_commit: None,
            }),
            toolchain: Toolchain {
                kind: crate::toolchain::Kind::System,
                version: crate::toolchain::RUST_VERSION.into(),
                host: "x".into(),
                bin: t.path().join("nobin"),
                path_prepend: vec![],
            },
        };
        let e = b
            .build(&req, &|_| {}, &CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(e.code(), "consent_required");
        Consent::accept(&inst).unwrap();
        let e = b
            .build(&req, &|_| {}, &CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(e.code(), "no_mods");
        assert!(!running());
    }
}
