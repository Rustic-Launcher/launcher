//! The app's state: every manager, built once at start and shared by the commands.

use crate::cloud_link::{ApiCloud, api_base};
use crate::dto::{
    ConsoleLine, GameSettings, LaunchStatus, LauncherSettings, PrivacySettings, Resolution,
    Settings,
};
use crate::tasks::Tasks;
use rusticmc_auth::{AccountManager, AuthApi, AuthConfig, RusticAccount};
use rusticmc_instances::{BuildManager, BuildSource, InstanceManager, InstanceStore};
use rusticmc_launcher_core::{DownloadManager, IpcError, LauncherConfig, LauncherPaths, fsutil};
use rusticmc_servers::ServerManager;
use rusticmc_servers::porkbun::PorkbunDns;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use tauri::AppHandle;

/// A browser or device-code sign-in's outcome, polled by `auth_microsoft_login_poll`.
pub enum LoginOutcome {
    Pending,
    Done(Result<rusticmc_auth::AccountView, rusticmc_auth::AuthError>),
}

pub struct AppState {
    pub app: AppHandle,
    pub paths: LauncherPaths,
    pub config: RwLock<LauncherConfig>,
    pub settings: Mutex<Settings>,
    pub downloads: DownloadManager,
    pub accounts: Arc<AccountManager>,
    pub rustic: Arc<RusticAccount>,
    /// The Rustic Cloud API (sign-up and the account's other calls).
    pub cloud: ApiCloud,
    /// A pending Rustic login waiting for its TOTP code (never leaves Rust).
    pub rustic_mfa: Mutex<Option<rusticmc_launcher_core::Secret>>,
    pub instances: Arc<InstanceManager>,
    pub servers: Arc<ServerManager>,
    pub porkbun: Arc<PorkbunDns>,
    pub tasks: Arc<Tasks>,
    pub logins: Mutex<HashMap<String, LoginOutcome>>,
    pub launches: Mutex<HashMap<String, LaunchStatus>>,
    pub console: Mutex<HashMap<String, VecDeque<ConsoleLine>>>,
    pub invites: Mutex<HashMap<String, String>>,
    pub preparing: Mutex<Option<String>>,
    /// Local servers behind the hosting trait.
    pub local: Arc<rusticmc_servers::LocalServers>,
    /// VPS connection profiles and their servers (`<data>/hosting/registry.json`).
    pub registry: Arc<rusticmc_hosting::registry::Registry>,
    pub known_hosts: Arc<rusticmc_hosting::KnownHosts>,
    pub hosting_secrets: Arc<dyn rusticmc_hosting::SecretStore>,
    pub remotes: Mutex<HashMap<String, Arc<rusticmc_hosting::RemoteSshProvider>>>,
    /// Discord Rich Presence.
    pub discord: crate::discord::Discord,
    /// Other players' skin URLs (their heads in lists).
    pub skins: crate::skins::SkinCache,
    /// The launcher's HTTP client (proxy, TLS and user agent settings).
    pub http: rusticmc_launcher_core::reqwest::Client,
}

/// What the launcher shows of a VPS server (the registry keeps only its reference).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerMeta {
    pub profile: String,
    pub name: String,
    pub display_name: String,
    pub icon: rusticmc_instances::InstanceIcon,
    pub rustic: bool,
    pub horizon_version: String,
    pub port: u16,
    pub world: String,
    pub max_players: u32,
    pub memory_mb: u32,
    pub created_at: u64,
}

pub type State<'a> = tauri::State<'a, Arc<AppState>>;
pub type Res<T> = Result<T, IpcError>;

fn default_settings(paths: &LauncherPaths, config: &LauncherConfig) -> Settings {
    Settings {
        game: GameSettings {
            default_memory_mb: 4096,
            prepare_java_id: None,
            prepare_java_custom_path: config
                .java
                .preferred_home
                .as_ref()
                .map(|p| p.display().to_string()),
            game_dir: config
                .game_files
                .minecraft_dir
                .clone()
                .or_else(rusticmc_instances::java::default_minecraft_dir)
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            instances_dir: paths.instances().display().to_string(),
            servers_dir: paths.servers().display().to_string(),
            default_resolution: Resolution {
                width: 1280,
                height: 720,
                fullscreen: false,
            },
        },
        launcher: LauncherSettings {
            start_on_boot: false,
            start_minimized: false,
            on_game_launch: "keep_open".into(),
            on_close: "quit".into(),
            auto_update: true,
            concurrent_downloads: config.downloads.concurrency as u32,
            hardware_acceleration: true,
        },
        privacy: PrivacySettings {
            telemetry: config.privacy.telemetry,
            crash_reports: false,
            performance_reports: false,
            share_activity_with_friends: false,
            friend_requests_from: "friends_of_friends".into(),
            discord_presence: true,
            discord_show_server: true,
        },
        onboarding_completed: false,
    }
}

impl AppState {
    pub fn new(app: AppHandle, paths: LauncherPaths) -> Result<Self, IpcError> {
        paths.ensure()?;
        let config = LauncherConfig::load(&paths.config_file())?;
        let http = rusticmc_launcher_core::http::client()?;
        let downloads = DownloadManager::with_client(
            http.clone(),
            paths.download_cache(),
            config.downloads.concurrency,
            config.downloads.retries,
        );
        let store = rusticmc_auth::store::default_store(rusticmc_launcher_core::APP_ID);
        let api = AuthApi::new(http.clone(), AuthConfig::from_launcher(&config));
        let accounts = Arc::new(AccountManager::new(
            paths.accounts_file(),
            store.clone(),
            api,
        ));
        let cloud = ApiCloud::new(api_base(&config.cloud.api_base), http.clone());
        let rustic = Arc::new(RusticAccount::new(store.clone(), Arc::new(cloud.clone())));
        let builds = BuildManager::new(paths.builds(), http.clone(), downloads.clone());
        let instances = Arc::new(InstanceManager::new(
            InstanceStore::new(paths.instances()),
            builds.clone(),
            paths.prepared(),
        ));
        let servers = Arc::new(ServerManager::new(
            paths.servers(),
            http.clone(),
            builds,
            paths.prepared(),
            config.rustic_server.executable.clone(),
        ));
        let porkbun = Arc::new(PorkbunDns::new(
            http.clone(),
            config.dns.porkbun_api.clone(),
            store.clone(),
        ));
        let hosting_dir = paths.data().join("hosting");
        std::fs::create_dir_all(&hosting_dir).map_err(|e| IpcError::internal(e.to_string()))?;
        let registry = Arc::new(
            rusticmc_hosting::registry::Registry::open(hosting_dir.join("registry.json"))
                .map_err(|e| IpcError::internal(e.to_string()))?,
        );
        let known_hosts = Arc::new(
            rusticmc_hosting::KnownHosts::open(hosting_dir.join("known_hosts.json"))
                .map_err(|e| IpcError::internal(e.to_string()))?,
        );
        let local = {
            let builds = instances.builds.clone();
            let default_source = config.builds.default_source.clone();
            Arc::new(rusticmc_servers::LocalServers {
                manager: servers.clone(),
                default_build: Arc::new(move || {
                    let installed = builds.installed();
                    installed
                        .iter()
                        .find(|b| b.source == default_source)
                        .or_else(|| installed.first())
                        .map(|b| b.version.clone())
                }),
            })
        };
        let settings_file = paths.config_dir().join("settings.json");
        let settings = fsutil::read_json::<Settings>(&settings_file)
            .ok()
            .flatten()
            .unwrap_or_else(|| default_settings(&paths, &config));
        let discord = crate::discord::Discord::start(settings.privacy.discord());
        Ok(Self {
            discord,
            skins: Default::default(),
            http: http.clone(),
            tasks: Arc::new(Tasks::new(app.clone())),
            app,
            paths,
            config: RwLock::new(config),
            settings: Mutex::new(settings),
            downloads,
            accounts,
            rustic,
            cloud,
            rustic_mfa: Mutex::new(None),
            instances,
            servers,
            porkbun,
            logins: Mutex::new(HashMap::new()),
            launches: Mutex::new(HashMap::new()),
            console: Mutex::new(HashMap::new()),
            invites: Mutex::new(HashMap::new()),
            preparing: Mutex::new(None),
            local,
            registry,
            known_hosts,
            hosting_secrets: Arc::new(rusticmc_hosting::KeyringStore::new()),
            remotes: Mutex::new(HashMap::new()),
        })
    }

    pub fn config(&self) -> LauncherConfig {
        self.config.read().expect("config").clone()
    }

    pub fn save_config(&self, config: LauncherConfig) -> Res<()> {
        config.save(&self.paths.config_file())?;
        *self.config.write().expect("config") = config;
        Ok(())
    }

    pub fn save_settings(&self, settings: &Settings) -> Res<()> {
        fsutil::write_json(&self.paths.config_dir().join("settings.json"), settings)?;
        Ok(())
    }

    fn metas_path(&self) -> std::path::PathBuf {
        self.paths.data().join("hosting").join("servers-meta.json")
    }

    pub fn server_metas(&self) -> Vec<ServerMeta> {
        let refs = self.registry.snapshot().servers;
        let metas: Vec<ServerMeta> = fsutil::read_json(&self.metas_path())
            .ok()
            .flatten()
            .unwrap_or_default();
        // Servers the registry knows but the launcher has no record of still show.
        let mut out = metas.clone();
        for r in refs {
            if !metas
                .iter()
                .any(|m| m.profile == r.target && m.name == r.name)
            {
                out.push(ServerMeta {
                    profile: r.target.clone(),
                    name: r.name.clone(),
                    display_name: r.name.clone(),
                    icon: rusticmc_instances::InstanceIcon::default(),
                    rustic: false,
                    horizon_version: String::new(),
                    port: 25565,
                    world: "world".into(),
                    max_players: 20,
                    memory_mb: 0,
                    created_at: 0,
                });
            }
        }
        out
    }

    pub fn server_meta(&self, profile: &str, name: &str) -> Option<ServerMeta> {
        self.server_metas()
            .into_iter()
            .find(|m| m.profile == profile && m.name == name)
    }

    pub fn add_server_meta(&self, meta: ServerMeta) {
        let mut metas: Vec<ServerMeta> = fsutil::read_json(&self.metas_path())
            .ok()
            .flatten()
            .unwrap_or_default();
        metas.retain(|m| !(m.profile == meta.profile && m.name == meta.name));
        metas.push(meta);
        let _ = fsutil::write_json(&self.metas_path(), &metas);
    }

    pub fn remove_server_meta(&self, profile: &str, name: &str) {
        let mut metas: Vec<ServerMeta> = fsutil::read_json(&self.metas_path())
            .ok()
            .flatten()
            .unwrap_or_default();
        metas.retain(|m| !(m.profile == profile && m.name == name));
        let _ = fsutil::write_json(&self.metas_path(), &metas);
        let _ = self
            .registry
            .remove_server(&rusticmc_hosting::model::ServerRef {
                provider: rusticmc_hosting::model::ProviderKind::RemoteSsh,
                target: profile.to_owned(),
                name: name.to_owned(),
            });
    }

    /// Every configured build source with its keys.
    pub fn sources(&self) -> Vec<BuildSource> {
        self.config()
            .builds
            .sources
            .into_iter()
            .filter_map(|c| match BuildSource::new(c.clone()) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::warn!(source = %c.id, error = %e, "build source ignored");
                    None
                }
            })
            .collect()
    }
}
