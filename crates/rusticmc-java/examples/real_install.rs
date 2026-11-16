//! A by-hand check against the real official hosts (never run by `cargo test`):
//!
//! ```text
//! cargo run -p rusticmc-java --example real_install -- <scratch folder> <version> [loader] [--launch <seconds>]
//! ```
//!
//! Installs `<version>` (files, assets, Mojang's Java) and the loader (`vanilla`, `fabric`,
//! `quilt`, `forge`, `neoforge`; newest stable build) into `<scratch folder>`, prints the launch
//! command with the token masked, and with `--launch` starts the game for that many seconds with a
//! placeholder account (no Microsoft account: the game runs offline, as in a development setup),
//! printing its log, then stops it. The launcher itself only ever launches after the ownership
//! check (ADR 0172).

use rusticmc_java::install::{Installer, Step};
use rusticmc_java::launch::{self, Auth, LaunchOptions};
use rusticmc_java::loaders::{self, LoaderKind};
use rusticmc_java::{Hosts, Platform, process, runtime};
use rusticmc_launcher_core::{CancellationToken, DownloadManager, Secret};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rusticmc_launcher_core::http::install_crypto_provider();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = PathBuf::from(
        args.first()
            .ok_or("usage: <root> <version> [loader] [--launch s]")?,
    );
    let version = args.get(1).ok_or("no version")?.clone();
    let loader = match args.get(2).map(String::as_str) {
        None | Some("vanilla") | Some("--launch") => LoaderKind::Vanilla,
        Some("fabric") => LoaderKind::Fabric,
        Some("quilt") => LoaderKind::Quilt,
        Some("forge") => LoaderKind::Forge,
        Some("neoforge") => LoaderKind::NeoForge,
        Some(o) => return Err(format!("unknown loader {o}").into()),
    };
    let launch_secs: Option<u64> = args
        .iter()
        .position(|a| a == "--launch")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());
    let http = rusticmc_launcher_core::http::client()?;
    let installer = Installer {
        root: root.clone(),
        http: http.clone(),
        downloads: DownloadManager::with_client(http, root.join(".cache"), 16, 3),
        hosts: Hosts::default(),
        extra_hosts: vec![],
        platform: Platform::current(),
    };
    let cancel = CancellationToken::new();
    let started = Instant::now();
    let last = std::sync::Mutex::new((Step::Metadata, 0u64));
    let progress = |step: Step, done: u64, total: u64| {
        let pct = (done * 100).checked_div(total).unwrap_or(100);
        let mut l = last.lock().unwrap();
        if l.0 != step || pct >= l.1 + 10 || pct == 100 && l.1 != 100 {
            println!("  {step:?}: {pct}% ({done}/{total} bytes)");
            *l = (step, pct);
        }
    };
    let manifest = installer.manifest().await?;
    println!(
        "manifest: latest release {} snapshot {}, {} versions",
        manifest.latest.release,
        manifest.latest.snapshot,
        manifest.versions.len()
    );
    println!("installing {version}...");
    let vanilla = installer
        .install(
            &version,
            &launch::features_for(&opts_stub()),
            &cancel,
            &progress,
        )
        .await?;
    let (component, major) = vanilla.version.runtime_component();
    println!("java runtime {component} (Java {major})...");
    let java = installer
        .ensure_runtime(&component, &cancel, &progress)
        .await?;
    println!("java: {}", java.display());
    let id = match loader {
        LoaderKind::Vanilla => version.clone(),
        LoaderKind::Fabric | LoaderKind::Quilt => {
            let list = loaders::fabric::versions(&installer, loader, &version).await?;
            let pick = loaders::pick_default(&list).ok_or("no loader version")?;
            println!("{} {} (of {})", loader.label(), pick.version, list.len());
            loaders::fabric::install(&installer, loader, &version, &pick.version).await?
        }
        LoaderKind::Forge | LoaderKind::NeoForge => {
            let list = loaders::forge::versions(&installer, loader, &version).await?;
            let pick = loaders::pick_default(&list).ok_or("no loader version")?;
            println!(
                "{} {} (of {}), running its installer...",
                loader.label(),
                pick.version,
                list.len()
            );
            loaders::forge::install(
                &installer,
                loader,
                &version,
                &pick.version,
                &java,
                &cancel,
                &|l| println!("    installer: {l}"),
            )
            .await?
        }
    };
    let installed = installer
        .install(&id, &launch::features_for(&opts_stub()), &cancel, &progress)
        .await?;
    println!(
        "installed {id} in {:.1}s: {} classpath entries, {} old-style natives",
        started.elapsed().as_secs_f64(),
        installed.libraries.classpath.len(),
        installed.libraries.natives.len()
    );
    let game_dir = root.join("profiles").join(format!("{id}-test"));
    let natives = root.join("natives").join(&id);
    launch::prepare(&installed, &root, &natives, &game_dir)?;
    let mut opts = opts_stub();
    opts.game_dir = game_dir.clone();
    opts.java = java.clone();
    opts.java_major = runtime::java_major(&java).or(Some(major));
    let auth = Auth {
        name: "RusticTest".into(),
        uuid: "00000000000000000000000000000000".into(),
        access_token: Secret::new("placeholder-token-not-a-real-one"),
        xuid: None,
    };
    let mut cmd = launch::build(
        &installed,
        &root,
        &natives,
        &auth,
        &opts,
        &installer.platform,
    )?;
    if args.iter().any(|a| a == "--dump-argfile") {
        std::fs::write(root.join("dump.args"), cmd.argfile_text())?;
    }
    if args.iter().any(|a| a == "--no-argfile") {
        cmd.use_argfile = false;
    }
    println!(
        "command ({} args, argfile: {}):\n{}",
        cmd.args.len(),
        cmd.use_argfile,
        cmd.display()
    );
    if let Some(secs) = launch_secs {
        println!("starting for {secs}s...");
        let handle = process::spawn(
            &cmd,
            &root.join("run"),
            Arc::new(|l: process::LogLine| {
                println!(
                    "  [{}{}] {}",
                    l.level.as_deref().unwrap_or("-"),
                    l.thread.map(|t| format!("/{t}")).unwrap_or_default(),
                    l.text
                )
            }),
        )?;
        let stop = handle.stop_token();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(secs)).await;
            stop.cancel();
        });
        let exit = handle.done.await?;
        println!(
            "exit: code {:?}, crashed {}, killed {}, crash report {:?}",
            exit.code, exit.crashed, exit.killed, exit.crash_report
        );
        println!(
            "argfiles left in run/: {}",
            std::fs::read_dir(root.join("run"))
                .map(|d| d.count())
                .unwrap_or(0)
        );
    }
    Ok(())
}

fn opts_stub() -> LaunchOptions {
    LaunchOptions {
        game_dir: PathBuf::new(),
        java: PathBuf::new(),
        java_major: None,
        memory_min_mb: 512,
        memory_max_mb: 2048,
        resolution: Some((1024, 600)),
        fullscreen: false,
        jvm_args: vec![],
        launcher_name: "rustic-launcher".into(),
        launcher_version: rusticmc_launcher_core::VERSION.into(),
        quick_play_server: None,
    }
}
