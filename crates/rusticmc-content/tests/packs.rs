//! `.mrpack` import, Rustic pack export/import, Modrinth update checks.
mod common;
use common::*;
use rusticmc_content::download::{MemoryDownloader, digest};
use rusticmc_content::fairplay::FairPlayPolicy;
use rusticmc_content::install::{InstallOptions, Installer};
use rusticmc_content::ledger::Ledger;
use rusticmc_content::model::*;
use rusticmc_content::mrpack::{self, Action, ImportOptions, PortPack};
use rusticmc_content::rustic_pack::{self, ExportOptions};
use rusticmc_content::sources::modrinth::{Modrinth, ModrinthConfig};
use rusticmc_content::{HorizonEnv, switches, updates};
use std::path::Path;
use std::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CDN: &str = "https://cdn.modrinth.com/data";

fn mr_file(path: &str, url: &str, bytes: &[u8], env: Option<&str>) -> String {
    let d = digest(bytes);
    let env = env
        .map(|c| format!(r#", "env": {{ "client": "{c}", "server": "required" }}"#))
        .unwrap_or_default();
    format!(
        r#"{{ "path": "{path}", "hashes": {{ "sha1": "{}", "sha512": "{}" }}, "downloads": ["{url}"], "fileSize": {}{env} }}"#,
        d.sha1.unwrap(),
        d.sha512.unwrap(),
        bytes.len()
    )
}

struct FakePort(Mutex<Vec<String>>);

impl PortPack for FakePort {
    fn port(&self, pack: &Path, _work: &Path) -> rusticmc_content::Result<String> {
        self.0.lock().unwrap().push(pack.display().to_string());
        Ok("ported 1 content mod".into())
    }
}

#[tokio::test]
async fn mrpack_imports_the_horizon_compatible_part_with_a_report() {
    let dir = tempfile::tempdir().unwrap();
    let rp = resource_pack(34);
    let shader = zip(&[("shaders/composite.fsh", b"void main(){}")]);
    let dp = data_pack();
    let files = [
        mr_file(
            "mods/sodium.jar",
            &format!("{CDN}/a/sodium.jar"),
            b"jar",
            None,
        ),
        mr_file(
            "resourcepacks/Faithful.zip",
            &format!("{CDN}/b/Faithful.zip"),
            &rp,
            None,
        ),
        mr_file(
            "shaderpacks/BSL.zip",
            &format!("{CDN}/c/BSL.zip"),
            &shader,
            None,
        ),
        mr_file(
            "datapacks/Trades.zip",
            &format!("{CDN}/d/Trades.zip"),
            &dp,
            None,
        ),
        mr_file(
            "mods/server-only.jar",
            &format!("{CDN}/e/s.jar"),
            b"s",
            Some("unsupported"),
        ),
        mr_file("../escape.zip", &format!("{CDN}/f/x.zip"), b"x", None),
        mr_file(
            "resourcepacks/Evil.zip",
            "https://evil.example/e.zip",
            b"e",
            None,
        ),
    ];
    let index = format!(
        r#"{{ "formatVersion": 1, "game": "minecraft", "versionId": "1.4.0", "name": "Cozy Fabric",
           "files": [{}], "dependencies": {{ "minecraft": "1.21.1", "fabric-loader": "0.16.0" }} }}"#,
        files.join(",")
    );
    let pack_path = dir.path().join("cozy.mrpack");
    std::fs::write(
        &pack_path,
        zip(&[
            ("modrinth.index.json", index.as_bytes()),
            ("overrides/config/sodium.json", b"{}"),
            ("overrides/options.txt", b"fov:1\n"),
            ("overrides/resourcepacks/Local.zip", b"common"),
            ("client-overrides/resourcepacks/Local.zip", b"client wins"),
            ("overrides/kubejs/startup.js", b"//"),
            ("server-overrides/server.properties", b"motd=x"),
        ]),
    )
    .unwrap();
    let dl = MemoryDownloader::default()
        .with(format!("{CDN}/b/Faithful.zip"), rp)
        .with(format!("{CDN}/c/BSL.zip"), shader)
        .with(format!("{CDN}/d/Trades.zip"), dp);
    let game = dir.path().join("instance");
    let port = FakePort(Mutex::new(Vec::new()));
    let report = mrpack::import(
        &pack_path,
        &game,
        &dl,
        Some(&port),
        &ImportOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(report.name, "Cozy Fabric");
    assert_eq!(report.loader.as_deref(), Some("fabric-loader"));
    let line = |p: &str| report.lines.iter().find(|l| l.path == p).unwrap().clone();
    assert_eq!(line("mods/sodium.jar").action, Action::NeedsPort);
    assert_eq!(line("resourcepacks/Faithful.zip").action, Action::Imported);
    assert_eq!(line("shaderpacks/BSL.zip").action, Action::ImportedPartly);
    assert_eq!(line("datapacks/Trades.zip").action, Action::ImportedPartly);
    assert_eq!(line("mods/server-only.jar").action, Action::Skipped);
    assert_eq!(line("../escape.zip").action, Action::Failed);
    assert_eq!(line("resourcepacks/Evil.zip").action, Action::Failed);
    assert_eq!(line("config/sodium.json").action, Action::Skipped);
    assert_eq!(line("options.txt").action, Action::Skipped);
    assert_eq!(line("kubejs/startup.js").action, Action::Skipped);
    assert_eq!(line("server.properties").action, Action::Skipped);
    assert!(game.join("resourcepacks/Faithful.zip").exists());
    assert!(game.join("shaderpacks/BSL.zip").exists());
    assert!(game.join("new-world-datapacks/Trades.zip").exists());
    assert!(!game.join("options.txt").exists());
    assert!(!game.join("mods").exists());
    assert_eq!(
        std::fs::read(game.join("resourcepacks/Local.zip")).unwrap(),
        b"client wins"
    );
    assert_eq!(
        report
            .lines
            .iter()
            .filter(|l| l.path == "resourcepacks/Local.zip")
            .count(),
        1
    );
    assert!(
        report
            .port
            .as_deref()
            .unwrap()
            .contains("ported 1 content mod")
    );
    assert_eq!(port.0.lock().unwrap().len(), 1);
    // The older game version is called out.
    assert!(report.lines.iter().any(|l| l.reason.contains("1.21.1")));
}

#[test]
fn not_an_mrpack_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.mrpack");
    std::fs::write(&p, zip(&[("readme.txt", b"hi")])).unwrap();
    assert!(mrpack::read_index(&p).is_err());
}

#[tokio::test]
async fn rustic_pack_round_trip() {
    let src = tempfile::tempdir().unwrap();
    let rp = resource_pack(97);
    let dp = data_pack();
    let rp_url = format!("{CDN}/RP/v1/Faithful.zip");
    let dp_url = format!("{CDN}/DP/v1/Trades.zip");
    let dl = MemoryDownloader::default()
        .with(rp_url.clone(), rp.clone())
        .with(dp_url.clone(), dp.clone());
    let env = HorizonEnv::default();
    let fp = FairPlayPolicy::default();
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    let on = InstallOptions {
        enable: true,
        ..InstallOptions::default()
    };
    inst.install(
        &[modrinth_version(
            "RP",
            ContentKind::ResourcePack,
            &[],
            file(&rp_url, "Faithful.zip", &rp),
        )],
        &instance(src.path()),
        &on,
    )
    .await
    .unwrap();
    let dpv = modrinth_version(
        "DP",
        ContentKind::DataPack,
        &["datapack"],
        file(&dp_url, "Trades.zip", &dp),
    );
    inst.install(
        std::slice::from_ref(&dpv),
        &Target::NewWorlds {
            game_dir: src.path().to_path_buf(),
        },
        &on,
    )
    .await
    .unwrap();
    inst.install(
        &[dpv],
        &Target::World {
            game_dir: src.path().to_path_buf(),
            world: "Old".into(),
        },
        &on,
    )
    .await
    .unwrap();
    std::fs::write(src.path().join("horizon-graphics.txt"), "fancy:true\n").unwrap();
    let out = src.path().join("cozy.rusticpack");
    let report = rustic_pack::export(
        src.path(),
        &env,
        &ExportOptions {
            name: "Cozy".into(),
            version: "1.0.0".into(),
            settings: true,
            ..ExportOptions::default()
        },
        &out,
    )
    .unwrap();
    assert_eq!(report.listed, 2);
    assert!(
        report
            .left_out
            .iter()
            .any(|(_, why)| why.contains("one world"))
    );
    assert!(report.bundled.contains(&"horizon-graphics.txt".to_owned()));
    let manifest = rustic_pack::read(&out).unwrap();
    assert_eq!(manifest.game, "26.3");
    assert!(manifest.content.iter().all(|c| c.file.url.starts_with(CDN)));

    let dst = tempfile::tempdir().unwrap();
    let lines = rustic_pack::import(&out, dst.path(), &dl, &env, None)
        .await
        .unwrap();
    assert!(lines.iter().all(|l| l.ok), "{lines:?}");
    assert!(dst.path().join("resourcepacks/Faithful.zip").exists());
    assert!(dst.path().join("new-world-datapacks/Trades.zip").exists());
    assert_eq!(
        switches::resource_packs(dst.path()),
        vec!["vanilla", "file/Faithful.zip"]
    );
    assert_eq!(
        std::fs::read_to_string(dst.path().join("horizon-graphics.txt")).unwrap(),
        "fancy:true\n"
    );
    assert!(!rustic_pack::allowed_override("../x"));
    assert!(!rustic_pack::allowed_override("mods/x.jar"));
}

#[tokio::test]
async fn modrinth_update_check() {
    let dir = tempfile::tempdir().unwrap();
    let mut ledger = Ledger::default();
    ledger.entries.push(rusticmc_content::ledger::Entry {
        source: SourceKind::Modrinth,
        project_id: "RP1".into(),
        version_id: "old".into(),
        version: "1.0.0".into(),
        name: "Faithful".into(),
        kind: ContentKind::ResourcePack,
        target: "instance".into(),
        files: vec![],
        enabled: true,
        game_id: None,
        needed_by: vec![],
        url: None,
    });
    ledger.save(dir.path()).unwrap();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/project/RP1/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{ "id": "new", "project_id": "RP1", "version_number": "1.1.0", "game_versions": ["26.3"],
                  "loaders": ["minecraft"], "project_type": "resourcepack", "date_published": "2026-11-01",
                  "files": [{ "url": "https://cdn.modrinth.com/x.zip", "filename": "x.zip", "primary": true,
                              "hashes": { "sha512": "aa", "sha1": "bb" } }] },
                { "id": "old", "project_id": "RP1", "version_number": "1.0.0", "game_versions": ["26.3"],
                  "loaders": ["minecraft"], "project_type": "resourcepack", "date_published": "2026-01-01", "files": [] }]"#,
        ))
        .mount(&server)
        .await;
    let m = Modrinth::new(ModrinthConfig {
        base_url: format!("{}/v2", server.uri()),
        ..ModrinthConfig::default()
    })
    .unwrap();
    let ups = updates::modrinth_updates(
        &Ledger::load(dir.path()).unwrap(),
        &m,
        &HorizonEnv::default(),
    )
    .await
    .unwrap();
    assert_eq!(ups.len(), 1);
    assert_eq!(ups[0].available.version, "1.1.0");
    assert_eq!(ups[0].installed, "1.0.0");
}
