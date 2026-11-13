//! horizon-mixins (<https://github.com/horizon-rust/horizon-mixins>, MIT, by levzzz5154, made for
//! Horizon): the tool that applies native mods to Horizon's source. Built once with the build's
//! Rust (`cargo install --locked --git … --rev …`) at the commit Horizon's launcher pins for the
//! Horizon the edition is built on, into `native/tools/<rev>/`, then run as Horizon's launcher runs
//! it: `horizon-mixins apply --workspace <Horizon> --mods <bundles> --out <staged copy>`.
use crate::error::{IoContext, NativeError, Result};
use crate::paths::NativeRoot;
use crate::process;
use crate::toolchain::Toolchain;
use rusticmc_launcher_core::CancellationToken;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const GIT: &str = "https://github.com/horizon-rust/horizon-mixins";
/// Horizon's launcher `native_mods::MIXINS_REV` at a84f637d.
pub const REV: &str = "5a64af938ec86b00d53d16a408aabc0038fb3e57";
/// The package of its command-line program.
pub const PACKAGE: &str = "horizon-mixins-cli";

/// The tool's program once built.
pub fn program(root: &NativeRoot) -> PathBuf {
    root.tools(REV)
        .join("bin")
        .join(crate::exe("horizon-mixins"))
}

/// Builds horizon-mixins unless it is there; `log` hears the build's output.
pub async fn ensure(
    rust: &Toolchain,
    root: &NativeRoot,
    cancel: &CancellationToken,
    log: &mut (dyn FnMut(&str) + Send),
) -> Result<PathBuf> {
    let bin = program(root);
    if bin.is_file() {
        return Ok(bin);
    }
    let dir = root.tools(REV);
    std::fs::create_dir_all(&dir).at(&dir)?;
    let mut cmd = rust.command(&rust.cargo(), root);
    cmd.args(["install", "--locked", "--git", GIT, "--rev", REV, "--root"])
        .arg(&dir)
        .arg("--target-dir")
        .arg(root.work().join("tool-target"))
        .arg(PACKAGE);
    let lines = std::sync::Mutex::new(Vec::new());
    let finished = process::run(
        cmd,
        cancel,
        |l| lines.lock().expect("lines").push(l.to_owned()),
        |l| lines.lock().expect("lines").push(l.to_owned()),
    )
    .await?;
    for l in lines.into_inner().expect("lines") {
        log(&l);
    }
    let _ = std::fs::remove_dir_all(root.work().join("tool-target"));
    if !finished.success || !bin.is_file() {
        return Err(NativeError::Mixins {
            mod_id: None,
            message: format!(
                "building horizon-mixins failed: {}",
                finished.stderr_tail.join("\n")
            ),
        });
    }
    Ok(bin)
}

/// What `apply` reports on success.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Applied {
    #[serde(default)]
    pub injections: u32,
    #[serde(default)]
    pub modules: u32,
    #[serde(default)]
    pub crates: u32,
    #[serde(default)]
    pub post_build: u32,
    #[serde(default)]
    pub data: u32,
}

/// A refusal of horizon-mixins: its JSON diagnostic, with the mod it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub mod_id: Option<String>,
    pub code: String,
    pub message: String,
}

impl Refusal {
    pub fn line(&self) -> String {
        format!("{}: {}", self.code, self.message)
    }
}

/// Reads a diagnostic (`{"code", "message", "file", "mod_id"?, "injection_id"?}`) from the tool's
/// output, naming the mod from `mod_id` or from a file under `mods` (`<mods>/<id>/…`).
pub fn refusal(text: &str, mods: &Path) -> Option<Refusal> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    let value: serde_json::Value = serde_json::from_str(text.get(start..=end)?).ok()?;
    let code = value["code"].as_str()?.to_owned();
    let message = value["message"].as_str().unwrap_or_default().to_owned();
    let mod_id = value["mod_id"].as_str().map(str::to_owned).or_else(|| {
        let file = value["file"].as_str()?;
        mod_of_file(file, mods)
    });
    Some(Refusal {
        mod_id,
        code,
        message,
    })
}

/// The mod a file of the bundles' folder belongs to.
fn mod_of_file(file: &str, mods: &Path) -> Option<String> {
    let file = file.trim_start_matches(r"\\?\").replace('\\', "/");
    let mods = mods
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");
    let i = file.to_ascii_lowercase().find(&mods.to_ascii_lowercase())?;
    let rest = &file[i + mods.len()..];
    rest.trim_start_matches('/')
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Runs `apply` (see the module). `Ok(Err(refusal))` when the tool refused the mods.
#[allow(clippy::too_many_arguments)]
pub async fn apply(
    tool: &Path,
    rust: &Toolchain,
    root: &NativeRoot,
    workspace: &Path,
    mods: &Path,
    out: &Path,
    cancel: &CancellationToken,
    log: &mut (dyn FnMut(&str) + Send),
) -> Result<std::result::Result<Applied, Refusal>> {
    let _ = std::fs::remove_dir_all(out);
    // It runs `cargo metadata`: the build's Rust, as a build.
    let mut cmd = rust.command(tool, root);
    cmd.arg("apply")
        .arg("--workspace")
        .arg(workspace)
        .arg("--mods")
        .arg(mods)
        .arg("--out")
        .arg(out);
    let mut stdout = String::new();
    let mut stderr = String::new();
    let finished = process::run(
        cmd,
        cancel,
        |l| {
            stdout.push_str(l);
            stdout.push('\n');
        },
        |l| {
            stderr.push_str(l);
            stderr.push('\n');
        },
    )
    .await?;
    log(&stdout);
    log(&stderr);
    if finished.success {
        let applied = stdout
            .find('{')
            .and_then(|i| serde_json::from_str::<Applied>(&stdout[i..]).ok())
            .unwrap_or_default();
        return Ok(Ok(applied));
    }
    let both = format!("{stdout}\n{stderr}");
    Ok(Err(refusal(&stderr, mods)
        .or_else(|| refusal(&stdout, mods))
        .unwrap_or_else(|| Refusal {
            mod_id: None,
            code: "E_TOOL".into(),
            message: both.trim().lines().last().unwrap_or("failed").to_owned(),
        })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_names_its_mod() {
        let mods = Path::new(r"C:\data\native\work\mods");
        let text = r#"{
  "code": "E_TARGET_NOT_FOUND",
  "message": "no function horizon_client::x",
  "file": "\\\\?\\C:\\data\\native\\work\\mods\\zoom\\horizon-mixin.toml"
}"#;
        let r = refusal(text, mods).unwrap();
        assert_eq!(r.mod_id.as_deref(), Some("zoom"));
        assert_eq!(r.code, "E_TARGET_NOT_FOUND");
        assert_eq!(
            r.line(),
            "E_TARGET_NOT_FOUND: no function horizon_client::x"
        );
        let with_id = r#"error {"code":"E_MODULE_CONFLICT","message":"m","mod_id":"fps_overlay"}"#;
        assert_eq!(
            refusal(with_id, mods).unwrap().mod_id.as_deref(),
            Some("fps_overlay")
        );
        let unix =
            r#"{"code":"E_MODULE","message":"m","file":"/home/p/native/work/mods/veinminer/a.rs"}"#;
        assert_eq!(
            refusal(unix, Path::new("/home/p/native/work/mods"))
                .unwrap()
                .mod_id
                .as_deref(),
            Some("veinminer")
        );
        assert!(refusal("no json here", mods).is_none());
    }

    #[test]
    fn the_success_report_is_read() {
        let a: Applied = serde_json::from_str(
            r#"{"bridges":0,"crates":0,"data":0,"injections":4,"modules":1,"post_build":0,"status":"ok"}"#,
        )
        .unwrap();
        assert_eq!(a.injections, 4);
        assert_eq!(a.modules, 1);
    }
}
