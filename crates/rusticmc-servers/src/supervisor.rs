//! The `horizon-server` command line (horizon/docs/api/server-reference.md, "Commands"): what
//! the launcher runs while no supervisor is up (`init`, `config set`, `start`), and as a fallback.
//! Every command takes `--dir DIR`. `start` runs the supervisor in the background (it spawns
//! itself; output goes to `logs/supervisor.log`), so a server outlives the launcher.

use crate::error::{Result, ServerError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// `horizon-server init`'s answers, as flags (`--eula --world NAME --seed SEED --port N
/// --whitelist/--no-whitelist --player NAME... --op NAME --offline --yes`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitOptions {
    /// The player agreed to the Minecraft EULA in the launcher (required).
    pub eula: bool,
    pub world: Option<String>,
    pub seed: Option<String>,
    pub port: Option<u16>,
    pub whitelist: bool,
    pub players: Vec<String>,
    pub op: Option<String>,
    /// `online_mode` (accounts checked). Off only for a LAN-only server.
    pub online_mode: bool,
}

impl InitOptions {
    pub fn args(&self) -> Result<Vec<String>> {
        if !self.eula {
            return Err(ServerError::EulaNotAccepted);
        }
        let mut args = vec!["init".to_owned(), "--eula".into(), "--yes".into()];
        if let Some(w) = &self.world {
            args.extend(["--world".into(), w.clone()]);
        }
        if let Some(s) = self.seed.as_ref().filter(|s| !s.is_empty()) {
            args.extend(["--seed".into(), s.clone()]);
        }
        if let Some(p) = self.port {
            args.extend(["--port".into(), p.to_string()]);
        }
        args.push(
            if self.whitelist {
                "--whitelist"
            } else {
                "--no-whitelist"
            }
            .into(),
        );
        for p in &self.players {
            args.extend(["--player".into(), p.clone()]);
        }
        if let Some(op) = &self.op {
            args.extend(["--op".into(), op.clone()]);
        }
        if !self.online_mode {
            args.push("--offline".into());
        }
        Ok(args)
    }
}

/// One server folder's `horizon-server`.
#[derive(Clone, Debug)]
pub struct Supervisor {
    pub program: PathBuf,
    pub dir: PathBuf,
}

/// A finished command.
#[derive(Clone, Debug)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Supervisor {
    pub fn new(program: PathBuf, dir: PathBuf) -> Self {
        Self { program, dir }
    }

    /// Runs `horizon-server --dir DIR <args>`.
    pub async fn run(&self, args: &[String], timeout: Duration) -> Result<Output> {
        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.arg("--dir")
            .arg(&self.dir)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        if let Some(parent) = self.program.parent() {
            cmd.current_dir(parent);
        }
        no_window(&mut cmd);
        let out = tokio::time::timeout(timeout, cmd.output())
            .await
            .map_err(|_| ServerError::Supervisor(format!("{} timed out", args.join(" "))))?
            .map_err(|e| ServerError::io(&self.program, e))?;
        Ok(Output {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    async fn ok(&self, args: &[String], timeout: Duration) -> Result<Output> {
        let out = self.run(args, timeout).await?;
        if out.code == Some(0) {
            Ok(out)
        } else {
            let why = out
                .stderr
                .lines()
                .chain(out.stdout.lines())
                .rfind(|l| !l.trim().is_empty())
                .unwrap_or("it failed")
                .to_owned();
            Err(ServerError::Supervisor(why))
        }
    }

    /// `init` with flags (unattended).
    pub async fn init(&self, options: &InitOptions) -> Result<String> {
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| ServerError::io(&self.dir, e))?;
        Ok(self
            .ok(&options.args()?, Duration::from_secs(120))
            .await?
            .stdout)
    }

    /// `config set KEY=VALUE... --json`: checked by horizon-server first; a running server takes
    /// what it can at once. Unlike the control API's PATCH, this may set `engine.*`.
    pub async fn config_set(
        &self,
        changes: &[(String, String)],
    ) -> Result<crate::control::ConfigOutcome> {
        let mut args = vec!["config".to_owned(), "set".into()];
        for (k, v) in changes {
            if k.contains(['=', '\n']) || v.contains('\n') {
                return Err(ServerError::Invalid(format!("bad setting {k}")));
            }
            args.push(format!("{k}={v}"));
        }
        args.push("--json".into());
        let out = self.ok(&args, Duration::from_secs(60)).await?;
        serde_json::from_str(out.stdout.trim())
            .map_err(|e| ServerError::Supervisor(format!("config set answered: {e}")))
    }

    /// `config --json`: server.toml's settings by section.
    pub async fn config(&self) -> Result<Value> {
        let out = self
            .ok(&["config".into(), "--json".into()], Duration::from_secs(30))
            .await?;
        serde_json::from_str(out.stdout.trim())
            .map_err(|e| ServerError::Supervisor(format!("config answered: {e}")))
    }

    /// `start`: the supervisor in the background, or a stopped supervisor resumed.
    ///
    /// Its output goes to `.rustic/start.log`, not to pipes: the background supervisor it
    /// spawns would inherit a pipe's handle (Windows passes every inheritable handle on), and a
    /// read to the end would then wait for the server to exit.
    pub async fn start(&self) -> Result<()> {
        let log_dir = self.dir.join(".rustic");
        tokio::fs::create_dir_all(&log_dir)
            .await
            .map_err(|e| ServerError::io(&log_dir, e))?;
        let log_path = log_dir.join("start.log");
        let log = std::fs::File::create(&log_path).map_err(|e| ServerError::io(&log_path, e))?;
        let err = log.try_clone().map_err(|e| ServerError::io(&log_path, e))?;
        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.arg("--dir")
            .arg(&self.dir)
            .arg("start")
            .stdin(std::process::Stdio::null())
            .stdout(log)
            .stderr(err)
            .kill_on_drop(false);
        if let Some(parent) = self.program.parent() {
            cmd.current_dir(parent);
        }
        no_window(&mut cmd);
        let status = tokio::time::timeout(Duration::from_secs(330), cmd.status())
            .await
            .map_err(|_| ServerError::Supervisor("start timed out".into()))?
            .map_err(|e| ServerError::io(&self.program, e))?;
        if status.success() {
            return Ok(());
        }
        let text = std::fs::read_to_string(&log_path).unwrap_or_default();
        Err(ServerError::Supervisor(
            text.lines()
                .rfind(|l| !l.trim().is_empty())
                .unwrap_or("the server did not start")
                .to_owned(),
        ))
    }

    pub async fn stop(&self) -> Result<()> {
        self.ok(&["stop".into()], Duration::from_secs(120))
            .await
            .map(|_| ())
    }

    pub async fn restart(&self) -> Result<()> {
        self.ok(&["restart".into()], Duration::from_secs(330))
            .await
            .map(|_| ())
    }

    /// `status --json`.
    pub async fn status(&self) -> Result<Value> {
        let out = self
            .ok(&["status".into(), "--json".into()], Duration::from_secs(30))
            .await?;
        serde_json::from_str(out.stdout.trim())
            .map_err(|e| ServerError::Supervisor(format!("status answered: {e}")))
    }

    /// `import-resources DIR`: copies game files prepared elsewhere into place after checking them.
    pub async fn import_resources(&self, prepared: &Path) -> Result<()> {
        self.ok(
            &["import-resources".into(), prepared.display().to_string()],
            Duration::from_secs(600),
        )
        .await
        .map(|_| ())
    }
}

pub(crate) fn no_window(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_flags() {
        let o = InitOptions {
            eula: true,
            world: Some("survival".into()),
            seed: Some("123".into()),
            port: Some(25570),
            whitelist: true,
            players: vec!["Alex".into()],
            op: Some("Alex".into()),
            online_mode: true,
        };
        assert_eq!(
            o.args().unwrap(),
            [
                "init",
                "--eula",
                "--yes",
                "--world",
                "survival",
                "--seed",
                "123",
                "--port",
                "25570",
                "--whitelist",
                "--player",
                "Alex",
                "--op",
                "Alex"
            ]
        );
        let lan = InitOptions {
            online_mode: false,
            whitelist: false,
            ..o.clone()
        };
        assert!(lan.args().unwrap().ends_with(&["--offline".to_owned()]));
        let no = InitOptions { eula: false, ..o };
        assert!(matches!(no.args(), Err(ServerError::EulaNotAccepted)));
    }
}
