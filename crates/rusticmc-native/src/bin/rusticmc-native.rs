//! `rusticmc-native`: the native build pipeline from the command line, for developers and the
//! edition's CI. The launcher runs the same code.
//!
//! ```text
//! rusticmc-native toolchain [--data DIR]
//! rusticmc-native pack-source --horizon DIR --launcher DIR [--commit SHA] --out FILE.tar.gz
//! rusticmc-native status --instance-dir DIR --base-build NAME
//! rusticmc-native build --data DIR --instance-dir DIR --base-build NAME --base-build-dir DIR
//!                       [--local-horizon DIR --local-launcher DIR] [--cache-limit-gib N]
//!                       --trust-native-code
//! ```
//!
//! `build` needs `--trust-native-code`: native mods run unsandboxed, and building them is the
//! player's decision (it records the consent for that instance, as the launcher's notice does).
use rusticmc_launcher_core::{CancellationToken, DownloadManager};
use rusticmc_native::{
    BuildRequest, Builder, Consent, LocalSource, NativeRoot, SourceSpec, ToolchainStatus, instance,
    source, toolchain,
};
use std::path::PathBuf;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn need(args: &[String], name: &str) -> PathBuf {
    PathBuf::from(arg(args, name).unwrap_or_else(|| {
        eprintln!("{name} is required");
        std::process::exit(2)
    }))
}

fn data_dir(args: &[String]) -> PathBuf {
    arg(args, "--data")
        .map(PathBuf::from)
        .or_else(|| {
            rusticmc_launcher_core::LauncherPaths::from_os()
                .ok()
                .map(|p| p.data().to_path_buf())
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("toolchain") => {
            let root = NativeRoot::new(&data_dir(&args));
            let status = toolchain::detect(&root).await;
            println!("{}", serde_json::to_string_pretty(&status).expect("json"));
            i32::from(status.ready().is_none())
        }
        Some("pack-source") => {
            let local = LocalSource {
                horizon: need(&args, "--horizon"),
                launcher: need(&args, "--launcher"),
                horizon_commit: arg(&args, "--commit"),
            };
            let out = need(&args, "--out");
            match source::pack(&local, &out) {
                Ok(sha) => {
                    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "format": 1,
                            "horizon_commit": local.horizon_commit.as_deref().unwrap_or(source::PINNED_HORIZON),
                            "archive": {"url": "https://…/<release>/<this file>", "sha256": sha, "size": size},
                        }))
                        .expect("json")
                    );
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        Some("status") => {
            let inst = need(&args, "--instance-dir");
            let game = arg(&args, "--game-dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| inst.join("game"));
            match instance::status(
                &inst,
                &game,
                &arg(&args, "--base-build").unwrap_or_default(),
            ) {
                Ok(s) => {
                    println!("{}", serde_json::to_string_pretty(&s).expect("json"));
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        Some("build") => build(&args).await,
        _ => {
            eprintln!(
                "usage: rusticmc-native toolchain | pack-source | status | build (see the source's header)"
            );
            2
        }
    };
    std::process::exit(code);
}

async fn build(args: &[String]) -> i32 {
    if !args.iter().any(|a| a == "--trust-native-code") {
        eprintln!(
            "Native mods run with your rights, unsandboxed. Read their source; pass \
             --trust-native-code to build them."
        );
        return 2;
    }
    let data = data_dir(args);
    let root = NativeRoot::new(&data);
    let inst = need(args, "--instance-dir");
    let game = arg(args, "--game-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| inst.join("game"));
    let base_dir = need(args, "--base-build-dir");
    let base = arg(args, "--base-build").unwrap_or_else(|| {
        base_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    let local = match (arg(args, "--local-horizon"), arg(args, "--local-launcher")) {
        (Some(h), Some(l)) => Some(LocalSource {
            horizon: h.into(),
            launcher: l.into(),
            horizon_commit: arg(args, "--commit"),
        }),
        _ => None,
    };
    let spec = match SourceSpec::for_build(&base_dir, local.as_ref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let tc = match toolchain::detect(&root).await {
        ToolchainStatus::Ready { toolchain } => toolchain,
        ToolchainStatus::Missing { reason, .. } => {
            eprintln!("No usable Rust {}: {reason}", toolchain::RUST_VERSION);
            return 1;
        }
    };
    if let Err(e) = Consent::accept(&inst) {
        eprintln!("{e}");
        return 1;
    }
    let downloads = match DownloadManager::new(data.join("cache").join("downloads"), 4, 3) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let mut builder = Builder::new(root, downloads);
    if let Some(n) = arg(args, "--cache-limit-gib").and_then(|n| n.parse().ok()) {
        builder.cache_limit_gib = n;
    }
    let req = BuildRequest {
        instance_dir: inst,
        game_dir: game,
        base_build: base,
        base_build_dir: base_dir,
        source: spec,
        toolchain: tc,
    };
    let started = std::time::Instant::now();
    let last = std::sync::Mutex::new((String::new(), 0u32));
    let on = |p: rusticmc_native::Progress| {
        let pct = (p.fraction * 100.0) as u32;
        let mut l = last.lock().expect("last");
        if l.0 != p.message || pct != l.1 {
            eprintln!(
                "[{:>5.0}s {:>3}%] {:?}: {}{}",
                started.elapsed().as_secs_f64(),
                pct,
                p.phase,
                p.message,
                p.units
                    .map(|(d, t)| format!(" ({d}/{t})"))
                    .unwrap_or_default()
            );
            *l = (p.message.clone(), pct);
        }
    };
    let cancel = CancellationToken::new();
    let c2 = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            c2.cancel();
        }
    });
    match builder.build(&req, &on, &cancel).await {
        Ok(record) => {
            println!("{}", serde_json::to_string_pretty(&record).expect("json"));
            0
        }
        Err(e) => {
            eprintln!("native build failed [{}]: {e}", e.code());
            1
        }
    }
}
