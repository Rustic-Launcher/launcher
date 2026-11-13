//! Running the build's programs: no console window, below-normal priority (the desktop and a game
//! in play stay smooth), an environment that only the build controls, output streamed line by line
//! (Cargo's progress bar redraws with `\r`), and cancellation that ends the whole process tree.
use crate::error::{NativeError, Result};
use rusticmc_launcher_core::CancellationToken;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

/// Variables that change what Cargo or rustc build, or where they look, dropped from every
/// command, as Horizon's launcher does (horizon/docs/native-mods.md, "Rebuilds stay on the
/// instance's Horizon"): the build compiles as the edition's CI does, whatever the player's own
/// Rust setup. `CARGO_BUILD_JOBS` stays (the player's choice of cores).
pub fn scrub(cmd: &mut Command) {
    for (key, _) in std::env::vars_os() {
        if let Some(key) = key.to_str()
            && dropped(key)
        {
            cmd.env_remove(key);
        }
    }
}

/// Whether [`scrub`] drops the variable `key`.
pub fn dropped(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "RUSTFLAGS"
            | "CARGO_ENCODED_RUSTFLAGS"
            | "RUSTDOCFLAGS"
            | "CARGO_ENCODED_RUSTDOCFLAGS"
            | "CARGO_INCREMENTAL"
            | "RUSTC"
            | "RUSTC_WRAPPER"
            | "RUSTC_WORKSPACE_WRAPPER"
            | "RUSTUP_TOOLCHAIN"
            | "RUSTUP_HOME"
            | "CARGO_HOME"
            | "CARGO_TARGET_DIR"
            | "CARGO"
            | "CARGO_MANIFEST_DIR"
            | "CARGO_NET_OFFLINE"
            | "CARGO_UNSTABLE_BUILD_STD"
    ) || upper.starts_with("CARGO_PROFILE_")
        || (upper.starts_with("CARGO_BUILD_") && upper != "CARGO_BUILD_JOBS")
        || upper.starts_with("CARGO_TARGET_")
        || upper.starts_with("CARGO_REGISTRIES_")
        || upper.starts_with("CARGO_SOURCE_")
        || upper.starts_with("CARGO_ALIAS_")
        || upper.starts_with("CARGO_PATCH_")
}

/// No console window on Windows, below-normal priority everywhere.
pub fn background(cmd: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    #[cfg(unix)]
    {
        // Its own process group, so cancelling ends Cargo and every rustc it started.
        cmd.process_group(0);
    }
}

/// The program's own name, for messages.
pub fn name(program: &Path) -> String {
    program
        .file_stem()
        .unwrap_or(OsStr::new("program"))
        .to_string_lossy()
        .into_owned()
}

/// What a finished program left.
#[derive(Debug, Default)]
pub struct Finished {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout_tail: Vec<String>,
    pub stderr_tail: Vec<String>,
}

const TAIL: usize = 60;

/// Runs `cmd`, giving each line of standard output to `on_stdout` and each line (or `\r`-ended
/// redraw) of standard error to `on_stderr`, until it ends or `cancel` fires (then the process
/// tree is ended and [`NativeError::Cancelled`] returned).
pub async fn run(
    mut cmd: Command,
    cancel: &CancellationToken,
    mut on_stdout: impl FnMut(&str),
    mut on_stderr: impl FnMut(&str),
) -> Result<Finished> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let program = cmd.as_std().get_program().to_owned();
    let mut child = cmd
        .spawn()
        .map_err(|e| NativeError::io(Path::new(&program), e))?;
    let pid = child.id();
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    let mut finished = Finished::default();
    let (mut out_buf, mut err_buf) = (Vec::new(), Vec::new());
    let (mut out_open, mut err_open) = (true, true);
    let mut chunk_out = vec![0u8; 16 * 1024];
    let mut chunk_err = vec![0u8; 16 * 1024];
    while out_open || err_open {
        tokio::select! {
            () = cancel.cancelled() => {
                kill_tree(pid, &mut child).await;
                return Err(NativeError::Cancelled);
            }
            n = read(&mut out, &mut chunk_out), if out_open => match n {
                Some(n) => split(&mut out_buf, &chunk_out[..n], false, |l| {
                    push_tail(&mut finished.stdout_tail, l);
                    on_stdout(l);
                }),
                None => {
                    out_open = false;
                    flush(&mut out_buf, |l| { push_tail(&mut finished.stdout_tail, l); on_stdout(l) });
                }
            },
            n = read(&mut err, &mut chunk_err), if err_open => match n {
                Some(n) => split(&mut err_buf, &chunk_err[..n], true, |l| {
                    push_tail(&mut finished.stderr_tail, l);
                    on_stderr(l);
                }),
                None => {
                    err_open = false;
                    flush(&mut err_buf, |l| { push_tail(&mut finished.stderr_tail, l); on_stderr(l) });
                }
            },
        }
    }
    let status = tokio::select! {
        () = cancel.cancelled() => {
            kill_tree(pid, &mut child).await;
            return Err(NativeError::Cancelled);
        }
        s = child.wait() => s.map_err(|e| NativeError::io(Path::new(&program), e))?,
    };
    finished.success = status.success();
    finished.code = status.code();
    Ok(finished)
}

async fn read(r: &mut (impl AsyncRead + Unpin), buf: &mut [u8]) -> Option<usize> {
    match r.read(buf).await {
        Ok(0) | Err(_) => None,
        Ok(n) => Some(n),
    }
}

fn push_tail(tail: &mut Vec<String>, line: &str) {
    if line.trim().is_empty() {
        return;
    }
    if tail.len() == TAIL {
        tail.remove(0);
    }
    tail.push(line.to_owned());
}

/// Appends `bytes` to `buf` and hands out every complete line (`\n`, and `\r` when `cr`).
fn split(buf: &mut Vec<u8>, bytes: &[u8], cr: bool, mut each: impl FnMut(&str)) {
    for &b in bytes {
        if b == b'\n' || (cr && b == b'\r') {
            if buf.last() == Some(&b'\r') {
                buf.pop();
            }
            // With CR as a line end too, the LF of a CRLF ends an empty line: skipped.
            if !(cr && buf.is_empty()) {
                let line = String::from_utf8_lossy(buf);
                each(&line);
            }
            buf.clear();
        } else {
            buf.push(b);
        }
    }
}

fn flush(buf: &mut Vec<u8>, mut each: impl FnMut(&str)) {
    if !buf.is_empty() {
        each(&String::from_utf8_lossy(buf));
        buf.clear();
    }
}

/// Ends `pid` and its children (Cargo's rustc processes outlive Cargo otherwise).
async fn kill_tree(pid: Option<u32>, child: &mut tokio::process::Child) {
    if let Some(pid) = pid {
        #[cfg(windows)]
        {
            let mut kill = Command::new("taskkill");
            kill.args(["/T", "/F", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            background(&mut kill);
            let _ = kill.status().await;
        }
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &format!("-{pid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }
    }
    let _ = child.kill().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_on_newlines_and_redraws() {
        let mut buf = Vec::new();
        let mut lines = Vec::new();
        split(
            &mut buf,
            b"a\r\nBuilding [=> ] 1/9\rBuilding [==>] 2/9\rb",
            true,
            |l| lines.push(l.to_owned()),
        );
        assert_eq!(lines, ["a", "Building [=> ] 1/9", "Building [==>] 2/9"]);
        flush(&mut buf, |l| lines.push(l.to_owned()));
        assert_eq!(lines.last().unwrap(), "b");
        let mut out = Vec::new();
        split(&mut buf, b"x\ry\n", false, |l| out.push(l.to_owned()));
        assert_eq!(out, ["x\ry"]);
    }

    #[test]
    fn the_players_rust_settings_are_dropped() {
        for key in [
            "RUSTFLAGS",
            "rustflags",
            "CARGO_PROFILE_FAST_LTO",
            "CARGO_BUILD_TARGET",
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER",
            "CARGO_HOME",
            "RUSTC_WRAPPER",
            "CARGO_INCREMENTAL",
        ] {
            assert!(dropped(key), "{key}");
        }
        for key in ["CARGO_BUILD_JOBS", "PATH", "HOME", "CARGO_TERM_COLOR"] {
            assert!(!dropped(key), "{key}");
        }
    }

    #[tokio::test]
    async fn a_program_runs_and_its_lines_arrive() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", "echo one&& echo two 1>&2"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "echo one; echo two >&2"]);
            c
        };
        background(&mut cmd);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let done = run(
            cmd,
            &CancellationToken::new(),
            |l| out.push(l.trim().to_owned()),
            |l| err.push(l.trim().to_owned()),
        )
        .await
        .unwrap();
        assert!(done.success);
        assert_eq!(out, ["one"]);
        assert_eq!(err, ["two"]);
    }

    #[tokio::test]
    async fn cancelling_ends_the_program() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("30");
            c
        };
        background(&mut cmd);
        let cancel = CancellationToken::new();
        let c2 = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            c2.cancel();
        });
        let started = std::time::Instant::now();
        let r = run(cmd, &cancel, |_| {}, |_| {}).await;
        assert!(matches!(r, Err(NativeError::Cancelled)));
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
    }
}
