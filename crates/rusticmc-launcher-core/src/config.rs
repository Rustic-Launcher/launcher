//! The launcher's configuration: `launcher.toml`, versioned.
//!
//! Every section has defaults, so a file only holds what the player changed. The file carries
//! `version`; [`LauncherConfig::load`] migrates older versions in memory and refuses a newer one
//! (written by a newer launcher) rather than dropping settings it doesn't know.
//! Nothing secret is ever stored here: tokens and API keys live in the OS keychain.

use crate::error::{CoreError, Result};
use crate::fsutil;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The configuration format this launcher writes.
pub const CONFIG_VERSION: u32 = 1;

/// Overrides `microsoft.client_id` at run time.
pub const CLIENT_ID_ENV: &str = "RUSTIC_MSA_CLIENT_ID";
/// The Azure application id built in by release CI (`RUSTIC_MSA_CLIENT_ID` at build time).
pub const BUILT_IN_CLIENT_ID: Option<&str> = option_env!("RUSTIC_MSA_CLIENT_ID");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LauncherConfig {
    pub version: u32,
    pub microsoft: MicrosoftConfig,
    pub ownership: OwnershipConfig,
    pub downloads: DownloadConfig,
    pub builds: BuildsConfig,
    pub rustic_server: RusticServerConfig,
    pub cloud: CloudConfig,
    pub java: JavaConfig,
    pub game_files: GameFilesConfig,
    pub privacy: PrivacyConfig,
    pub logging: LoggingConfig,
    pub network: NetworkConfig,
    pub dns: DnsConfig,
    pub native: NativeConfig,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            microsoft: MicrosoftConfig::default(),
            ownership: OwnershipConfig::default(),
            downloads: DownloadConfig::default(),
            builds: BuildsConfig::default(),
            rustic_server: RusticServerConfig::default(),
            cloud: CloudConfig::default(),
            java: JavaConfig::default(),
            game_files: GameFilesConfig::default(),
            privacy: PrivacyConfig::default(),
            logging: LoggingConfig::default(),
            network: NetworkConfig::default(),
            dns: DnsConfig::default(),
            native: NativeConfig::default(),
        }
    }
}

/// Microsoft sign-in (launcher/docs/TODO_HUMAN.md: the Azure app registration).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MicrosoftConfig {
    /// The Azure "Application (client) ID". `None`: the built-in one, else sign-in is unavailable.
    pub client_id: Option<String>,
    /// The sign-in flow offered first.
    pub preferred_flow: AuthFlow,
}

impl Default for MicrosoftConfig {
    fn default() -> Self {
        Self {
            client_id: None,
            preferred_flow: AuthFlow::AuthorizationCode,
        }
    }
}

impl MicrosoftConfig {
    /// The client id in effect: the environment, then the file, then the built-in one.
    pub fn effective_client_id(&self) -> Option<String> {
        std::env::var(CLIENT_ID_ENV)
            .ok()
            .or_else(|| self.client_id.clone())
            .or_else(|| BUILT_IN_CLIENT_ID.map(str::to_owned))
            .filter(|id| !id.trim().is_empty())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthFlow {
    /// Browser sign-in with PKCE and a loopback redirect.
    AuthorizationCode,
    /// A code typed at microsoft.com/link.
    DeviceCode,
}

/// The owner gate for offline profiles (launcher/AGENTS.md).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct OwnershipConfig {
    /// Offline profiles stay usable this many days after the last successful ownership check of
    /// any linked Microsoft account. Clamped to 1..=90 when read.
    pub revalidation_days: u32,
}

impl Default for OwnershipConfig {
    fn default() -> Self {
        Self {
            revalidation_days: 30,
        }
    }
}

impl OwnershipConfig {
    pub fn window_seconds(&self) -> u64 {
        crate::time::days(u64::from(self.revalidation_days.clamp(1, 90)))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DownloadConfig {
    /// Downloads at once.
    pub concurrency: usize,
    /// Attempts per file after the first.
    pub retries: u32,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            concurrency: 6,
            retries: 3,
        }
    }
}

/// Where Horizon builds come from (ADR 0045).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BuildsConfig {
    /// The id of the source new instances use.
    pub default_source: String,
    pub sources: Vec<BuildSourceConfig>,
}

impl Default for BuildsConfig {
    fn default() -> Self {
        Self {
            default_source: "rustic-edition".into(),
            sources: vec![
                BuildSourceConfig {
                    id: "rustic-edition".into(),
                    name: "Horizon, Rustic edition".into(),
                    kind: BuildSourceKind::Channel,
                    repo: None,
                    channel_url: Some(
                        "https://cdn.rustic-launcher.com/releases/horizon/channel.json".into(),
                    ),
                    asset_prefix: "rustic-horizon".into(),
                    signature_namespace: "rustic release".into(),
                    public_keys: Vec::new(),
                    builtin_horizon_keys: false,
                },
                BuildSourceConfig {
                    id: "horizon".into(),
                    name: "Horizon".into(),
                    kind: BuildSourceKind::Github,
                    repo: Some("horizon-rust/horizon".into()),
                    channel_url: None,
                    asset_prefix: "horizon".into(),
                    signature_namespace: "horizon release".into(),
                    public_keys: Vec::new(),
                    builtin_horizon_keys: true,
                },
            ],
        }
    }
}

/// One source of game builds.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BuildSourceConfig {
    pub id: String,
    pub name: String,
    pub kind: BuildSourceKind,
    /// `owner/repo` for [`BuildSourceKind::Github`].
    #[serde(default)]
    pub repo: Option<String>,
    /// The channel index for [`BuildSourceKind::Channel`].
    #[serde(default)]
    pub channel_url: Option<String>,
    /// Archive names are `<prefix>-<platform>.<ext>` (`horizon-windows-x64.zip`).
    pub asset_prefix: String,
    /// Signatures cover `<namespace> <version>\n` followed by SHA256SUMS.
    pub signature_namespace: String,
    /// Base64 Ed25519 public keys this source's checksums must be signed with.
    #[serde(default)]
    pub public_keys: Vec<String>,
    /// Also trust Horizon's own release keys (built in from Horizon's release-keys.txt).
    #[serde(default)]
    pub builtin_horizon_keys: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildSourceKind {
    /// GitHub releases with Horizon's SHA256SUMS / SHA256SUMS(.v2).sig layout.
    Github,
    /// A JSON channel index (launcher/docs/DECISIONS/0045).
    Channel,
}

/// Rustic Server (`rustic-server`, the engine `horizon-server` runs for Rustic servers).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct RusticServerConfig {
    /// A manual override of the `rustic-server` executable (Settings > Game and instances).
    /// `None`: looked up in the server's build folder, beside the launcher, then on the PATH
    /// (ADR 0152).
    pub executable: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CloudConfig {
    /// Rustic Cloud's API.
    pub api_base: String,
    /// The Rustic Relay control endpoint.
    pub relay_base: String,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            api_base: "https://api.rustic-launcher.com".into(),
            relay_base: "https://relay.rustic-launcher.com".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct JavaConfig {
    /// Extra Java homes to consider for preparing game files.
    pub extra_homes: Vec<PathBuf>,
    /// A Java home forced for preparation.
    pub preferred_home: Option<PathBuf>,
}

/// The player's own game files, read by Horizon's preparer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct GameFilesConfig {
    /// The official launcher's game folder; `None`: its usual place.
    pub minecraft_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct PrivacyConfig {
    /// Anonymous telemetry: off until the player opts in (GDPR).
    pub telemetry: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LoggingConfig {
    /// `tracing` filter (`info`, `rusticmc_auth=debug,info`...).
    pub level: String,
    /// Daily log files kept.
    pub keep_files: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
            keep_files: 7,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NetworkConfig {
    /// Ask the router (UPnP IGD) to forward local servers' ports.
    pub upnp: bool,
    /// A service that tells whether a public address and port answer, for the reachability test.
    pub reachability_url: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            upnp: true,
            reachability_url: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DnsConfig {
    /// DNS-over-HTTPS JSON resolver used to check records (`application/dns-json`).
    pub doh_url: String,
    /// Porkbun's API base.
    pub porkbun_api: String,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            doh_url: "https://cloudflare-dns.com/dns-query".into(),
            porkbun_api: "https://api.porkbun.com/api/json/v3".into(),
        }
    }
}

/// Native mods compiled on the player's computer (ADR 0157).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NativeConfig {
    /// The compile cache (`<data>/native/target`) is cleared after a build when it exceeds this
    /// many GiB (Horizon's default: 30); 0 never clears it automatically.
    pub cache_limit_gib: u64,
    /// Development: build from these checkouts instead of the edition's source archive
    /// (`RUSTIC_NATIVE_HORIZON` and `RUSTIC_NATIVE_LAUNCHER` override them).
    pub local_source: Option<NativeLocalSource>,
}

impl Default for NativeConfig {
    fn default() -> Self {
        Self {
            cache_limit_gib: 30,
            local_source: None,
        }
    }
}

/// Checkouts a development build of native mods compiles from.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NativeLocalSource {
    /// `Rustic-MC/horizon` with Rustic's patches applied.
    pub horizon: PathBuf,
    /// `Rustic-MC-launcher/launcher`.
    pub launcher: PathBuf,
    /// The upstream Horizon commit (default: the edition's pin).
    #[serde(default)]
    pub horizon_commit: Option<String>,
}

impl NativeConfig {
    /// The local source, the environment's first.
    pub fn local_source(&self) -> Option<NativeLocalSource> {
        match (
            std::env::var_os("RUSTIC_NATIVE_HORIZON"),
            std::env::var_os("RUSTIC_NATIVE_LAUNCHER"),
        ) {
            (Some(h), Some(l)) => Some(NativeLocalSource {
                horizon: h.into(),
                launcher: l.into(),
                horizon_commit: std::env::var("RUSTIC_NATIVE_HORIZON_COMMIT").ok(),
            }),
            _ => self.local_source.clone(),
        }
    }
}

impl LauncherConfig {
    /// Reads `path`, migrating older versions; the defaults when it doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(CoreError::io(path, e)),
        };
        Self::parse(path, &text)
    }

    /// Parses a configuration's text (`path` names it in errors).
    pub fn parse(path: &Path, text: &str) -> Result<Self> {
        let invalid = |message: String| CoreError::ConfigInvalid {
            path: path.into(),
            message,
        };
        let mut table: toml::Table = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
        let found = table
            .get("version")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0);
        let found = u32::try_from(found).map_err(|_| invalid("bad version".into()))?;
        if found > CONFIG_VERSION {
            return Err(CoreError::ConfigTooNew {
                path: path.into(),
                found,
                supported: CONFIG_VERSION,
            });
        }
        migrate(&mut table, found);
        table.insert(
            "version".into(),
            toml::Value::Integer(CONFIG_VERSION.into()),
        );
        table
            .try_into()
            .map_err(|e: toml::de::Error| invalid(e.to_string()))
    }

    /// Writes the configuration atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        fsutil::write_toml(path, self)
    }

    /// The build source named `id`.
    pub fn build_source(&self, id: &str) -> Option<&BuildSourceConfig> {
        self.builds.sources.iter().find(|s| s.id == id)
    }
}

/// Brings a table written as version `from` up to [`CONFIG_VERSION`].
fn migrate(table: &mut toml::Table, from: u32) {
    if from == 0 {
        // Version 0 (pre-release files) called the client id `azure_client_id` at the top level.
        if let Some(id) = table.remove("azure_client_id") {
            let microsoft = table
                .entry("microsoft")
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            if let Some(microsoft) = microsoft.as_table_mut() {
                microsoft.entry("client_id").or_insert(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("launcher.toml");
        let config = LauncherConfig::load(&path).unwrap();
        assert_eq!(config, LauncherConfig::default());
        config.save(&path).unwrap();
        assert_eq!(LauncherConfig::load(&path).unwrap(), config);
        assert!(!config.privacy.telemetry, "telemetry is off by default");
    }

    #[test]
    fn partial_file_keeps_defaults() {
        let config = LauncherConfig::parse(
            Path::new("x"),
            "version = 1\n[ownership]\nrevalidation_days = 10\n",
        )
        .unwrap();
        assert_eq!(config.ownership.revalidation_days, 10);
        assert_eq!(config.downloads.concurrency, 6);
        assert_eq!(config.native.cache_limit_gib, 30);
    }

    #[test]
    fn native_settings_round_trip() {
        let config = LauncherConfig::parse(
            Path::new("x"),
            "version = 1\n[native]\ncache_limit_gib = 12\n[native.local_source]\nhorizon = \"h\"\nlauncher = \"l\"\n",
        )
        .unwrap();
        assert_eq!(config.native.cache_limit_gib, 12);
        let local = config.native.local_source.clone().unwrap();
        assert_eq!(local.horizon, PathBuf::from("h"));
        assert!(local.horizon_commit.is_none());
    }

    #[test]
    fn migrates_version_zero() {
        let config = LauncherConfig::parse(Path::new("x"), "azure_client_id = \"abc\"\n").unwrap();
        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.microsoft.client_id.as_deref(), Some("abc"));
    }

    #[test]
    fn refuses_newer_versions() {
        let err = LauncherConfig::parse(Path::new("x"), "version = 99\n").unwrap_err();
        assert!(matches!(err, CoreError::ConfigTooNew { found: 99, .. }));
    }

    #[test]
    fn window_is_clamped() {
        let w = OwnershipConfig {
            revalidation_days: 1000,
        };
        assert_eq!(w.window_seconds(), crate::time::days(90));
    }
}
