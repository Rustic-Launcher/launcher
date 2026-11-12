//! Asking a build what it takes (horizon/docs/api/launch.md, launch-reference.md):
//! `horizon-client --launch-api` and `horizon-prepare --launch-api` each print one JSON object and
//! exit 0. Readers ignore fields they don't know; only listed options may be passed (any other is
//! refused with exit code 2). This launcher speaks the 0.1 line.

use crate::error::{InstanceError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// The Launch API line this launcher speaks.
pub const SUPPORTED_LINE: &str = "0.1";

/// An accepted option: its name and its value's placeholder (`None`: a flag).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionSpec {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

/// `horizon-client --launch-api`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientLaunchApi {
    pub launch_api: String,
    #[serde(default)]
    pub horizon: Option<String>,
    #[serde(default)]
    pub minecraft: Option<String>,
    #[serde(default)]
    pub options: Vec<OptionSpec>,
    /// What content this build loads (mods, packs, shaders...), passed through to the UI.
    #[serde(default)]
    pub content: serde_json::Value,
    /// `Horizon: crash report saved to `: the last stderr line after a crash starts with it.
    #[serde(default)]
    pub crash_report_line: Option<String>,
    #[serde(default)]
    pub exit_codes: Vec<i64>,
}

/// `horizon-prepare --launch-api`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrepareLaunchApi {
    pub launch_api: String,
    #[serde(default)]
    pub horizon: Option<String>,
    /// The Minecraft version whose files it needs.
    #[serde(default)]
    pub minecraft: Option<String>,
    /// The Java `--java-home` must give.
    #[serde(default)]
    pub java_major: Option<u32>,
    #[serde(default)]
    pub schema: Option<i64>,
    #[serde(default)]
    pub output_version: Option<i64>,
    #[serde(default)]
    pub options: Vec<OptionSpec>,
    #[serde(default)]
    pub exit_codes: Vec<i64>,
}

/// Whether `options` lists `name`.
pub fn lists(options: &[OptionSpec], name: &str) -> bool {
    options.iter().any(|o| o.name == name)
}

impl ClientLaunchApi {
    pub fn accepts(&self, option: &str) -> bool {
        lists(&self.options, option)
    }
}

impl PrepareLaunchApi {
    pub fn accepts(&self, option: &str) -> bool {
        lists(&self.options, option)
    }
}

/// Refuses a version outside the 0.1 line (within it a newer build only adds).
pub fn check_line(version: &str) -> Result<()> {
    let mut parts = version.split('.');
    let line = match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        _ => String::new(),
    };
    if line == SUPPORTED_LINE {
        Ok(())
    } else {
        Err(InstanceError::LaunchApiVersion {
            found: version.into(),
        })
    }
}

async fn ask(program: &Path) -> Result<serde_json::Value> {
    let name = program.display().to_string();
    let mut cmd = tokio::process::Command::new(program);
    cmd.arg("--launch-api")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if let Some(dir) = program.parent() {
        cmd.current_dir(dir);
    }
    crate::process::no_window(&mut cmd);
    let output = tokio::time::timeout(Duration::from_secs(20), cmd.output())
        .await
        .map_err(|_| InstanceError::LaunchApi {
            program: name.clone(),
            message: "no answer within 20 s".into(),
        })?
        .map_err(|e| InstanceError::io(program, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(InstanceError::LaunchApi {
            program: name,
            message: stderr.lines().last().unwrap_or("it failed").to_owned(),
        });
    }
    serde_json::from_slice(&output.stdout).map_err(|e| InstanceError::LaunchApi {
        program: name,
        message: e.to_string(),
    })
}

/// Asks `horizon-client`.
pub async fn probe_client(program: &Path) -> Result<ClientLaunchApi> {
    let v = ask(program).await?;
    let api: ClientLaunchApi = serde_json::from_value(v).map_err(|e| InstanceError::LaunchApi {
        program: program.display().to_string(),
        message: e.to_string(),
    })?;
    check_line(&api.launch_api)?;
    Ok(api)
}

/// Asks `horizon-prepare`.
pub async fn probe_preparer(program: &Path) -> Result<PrepareLaunchApi> {
    let v = ask(program).await?;
    let api: PrepareLaunchApi =
        serde_json::from_value(v).map_err(|e| InstanceError::LaunchApi {
            program: program.display().to_string(),
            message: e.to_string(),
        })?;
    check_line(&api.launch_api)?;
    Ok(api)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_reply_parses() {
        // horizon/docs/api/launch-reference.md, trimmed; unknown fields are ignored.
        let v = serde_json::json!({
            "content": {"mod": {"api": ["0.2", "0.3"]}},
            "crash_report_line": "Horizon: crash report saved to ",
            "exit_codes": [0, 1, 2, 101],
            "horizon": "nightly", "launch_api": "0.1.0", "minecraft": "26.3",
            "options": [{"name": "--resources", "value": "DIR"}, {"name": "--session-stdin", "value": null}],
            "something_new": true
        });
        let api: ClientLaunchApi = serde_json::from_value(v).unwrap();
        assert!(api.accepts("--session-stdin") && !api.accepts("--name"));
        check_line(&api.launch_api).unwrap();
    }

    #[test]
    fn other_lines_are_refused() {
        assert!(check_line("0.1.7").is_ok());
        assert!(check_line("0.2.0").is_err());
        assert!(check_line("1.0.0").is_err());
        assert!(check_line("x").is_err());
    }
}
