//! A stand-in for Horizon's programs in this crate's tests: copied as `horizon-client` or
//! `horizon-prepare`, it acts as that program, following the documented Launch API 0.1
//! (horizon/docs/api/launch.md, launch-reference.md). It is not shipped.
//!
//! - `--launch-api`: the reply of the program it acts as.
//! - client: refuses unknown options (exit 2, `Horizon: --x: unknown option`); with
//!   `--session-stdin` reads the session to EOF and writes it to `<game-dir>/received-session.json`;
//!   writes its argv to `<game-dir>/argv.txt`; logs a line on stdout; if `<game-dir>/crash-me`
//!   exists, writes a crash report and exits 1 naming it; if `<game-dir>/hang` exists, sleeps.
//! - preparer: `--prepare ... --progress json` prints progress lines and writes `<output>/ready`
//!   (with `fail` in the minecraft dir's name: an `{"error"}` line and exit 1); `--check --output`
//!   exits 0 when `ready` exists, else lists a gap and exits 1.

use std::io::{Read, Write};
use std::path::PathBuf;

const CLIENT_OPTIONS: &[(&str, Option<&str>)] = &[
    ("--resources", Some("DIR")),
    ("--game-dir", Some("DIR")),
    ("--home", Some("DIR")),
    ("--player-dir", Some("DIR")),
    ("--width", Some("N")),
    ("--height", Some("N")),
    ("--fullscreen", None),
    ("--session-file", Some("PATH")),
    ("--session-stdin", None),
    ("--quickPlaySingleplayer", Some("WORLD")),
    ("--quickPlayMultiplayer", Some("HOST[:PORT]")),
    ("--launch-api", None),
];

const PREPARE_OPTIONS: &[(&str, Option<&str>)] = &[
    ("--prepare", None),
    ("--server-jar", Some("PATH")),
    ("--minecraft-dir", Some("PATH")),
    ("--prism-dir", Some("PATH")),
    ("--version-json", Some("PATH")),
    ("--client-jar", Some("PATH")),
    ("--libraries-dir", Some("PATH")),
    ("--assets-dir", Some("PATH")),
    ("--java-home", Some("PATH")),
    ("--root", Some("PATH")),
    ("--output", Some("PATH")),
    ("--check", None),
    ("--complete", None),
    ("--progress", Some("json")),
    ("--launch-api", None),
];

fn options_json(options: &[(&str, Option<&str>)]) -> String {
    let items: Vec<String> = options
        .iter()
        .map(|(n, v)| match v {
            Some(v) => format!(r#"{{"name": "{n}", "value": "{v}"}}"#),
            None => format!(r#"{{"name": "{n}", "value": null}}"#),
        })
        .collect();
    format!("[{}]", items.join(", "))
}

/// Parses `args` against `options`: `Err(option)` for an unknown one or a missing value.
fn parse(
    args: &[String],
    options: &[(&str, Option<&str>)],
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let Some((_, takes)) = options.iter().find(|(n, _)| n == a) else {
            return Err(format!("{a}: unknown option"));
        };
        if takes.is_some() {
            let v = args
                .get(i + 1)
                .ok_or_else(|| format!("{a}: missing value"))?;
            out.push((a.clone(), Some(v.clone())));
            i += 2;
        } else {
            out.push((a.clone(), None));
            i += 1;
        }
    }
    Ok(out)
}

fn value(parsed: &[(String, Option<String>)], name: &str) -> Option<String> {
    parsed
        .iter()
        .find(|(n, _)| n == name)
        .and_then(|(_, v)| v.clone())
}

fn has(parsed: &[(String, Option<String>)], name: &str) -> bool {
    parsed.iter().any(|(n, _)| n == name)
}

fn client(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "--launch-api") {
        println!(
            r#"{{"content": {{}}, "crash_report_line": "Horizon: crash report saved to ", "exit_codes": [0, 1, 2, 101], "horizon": "fake", "launch_api": "0.1.0", "minecraft": "26.3", "options": {}}}"#,
            options_json(CLIENT_OPTIONS)
        );
        return 0;
    }
    let parsed = match parse(args, CLIENT_OPTIONS) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Horizon: {e}");
            return 2;
        }
    };
    let game_dir = PathBuf::from(value(&parsed, "--game-dir").unwrap_or_else(|| ".".into()));
    let home = PathBuf::from(value(&parsed, "--home").unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&game_dir).unwrap();
    std::fs::write(game_dir.join("argv.txt"), args.join("\n")).unwrap();
    if has(&parsed, "--session-stdin") {
        let mut session = String::new();
        std::io::stdin().read_to_string(&mut session).unwrap();
        std::fs::write(game_dir.join("received-session.json"), &session).unwrap();
        if serde_json::from_str::<serde_json::Value>(&session).is_err() {
            eprintln!("Horizon: not signed in: the session is not JSON");
        }
    }
    println!("[main/INFO]: Starting Horizon (fake)");
    std::io::stdout().flush().unwrap();
    if game_dir.join("hang").exists() {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    if game_dir.join("crash-me").exists() {
        let dir = home.join("crash-reports");
        std::fs::create_dir_all(&dir).unwrap();
        let report = dir.join("crash-2026-11-10_12.00.00-client.txt");
        std::fs::write(&report, "---- Horizon Crash Report ----").unwrap();
        eprintln!("thread 'main' panicked");
        eprintln!("Horizon: crash report saved to {}", report.display());
        return 1;
    }
    0
}

fn prepare(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "--launch-api") {
        println!(
            r#"{{"launch_api": "0.1.0", "horizon": "fake", "minecraft": "26.3", "java_major": 25, "schema": 3, "output_version": 7, "options": {}, "exit_codes": [0, 1, 2]}}"#,
            options_json(PREPARE_OPTIONS)
        );
        return 0;
    }
    let parsed = match parse(args, PREPARE_OPTIONS) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let output = PathBuf::from(value(&parsed, "--output").unwrap_or_else(|| "out".into()));
    if has(&parsed, "--check") {
        if has(&parsed, "--progress") || has(&parsed, "--complete") {
            return 2;
        }
        if output.join("ready").exists() {
            return 0;
        }
        println!(
            "resources missing: run horizon-prepare --prepare --output {}",
            output.display()
        );
        return 1;
    }
    if !has(&parsed, "--prepare") {
        return 2;
    }
    let json = value(&parsed, "--progress").as_deref() == Some("json");
    let say = |line: &str| {
        if json {
            println!("{line}");
            std::io::stdout().flush().unwrap();
        }
    };
    say(r#"{"message": "Checking your Minecraft files"}"#);
    say(r#"{"fraction": 0.1}"#);
    say(r#"{"some_future_key": true}"#);
    eprintln!("importing (human text)");
    if value(&parsed, "--minecraft-dir").is_some_and(|d| d.contains("fail")) {
        say(r#"{"error": "the client jar doesn't match the pinned checksum"}"#);
        return 1;
    }
    say(r#"{"fraction": 0.6}"#);
    say(r#"{"message": "Exporting registries"}"#);
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(
        output.join("ready"),
        format!("java={:?}", value(&parsed, "--java-home")),
    )
    .unwrap();
    say(r#"{"fraction": 1.0}"#);
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let me = std::env::current_exe().unwrap();
    let stem = me
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let code = if stem.contains("prepare") {
        prepare(&args)
    } else {
        client(&args)
    };
    std::process::exit(code);
}
