//! The compile: `cargo build --locked --message-format=json --profile fast -p rustic-edition
//! --bin rustic-client` on the staged companion workspace, into `native/target/`. `--locked`: only
//! the crates the edition's lockfile names (Horizon's exact versions) are downloaded, from
//! crates.io; nothing else is fetched. Progress comes from Cargo's own bar (units done of the
//! total, on standard error, kept on with `CARGO_TERM_PROGRESS_WHEN=always`), errors from its JSON
//! messages, which also say which mod an error is in ([`crate::diagnostics`]).
use crate::diagnostics::{self, Plan};
use crate::error::Result;
use crate::paths::NativeRoot;
use crate::process;
use crate::toolchain::Toolchain;
use rusticmc_launcher_core::CancellationToken;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What the compile needs.
pub struct Compile<'a> {
    pub rust: &'a Toolchain,
    pub root: &'a NativeRoot,
    /// The staged companion workspace (its `Cargo.toml`).
    pub manifest: PathBuf,
    /// The staged and the pristine Horizon, for tracing errors to mods.
    pub horizon: PathBuf,
    pub pristine: PathBuf,
}

/// How it ended.
#[derive(Debug)]
pub enum Outcome {
    Built {
        program: PathBuf,
        /// Units compiled (not fresh).
        compiled: u32,
    },
    Failed {
        /// The first errors, as the compiler rendered them.
        errors: Vec<String>,
        /// The mods they were traced to.
        owners: BTreeSet<String>,
        /// The end of the output when there was no compiler error (a build script, the linker).
        tail: Vec<String>,
    },
}

/// Cargo's progress bar: `Building [=====>    ] 123/456: a, b`.
pub fn progress_of(line: &str) -> Option<(u32, u32, String)> {
    let at = line.find("Building [")?;
    let rest = &line[at..];
    let close = rest.find(']')?;
    let after = rest[close + 1..].trim_start();
    let (counts, names) = after.split_once(':').unwrap_or((after, ""));
    let (done, total) = counts.trim().split_once('/')?;
    Some((
        done.trim().parse().ok()?,
        total.trim().parse().ok()?,
        names.trim().to_owned(),
    ))
}

impl Compile<'_> {
    /// The arguments (after `cargo`).
    pub fn args(&self) -> Vec<String> {
        let mut args: Vec<String> = [
            "build",
            "--locked",
            "--message-format=json",
            "--profile",
            crate::PROFILE,
            "-p",
            crate::EDITION_PACKAGE,
            "--bin",
            crate::EDITION_BIN,
            "--manifest-path",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        args.push(self.manifest.to_string_lossy().into_owned());
        args
    }

    /// Runs the compile: `on_units(done, total, crates)` while it goes, `log` gets every line.
    pub async fn run(
        &self,
        cancel: &CancellationToken,
        on_units: &(dyn Fn(u32, u32, &str) + Send + Sync),
        log: &mut (dyn FnMut(&str) + Send),
    ) -> Result<Outcome> {
        let mut cmd = self.rust.command(&self.rust.cargo(), self.root);
        cmd.args(self.args())
            .env("CARGO_TARGET_DIR", self.root.target())
            .env("CARGO_TERM_PROGRESS_WHEN", "always")
            .env("CARGO_TERM_PROGRESS_WIDTH", "100");
        let plan = Plan::read(&self.horizon);
        let mut program: Option<PathBuf> = None;
        let mut compiled = 0u32;
        let mut errors: Vec<String> = Vec::new();
        let mut owners = BTreeSet::new();
        let mut lines: Vec<String> = Vec::new();
        let mut stderr_lines: Vec<String> = Vec::new();
        let finished = process::run(
            cmd,
            cancel,
            |line| {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                    lines.push(line.to_owned());
                    return;
                };
                match v["reason"].as_str() {
                    Some("compiler-artifact") => {
                        if v["fresh"] == false {
                            compiled += 1;
                        }
                        if v["target"]["name"] == crate::EDITION_BIN
                            && let Some(exe) = v["executable"].as_str()
                        {
                            program = Some(PathBuf::from(exe));
                        }
                    }
                    Some("compiler-message") => {
                        let m = &v["message"];
                        let rendered = m["rendered"].as_str().unwrap_or_default();
                        lines.push(rendered.trim_end().to_owned());
                        if m["level"] == "error" {
                            if errors.len() < 4 {
                                errors.push(rendered.trim_end().to_owned());
                            }
                            if let Some(plan) = &plan {
                                owners.extend(diagnostics::owners_of(
                                    plan,
                                    &v,
                                    &self.horizon,
                                    &self.pristine,
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            },
            |line| {
                if let Some((done, total, names)) = progress_of(line) {
                    on_units(done, total, &names);
                } else if !line.trim().is_empty() {
                    stderr_lines.push(line.to_owned());
                }
            },
        )
        .await?;
        for l in lines.iter().chain(&stderr_lines) {
            log(l);
        }
        if finished.success
            && let Some(program) = program.filter(|p| p.is_file())
        {
            return Ok(Outcome::Built { program, compiled });
        }
        let tail = stderr_lines.iter().rev().take(25).rev().cloned().collect();
        Ok(Outcome::Failed {
            errors,
            owners,
            tail,
        })
    }
}

/// Where Cargo puts the program for the profile (also when the compile reported nothing new).
pub fn program_path(root: &NativeRoot) -> PathBuf {
    root.target()
        .join(crate::PROFILE)
        .join(crate::exe(crate::EDITION_BIN))
}

/// The staged companion's manifest.
pub fn manifest(stage: &Path) -> PathBuf {
    stage.join(crate::source::COMPANION).join("Cargo.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargos_bar_is_read() {
        assert_eq!(
            progress_of("    Building [=======>         ] 123/456: horizon-client, naga"),
            Some((123, 456, "horizon-client, naga".into()))
        );
        assert_eq!(
            progress_of("Building [=====] 7/7"),
            Some((7, 7, String::new()))
        );
        assert!(progress_of("   Compiling serde v1.0").is_none());
        assert!(progress_of("Building [==").is_none());
    }

    #[test]
    fn the_build_is_locked_and_aimed_at_the_edition() {
        let rust = Toolchain {
            kind: crate::toolchain::Kind::System,
            version: crate::toolchain::RUST_VERSION.into(),
            host: "x".into(),
            bin: PathBuf::from("/r"),
            path_prepend: vec![],
        };
        let root = NativeRoot::at("/n");
        let c = Compile {
            rust: &rust,
            root: &root,
            manifest: manifest(&root.stage()),
            horizon: PathBuf::new(),
            pristine: PathBuf::new(),
        };
        let args = c.args();
        for want in [
            "--locked",
            "--message-format=json",
            "fast",
            "rustic-edition",
            "rustic-client",
        ] {
            assert!(args.iter().any(|a| a == want), "{want} in {args:?}");
        }
        assert!(args.last().unwrap().ends_with("Cargo.toml"));
        assert!(program_path(&root).ends_with(crate::exe("rustic-client")));
    }
}
