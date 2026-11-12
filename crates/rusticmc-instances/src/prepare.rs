//! The guided preparation (horizon/docs/api/launch.md "Preparing", launch-guide.md steps 3-5):
//! `horizon-prepare` turns the player's own Minecraft files into the resources `horizon-client
//! --resources` reads. It downloads nothing; it reads the official launcher's game folder (or
//! named files), the server jar (the first time; downloaded only for a verified owner), and runs a
//! Java of `java_major`.
//!
//! `--progress json`: every stdout line is one JSON object, `{"message"}`, `{"fraction"}` (0..1,
//! never decreasing) or `{"error"}` (the last line, exit code 1). Unknown keys and lines are
//! ignored. `--check` before every start: exit 0 current, 1 with gaps on stdout.

use crate::error::{InstanceError, Result};
use crate::launch_api::{PrepareLaunchApi, probe_preparer};
use rusticmc_auth::VerifiedOwner;
use rusticmc_launcher_core::{CancellationToken, Checksum, DownloadManager, DownloadRequest};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};

/// One line of `--progress json`, or of stderr.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PrepareEvent {
    Message {
        text: String,
    },
    Fraction {
        value: f64,
    },
    Error {
        text: String,
    },
    /// Human text from stderr (for the details pane).
    Log {
        text: String,
    },
}

/// Parses one stdout line. `None` for lines with no key this launcher knows.
pub fn parse_progress_line(line: &str) -> Option<PrepareEvent> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let obj = v.as_object()?;
    if let Some(e) = obj.get("error").and_then(|e| e.as_str()) {
        return Some(PrepareEvent::Error { text: e.into() });
    }
    if let Some(f) = obj.get("fraction").and_then(serde_json::Value::as_f64) {
        return Some(PrepareEvent::Fraction {
            value: f.clamp(0.0, 1.0),
        });
    }
    if let Some(m) = obj.get("message").and_then(|m| m.as_str()) {
        return Some(PrepareEvent::Message { text: m.into() });
    }
    None
}

/// Where the player's game files are.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameFiles {
    /// The official launcher's game folder (`--minecraft-dir`).
    pub minecraft_dir: Option<PathBuf>,
    /// A Prism Launcher folder (`--prism-dir`).
    pub prism_dir: Option<PathBuf>,
    pub version_json: Option<PathBuf>,
    pub client_jar: Option<PathBuf>,
    pub libraries_dir: Option<PathBuf>,
    pub assets_dir: Option<PathBuf>,
}

/// One preparation.
#[derive(Clone, Debug)]
pub struct PrepareRequest {
    pub preparer: PathBuf,
    /// Where the resources go (one folder per build, shared by its instances).
    pub output: PathBuf,
    pub files: GameFiles,
    /// Needed by the first preparation only.
    pub server_jar: Option<PathBuf>,
    pub java_home: Option<PathBuf>,
}

/// What `--check` said.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckResult {
    pub current: bool,
    /// Each gap, with the command that fills it.
    pub gaps: Vec<String>,
}

fn push_opt(
    api: &PrepareLaunchApi,
    args: &mut Vec<OsString>,
    name: &str,
    value: Option<&Path>,
) -> Result<()> {
    let Some(value) = value else { return Ok(()) };
    if !api.accepts(name) {
        return Err(InstanceError::OptionNotSupported(name.into()));
    }
    args.push(name.into());
    args.push(value.as_os_str().to_owned());
    Ok(())
}

/// The preparer's command line for `req`, only with options its `--launch-api` lists.
pub fn prepare_args(api: &PrepareLaunchApi, req: &PrepareRequest) -> Result<Vec<OsString>> {
    for needed in ["--prepare", "--output", "--progress"] {
        if !api.accepts(needed) {
            return Err(InstanceError::OptionNotSupported(needed.into()));
        }
    }
    let mut args: Vec<OsString> = vec!["--prepare".into()];
    push_opt(api, &mut args, "--server-jar", req.server_jar.as_deref())?;
    push_opt(
        api,
        &mut args,
        "--minecraft-dir",
        req.files.minecraft_dir.as_deref(),
    )?;
    push_opt(
        api,
        &mut args,
        "--prism-dir",
        req.files.prism_dir.as_deref(),
    )?;
    push_opt(
        api,
        &mut args,
        "--version-json",
        req.files.version_json.as_deref(),
    )?;
    push_opt(
        api,
        &mut args,
        "--client-jar",
        req.files.client_jar.as_deref(),
    )?;
    push_opt(
        api,
        &mut args,
        "--libraries-dir",
        req.files.libraries_dir.as_deref(),
    )?;
    push_opt(
        api,
        &mut args,
        "--assets-dir",
        req.files.assets_dir.as_deref(),
    )?;
    push_opt(api, &mut args, "--java-home", req.java_home.as_deref())?;
    push_opt(api, &mut args, "--output", Some(&req.output))?;
    if api.accepts("--complete") {
        args.push("--complete".into());
    }
    args.push("--progress".into());
    args.push("json".into());
    Ok(args)
}

/// Runs a preparation, reporting each event. Cancelling kills the preparer (its own folder lock
/// and staging keep the output consistent: a stopped preparation is "not current").
pub async fn prepare(
    req: &PrepareRequest,
    on_event: impl Fn(PrepareEvent) + Send + Sync,
    cancel: &CancellationToken,
) -> Result<()> {
    let api = probe_preparer(&req.preparer).await?;
    let args = prepare_args(&api, req)?;
    tokio::fs::create_dir_all(&req.output)
        .await
        .map_err(|e| InstanceError::io(&req.output, e))?;
    let mut cmd = tokio::process::Command::new(&req.preparer);
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if let Some(dir) = req.preparer.parent() {
        cmd.current_dir(dir);
    }
    crate::process::no_window(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| InstanceError::io(&req.preparer, e))?;
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let mut out_lines = BufReader::new(stdout).lines();
    let mut err_lines = BufReader::new(stderr).lines();
    let mut last_error: Option<String> = None;
    let mut fraction = 0.0f64;
    let (mut out_done, mut err_done) = (false, false);
    while !(out_done && err_done) {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                return Err(InstanceError::Prepare("cancelled".into()));
            }
            line = out_lines.next_line(), if !out_done => match line {
                Ok(Some(line)) => match parse_progress_line(&line) {
                    Some(PrepareEvent::Fraction { value }) => {
                        // Never decreasing, as documented; guard anyway.
                        if value >= fraction {
                            fraction = value;
                            on_event(PrepareEvent::Fraction { value });
                        }
                    }
                    Some(PrepareEvent::Error { text }) => {
                        last_error = Some(text.clone());
                        on_event(PrepareEvent::Error { text });
                    }
                    Some(e) => on_event(e),
                    None => {}
                },
                _ => out_done = true,
            },
            line = err_lines.next_line(), if !err_done => match line {
                Ok(Some(line)) => on_event(PrepareEvent::Log { text: line }),
                _ => err_done = true,
            },
        }
    }
    let status = child
        .wait()
        .await
        .map_err(|e| InstanceError::io(&req.preparer, e))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(2) => Err(InstanceError::Prepare(
            "the preparer refused its command line (a launcher bug)".into(),
        )),
        _ => {
            Err(InstanceError::Prepare(last_error.unwrap_or_else(|| {
                format!("the preparer stopped ({status})")
            })))
        }
    }
}

/// `horizon-prepare --check --output DIR`.
pub async fn check(preparer: &Path, output: &Path) -> Result<CheckResult> {
    let mut cmd = tokio::process::Command::new(preparer);
    cmd.arg("--check")
        .arg("--output")
        .arg(output)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if let Some(dir) = preparer.parent() {
        cmd.current_dir(dir);
    }
    crate::process::no_window(&mut cmd);
    let out = cmd
        .output()
        .await
        .map_err(|e| InstanceError::io(preparer, e))?;
    match out.status.code() {
        Some(0) => Ok(CheckResult {
            current: true,
            gaps: Vec::new(),
        }),
        Some(1) if !out.stdout.is_empty() => Ok(CheckResult {
            current: false,
            gaps: String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_owned)
                .collect(),
        }),
        _ => {
            let err = String::from_utf8_lossy(&out.stderr);
            Err(InstanceError::Prepare(
                err.lines()
                    .find(|l| l.starts_with("error:"))
                    .unwrap_or("the check failed")
                    .to_owned(),
            ))
        }
    }
}

/// The server jar for the preparer's first run (launch.md: "Horizon never downloads it; a
/// launcher downloads it (the version JSON's `downloads.server`) only for a signed-in account that
/// owns Java Edition"). `owner` is that proof. Checked against the sha1 and size Mojang lists.
pub async fn download_server_jar(
    owner: &VerifiedOwner,
    version_json: &Path,
    dest_dir: &Path,
    downloads: &DownloadManager,
) -> Result<PathBuf> {
    let text = tokio::fs::read_to_string(version_json)
        .await
        .map_err(|e| InstanceError::io(version_json, e))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| InstanceError::Prepare(e.to_string()))?;
    let server = &v["downloads"]["server"];
    let url = server["url"].as_str().ok_or_else(|| {
        InstanceError::Prepare("the version JSON lists no server download".into())
    })?;
    let sha1 = server["sha1"]
        .as_str()
        .and_then(Checksum::sha1)
        .ok_or_else(|| InstanceError::Prepare("the server download has no sha1".into()))?;
    let id = v["id"].as_str().unwrap_or("server");
    let dest = dest_dir.join(format!("server-{id}.jar"));
    let mut req = DownloadRequest::new(url, &dest)
        .checksum(sha1)
        .label(format!("Server files {id}"));
    if let Some(size) = server["size"].as_u64() {
        req = req.size(size);
    }
    tracing::info!(
        owner = owner.uuid(),
        version = id,
        "downloading the server jar for a verified owner"
    );
    Ok(downloads.download(req).await?)
}

/// The version JSON of `version` in an official-launcher game folder.
pub fn version_json_in(minecraft_dir: &Path, version: &str) -> PathBuf {
    minecraft_dir
        .join("versions")
        .join(version)
        .join(format!("{version}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_lines() {
        assert_eq!(
            parse_progress_line(r#"{"message": "Importing"}"#),
            Some(PrepareEvent::Message {
                text: "Importing".into()
            })
        );
        assert_eq!(
            parse_progress_line(r#"{"fraction": 0.25, "new_key": 1}"#),
            Some(PrepareEvent::Fraction { value: 0.25 })
        );
        assert_eq!(
            parse_progress_line(r#"{"error": "no client jar"}"#),
            Some(PrepareEvent::Error {
                text: "no client jar".into()
            })
        );
        assert_eq!(parse_progress_line(r#"{"unknown": 1}"#), None);
        assert_eq!(parse_progress_line("not json"), None);
    }

    fn api(options: &[&str]) -> PrepareLaunchApi {
        PrepareLaunchApi {
            launch_api: "0.1.0".into(),
            horizon: None,
            minecraft: Some("26.3".into()),
            java_major: Some(25),
            schema: None,
            output_version: None,
            options: options
                .iter()
                .map(|n| crate::launch_api::OptionSpec {
                    name: (*n).into(),
                    value: None,
                })
                .collect(),
            exit_codes: vec![0, 1, 2],
        }
    }

    #[test]
    fn only_listed_options_are_passed() {
        let req = PrepareRequest {
            preparer: "p".into(),
            output: "out".into(),
            files: GameFiles {
                minecraft_dir: Some("mc".into()),
                ..GameFiles::default()
            },
            server_jar: None,
            java_home: Some("jdk".into()),
        };
        let full = api(&[
            "--prepare",
            "--output",
            "--progress",
            "--minecraft-dir",
            "--java-home",
            "--complete",
        ]);
        let args: Vec<String> = prepare_args(&full, &req)
            .unwrap()
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "--prepare",
                "--minecraft-dir",
                "mc",
                "--java-home",
                "jdk",
                "--output",
                "out",
                "--complete",
                "--progress",
                "json"
            ]
        );
        let no_java = api(&["--prepare", "--output", "--progress", "--minecraft-dir"]);
        assert!(matches!(
            prepare_args(&no_java, &req),
            Err(InstanceError::OptionNotSupported(o)) if o == "--java-home"
        ));
    }
}
