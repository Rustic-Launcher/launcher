//! A stand-in for `horizon-server` in this crate's tests, following its documented CLI and
//! control API (horizon/docs/api/server-reference.md). Not shipped.
//!
//! `--dir D init ...` writes server.toml and eula.txt (refuses without `--eula`); `config set
//! K=V... --json` records the settings in `fake-settings.json`; `config --json` prints them;
//! `start` runs `__serve` in the background and waits for `.horizon/api.json`; `backup [NAME]`
//! writes a zip into backups/; `__serve` answers the control API on 127.0.0.1 with a bearer token.
//! It also stands in for `rustic-server` where only its `--rustic-version` line matters.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

fn settings(dir: &Path) -> serde_json::Map<String, Value> {
    std::fs::read_to_string(dir.join("fake-settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_settings(dir: &Path, s: &serde_json::Map<String, Value>) {
    std::fs::write(
        dir.join("fake-settings.json"),
        serde_json::to_string(s).unwrap(),
    )
    .unwrap();
}

fn backup(dir: &Path, name: &str) -> String {
    let backups = dir.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let file = format!("2026-11-10_12-00-00-{name}.zip");
    std::fs::write(
        backups.join(&file),
        b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    )
    .unwrap();
    file
}

#[allow(clippy::zombie_processes)] // `start` detaches the supervisor on purpose
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dir, rest) = match args.first().map(String::as_str) {
        Some("--dir") => (PathBuf::from(&args[1]), args[2..].to_vec()),
        _ => (PathBuf::from("."), args.clone()),
    };
    let code = match rest.first().map(String::as_str) {
        Some("--rustic-version") => {
            println!("Rustic 0.0.0-fake (rustic:api 0.1) on Horizon (fake)");
            0
        }
        Some("init") => {
            if !rest.iter().any(|a| a == "--eula") {
                eprintln!("the server can't run until you agree to the EULA");
                1
            } else if dir.join("server.toml").exists() {
                eprintln!("server.toml already exists");
                1
            } else {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("eula.txt"), "eula=true\n").unwrap();
                let port = rest
                    .iter()
                    .position(|a| a == "--port")
                    .map(|i| rest[i + 1].clone())
                    .unwrap_or_else(|| "25565".into());
                let online = !rest.iter().any(|a| a == "--offline");
                std::fs::write(
                    dir.join("server.toml"),
                    format!("[server]\nport = {port}\nonline_mode = {online}\n"),
                )
                .unwrap();
                std::fs::write(dir.join("init-args.txt"), rest.join(" ")).unwrap();
                println!("Wrote server.toml.");
                0
            }
        }
        Some("config") if rest.get(1).map(String::as_str) == Some("set") => {
            let mut s = settings(&dir);
            let mut saved = Vec::new();
            for kv in rest[2..].iter().filter(|a| *a != "--json") {
                let Some((k, v)) = kv.split_once('=') else {
                    eprintln!("bad change {kv}");
                    std::process::exit(1);
                };
                s.insert(k.into(), json!(v));
                saved.push(k.to_owned());
            }
            save_settings(&dir, &s);
            println!(
                "{}",
                json!({"saved": saved, "applied": [], "restart": [], "running": false})
            );
            0
        }
        Some("config") => {
            println!("{}", json!({ "fake": settings(&dir) }));
            0
        }
        Some("backup") => {
            let name = rest.get(1).cloned().unwrap_or_else(|| "manual".into());
            println!("{}", backup(&dir, &name));
            0
        }
        Some("start") => {
            let exe = std::env::current_exe().unwrap();
            let _ = std::fs::remove_file(dir.join(".horizon/api.json"));
            std::process::Command::new(exe)
                .arg("--dir")
                .arg(&dir)
                .arg("__serve")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            for _ in 0..100 {
                if dir.join(".horizon/api.json").exists() {
                    println!("The server is running.");
                    std::process::exit(0);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            eprintln!("the server did not start");
            1
        }
        Some("__serve") => {
            serve(&dir);
            0
        }
        _ => 2,
    };
    std::process::exit(code);
}

fn serve(dir: &Path) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let token = "fake-token-0123456789";
    std::fs::create_dir_all(dir.join(".horizon")).unwrap();
    let endpoint = json!({ "address": address, "token": token, "pid": std::process::id() });
    std::fs::write(dir.join(".horizon/api.json"), endpoint.to_string()).unwrap();
    let mut phase = "running";
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        if let Some(stop) = handle(stream, dir, token, &mut phase)
            && stop
        {
            let _ = std::fs::remove_file(dir.join(".horizon/api.json"));
            return;
        }
    }
}

fn respond(stream: &mut TcpStream, status: u16, body: &Value) {
    let text = body.to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    );
}

/// Some(true) when the supervisor must exit.
fn handle(mut stream: TcpStream, dir: &Path, token: &str, phase: &mut &str) -> Option<bool> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_owned();
    let target = parts.next()?.to_owned();
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end().to_owned();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        }
    }
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let len: usize = header("content-length")
        .and_then(|l| l.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).ok()?;
    let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    if header("authorization").as_deref() != Some(&format!("Bearer {token}")) {
        respond(
            &mut stream,
            401,
            &json!({"error": "missing or wrong token (see .horizon/api.json)"}),
        );
        return None;
    }
    let ok = json!({"ok": true});
    let changed = |what: &str| json!({"changed": true, "output": [what]});
    match (method.as_str(), path) {
        ("GET", "/api/v1/status") => respond(
            &mut stream,
            200,
            &json!({
                "phase": *phase, "version": "fake", "pid": std::process::id(), "uptime_seconds": 3,
                "engine_pid": 4242, "engine_uptime_seconds": 2, "crashes": 0, "last_crash": null,
                "sample": {"pid": 4242, "rss_bytes": 1_420_000_000u64, "cpu_percent": 2.5,
                           "game": {"tps": 20.0, "mspt": {"average": 1.0, "p95": 3.1}, "players": [{"name": "Steve"}]}},
                "update": null
            }),
        ),
        ("GET", "/api/v1/metrics") => respond(
            &mut stream,
            200,
            &json!({"samples": [{"t": 1, "tps": 20.0, "mspt": 1.0, "mspt_p95": 3.0, "rss_bytes": 10, "cpu_percent": 1.0, "players": 1}]}),
        ),
        ("POST", "/api/v1/command") => {
            if *phase != "running" {
                respond(
                    &mut stream,
                    409,
                    &json!({"error": "the game isn't running right now"}),
                );
            } else {
                let line = body["line"].as_str().unwrap_or("");
                respond(
                    &mut stream,
                    200,
                    &json!({"lines": [format!("ran: {line}")]}),
                );
            }
        }
        ("GET", "/api/v1/players") => respond(
            &mut stream,
            200,
            &json!({"online": [{"name": "Steve", "uuid": "u", "ping_ms": 20}], "max_players": 20, "ops": [], "whitelist": {"enabled": true, "players": []}, "bans": [], "ip_bans": []}),
        ),
        ("POST", p) if p.starts_with("/api/v1/players/") || p.starts_with("/api/v1/whitelist") => {
            let what = format!(
                "{} {}",
                p.rsplit('/').next().unwrap_or(""),
                body["name"].as_str().or(body["ip"].as_str()).unwrap_or("")
            );
            respond(&mut stream, 200, &changed(what.trim()))
        }
        ("GET", "/api/v1/backups") => {
            let list: Vec<Value> = std::fs::read_dir(dir.join("backups"))
                .map(|e| {
                    e.flatten()
                        .map(|e| json!({"name": e.file_name().to_string_lossy(), "bytes": 22}))
                        .collect()
                })
                .unwrap_or_default();
            respond(&mut stream, 200, &json!({"backups": list}))
        }
        ("POST", "/api/v1/backups") => {
            let name = backup(dir, body["name"].as_str().unwrap_or("manual"));
            respond(&mut stream, 200, &json!({"name": name}))
        }
        ("DELETE", "/api/v1/backups") => {
            let name = query.strip_prefix("name=").unwrap_or("");
            let _ = std::fs::remove_file(dir.join("backups").join(name));
            respond(&mut stream, 200, &json!({"name": name}))
        }
        ("POST", "/api/v1/restore") => respond(
            &mut stream,
            200,
            &json!({"name": body["name"], "dimension": body["dimension"]}),
        ),
        ("GET", "/api/v1/config") => respond(&mut stream, 200, &json!({"config": settings(dir)})),
        ("GET", "/api/v1/events") => {
            if header("accept").as_deref() == Some("text/event-stream") {
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n"
                );
                let _ = write!(
                    stream,
                    "id: 1\nevent: log\ndata: {{\"id\":1,\"type\":\"log\",\"line\":\"[INFO] Done\"}}\n\n"
                );
                let _ = write!(
                    stream,
                    "id: 2\nevent: join\ndata: {{\"id\":2,\"type\":\"join\",\"name\":\"Steve\"}}\n\n"
                );
                let _ = stream.flush();
                // Hold the stream open a moment, like a live server between events.
                std::thread::sleep(std::time::Duration::from_millis(300));
            } else {
                respond(
                    &mut stream,
                    200,
                    &json!({"next": 2, "events": [{"id": 1, "type": "log", "line": "[INFO] Done"}, {"id": 2, "type": "join", "name": "Steve"}]}),
                )
            }
        }
        ("POST", "/api/v1/stop") => {
            respond(&mut stream, 200, &ok);
            return Some(true);
        }
        ("POST", "/api/v1/restart") => {
            *phase = "running";
            respond(&mut stream, 200, &ok)
        }
        ("POST", "/api/v1/start") => {
            *phase = "running";
            respond(&mut stream, 200, &ok)
        }
        _ => respond(&mut stream, 404, &json!({"error": "not found"})),
    }
    None
}
