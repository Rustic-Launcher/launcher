//! Installs from Modrinth-shaped versions (no network: an in-memory downloader), switches,
//! uninstall, dependencies, fair play, data packs into worlds and new worlds.
mod common;
use common::*;
use rusticmc_content::download::MemoryDownloader;
use rusticmc_content::fairplay::FairPlayPolicy;
use rusticmc_content::install::{InstallOptions, Installer};
use rusticmc_content::ledger::Ledger;
use rusticmc_content::model::*;
use rusticmc_content::resolve::{DependencySource, ResolveContext, resolve};
use rusticmc_content::{ContentError, HorizonEnv, compat, switches};
use std::collections::HashMap;

const CDN: &str = "https://cdn.modrinth.com/data";

fn on() -> InstallOptions {
    InstallOptions {
        enable: true,
        ..InstallOptions::default()
    }
}

#[tokio::test]
async fn resource_pack_install_enable_disable_uninstall() {
    let dir = tempfile::tempdir().unwrap();
    let rp = resource_pack(97);
    let url = format!("{CDN}/RP1/versions/v1/Faithful.zip");
    let v = modrinth_version(
        "RP1",
        ContentKind::ResourcePack,
        &["minecraft"],
        file(&url, "Faithful.zip", &rp),
    );
    let dl = MemoryDownloader::default().with(url, rp);
    let env = HorizonEnv::default();
    let fp = FairPlayPolicy::default();
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    let t = instance(dir.path());
    let done = inst.install(&[v], &t, &on()).await.unwrap();
    assert_eq!(done[0].compatibility.badge, Badge::Works);
    assert!(dir.path().join("resourcepacks/Faithful.zip").exists());
    assert_eq!(
        switches::resource_packs(dir.path()),
        vec!["vanilla", "file/Faithful.zip"]
    );
    assert!(!dir.path().join(".rustic-staging").exists());

    inst.set_enabled(&t, SourceKind::Modrinth, "RP1", false)
        .unwrap();
    assert_eq!(switches::resource_packs(dir.path()), vec!["vanilla"]);
    assert!(!Ledger::load(dir.path()).unwrap().entries[0].enabled);

    inst.uninstall(&t, SourceKind::Modrinth, "RP1").unwrap();
    assert!(!dir.path().join("resourcepacks/Faithful.zip").exists());
    assert!(Ledger::load(dir.path()).unwrap().entries.is_empty());

    // The pack's own format against the build's.
    let p = dir.path().join("old.zip");
    std::fs::write(&p, resource_pack(34)).unwrap();
    let c = compat::check_pack_file(&p, ContentKind::ResourcePack, &env).unwrap();
    assert_eq!(c.verdict, Verdict::Partial);
    assert!(c.reasons[0].contains("too old"));
}

#[tokio::test]
async fn java_mods_and_cheats_are_never_installed() {
    let dir = tempfile::tempdir().unwrap();
    let jar = b"PK-not-really".to_vec();
    let url = format!("{CDN}/AANobbMI/versions/v1/sodium.jar");
    let sodium = modrinth_version(
        "AANobbMI",
        ContentKind::Mod,
        &["fabric", "neoforge"],
        file(&url, "sodium.jar", &jar),
    );
    let env = HorizonEnv::default();
    let c = compat::check(&sodium, None, &env);
    assert_eq!(c.badge, Badge::NeedsPort);
    assert!(!c.installable());
    let dl = MemoryDownloader::default().with(url.clone(), jar.clone());
    let fp = FairPlayPolicy::default();
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    let err = inst
        .install(&[sodium], &instance(dir.path()), &on())
        .await
        .unwrap_err();
    assert!(matches!(err, ContentError::Incompatible(_)));

    // A resource pack named as an X-ray is refused by the fair-play policy.
    let rp = resource_pack(97);
    let url = format!("{CDN}/XR/versions/v1/xray.zip");
    let mut xray = modrinth_version(
        "XR",
        ContentKind::ResourcePack,
        &["minecraft"],
        file(&url, "xray.zip", &rp),
    );
    xray.name = "Xray Ultimate".into();
    let dl = MemoryDownloader::default().with(url, rp);
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    let err = inst
        .install(&[xray], &instance(dir.path()), &on())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not installed by Rustic"));
    assert!(!dir.path().join("resourcepacks").exists());
}

#[tokio::test]
async fn files_from_other_hosts_or_with_bad_hashes_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let rp = resource_pack(97);
    let env = HorizonEnv::default();
    let fp = FairPlayPolicy::default();
    let url = "https://evil.example/pack.zip";
    let v = modrinth_version(
        "RP2",
        ContentKind::ResourcePack,
        &[],
        file(url, "pack.zip", &rp),
    );
    let dl = MemoryDownloader::default().with(url, rp.clone());
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    assert!(
        inst.install(&[v], &instance(dir.path()), &on())
            .await
            .unwrap_err()
            .to_string()
            .contains("not one of Modrinth's file hosts")
    );
    let url = format!("{CDN}/RP3/v/pack.zip");
    let mut v = modrinth_version(
        "RP3",
        ContentKind::ResourcePack,
        &[],
        file(&url, "pack.zip", &rp),
    );
    v.files[0].hashes.sha512 = Some("00".repeat(64));
    let dl = MemoryDownloader::default().with(url, rp);
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    assert!(
        inst.install(&[v], &instance(dir.path()), &on())
            .await
            .unwrap_err()
            .to_string()
            .contains("sha512 mismatch")
    );
    assert!(!dir.path().join("resourcepacks/pack.zip").exists());
}

#[tokio::test]
async fn data_packs_go_into_a_world_or_new_worlds() {
    let dir = tempfile::tempdir().unwrap();
    let dp = data_pack();
    let url = format!("{CDN}/DP/v/Trades.zip");
    let v = modrinth_version(
        "DP",
        ContentKind::DataPack,
        &["datapack"],
        file(&url, "Trades.zip", &dp),
    );
    let env = HorizonEnv::default();
    // Not into an instance.
    assert_eq!(
        compat::check(&v, Some(&instance(dir.path())), &env).badge,
        Badge::WrongTarget
    );
    let dl = MemoryDownloader::default().with(url, dp);
    let fp = FairPlayPolicy::default();
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    let world = Target::World {
        game_dir: dir.path().to_path_buf(),
        world: "My World".into(),
    };
    let done = inst
        .install(std::slice::from_ref(&v), &world, &on())
        .await
        .unwrap();
    assert_eq!(done[0].compatibility.verdict, Verdict::Partial);
    assert!(
        dir.path()
            .join("saves/My World/datapacks/Trades.zip")
            .exists()
    );
    let json: serde_json::Value = serde_json::from_slice(
        &std::fs::read(dir.path().join("saves/My World/datapacks.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(json["enabled"][0], "file/Trades.zip");

    let new_worlds = Target::NewWorlds {
        game_dir: dir.path().to_path_buf(),
    };
    inst.install(&[v], &new_worlds, &on()).await.unwrap();
    assert!(dir.path().join("new-world-datapacks/Trades.zip").exists());
    // A world name that leaves the saves folder is refused.
    let escape = Target::World {
        game_dir: dir.path().to_path_buf(),
        world: "..".into(),
    };
    assert!(rusticmc_content::install::folder_for(ContentKind::DataPack, &escape).is_err());
}

/// An in-memory source for dependency resolution.
struct Source(HashMap<String, ContentVersion>);

#[async_trait::async_trait]
impl DependencySource for Source {
    async fn by_version_id(&self, id: &str) -> rusticmc_content::Result<ContentVersion> {
        self.0
            .values()
            .find(|v| v.id == id)
            .cloned()
            .ok_or_else(|| ContentError::NotFound(id.into()))
    }
    async fn best_for_project(
        &self,
        project: &str,
        _ctx: &ResolveContext,
    ) -> rusticmc_content::Result<Option<ContentVersion>> {
        Ok(self.0.get(project).cloned())
    }
}

fn dep(project: &str, kind: DependencyKind) -> Dependency {
    Dependency {
        project_id: Some(project.into()),
        version_id: None,
        version_req: None,
        sha256: None,
        kind,
    }
}

#[tokio::test]
async fn dependencies_resolve_depth_first_once() {
    let mk = |p: &str, deps: Vec<Dependency>| {
        let mut v = modrinth_version(
            p,
            ContentKind::ResourcePack,
            &[],
            file(&format!("{CDN}/{p}/f.zip"), &format!("{p}.zip"), b"x"),
        );
        v.dependencies = deps;
        v
    };
    let lib = mk("lib", vec![]);
    let core = mk("core", vec![dep("lib", DependencyKind::Required)]);
    let ui = mk(
        "ui",
        vec![
            dep("lib", DependencyKind::Required),
            dep("extra", DependencyKind::Optional),
        ],
    );
    let root = mk(
        "root",
        vec![
            dep("core", DependencyKind::Required),
            dep("ui", DependencyKind::Required),
        ],
    );
    let src = Source(
        [lib, core, ui]
            .into_iter()
            .map(|v| (v.project_id.clone(), v))
            .collect(),
    );
    let order = resolve(
        root.clone(),
        &src,
        &Ledger::default(),
        &ResolveContext::default(),
    )
    .await
    .unwrap();
    let ids: Vec<&str> = order.iter().map(|v| v.project_id.as_str()).collect();
    assert_eq!(ids, vec!["lib", "core", "ui", "root"]);

    // Already installed: left out.
    let mut installed = Ledger::default();
    installed.entries.push(rusticmc_content::ledger::Entry {
        source: SourceKind::Modrinth,
        project_id: "lib".into(),
        version_id: "lib-v1".into(),
        version: "1.0.0".into(),
        name: "lib".into(),
        kind: ContentKind::ResourcePack,
        target: "instance".into(),
        files: vec![],
        enabled: true,
        game_id: None,
        needed_by: vec![],
        url: None,
    });
    let order = resolve(root.clone(), &src, &installed, &ResolveContext::default())
        .await
        .unwrap();
    assert_eq!(order.len(), 3);

    // An incompatible project installed stops it.
    let mut clash = root.clone();
    clash
        .dependencies
        .push(dep("lib", DependencyKind::Incompatible));
    assert!(matches!(
        resolve(clash, &src, &installed, &ResolveContext::default()).await,
        Err(ContentError::Dependency(_))
    ));

    // A missing required dependency is an error naming it.
    let lonely = mk("lonely", vec![dep("ghost", DependencyKind::Required)]);
    let err = resolve(lonely, &src, &Ledger::default(), &ResolveContext::default())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("ghost"));
}

#[tokio::test]
async fn uninstall_is_refused_while_a_dependent_remains() {
    let dir = tempfile::tempdir().unwrap();
    let a = resource_pack(97);
    let b = resource_pack(97);
    let ua = format!("{CDN}/A/a.zip");
    let ub = format!("{CDN}/B/b.zip");
    let lib = modrinth_version("A", ContentKind::ResourcePack, &[], file(&ua, "a.zip", &a));
    let mut top = modrinth_version("B", ContentKind::ResourcePack, &[], file(&ub, "b.zip", &b));
    top.dependencies = vec![dep("A", DependencyKind::Required)];
    let dl = MemoryDownloader::default().with(ua, a).with(ub, b);
    let env = HorizonEnv::default();
    let fp = FairPlayPolicy::default();
    let inst = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &fp,
        index: None,
    };
    let t = instance(dir.path());
    let done = inst.install(&[lib, top], &t, &on()).await.unwrap();
    assert_eq!(done[0].dependency_of.as_deref(), Some("B"));
    assert!(matches!(
        inst.uninstall(&t, SourceKind::Modrinth, "A"),
        Err(ContentError::Dependency(_))
    ));
    inst.uninstall(&t, SourceKind::Modrinth, "B").unwrap();
    inst.uninstall(&t, SourceKind::Modrinth, "A").unwrap();
}

#[test]
fn shader_packs_and_bukkit_plugins() {
    let env = HorizonEnv::default();
    let iris = modrinth_version(
        "BSL",
        ContentKind::ShaderPack,
        &["iris", "optifine"],
        file("https://cdn.modrinth.com/x.zip", "x.zip", b"x"),
    );
    let c = compat::check(&iris, None, &env);
    assert_eq!(c.verdict, Verdict::Partial);
    assert!(c.reasons[0].contains("Iris/OptiFine"));
    let essentials = modrinth_version(
        "EX",
        ContentKind::Plugin,
        &["paper", "spigot"],
        file("https://cdn.modrinth.com/e.jar", "e.jar", b"x"),
    );
    assert_eq!(
        compat::check(&essentials, None, &env).verdict,
        Verdict::Incompatible
    );
}
