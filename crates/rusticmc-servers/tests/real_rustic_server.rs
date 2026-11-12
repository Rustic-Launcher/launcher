//! The launcher's own server path against a real Rustic edition build (ADR 0152): the build folder
//! ships `rustic-server`, which must run on that build's prepared resources. Needs the player's
//! data folder, so it only runs when asked:
//!
//! ```text
//! RUSTIC_REAL_DATA="%APPDATA%/Rustic/Rustic Launcher/data" \
//! RUSTIC_REAL_BUILD=rustic-edition-dev-a84f637 \
//!   cargo test -p rusticmc-servers --test real_rustic_server -- --ignored --nocapture
//! ```
//!
//! It creates a server in a temporary folder (never in the player's `servers/`), starts it on a
//! free port, waits for the control API to say it runs and for a status ping to answer, stops it
//! and deletes it.

use rusticmc_instances::BuildManager;
use rusticmc_launcher_core::DownloadManager;
use rusticmc_servers::control::Phase;
use rusticmc_servers::engine::EngineOrigin;
use rusticmc_servers::hosting::{CreateServer, ServerKind};
use rusticmc_servers::{ServerError, ServerManager};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a real Rustic edition build and its prepared resources (RUSTIC_REAL_DATA)"]
async fn a_real_rustic_server_starts_answers_and_stops() {
    let data = PathBuf::from(std::env::var("RUSTIC_REAL_DATA").expect("RUSTIC_REAL_DATA"));
    let build =
        std::env::var("RUSTIC_REAL_BUILD").unwrap_or_else(|_| "rustic-edition-dev-a84f637".into());
    let tmp = tempfile::tempdir().unwrap();
    let http = rusticmc_launcher_core::http::client().unwrap();
    let builds = BuildManager::new(
        data.join("builds"),
        http.clone(),
        DownloadManager::new(tmp.path().join("cache"), 1, 0).unwrap(),
    );
    // No manual override: what a player with a fresh launcher.toml gets.
    let m = ServerManager::new(
        tmp.path().join("servers"),
        http,
        builds,
        data.join("prepared"),
        None,
    );

    let engine = m
        .rustic_server_for_build(Some(&build))
        .expect("rustic-server");
    assert_eq!(engine.origin, EngineOrigin::Build, "{engine:?}");
    let version = rusticmc_servers::engine::rustic_server_version(&engine.path)
        .await
        .unwrap();
    println!("engine: {} ({version})", engine.path.display());

    let port = free_port();
    let profile = m
        .create(CreateServer {
            name: "Rustic harness".into(),
            kind: ServerKind::RusticServer,
            build: build.clone(),
            eula_accepted: true,
            world: Some("world".into()),
            seed: Some("12345".into()),
            port: Some(port),
            whitelist: true,
            players: Vec::new(),
            op: None,
            online_mode: true,
            engine_path: None,
        })
        .await
        .unwrap();
    let toml = std::fs::read_to_string(m.dir(&profile.id).join("server.toml")).unwrap();
    println!("server.toml:\n{toml}");
    assert!(
        toml.contains("rustic-server"),
        "engine.path is not rustic-server"
    );

    let result = async {
        m.start_server(&profile.id).await?;
        let deadline = Instant::now() + Duration::from_secs(300);
        let status = loop {
            if let Some(s) = m.live_status(&profile.id).await
                && s.phase == Phase::Running
            {
                break s;
            }
            if Instant::now() > deadline {
                return Err(ServerError::Supervisor("not running after 300 s".into()));
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        };
        println!("status: {status:?}");
        // The game answers a status ping on its port (the supervisor's "running" may come first).
        let ping = loop {
            match rusticmc_hosting::slp::ping("127.0.0.1", port, Duration::from_secs(5)).await {
                Ok(p) => break p,
                Err(e) if Instant::now() < deadline => {
                    println!("ping: {e}");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(e) => return Err(ServerError::Network(e.to_string())),
            }
        };
        println!("ping: {ping:?}");
        assert!(ping.version.is_some(), "{ping:?}");
        // And it is Rustic's engine: its runtime said so in the game's log.
        let log = m.dir(&profile.id).join("logs/latest.log");
        loop {
            let text = std::fs::read_to_string(&log).unwrap_or_default();
            if let Some(line) = text.lines().find(|l| l.contains("[rustic] Rustic ready")) {
                println!("log: {line}");
                break;
            }
            if Instant::now() > deadline {
                return Err(ServerError::Supervisor(format!(
                    "no \"Rustic ready\" line in {}",
                    log.display()
                )));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }
    .await;

    let stopped = m.stop_server(&profile.id).await;
    let log =
        std::fs::read_to_string(m.dir(&profile.id).join(".rustic/start.log")).unwrap_or_default();
    if result.is_err() {
        println!("start.log:\n{log}");
    }
    m.delete(&profile.id, false).await.unwrap();
    result.unwrap();
    stopped.unwrap();
    assert!(!m.dir(&profile.id).exists());
}
