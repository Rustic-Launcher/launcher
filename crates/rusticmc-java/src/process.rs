//! The running game: spawn (with an argument file on Java 9+), stream its output as log lines
//! (Log4j XML events from the official logging setup read back as plain lines), stop it, and say
//! how it ended (a crash: non-zero exit, the game's crash marker, or a new crash report).

use crate::launch::LaunchCommand;
use crate::{JavaError, Result};
use rusticmc_launcher_core::CancellationToken;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// One line of the game's output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LogLine {
    pub stream: Stream,
    /// `INFO`, `WARN`, `ERROR`... when the line says.
    pub level: Option<String>,
    pub thread: Option<String>,
    pub text: String,
}

/// How the game ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Exit {
    pub code: Option<i32>,
    pub crashed: bool,
    /// Stopped from the launcher.
    pub killed: bool,
    pub crash_report: Option<PathBuf>,
}

/// Reads the official logging setup's XML events (`<log4j:Event ...>`) back into lines; other
/// lines pass through.
#[derive(Default)]
pub struct Log4jReader {
    event: Option<String>,
}

const CRASH_MARKER: &str = "#@!@# Game crashed!";

impl Log4jReader {
    /// Feeds one raw line; returns the lines it completes.
    pub fn feed(&mut self, raw: &str, stream: Stream) -> Vec<LogLine> {
        if let Some(buf) = &mut self.event {
            buf.push('\n');
            buf.push_str(raw);
            if raw.contains("</log4j:Event>") {
                let ev = self.event.take().unwrap_or_default();
                return parse_event(&ev, stream);
            }
            return Vec::new();
        }
        let t = raw.trim_start();
        if t.starts_with("<log4j:Event") {
            if t.contains("</log4j:Event>") {
                return parse_event(t, stream);
            }
            self.event = Some(raw.to_owned());
            return Vec::new();
        }
        vec![LogLine {
            stream,
            level: plain_level(raw),
            thread: None,
            text: raw.to_owned(),
        }]
    }
}

fn plain_level(line: &str) -> Option<String> {
    // `[12:00:00] [Render thread/WARN]: ...` (the game's own pattern without the XML setup).
    let start = line.find('/')?;
    let end = line[start..].find(']')? + start;
    let level = &line[start + 1..end];
    matches!(
        level,
        "INFO" | "WARN" | "ERROR" | "DEBUG" | "FATAL" | "TRACE"
    )
    .then(|| level.to_owned())
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(unescape(&tag[start..end]))
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn cdata(ev: &str, element: &str) -> Option<String> {
    let open = format!("<log4j:{element}>");
    let start = ev.find(&open)? + open.len();
    let end = ev[start..].find(&format!("</log4j:{element}>"))? + start;
    let inner = ev[start..end].trim();
    Some(match inner.strip_prefix("<![CDATA[") {
        Some(rest) => rest.strip_suffix("]]>").unwrap_or(rest).to_owned(),
        None => unescape(inner),
    })
}

fn parse_event(ev: &str, stream: Stream) -> Vec<LogLine> {
    let head_end = ev.find('>').unwrap_or(ev.len());
    let head = &ev[..head_end];
    let level = attr(head, "level");
    let thread = attr(head, "thread");
    let mut out = Vec::new();
    let message = cdata(ev, "Message").unwrap_or_default();
    for line in message.lines() {
        out.push(LogLine {
            stream,
            level: level.clone(),
            thread: thread.clone(),
            text: line.to_owned(),
        });
    }
    if let Some(t) = cdata(ev, "Throwable") {
        for line in t.lines() {
            out.push(LogLine {
                stream,
                level: level.clone(),
                thread: thread.clone(),
                text: line.to_owned(),
            });
        }
    }
    if out.is_empty() {
        out.push(LogLine {
            stream,
            level,
            thread,
            text: String::new(),
        });
    }
    out
}

/// A started game.
pub struct GameHandle {
    pub pid: Option<u32>,
    stop: CancellationToken,
    pub done: tokio::task::JoinHandle<Exit>,
}

impl GameHandle {
    /// Asks the game to stop (kills the process).
    pub fn stop(&self) {
        self.stop.cancel();
    }
    pub fn stop_token(&self) -> CancellationToken {
        self.stop.clone()
    }
}

/// Starts `cmd`. With [`LaunchCommand::use_argfile`] the arguments go to a file in `run_dir`
/// (this user only), deleted once the game prints its first line (or after 30 s, or at exit).
/// `on_line` gets every output line.
pub fn spawn(
    cmd: &LaunchCommand,
    run_dir: &Path,
    on_line: Arc<dyn Fn(LogLine) + Send + Sync>,
) -> Result<GameHandle> {
    if !cmd.java.is_file() && cmd.java.components().count() > 1 {
        return Err(JavaError::NoJavaExecutable(cmd.java.clone()));
    }
    let mut command = tokio::process::Command::new(&cmd.java);
    let mut argfile = None;
    if cmd.use_argfile {
        std::fs::create_dir_all(run_dir).map_err(|e| JavaError::io(run_dir, e))?;
        let path = run_dir.join(format!("{}.args", uuid::Uuid::new_v4().simple()));
        write_private(&path, cmd.argfile_text().as_bytes())?;
        command.arg(format!("@{}", path.display()));
        argfile = Some(path);
    } else {
        command.args(&cmd.args);
    }
    command
        .current_dir(&cmd.cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    no_window(&mut command);
    let started = SystemTime::now();
    let mut child = command.spawn().map_err(|e| {
        if let Some(p) = &argfile {
            let _ = std::fs::remove_file(p);
        }
        JavaError::Process(format!("could not start {}: {e}", cmd.java.display()))
    })?;
    let pid = child.id();
    let stop = CancellationToken::new();
    let first_line = CancellationToken::new();
    let crash_seen = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let mut readers = Vec::new();
    for (pipe, stream) in [
        (
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
            Stream::Stdout,
        ),
        (
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
            Stream::Stderr,
        ),
    ] {
        let Some(pipe) = pipe else { continue };
        let on_line = on_line.clone();
        let first = first_line.clone();
        let crash = crash_seen.clone();
        readers.push(tokio::spawn(async move {
            let mut lines = BufReader::new(pipe).lines();
            let mut reader = Log4jReader::default();
            while let Ok(Some(raw)) = lines.next_line().await {
                first.cancel();
                if raw.contains(CRASH_MARKER) {
                    crash.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                for line in reader.feed(&raw, stream) {
                    on_line(line);
                }
            }
        }));
    }
    if let Some(path) = argfile.clone() {
        let first = first_line.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = first.cancelled() => {}
                _ = tokio::time::sleep(Duration::from_secs(30)) => {}
            }
            // The Java launcher has read it before the first line of output.
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = std::fs::remove_file(&path);
        });
    }
    let stop2 = stop.clone();
    let game_dir = cmd.cwd.clone();
    let done = tokio::spawn(async move {
        let mut killed = false;
        let status = tokio::select! {
            s = child.wait() => s.ok(),
            _ = stop2.cancelled() => {
                killed = true;
                let _ = child.start_kill();
                child.wait().await.ok()
            }
        };
        for r in readers {
            let _ = tokio::time::timeout(Duration::from_secs(2), r).await;
        }
        if let Some(p) = &argfile {
            let _ = std::fs::remove_file(p);
        }
        let code = status.and_then(|s| s.code());
        let crash_report = newest_crash_report(&game_dir, started);
        let crashed = !killed
            && (code.is_some_and(|c| c != 0)
                || crash_seen.load(std::sync::atomic::Ordering::Relaxed)
                || crash_report.is_some());
        Exit {
            code,
            crashed,
            killed,
            crash_report,
        }
    });
    Ok(GameHandle { pid, stop, done })
}

/// A crash report written in `game_dir/crash-reports` since `since`.
pub fn newest_crash_report(game_dir: &Path, since: SystemTime) -> Option<PathBuf> {
    let dir = game_dir.join("crash-reports");
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let m = e.metadata().ok()?.modified().ok()?;
            (m >= since).then(|| (m, e.path()))
        })
        .max_by_key(|(m, _)| *m)
        .map(|(_, p)| p)
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| JavaError::io(path, e))?;
        f.write_all(bytes).map_err(|e| JavaError::io(path, e))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        // Under the user's own profile folder, which other users can't read.
        std::fs::write(path, bytes).map_err(|e| JavaError::io(path, e))
    }
}

/// Removes argument files a crashed launcher left behind.
pub fn clean_run_dir(run_dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(run_dir) {
        for e in entries.flatten() {
            if e.path().extension().is_some_and(|x| x == "args") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// No console window for a child on Windows.
pub fn no_window(cmd: &mut tokio::process::Command) {
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
    fn reads_log4j_events_back_into_lines() {
        let mut r = Log4jReader::default();
        let mut out = Vec::new();
        for l in [
            r#"<log4j:Event logger="net.minecraft.client.Minecraft" timestamp="1" level="INFO" thread="Render thread">"#,
            r#"  <log4j:Message><![CDATA[Setting user: Alex]]></log4j:Message>"#,
            r#"</log4j:Event>"#,
            "plain line",
            r#"<log4j:Event logger="x" timestamp="2" level="ERROR" thread="main"><log4j:Message><![CDATA[boom &amp; <b>]]></log4j:Message><log4j:Throwable><![CDATA[java.lang.RuntimeException
	at a.b(c)]]></log4j:Throwable></log4j:Event>"#,
            "[12:00:00] [Worker-Main-1/WARN]: careful",
        ] {
            out.extend(r.feed(l, Stream::Stdout));
        }
        assert_eq!(out[0].text, "Setting user: Alex");
        assert_eq!(out[0].level.as_deref(), Some("INFO"));
        assert_eq!(out[0].thread.as_deref(), Some("Render thread"));
        assert_eq!(out[1].text, "plain line");
        assert_eq!(out[1].level, None);
        assert_eq!(out[2].text, "boom &amp; <b>");
        assert_eq!(out[2].level.as_deref(), Some("ERROR"));
        assert_eq!(out[3].text, "java.lang.RuntimeException");
        assert_eq!(out[4].text, "\tat a.b(c)");
        assert_eq!(out[5].level.as_deref(), Some("WARN"));
    }

    #[test]
    fn finds_new_crash_reports_only() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("crash-reports");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("old.txt"), "x").unwrap();
        let later = SystemTime::now() + Duration::from_secs(3600);
        assert!(newest_crash_report(tmp.path(), later).is_none());
        let earlier = SystemTime::now() - Duration::from_secs(3600);
        assert_eq!(
            newest_crash_report(tmp.path(), earlier).unwrap(),
            dir.join("old.txt")
        );
    }
}
