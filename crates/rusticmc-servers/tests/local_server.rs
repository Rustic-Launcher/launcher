//! A local server's whole life against a fake `horizon-server` that follows its CLI and API.

use rusticmc_instances::BuildManager;
use rusticmc_launcher_core::DownloadManager;
use rusticmc_servers::control::PlayerAction;
use rusticmc_servers::engine::{self, EngineOrigin, EngineSearch};
use rusticmc_servers::hosting::{CreateServer, ServerKind};
use rusticmc_servers::{ServerError, ServerManager};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FAKE: &str = env!("CARGO_BIN_EXE_rusticmc-fake-horizon-server");

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

fn manager(root: &Path) -> ServerManager {
    let rustic = root.join(exe("rustic-server"));
    std::fs::write(&rustic, b"").unwrap();
    manager_with(root, Some(rustic))
}

/// A manager with build `b1` (fake supervisor) and `configured` as the Settings override.
fn manager_with(root: &Path, configured: Option<PathBuf>) -> ServerManager {
    let build = root.join("builds/b1");
    std::fs::create_dir_all(&build).unwrap();
    std::fs::copy(FAKE, build.join(exe("horizon-server"))).unwrap();
    std::fs::copy(FAKE, build.join(exe("horizon-client"))).unwrap();
    let http = rusticmc_launcher_core::http::client().unwrap();
    let builds = BuildManager::new(
        root.join("builds"),
        http.clone(),
        DownloadManager::new(root.join("cache"), 1, 0).unwrap(),
    );
    builds.register_local("b1", &build).unwrap();
    // The prepared files' command tree, for completion.
    std::fs::create_dir_all(root.join("prepared/b1")).unwrap();
    std::fs::write(
        root.join("prepared/b1/commands.json"),
        r#"{"type":"root","children":{"kick":{"type":"literal","children":{"targets":{"type":"argument","parser":"minecraft:entity","executable":true}}},"kill":{"type":"literal","executable":true}}}"#,
    )
    .unwrap();
    ServerManager::new(
        root.join("servers"),
        http,
        builds,
        root.join("prepared"),
        configured,
    )
}

fn request(kind: ServerKind) -> CreateServer {
    CreateServer {
        name: "Weekend SMP".into(),
        kind,
        build: "b1".into(),
        eula_accepted: true,
        world: Some("survival".into()),
        seed: None,
        port: Some(25570),
        whitelist: true,
        players: vec!["Alex".into()],
        op: Some("Alex".into()),
        online_mode: true,
        engine_path: None,
    }
}

#[tokio::test]
async fn the_eula_comes_first() {
    let t = tempfile::tempdir().unwrap();
    let m = manager(t.path());
    let err = m
        .create(CreateServer {
            eula_accepted: false,
            ..request(ServerKind::Horizon)
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ServerError::EulaNotAccepted));
    assert!(m.profiles().is_empty());
}

fn engine_path(m: &ServerManager, id: &str) -> String {
    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(m.dir(id).join("fake-settings.json")).unwrap(),
    )
    .unwrap();
    settings["engine.path"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_rustic_edition_build_brings_its_engine() {
    // Regression (2026-11-12): "Create a server" failed with "rustic-server was not found" while
    // the default build was a Rustic edition one; the build folder was never looked at.
    let t = tempfile::tempdir().unwrap();
    let m = manager_with(t.path(), None);
    let shipped = t.path().join("builds/b1").join(exe("rustic-server"));
    std::fs::copy(FAKE, &shipped).unwrap();

    let found = m.rustic_server_for_build(Some("b1")).unwrap();
    assert_eq!(found.origin, EngineOrigin::Build);
    assert_eq!(found.path, shipped);
    let p = m.create(request(ServerKind::RusticServer)).await.unwrap();
    assert_eq!(PathBuf::from(engine_path(&m, &p.id)), shipped);
    assert_eq!(m.rustic_server_for(&p).unwrap().origin, EngineOrigin::Build);
    // The program answers the probe Settings uses to check a chosen file.
    let line = engine::rustic_server_version(&shipped).await.unwrap();
    assert!(line.starts_with("Rustic "), "{line}");

    // A manual override wins, at once (no restart); clearing it goes back to the build's.
    let chosen = t.path().join("elsewhere").join(exe("rustic-server"));
    std::fs::create_dir_all(chosen.parent().unwrap()).unwrap();
    std::fs::copy(FAKE, &chosen).unwrap();
    m.set_rustic_server(Some(chosen.clone()));
    assert_eq!(m.configured_rustic_server(), Some(chosen.clone()));
    let found = m.rustic_server_for(&p).unwrap();
    assert_eq!(
        (found.origin, found.path),
        (EngineOrigin::Configured, chosen)
    );
    m.set_rustic_server(None);
    assert_eq!(m.rustic_server_for(&p).unwrap().origin, EngineOrigin::Build);
}

#[tokio::test]
async fn no_engine_anywhere_is_a_clear_error() {
    if engine::find_rustic_server(EngineSearch::default()).is_some() {
        eprintln!("a rustic-server is on this computer's PATH or beside the test: skipped");
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let m = manager_with(t.path(), None);
    let err = m
        .create(request(ServerKind::RusticServer))
        .await
        .unwrap_err();
    assert!(matches!(err, ServerError::NoRusticServer), "{err}");
    assert_eq!(err.code(), "server.no_rustic_server");
    assert!(
        err.to_string()
            .contains("Settings > Game and instances > Rustic Server")
    );
    // Nothing is left behind, and a Horizon server still works.
    assert!(m.profiles().is_empty());
    assert!(!m.dir("weekend-smp").exists());
    m.create(request(ServerKind::Horizon)).await.unwrap();
}

#[tokio::test]
async fn a_rustic_server_from_creation_to_deletion() {
    let t = tempfile::tempdir().unwrap();
    let m = manager(t.path());
    let p = m.create(request(ServerKind::RusticServer)).await.unwrap();
    assert_eq!(p.id, "weekend-smp");
    let dir = m.dir(&p.id);
    let init = std::fs::read_to_string(dir.join("init-args.txt")).unwrap();
    assert!(init.contains("--eula") && init.contains("--port 25570") && init.contains("--op Alex"));
    // The engine is rustic-server, the resources the build's prepared folder.
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("fake-settings.json")).unwrap())
            .unwrap();
    assert!(
        settings["engine.path"]
            .as_str()
            .unwrap()
            .ends_with(&exe("rustic-server"))
    );
    assert!(
        settings["engine.resources"]
            .as_str()
            .unwrap()
            .ends_with("b1")
    );
    assert!(m.plugins(&p.id).is_ok());

    // Stopped: backups by the CLI, no live state.
    assert!(!m.state(&p.id).await.unwrap().running);
    let name = m.backup(&p.id, Some("first".into())).await.unwrap();
    assert!(name.ends_with("-first.zip"));
    assert_eq!(m.backups(&p.id).await.unwrap().len(), 1);

    m.start(&p.id).await.unwrap();
    let state = m.state(&p.id).await.unwrap();
    assert!(state.running);
    assert_eq!(state.metrics.tps, Some(20.0));
    assert_eq!(state.metrics.players_online, Some(1));
    assert_eq!(m.command(&p.id, "/say hi").await.unwrap(), ["ran: say hi"]);
    let r = m
        .player_action(
            &p.id,
            PlayerAction::Ban {
                name: "Griefer".into(),
                reason: Some("griefing".into()),
            },
        )
        .await
        .unwrap();
    assert!(r.changed);
    assert_eq!(
        m.players(&p.id).await.unwrap()["online"][0]["name"],
        "Steve"
    );
    let made = m.backup(&p.id, None).await.unwrap();
    assert_eq!(m.backups(&p.id).await.unwrap().len(), 2);
    m.delete_backup(&p.id, &made).await.unwrap();
    m.restore(&p.id, &name, None).await.unwrap();
    assert_eq!(m.metrics_history(&p.id, None).await.unwrap().len(), 1);

    // Completion: the tree, and online players for entity arguments.
    let c = m.complete(&p.id, "kick St").await.unwrap();
    assert_eq!(c[0].text, "Steve");
    let c = m.complete(&p.id, "ki").await.unwrap();
    assert_eq!(c.len(), 2);

    // Events as server-sent events.
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    m.stream_events(&p.id, None, tx).unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!((first.id, first.kind.as_str()), (1, "log"));
    let second = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.data["name"], "Steve");
    m.stop_events(&p.id);

    // engine.* stays the launcher's.
    assert!(
        m.set_config(&p.id, vec![("engine.path".into(), "x".into())])
            .await
            .is_err()
    );
    m.set_config(&p.id, vec![("world.difficulty".into(), "hard".into())])
        .await
        .unwrap();

    m.stop(&p.id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!dir.join(".horizon/api.json").exists());
    assert!(!m.state(&p.id).await.unwrap().running);

    let invite = m.invite_code(&p.id);
    // A machine with no network interface at all has nothing to share; otherwise a code.
    if let Ok(code) = invite {
        let decoded = rusticmc_servers::invite::Invite::decode(&code).unwrap();
        assert_eq!(decoded.port, 25570);
    }

    let listed = m.list().await.unwrap();
    assert_eq!(listed.len(), 1);
    m.delete(&p.id, false).await.unwrap();
    assert!(!dir.exists());
}

#[tokio::test]
async fn the_hosting_trait_drives_local_servers() {
    use rusticmc_hosting::HostingProvider;
    use rusticmc_hosting::model::{EngineSource, NoProgress, PlayerAction as HAction, ServerSpec};
    let t = tempfile::tempdir().unwrap();
    let manager = std::sync::Arc::new(manager(t.path()));
    let local = rusticmc_servers::LocalServers {
        manager: manager.clone(),
        default_build: std::sync::Arc::new(|| Some("b1".to_owned())),
    };
    let mut settings = std::collections::BTreeMap::new();
    settings.insert("world.difficulty".to_owned(), "hard".to_owned());
    let r = local
        .install(
            &ServerSpec {
                name: "trait-server".into(),
                port: 25571,
                world: "world".into(),
                seed: None,
                whitelist: true,
                players: vec!["Alex".into()],
                op: Some("Alex".into()),
                online_mode: true,
                eula_accepted: true,
                engine: EngineSource::Horizon,
                prepared_resources: None,
                settings,
            },
            &NoProgress,
        )
        .await
        .unwrap();
    assert_eq!(r.name, "trait-server");
    assert_eq!(local.status(&r.name).await.unwrap().phase, "stopped");
    local.start(&r.name).await.unwrap();
    let status = local.status(&r.name).await.unwrap();
    assert!(status.is_running());
    assert_eq!(status.players_online, ["Steve"]);
    let out = local
        .player_action(
            &r.name,
            &HAction::Op {
                name: "Steve".into(),
            },
        )
        .await
        .unwrap();
    assert!(!out.output.is_empty());
    assert_eq!(local.players(&r.name).await.unwrap().online.len(), 1);
    let name = local.backup_create(&r.name, Some("trait")).await.unwrap();
    assert!(
        local
            .backups_list(&r.name)
            .await
            .unwrap()
            .iter()
            .any(|b| b.name == name)
    );
    local.stop(&r.name).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    local
        .uninstall(&r.name, &Default::default(), &NoProgress)
        .await
        .unwrap();
    assert!(manager.profiles().is_empty() || !manager.dir(&r.name).join(".rustic").exists());
}
