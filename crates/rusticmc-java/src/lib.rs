//! Rustic Launcher's launcher for the official Java Edition (ADR 0172), next to the Rustic edition
//! (Horizon) the rest of the launcher plays.
//!
//! Everything comes at run time from the official public hosts, into the launcher's data folder,
//! in the official launcher's layout (`versions/`, `libraries/`, `assets/`, shared with the Rustic
//! edition's own game files, ADR 0147), never from the repository:
//!
//! - [`manifest`]: Mojang's version manifest (piston-meta), releases / snapshots / old versions;
//! - [`version`], [`rules`], [`merge`], [`maven`]: version files, their rules and features, the
//!   `inheritsFrom` merge loader profiles use, Maven coordinates;
//! - [`libraries`], [`assets`], [`runtime`], [`install`]: what a version needs on this system, the
//!   asset index and objects, Mojang's Java runtimes, and the installer that fetches them all
//!   (sha1 and size checked, parallel, resumable);
//! - [`loaders`]: Fabric and Quilt through their meta APIs, Forge and NeoForge through their
//!   official installers run headless with the downloaded Java;
//! - [`launch`], [`process`]: the argument list (new `arguments` and legacy
//!   `minecraftArguments`), natives, classpath, the account's token (in a Java argument file, never
//!   on the command line where Java allows it), then the process with its logs and crash detection;
//! - [`profiles`]: the player's Java profiles (separate from Rustic instances);
//! - [`bedrock`]: Bedrock Edition detection and start on Windows.
//!
//! Nothing of Mojang's is part of Rustic; playing requires a Microsoft account that owns the game
//! (checked by the caller with `rusticmc-auth` before [`launch`]).

pub mod assets;
pub mod bedrock;
pub mod download;
pub mod error;
pub mod install;
pub mod launch;
pub mod libraries;
pub mod loaders;
pub mod manifest;
pub mod maven;
pub mod merge;
pub mod platform;
pub mod process;
pub mod profiles;
pub mod rules;
pub mod runtime;
pub mod version;

pub use error::{JavaError, Result};
pub use platform::Platform;

/// The official hosts (another base only in tests).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hosts {
    /// Mojang's version manifest.
    pub version_manifest: String,
    /// Mojang's Java runtime manifest (the one the official launcher reads).
    pub runtime_manifest: String,
    /// Asset objects (`<host>/<xx>/<hash>`).
    pub resources: String,
    /// Libraries named without a download (old version files).
    pub libraries: String,
    pub fabric_meta: String,
    pub quilt_meta: String,
    pub forge_maven: String,
    pub neoforge_maven: String,
}

impl Default for Hosts {
    fn default() -> Self {
        Self {
            version_manifest: "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
                .into(),
            runtime_manifest: "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json".into(),
            resources: "https://resources.download.minecraft.net".into(),
            libraries: "https://libraries.minecraft.net".into(),
            fabric_meta: "https://meta.fabricmc.net".into(),
            quilt_meta: "https://meta.quiltmc.org".into(),
            forge_maven: "https://maven.minecraftforge.net".into(),
            neoforge_maven: "https://maven.neoforged.net/releases".into(),
        }
    }
}

/// The official edition icons the launcher shows next to the game names, fetched at run time from
/// minecraft.net (never bundled: no Mojang asset in the repository). Its CDN answers only to a
/// browser-like `User-Agent` ([`ICON_USER_AGENT`]).
pub const OFFICIAL_ICONS: [(&str, &str); 2] = [
    (
        "java",
        "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/logos/Java-24px.png",
    ),
    (
        "bedrock",
        "https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/logos/Bedrock-24px.png",
    ),
];

/// The `User-Agent` of the icon requests.
pub fn icon_user_agent() -> String {
    format!(
        "Mozilla/5.0 (compatible; RusticLauncher/{})",
        rusticmc_launcher_core::VERSION
    )
}

/// The hosts files may be downloaded from (a version or loader file naming another is refused).
pub const ALLOWED_DOWNLOAD_HOSTS: &[&str] = &[
    "piston-meta.mojang.com",
    "piston-data.mojang.com",
    "launchermeta.mojang.com",
    "launcher.mojang.com",
    "libraries.minecraft.net",
    "resources.download.minecraft.net",
    "maven.fabricmc.net",
    "meta.fabricmc.net",
    "maven.quiltmc.org",
    "meta.quiltmc.org",
    "maven.minecraftforge.net",
    "files.minecraftforge.net",
    "maven.neoforged.net",
];

/// Whether `url` is HTTPS on one of the allowed hosts (or `extra`, the tests' mock server).
pub fn url_allowed(url: &str, extra: &[String]) -> bool {
    if extra.iter().any(|base| url.starts_with(base.as_str())) {
        return true;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    ALLOWED_DOWNLOAD_HOSTS.contains(&host.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_official_https_hosts() {
        assert!(url_allowed(
            "https://libraries.minecraft.net/a/b/1/b-1.jar",
            &[]
        ));
        assert!(url_allowed("https://maven.fabricmc.net/x", &[]));
        assert!(!url_allowed("http://libraries.minecraft.net/a", &[]));
        assert!(!url_allowed(
            "https://libraries.minecraft.net.evil.example/a",
            &[]
        ));
        assert!(!url_allowed("https://example.com/a.jar", &[]));
        assert!(url_allowed(
            "http://127.0.0.1:9/x",
            &["http://127.0.0.1:9".into()]
        ));
    }
}

#[cfg(test)]
mod icon_tests {
    /// By hand (network): `cargo test -p rusticmc-java -- --ignored official_icons`.
    #[tokio::test]
    #[ignore = "reaches minecraft.net"]
    async fn official_icons_answer_the_launcher() {
        rusticmc_launcher_core::http::install_crypto_provider();
        let http = rusticmc_launcher_core::http::client().unwrap();
        for (edition, url) in super::OFFICIAL_ICONS {
            let resp = http
                .get(url)
                .header(
                    rusticmc_launcher_core::reqwest::header::USER_AGENT,
                    super::icon_user_agent(),
                )
                .send()
                .await
                .unwrap();
            assert!(resp.status().is_success(), "{edition}: {}", resp.status());
            let mime = resp.headers()["content-type"].to_str().unwrap().to_owned();
            assert!(mime.starts_with("image/"), "{edition}: {mime}");
            assert!(!resp.bytes().await.unwrap().is_empty());
        }
    }
}
