//! Horizon's registry: the real schema-1 sample index and files (copied from Horizon), the
//! checks before install, verdicts and installs into an instance.
mod common;
use common::*;
use rusticmc_content::download::{MemoryDownloader, digest};
use rusticmc_content::fairplay::FairPlayPolicy;
use rusticmc_content::install::{InstallOptions, Installer};
use rusticmc_content::model::*;
use rusticmc_content::sources::horizon::{Index, check_url, verify_signature};
use rusticmc_content::{HorizonEnv, compat, switches};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn index() -> Index {
    Index::parse(
        &std::fs::read(fixture("horizon-index/index-v1.json")).unwrap(),
        &HorizonEnv::default(),
    )
    .unwrap()
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture(&format!("horizon-index/{name}"))).unwrap()
}

#[test]
fn reads_the_sample_index_and_keeps_unknown_kinds_out_of_search() {
    let i = index();
    assert_eq!(i.schema, 1);
    assert_eq!(i.projects.len(), 4);
    let all = i.search("", None, &[]);
    assert_eq!(all.len(), 3, "the `world` kind is kept but not offered");
    assert_eq!(i.search("glow", None, &[])[0].kind, ContentKind::Shader);
    assert_eq!(i.search("", Some(ContentKind::Modpack), &[])[0].id, "cozy");
    assert_eq!(i.search("", None, &["bloom".into()]).len(), 1);
    let cozy = &i.versions("cozy")[0];
    assert_eq!(
        cozy.dependencies[0].project_id.as_deref(),
        Some("coords_hud")
    );
    assert!(i.revoked("bad_mod", &"1".repeat(64)).is_some());
    assert!(i.revoked("bad_mod", "anything").is_none());
}

#[test]
fn a_newer_schema_is_refused() {
    let err = Index::parse(br#"{"schema": 2}"#, &HorizonEnv::default()).unwrap_err();
    assert!(err.to_string().contains("update"));
}

#[test]
fn verdicts_follow_the_build() {
    let i = index();
    let env = HorizonEnv::default();
    // coords_hud 0.1.0 is a legacy API 0.1 (WebAssembly) mod: no longer loaded.
    let coords = &i.versions("coords_hud")[0];
    let c = compat::check(coords, None, &env);
    assert_eq!(c.verdict, Verdict::Incompatible);
    assert!(c.reasons[0].contains("0.1"));
    let glow = &i.versions("glow")[0];
    assert_eq!(compat::check(glow, None, &env).badge, Badge::Works);
    assert_eq!(i.installable("glow", &env).unwrap().version, "1.0.0");
    assert!(i.installable("coords_hud", &env).is_none());
}

#[test]
fn native_mod_badges_use_hooks_and_broken_on() {
    let mut v = index().versions("glow")[0].clone();
    v.kind = ContentKind::NativeMod;
    let mut env = HorizonEnv::default();
    v.horizon = Some(HorizonFacts {
        api: "native-1/abc/def".into(),
        hooks: Some(400),
        ..HorizonFacts::default()
    });
    let c = compat::check(&v, None, &env);
    assert_eq!(c.badge, Badge::BuildsOnYourHorizon);
    assert_eq!(c.verdict, Verdict::NeedsBuild);
    assert!(
        c.reasons
            .iter()
            .any(|r| r.contains("compiles it into this instance"))
    );
    env.native_hooks = Some(300);
    assert_eq!(
        compat::check(&v, None, &env).badge,
        Badge::NeedsNewerHorizon
    );
    env.native_hooks = Some(415);
    env.commit = Some("deadbeef".into());
    v.horizon.as_mut().unwrap().broken_on = vec!["deadbeef".into()];
    assert_eq!(
        compat::check(&v, None, &env).badge,
        Badge::DoesntBuildOnYourHorizon
    );
    // Pinned (no hooks): fits only its own commit.
    v.horizon = Some(HorizonFacts {
        api: "native-1/deadbeef/0000".into(),
        ..HorizonFacts::default()
    });
    assert_eq!(
        compat::check(&v, None, &env).badge,
        Badge::BuildsOnYourHorizon
    );
}

#[test]
fn only_the_two_release_shapes_are_accepted() {
    assert!(
        check_url(
            "https://github.com/horizon-rust/horizon-mods/releases/download/glow-1.0.0/glow-1.0.0.hshader",
            "glow-1.0.0.hshader"
        )
        .is_ok()
    );
    assert!(
        check_url(
            "https://github.com/someone/coords/releases/download/v1/coords.hmod",
            "coords.hmod"
        )
        .is_ok()
    );
    for bad in [
        "http://github.com/horizon-rust/horizon-mods/releases/download/a/x.hmod",
        "https://example.org/x.hmod",
        "https://github.com/someone/coords/raw/main/x.hmod",
        "https://github.com/horizon-rust/horizon-mods/releases/download/a/other.hmod",
    ] {
        assert!(check_url(bad, "x.hmod").is_err(), "{bad}");
    }
}

#[test]
fn minisign_signatures_verify_with_the_project_key() {
    let i = index();
    let p = i.project("coords_hud").unwrap();
    let key = p.key.as_deref().unwrap();
    let sig = p.versions[0].file.signature.as_deref().unwrap();
    let file = bytes("coords_hud-0.1.0.hmod");
    verify_signature(key, sig, &file).unwrap();
    let mut tampered = file.clone();
    tampered[10] ^= 1;
    assert!(verify_signature(key, sig, &tampered).is_err());
}

#[test]
fn check_file_runs_every_check() {
    let dir = tempfile::tempdir().unwrap();
    let i = index();
    let v = i.versions("coords_hud")[0].clone();
    let p = dir.path().join("coords_hud-0.1.0.hmod");
    std::fs::write(&p, bytes("coords_hud-0.1.0.hmod")).unwrap();
    i.check_file(&v, &p).unwrap();
    // Revoked by hash.
    let mut revoked = i.clone();
    revoked
        .revoked
        .push(rusticmc_content::sources::horizon::Revoked {
            id: "coords_hud".into(),
            sha256: None,
            reason: "test".into(),
            date: String::new(),
        });
    assert!(
        revoked
            .check_file(&v, &p)
            .unwrap_err()
            .to_string()
            .contains("taken down")
    );
    // A signed project's version without a signature.
    let mut unsigned = v.clone();
    unsigned.files[0].signature = None;
    assert!(i.check_file(&unsigned, &p).is_err());
}

#[tokio::test]
async fn installs_a_registry_shader_and_picks_it() {
    let dir = tempfile::tempdir().unwrap();
    let i = index();
    let env = HorizonEnv::default();
    let v = i.versions("glow")[0].clone();
    let dl = MemoryDownloader::default().with(v.files[0].url.clone(), bytes("glow-1.0.0.hshader"));
    let fp = FairPlayPolicy::default();
    let installer = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: Some(&i),
    };
    let done = installer
        .install(
            &[v],
            &instance(dir.path()),
            &InstallOptions {
                enable: true,
                ..InstallOptions::default()
            },
        )
        .await
        .unwrap();
    assert!(done[0].path.ends_with("shaderpacks/glow-1.0.0.hshader"));
    assert_eq!(
        switches::shader_pack(dir.path()).as_deref(),
        Some("glow-1.0.0.hshader")
    );
    // A tampered download never reaches the game folder.
    let dir2 = tempfile::tempdir().unwrap();
    let v = i.versions("glow")[0].clone();
    let mut bad = bytes("glow-1.0.0.hshader");
    bad[20] ^= 1;
    let dl = MemoryDownloader::default().with(v.files[0].url.clone(), bad);
    let installer = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: Some(&i),
    };
    assert!(
        installer
            .install(&[v], &instance(dir2.path()), &InstallOptions::default())
            .await
            .is_err()
    );
    assert!(!dir2.path().join("shaderpacks/glow-1.0.0.hshader").exists());
}

/// A synthetic index: a content mod `ruby` (API 0.2) and a pack `gems` listing it.
fn gems_index(ruby: &[u8], pack: &[u8]) -> Index {
    let host = "https://github.com/horizon-rust/horizon-mods/releases/download";
    let json = format!(
        r#"{{ "schema": 1, "projects": [
        {{ "id": "ruby", "kind": "mod", "name": "Ruby", "license": "MIT", "source": "https://x",
           "versions": [{{ "version": "1.0.0", "api": "0.2", "side": "both", "game": "26.3",
             "file": {{ "name": "ruby-1.0.0.hmod", "url": "{host}/ruby-1.0.0/ruby-1.0.0.hmod",
                        "size": {}, "sha256": "{}" }} }}] }},
        {{ "id": "gems", "kind": "modpack", "name": "Gems", "license": "MIT", "source": "https://x",
           "versions": [{{ "version": "1.0.0", "api": "1", "game": "26.3",
             "requires": [{{ "id": "ruby", "version": "1.0.0", "sha256": "{}" }}],
             "file": {{ "name": "gems-1.0.0.hpack", "url": "{host}/gems-1.0.0/gems-1.0.0.hpack",
                        "size": {}, "sha256": "{}" }} }}] }}
        ] }}"#,
        ruby.len(),
        digest(ruby).sha256.unwrap(),
        digest(ruby).sha256.unwrap(),
        pack.len(),
        digest(pack).sha256.unwrap(),
    );
    Index::parse(json.as_bytes(), &HorizonEnv::default()).unwrap()
}

fn ruby_mod() -> Vec<u8> {
    zip(&[
        (
            "horizon-mod.toml",
            b"id = \"ruby\"\nname = \"Ruby\"\nversion = \"1.0.0\"\napi = \"0.2\"\nside = \"both\"\ncapabilities = [\"registries\"]\n",
        ),
        ("data/ruby/horizon/item/ruby.json", b"{\"template\": \"minecraft:emerald\"}"),
    ])
}

fn gems_pack(ruby_sha: &str, extra_override: Option<&str>) -> Vec<u8> {
    let manifest = format!(
        "format = 1\nid = \"gems\"\nname = \"Gems\"\nversion = \"1.0.0\"\ngame = \"26.3\"\n\n[[mods]]\nid = \"ruby\"\nversion = \"1.0.0\"\nsha256 = \"{ruby_sha}\"\n"
    );
    let mut entries: Vec<(&str, &[u8])> = vec![
        ("horizon-modpack.toml", manifest.as_bytes()),
        ("overrides/options.txt", b"renderDistance:8\n"),
    ];
    if let Some(name) = extra_override {
        entries.push((name, b"x"));
    }
    zip(&entries)
}

#[tokio::test]
async fn hpack_installs_its_mods_from_the_index_and_its_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let ruby = ruby_mod();
    let pack = gems_pack(&digest(&ruby).sha256.unwrap(), None);
    let i = gems_index(&ruby, &pack);
    let env = HorizonEnv::default();
    let fp = FairPlayPolicy::default();
    let mut dl = MemoryDownloader::default();
    for p in ["ruby", "gems"] {
        let v = &i.versions(p)[0];
        let b = if p == "ruby" {
            ruby.clone()
        } else {
            pack.clone()
        };
        dl = dl.with(v.files[0].url.clone(), b);
    }
    let installer = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: Some(&i),
    };
    let gems = i.versions("gems")[0].clone();
    let done = installer
        .install(&[gems], &instance(dir.path()), &InstallOptions::default())
        .await
        .unwrap();
    assert_eq!(done.len(), 1);
    assert!(dir.path().join("horizon-mods/ruby-1.0.0.hmod").exists());
    assert!(
        switches::mod_on(dir.path(), "ruby"),
        "`on` defaults to true"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
        "renderDistance:8\n"
    );
    // The pack's mods update with the pack, not one by one.
    let ledger = rusticmc_content::ledger::Ledger::load(dir.path()).unwrap();
    assert!(rusticmc_content::updates::horizon_updates(&ledger, &i, &env).is_empty());

    // An override a pack may not hold refuses the whole pack.
    let dir2 = tempfile::tempdir().unwrap();
    let bad = gems_pack(
        &digest(&ruby).sha256.unwrap(),
        Some("overrides/mods/evil.jar"),
    );
    let i2 = gems_index(&ruby, &bad);
    let dl2 = MemoryDownloader::default()
        .with(i2.versions("gems")[0].files[0].url.clone(), bad)
        .with(i2.versions("ruby")[0].files[0].url.clone(), ruby.clone());
    let installer = Installer {
        downloader: &dl2,
        env: &env,
        fairplay: &fp,
        index: Some(&i2),
    };
    let err = installer
        .install(
            &[i2.versions("gems")[0].clone()],
            &instance(dir2.path()),
            &InstallOptions::default(),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("mods/evil.jar"));
    assert!(!dir2.path().join("horizon-mods").exists());
}

#[tokio::test]
async fn fetches_the_index_over_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/index.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes("index-v1.json")))
        .mount(&server)
        .await;
    let reg = rusticmc_content::sources::horizon::HorizonRegistry::new(Some(format!(
        "{}/index.json",
        server.uri()
    )))
    .unwrap();
    let i = reg.fetch(&HorizonEnv::default()).await.unwrap();
    assert_eq!(i.projects.len(), 4);
}
