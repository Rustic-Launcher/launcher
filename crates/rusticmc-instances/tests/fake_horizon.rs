//! Builds, preparation and launches against a fake Horizon that follows the Launch API.

use rusticmc_auth::GameSession;
use rusticmc_instances::*;
use rusticmc_launcher_core::config::{BuildSourceConfig, BuildSourceKind};
use rusticmc_launcher_core::{DownloadManager, Secret};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FAKE: &str = env!("CARGO_BIN_EXE_rusticmc-fake-horizon");

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// A build folder with the fake as horizon-client and horizon-prepare.
fn fake_build(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    for name in ["horizon-client", "horizon-prepare"] {
        std::fs::copy(FAKE, dir.join(exe(name))).unwrap();
    }
    std::fs::write(dir.join("research-lock.json"), "{}").unwrap();
}

struct Env {
    _t: tempfile::TempDir,
    root: PathBuf,
    manager: InstanceManager,
}

fn env() -> Env {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().to_path_buf();
    let downloads = DownloadManager::new(root.join("cache"), 2, 0).unwrap();
    let builds = BuildManager::new(
        root.join("builds"),
        rusticmc_launcher_core::http::client().unwrap(),
        downloads,
    );
    fake_build(&root.join("builds/test-1"));
    builds
        .register_local("test-1", &root.join("builds/test-1"))
        .unwrap();
    let manager = InstanceManager::new(
        InstanceStore::new(root.join("instances")),
        builds,
        root.join("prepared"),
    );
    Env {
        _t: t,
        root,
        manager,
    }
}

fn session() -> GameSession {
    GameSession {
        name: "Alex".into(),
        uuid: "069a79f444e94726a5befca90e38aaf5".into(),
        access_token: Secret::new("very-secret-token"),
        xuid: Some("2535".into()),
    }
}

fn instance(e: &Env) -> Instance {
    e.manager
        .store
        .create(
            "Test",
            BuildChoice {
                source: "local".into(),
                version: Some("test-1".into()),
            },
        )
        .unwrap()
}

async fn prepared(e: &Env) {
    e.manager
        .prepare_build("test-1", GameFiles::default(), None, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn probes_both_programs() {
    let e = env();
    let build = e.manager.builds.get("test-1").unwrap();
    let client = e.manager.client_api(&build).await.unwrap();
    assert!(client.accepts("--session-stdin"));
    assert_eq!(
        client.crash_report_line.as_deref(),
        Some("Horizon: crash report saved to ")
    );
    let prep = e.manager.preparer_api(&build).await.unwrap();
    assert_eq!(prep.java_major, Some(25));
    assert_eq!(prep.minecraft.as_deref(), Some("26.3"));
}

#[tokio::test]
async fn preparation_reports_progress_and_check_follows() {
    let e = env();
    assert!(!e.manager.check_prepared("test-1").await.unwrap().current);
    let mut events = e.manager.subscribe_prepare();
    let java = e.root.join("jdk-25");
    e.manager
        .prepare_build(
            "test-1",
            GameFiles {
                minecraft_dir: Some(e.root.join(".minecraft")),
                ..GameFiles::default()
            },
            Some(e.root.join("server.jar")),
            Some(java.clone()),
        )
        .await
        .unwrap();
    let mut fractions = Vec::new();
    let mut messages = Vec::new();
    let mut logs = 0;
    while let Ok(p) = events.try_recv() {
        assert_eq!(p.build, "test-1");
        match p.event {
            PrepareEvent::Fraction { value } => fractions.push(value),
            PrepareEvent::Message { text } => messages.push(text),
            PrepareEvent::Log { .. } => logs += 1,
            PrepareEvent::Error { text } => panic!("unexpected error {text}"),
        }
    }
    assert_eq!(fractions, [0.1, 0.6, 1.0]);
    assert_eq!(
        messages,
        ["Checking your Minecraft files", "Exporting registries"]
    );
    assert_eq!(logs, 1);
    // --java-home reached the preparer.
    let ready = std::fs::read_to_string(e.root.join("prepared/test-1/ready")).unwrap();
    assert!(ready.contains("jdk-25"), "{ready}");
    assert!(e.manager.check_prepared("test-1").await.unwrap().current);
}

#[tokio::test]
async fn a_failed_preparation_says_why() {
    let e = env();
    let err = e
        .manager
        .prepare_build(
            "test-1",
            GameFiles {
                minecraft_dir: Some(e.root.join("fail-here")),
                ..GameFiles::default()
            },
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("pinned checksum"), "{err}");
}

#[tokio::test]
async fn launch_hands_the_session_over_stdin() {
    let e = env();
    prepared(&e).await;
    let i = instance(&e);
    let mut events = e.manager.subscribe();
    e.manager
        .launch(&i.id, LaunchIdentity::Online(session()))
        .await
        .unwrap();
    let exited = wait_exit(&mut events).await;
    assert!(matches!(
        exited,
        GameEvent::Exited {
            outcome: ExitOutcome::Quit,
            ..
        }
    ));
    let game = e.manager.store.game_dir(&i.id);
    let received = std::fs::read_to_string(game.join("received-session.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&received).unwrap();
    assert_eq!(v["access_token"], "very-secret-token");
    assert_eq!(v["user_type"], "msa");
    assert_eq!(v["xuid"], "2535");
    let argv = std::fs::read_to_string(game.join("argv.txt")).unwrap();
    assert!(argv.contains("--session-stdin"));
    assert!(
        !argv.contains("very-secret-token"),
        "the token is never an argument"
    );
    // Playtime recorded.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(e.manager.store.get(&i.id).unwrap().last_played_at.is_some());
    assert!(e.manager.running().is_empty());
}

#[tokio::test]
async fn offline_starts_without_a_session() {
    let e = env();
    prepared(&e).await;
    let i = instance(&e);
    let mut events = e.manager.subscribe();
    e.manager
        .launch(&i.id, LaunchIdentity::Offline)
        .await
        .unwrap();
    wait_exit(&mut events).await;
    let game = e.manager.store.game_dir(&i.id);
    assert!(!game.join("received-session.json").exists());
    let argv = std::fs::read_to_string(game.join("argv.txt")).unwrap();
    assert!(!argv.contains("--session"));
}

#[tokio::test]
async fn an_unprepared_build_is_not_started() {
    let e = env();
    let i = instance(&e);
    assert!(matches!(
        e.manager.launch(&i.id, LaunchIdentity::Offline).await,
        Err(InstanceError::NotPrepared)
    ));
}

#[tokio::test]
async fn a_crash_names_its_report() {
    let e = env();
    prepared(&e).await;
    let i = instance(&e);
    std::fs::write(e.manager.store.game_dir(&i.id).join("crash-me"), "").unwrap();
    let mut events = e.manager.subscribe();
    e.manager
        .launch(&i.id, LaunchIdentity::Offline)
        .await
        .unwrap();
    match wait_exit(&mut events).await {
        GameEvent::Exited {
            outcome: ExitOutcome::Crashed { code: 1 },
            crash_report: Some(report),
            ..
        } => assert!(
            report.ends_with("crash-2026-11-10_12.00.00-client.txt"),
            "{report:?}"
        ),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_running_game_can_be_stopped() {
    let e = env();
    prepared(&e).await;
    let i = instance(&e);
    std::fs::write(e.manager.store.game_dir(&i.id).join("hang"), "").unwrap();
    let mut events = e.manager.subscribe();
    e.manager
        .launch(&i.id, LaunchIdentity::Offline)
        .await
        .unwrap();
    assert!(matches!(
        e.manager.launch(&i.id, LaunchIdentity::Offline).await,
        Err(InstanceError::AlreadyRunning)
    ));
    tokio::time::sleep(Duration::from_millis(300)).await;
    e.manager.kill(&i.id).await.unwrap();
    let exited = tokio::time::timeout(Duration::from_secs(10), wait_exit(&mut events))
        .await
        .expect("killed promptly");
    assert!(matches!(
        exited,
        GameEvent::Exited {
            outcome: ExitOutcome::Killed,
            ..
        }
    ));
}

async fn wait_exit(events: &mut tokio::sync::broadcast::Receiver<InstanceGameEvent>) -> GameEvent {
    loop {
        let e = tokio::time::timeout(Duration::from_secs(30), events.recv())
            .await
            .expect("an exit")
            .unwrap();
        if matches!(e.event, GameEvent::Exited { .. }) {
            return e.event;
        }
    }
}

// ---- signed installs ----------------------------------------------------------------------

fn zip_build(path: &Path) {
    use std::io::Write;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zip.add_directory("horizon-test/", opts).unwrap();
    zip.start_file(format!("horizon-test/{}", exe("horizon-client")), opts)
        .unwrap();
    zip.write_all(&std::fs::read(FAKE).unwrap()).unwrap();
    zip.start_file("horizon-test/research-lock.json", opts)
        .unwrap();
    zip.write_all(b"{}").unwrap();
    zip.finish().unwrap();
}

#[tokio::test]
async fn installs_a_signed_nightly_and_refuses_a_forged_one() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let Some(suffix) = builds::platform_suffix() else {
        return;
    };
    let server = MockServer::start().await;
    let t = tempfile::tempdir().unwrap();
    let archive_name = format!("horizon-{suffix}");
    // A .zip archive stands in on every platform (the suffix decides the unpacker, and Linux's
    // is .tar.gz): only test where the platform archive is a zip.
    if !archive_name.ends_with(".zip") {
        return;
    }
    let archive = t.path().join(&archive_name);
    zip_build(&archive);
    let bytes = std::fs::read(&archive).unwrap();
    let sha = rusticmc_launcher_core::download::hash_bytes(
        &bytes,
        rusticmc_launcher_core::HashAlgo::Sha256,
    );
    let sums = format!("{sha}  {archive_name}\n");
    let pkcs8 =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let public = {
        use base64::Engine;
        use ring::signature::KeyPair;
        base64::engine::general_purpose::STANDARD.encode(pair.public_key().as_ref())
    };
    let version = "nightly-2026-11-01-abcdef1";
    let sig = signing::sign_with(&pair, "horizon release", version, sums.as_bytes());
    let base = server.uri();
    Mock::given(method("GET"))
        .and(path("/repos/test/horizon/releases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"tag_name": "nightly", "name": "Nightly 2026-11-01 · abcdef1", "published_at": "2026-11-01T00:00:00Z", "prerelease": true,
             "assets": [
                {"name": archive_name, "browser_download_url": format!("{base}/dl/archive"), "size": bytes.len()},
                {"name": "SHA256SUMS", "browser_download_url": format!("{base}/dl/sums")},
                {"name": "SHA256SUMS.v2.sig", "browser_download_url": format!("{base}/dl/sig")}
             ]}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dl/archive"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dl/sums"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sums.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dl/sig"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!("{sig}\n")))
        .mount(&server)
        .await;
    let config = BuildSourceConfig {
        id: "test".into(),
        name: "Test".into(),
        kind: BuildSourceKind::Github,
        repo: Some("test/horizon".into()),
        channel_url: None,
        asset_prefix: "horizon".into(),
        signature_namespace: "horizon release".into(),
        public_keys: vec![public],
        builtin_horizon_keys: false,
    };
    let mut source = BuildSource::new(config.clone()).unwrap();
    source.github_api = base.clone();
    let downloads = DownloadManager::new(t.path().join("cache"), 2, 0).unwrap();
    let builds = BuildManager::new(
        t.path().join("builds"),
        rusticmc_launcher_core::http::client().unwrap(),
        downloads,
    );
    let releases = builds.releases(&source).await.unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].version, version);
    let stages = std::sync::Mutex::new(Vec::new());
    let installed = builds
        .install(&source, &releases[0], |s| stages.lock().unwrap().push(s))
        .await
        .unwrap();
    assert!(installed.client().is_file());
    assert_eq!(installed.archive_sha256, sha);
    assert!(matches!(
        stages.lock().unwrap().last(),
        Some(InstallStage::Done { .. })
    ));
    assert_eq!(builds.installed().len(), 1);
    // Another key: refused before anything is downloaded.
    let other =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let other = ring::signature::Ed25519KeyPair::from_pkcs8(other.as_ref()).unwrap();
    let other_public = {
        use base64::Engine;
        use ring::signature::KeyPair;
        base64::engine::general_purpose::STANDARD.encode(other.public_key().as_ref())
    };
    let mut wrong = BuildSource::new(BuildSourceConfig {
        public_keys: vec![other_public],
        ..config
    })
    .unwrap();
    wrong.github_api = base;
    builds.remove(version).unwrap();
    let err = builds
        .install(&wrong, &releases[0], |_| {})
        .await
        .unwrap_err();
    assert!(matches!(err, InstanceError::Signature(_)), "{err}");
    assert!(builds.installed().is_empty());
}
