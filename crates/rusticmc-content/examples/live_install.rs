//! A real install against the real sources (network required; not part of `cargo test`):
//!
//! ```sh
//! cargo run -p rusticmc-content --example live_install -- <instance folder> [modrinth search words]
//! ```
//!
//! Reads Horizon's registry index (GitHub Pages), searches Modrinth for a resource pack, applies
//! the fair-play filter, judges it against Horizon 26.3, installs the newest version with every
//! check (origin, size, SHA-512 and SHA-1, `pack.mcmeta`) through the launcher's download manager,
//! turns it on in `options.txt`, then prints the ledger.
use rusticmc_content::download::CoreDownloader;
use rusticmc_content::fairplay::FairPlayPolicy;
use rusticmc_content::install::{InstallOptions, Installer};
use rusticmc_content::sources::horizon::HorizonRegistry;
use rusticmc_content::sources::modrinth::{Modrinth, ModrinthConfig, SearchQuery};
use rusticmc_content::{ContentKind, FairPlay, HorizonEnv, Target, compat, switches};
use rusticmc_launcher_core::DownloadManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let game = std::path::PathBuf::from(args.next().ok_or("usage: <instance folder> [words]")?);
    let words: Vec<String> = args.collect();
    let query = if words.is_empty() {
        "fresh animations".to_owned()
    } else {
        words.join(" ")
    };
    std::fs::create_dir_all(&game)?;
    let env = HorizonEnv::default();

    let index = HorizonRegistry::new(None)?.fetch(&env).await?;
    println!(
        "Horizon registry: schema {}, {} projects, {} revoked",
        index.schema,
        index.projects.len(),
        index.revoked.len()
    );
    for p in index.search("", None, &[]).iter().take(5) {
        let best = index.installable(&p.id, &env);
        println!(
            "  horizon:{} ({:?}) -> {}",
            p.id,
            p.kind,
            best.map(|v| v.version)
                .unwrap_or_else(|| "no version for this Horizon".into())
        );
    }

    // A real registry file through every check of Horizon's check_file: the first resource pack
    // this Horizon can install.
    let rp = index
        .search("", Some(ContentKind::ResourcePack), &[])
        .into_iter()
        .find_map(|p| index.installable(&p.id, &env));
    let manager = DownloadManager::new(game.join(".download-cache"), 4, 2)?;
    let dl = CoreDownloader { manager };
    let policy = FairPlayPolicy::default();
    let target = Target::Instance {
        game_dir: game.clone(),
    };
    if let Some(v) = rp {
        let installer = Installer {
            downloader: &dl,
            env: &env,
            fairplay: &policy,
            index: Some(&index),
        };
        let signed = v.horizon.as_ref().and_then(|h| h.key.as_ref()).is_some();
        let done = installer
            .install(
                std::slice::from_ref(&v),
                &target,
                &InstallOptions {
                    enable: true,
                    allow_native: false,
                },
            )
            .await?;
        println!(
            "  horizon:{} {} installed ({}) -> {}",
            v.project_id,
            v.version,
            if signed {
                "minisign signature verified"
            } else {
                "unsigned project"
            },
            done[0].path.display()
        );
    }

    let m = Modrinth::new(ModrinthConfig {
        contact: "https://github.com/Rustic-Launcher/launcher".into(),
        ..Default::default()
    })?;
    let mut page = m
        .search(&SearchQuery {
            query: query.clone(),
            kind: Some(ContentKind::ResourcePack),
            limit: 5,
            ..Default::default()
        })
        .await?;
    FairPlayPolicy::default().apply(&mut page.hits);
    println!("Modrinth \"{query}\": {} hits", page.total);
    let hit = page
        .hits
        .iter()
        .find(|h| h.fair_play == FairPlay::Clean)
        .ok_or("no clean result")?;
    println!("  installing {} ({})", hit.name, hit.id);
    let version = m
        .versions(&hit.id, &[], &[], Some("resourcepack"))
        .await?
        .into_iter()
        .next()
        .ok_or("no version")?;
    let c = compat::check(&version, Some(&target), &env);
    println!(
        "  {} {}: {:?} / {:?}: {}",
        version.name,
        version.version,
        c.verdict,
        c.badge,
        c.reasons.join("; ")
    );
    let installer = Installer {
        downloader: &dl,
        env: &env,
        fairplay: &policy,
        index: None,
    };
    let done = installer
        .install(
            &[version],
            &target,
            &InstallOptions {
                enable: true,
                allow_native: false,
            },
        )
        .await?;
    for d in &done {
        println!("  installed {} -> {}", d.version, d.path.display());
        let pack = compat::check_pack_file(&d.path, ContentKind::ResourcePack, &env)?;
        println!(
            "  pack.mcmeta: {:?}: {}",
            pack.verdict,
            pack.reasons.join("; ")
        );
    }
    println!("  resourcePacks = {:?}", switches::resource_packs(&game));
    println!(
        "{}",
        std::fs::read_to_string(game.join(rusticmc_content::ledger::FILE))?
    );
    Ok(())
}
