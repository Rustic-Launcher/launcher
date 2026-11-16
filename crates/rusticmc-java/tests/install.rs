//! The installer end to end against a local server standing in for Mojang's and Fabric's hosts
//! (every file here is written by the test: no Mojang file is committed).

use rusticmc_java::install::Installer;
use rusticmc_java::launch::{self, Auth, LaunchOptions};
use rusticmc_java::loaders::{self, LoaderKind};
use rusticmc_java::platform::{Arch, Os, Platform};
use rusticmc_java::rules::Features;
use rusticmc_java::{Hosts, JavaError};
use rusticmc_launcher_core::download::hash_bytes;
use rusticmc_launcher_core::{CancellationToken, DownloadManager, HashAlgo, Secret};
use serde_json::json;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sha1(b: &[u8]) -> String {
    hash_bytes(b, HashAlgo::Sha1)
}

struct Fixture {
    server: MockServer,
    installer: Installer,
    _tmp: tempfile::TempDir,
}

async fn fixture() -> Fixture {
    rusticmc_launcher_core::http::install_crypto_provider();
    let server = MockServer::start().await;
    let base = server.uri();
    let client = b"client jar bytes".to_vec();
    let lib = b"library bytes".to_vec();
    let object = b"a sound".to_vec();
    let log_cfg = b"<Configuration/>".to_vec();
    let index = serde_json::to_vec(&json!({"objects": {"minecraft/sounds/a.ogg": {"hash": sha1(&object), "size": object.len()}}})).unwrap();
    let version = serde_json::to_vec(&json!({
        "id": "9.2", "type": "release", "mainClass": "net.minecraft.client.main.Main",
        "javaVersion": {"component": "java-runtime-delta", "majorVersion": 21},
        "downloads": {"client": {"url": format!("{base}/client.jar"), "sha1": sha1(&client), "size": client.len()}},
        "assetIndex": {"id": "30", "url": format!("{base}/index.json"), "sha1": sha1(&index), "size": index.len()},
        "logging": {"client": {"argument": "-Dlog4j.configurationFile=${path}", "type": "log4j2-xml",
            "file": {"id": "client-test.xml", "url": format!("{base}/log.xml"), "sha1": sha1(&log_cfg), "size": log_cfg.len()}}},
        "arguments": {"game": ["--username", "${auth_player_name}", "--accessToken", "${auth_access_token}"],
            "jvm": ["-Djava.library.path=${natives_directory}", "-cp", "${classpath}"]},
        "libraries": [
            {"name": "com.example:lib:1.0", "downloads": {"artifact": {"path": "com/example/lib/1.0/lib-1.0.jar",
                "url": format!("{base}/lib.jar"), "sha1": sha1(&lib), "size": lib.len()}}},
            {"name": "com.example:mac-only:1.0", "rules": [{"action": "allow", "os": {"name": "osx"}}],
             "downloads": {"artifact": {"path": "com/example/mac-only/1.0/mac-only-1.0.jar",
                "url": format!("{base}/missing.jar"), "sha1": sha1(b"x"), "size": 1}}}
        ]
    }))
    .unwrap();
    let manifest = json!({"latest": {"release": "9.2", "snapshot": "9.2"}, "versions": [
        {"id": "9.2", "type": "release", "url": format!("{base}/9.2.json"), "time": "", "releaseTime": "", "sha1": sha1(&version)}
    ]});
    let fabric_lib = b"fabric loader".to_vec();
    let fabric_profile = json!({
        "id": "fabric-loader-0.1.0-9.2", "inheritsFrom": "9.2", "type": "release",
        "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
        "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
        "libraries": [{"name": "net.fabricmc:fabric-loader:0.1.0", "url": format!("{base}/maven/"), "sha1": sha1(&fabric_lib), "size": fabric_lib.len()}]
    });
    let route = |p: &str, body: Vec<u8>| {
        Mock::given(method("GET"))
            .and(path(p.to_owned()))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
    };
    for (p, body) in [
        ("/manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("/9.2.json", version),
        ("/client.jar", client),
        ("/lib.jar", lib),
        ("/index.json", index),
        ("/log.xml", log_cfg),
        (
            &format!("/objects/{}/{}", &sha1(&object)[..2], sha1(&object)) as &str,
            object,
        ),
        (
            "/fabric/v2/versions/loader/9.2",
            serde_json::to_vec(&json!([{"loader": {"version": "0.1.0", "stable": true}}])).unwrap(),
        ),
        (
            "/fabric/v2/versions/loader/9.2/0.1.0/profile/json",
            serde_json::to_vec(&fabric_profile).unwrap(),
        ),
        (
            "/maven/net/fabricmc/fabric-loader/0.1.0/fabric-loader-0.1.0.jar",
            fabric_lib,
        ),
    ] {
        route(p, body).mount(&server).await;
    }
    let tmp = tempfile::tempdir().unwrap();
    let http = rusticmc_launcher_core::http::client().unwrap();
    let installer = Installer {
        root: tmp.path().join("game"),
        downloads: DownloadManager::with_client(http.clone(), tmp.path().join("cache"), 4, 0),
        http,
        hosts: Hosts {
            version_manifest: format!("{base}/manifest.json"),
            runtime_manifest: format!("{base}/runtimes.json"),
            resources: format!("{base}/objects"),
            libraries: format!("{base}/libraries"),
            fabric_meta: format!("{base}/fabric"),
            quilt_meta: format!("{base}/quilt"),
            forge_maven: format!("{base}/forge"),
            neoforge_maven: format!("{base}/neoforge"),
        },
        extra_hosts: vec![base.clone()],
        platform: Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
            os_version: None,
        },
    };
    Fixture {
        server,
        installer,
        _tmp: tmp,
    }
}

#[tokio::test]
async fn installs_a_version_then_a_fabric_profile_over_it() {
    let f = fixture().await;
    let inst = &f.installer;
    let cancel = CancellationToken::new();
    let m = inst.manifest().await.unwrap();
    assert_eq!(m.latest.release, "9.2");

    let vanilla = inst
        .install("9.2", &Features::none(), &cancel, &|_, _, _| {})
        .await
        .unwrap();
    let root = &inst.root;
    assert!(root.join("versions/9.2/9.2.json").is_file());
    assert_eq!(
        std::fs::read(root.join("versions/9.2/9.2.jar")).unwrap(),
        b"client jar bytes"
    );
    assert!(
        root.join("libraries/com/example/lib/1.0/lib-1.0.jar")
            .is_file()
    );
    // Not for this system: never fetched.
    assert!(!root.join("libraries/com/example/mac-only").exists());
    assert!(root.join("assets/indexes/30.json").is_file());
    assert!(root.join("assets/log_configs/client-test.xml").is_file());
    let objects: Vec<_> = walk(&root.join("assets/objects"));
    assert_eq!(objects.len(), 1);
    assert_eq!(vanilla.version.runtime_component().0, "java-runtime-delta");

    // Fabric: its profile inherits from the game; its jar is a copy named after the profile.
    let list = loaders::fabric::versions(inst, LoaderKind::Fabric, "9.2")
        .await
        .unwrap();
    assert_eq!(list[0].version, "0.1.0");
    let id = loaders::fabric::install(inst, LoaderKind::Fabric, "9.2", "0.1.0")
        .await
        .unwrap();
    assert_eq!(id, "fabric-loader-0.1.0-9.2");
    let fabric = inst
        .install(&id, &Features::none(), &cancel, &|_, _, _| {})
        .await
        .unwrap();
    assert_eq!(
        fabric.client_jar.as_deref(),
        Some(
            PathBuf::from("versions/fabric-loader-0.1.0-9.2/fabric-loader-0.1.0-9.2.jar").as_path()
        )
    );
    assert!(root.join(fabric.client_jar.as_ref().unwrap()).is_file());
    assert_eq!(fabric.libraries.classpath.len(), 2);

    let game_dir = root.join("profiles/p");
    let natives = root.join("natives/p");
    launch::prepare(&fabric, root, &natives, &game_dir).unwrap();
    let cmd = launch::build(
        &fabric,
        root,
        &natives,
        &Auth {
            name: "Alex".into(),
            uuid: "0".repeat(32),
            access_token: Secret::new("TOKEN"),
            xuid: None,
        },
        &LaunchOptions {
            game_dir,
            java: PathBuf::from("java"),
            java_major: Some(21),
            memory_min_mb: 512,
            memory_max_mb: 2048,
            resolution: None,
            fullscreen: false,
            jvm_args: vec![],
            launcher_name: "rustic-launcher".into(),
            launcher_version: "test".into(),
            quick_play_server: None,
        },
        &inst.platform,
    )
    .unwrap();
    assert!(
        cmd.args
            .contains(&"net.fabricmc.loader.impl.launch.knot.KnotClient".to_owned())
    );
    assert!(
        cmd.args
            .iter()
            .any(|a| a.starts_with("-Dlog4j.configurationFile="))
    );
    assert!(!cmd.display().contains("TOKEN"));

    // Everything is there: a second install fetches nothing at all (works offline).
    let before = f.server.received_requests().await.unwrap().len();
    inst.install(&id, &Features::none(), &cancel, &|_, _, _| {})
        .await
        .unwrap();
    let after = f.server.received_requests().await.unwrap();
    let fetched: Vec<_> = after[before..]
        .iter()
        .map(|r| r.url.path().to_owned())
        .collect();
    assert!(fetched.is_empty(), "{fetched:?}");
}

#[tokio::test]
async fn refuses_files_from_other_hosts_and_bad_checksums() {
    let f = fixture().await;
    let inst = &f.installer;
    // A version file naming a library elsewhere is refused before any download.
    std::fs::create_dir_all(inst.root.join("versions/evil")).unwrap();
    std::fs::write(
        inst.root.join("versions/evil/evil.json"),
        serde_json::to_vec(&json!({"id": "evil", "mainClass": "x", "libraries": [
            {"name": "a:b:1", "downloads": {"artifact": {"path": "a/b/1/b-1.jar", "url": "https://example.com/b.jar", "sha1": "0000000000000000000000000000000000000000"}}}
        ]}))
        .unwrap(),
    )
    .unwrap();
    let err = inst
        .install(
            "evil",
            &Features::none(),
            &CancellationToken::new(),
            &|_, _, _| {},
        )
        .await
        .unwrap_err();
    assert!(matches!(err, JavaError::HostNotAllowed(_)), "{err}");

    // A file whose sha1 doesn't match is discarded.
    Mock::given(method("GET"))
        .and(path("/bad.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"tampered".to_vec()))
        .mount(&f.server)
        .await;
    std::fs::create_dir_all(inst.root.join("versions/bad")).unwrap();
    std::fs::write(
        inst.root.join("versions/bad/bad.json"),
        serde_json::to_vec(&json!({"id": "bad", "mainClass": "x", "libraries": [
            {"name": "a:c:1", "downloads": {"artifact": {"path": "a/c/1/c-1.jar", "url": format!("{}/bad.jar", f.server.uri()), "sha1": sha1(b"original")}}}
        ]}))
        .unwrap(),
    )
    .unwrap();
    let err = inst
        .install(
            "bad",
            &Features::none(),
            &CancellationToken::new(),
            &|_, _, _| {},
        )
        .await
        .unwrap_err();
    assert!(matches!(err, JavaError::Download(_)), "{err}");
    assert!(!inst.root.join("libraries/a/c/1/c-1.jar").exists());
}

fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
