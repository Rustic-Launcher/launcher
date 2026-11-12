//! The IPC wire types, field for field the TypeScript types of `apps/launcher/src/ipc/types/`
//! (launcher/docs/IPC.md). Snake_case fields; tagged unions as the TS discriminants.

use rusticmc_instances::InstanceIcon;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type IsoDate = String;

// ---- app ------------------------------------------------------------------------------------

#[derive(Clone, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub channel: &'static str,
    pub os: &'static str,
    pub arch: String,
    pub native: bool,
    pub data_dir: String,
}

#[derive(Clone, Serialize)]
pub struct SystemInfo {
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub cpu_model: String,
    pub cpu_threads: usize,
}

#[derive(Clone, Serialize)]
pub struct JavaRuntime {
    pub id: String,
    pub path: String,
    pub version: String,
    pub vendor: String,
    pub source: &'static str,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub default_memory_mb: u32,
    pub prepare_java_id: Option<String>,
    pub prepare_java_custom_path: Option<String>,
    pub game_dir: String,
    pub instances_dir: String,
    pub servers_dir: String,
    pub default_resolution: Resolution,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LauncherSettings {
    pub start_on_boot: bool,
    pub start_minimized: bool,
    pub on_game_launch: String,
    pub on_close: String,
    pub auto_update: bool,
    pub concurrent_downloads: u32,
    pub hardware_acceleration: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub telemetry: bool,
    pub crash_reports: bool,
    #[serde(default)]
    pub performance_reports: bool,
    pub share_activity_with_friends: bool,
    pub friend_requests_from: String,
    /// Discord Rich Presence: "Playing Rustic" in the player's Discord profile.
    #[serde(default = "yes")]
    pub discord_presence: bool,
    /// The server's address in that presence (private addresses are never shown).
    #[serde(default = "yes")]
    pub discord_show_server: bool,
}

fn yes() -> bool {
    true
}

impl PrivacySettings {
    pub fn discord(&self) -> crate::discord::Options {
        crate::discord::Options {
            enabled: self.discord_presence,
            show_server: self.discord_show_server,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub game: GameSettings,
    pub launcher: LauncherSettings,
    pub privacy: PrivacySettings,
    pub onboarding_completed: bool,
}

/// `SettingsPatch`: partial sections, merged as JSON objects.
#[derive(Deserialize)]
pub struct SettingsPatch {
    #[serde(default)]
    pub game: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub launcher: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub privacy: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub onboarding_completed: Option<bool>,
}

#[derive(Clone, Serialize)]
pub struct DownloadTask {
    pub id: String,
    pub kind: &'static str,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bps: u64,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct GameInstallDetection {
    pub found: bool,
    pub path: Option<String>,
    pub versions: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct GamePreparation {
    pub state: &'static str,
    pub game_version: String,
    pub prepared_at: Option<IsoDate>,
    pub source_path: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct PrepareProgress {
    pub step: &'static str,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct LauncherUpdate {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct PickFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Deserialize)]
pub struct PickPathRequest {
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub filters: Option<Vec<PickFilter>>,
}

#[derive(Deserialize)]
pub struct PathArgs {
    pub path: String,
}

/// The `rustic-server` program new Rustic Servers run (Settings > Game and instances).
#[derive(Clone, Serialize)]
pub struct RusticServerStatus {
    /// The program found, if any.
    pub path: Option<String>,
    /// Where it was found.
    pub origin: Option<rusticmc_servers::engine::EngineOrigin>,
    /// The manual override from `launcher.toml`, even when that file is gone.
    pub configured: Option<String>,
    /// The build whose folder was searched (the default build).
    pub build: Option<String>,
    /// Its `--rustic-version` line, when it answered.
    pub version: Option<String>,
}

/// `path: None` goes back to the automatic lookup.
#[derive(Deserialize)]
pub struct RusticServerSetArgs {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct UrlArgs {
    pub url: String,
}

#[derive(Deserialize)]
pub struct IdArgs {
    pub id: String,
}

#[derive(Deserialize)]
pub struct GamePrepareArgs {
    #[serde(default)]
    pub source_path: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct TaskRef {
    pub task_id: String,
}

// ---- auth -----------------------------------------------------------------------------------

#[derive(Clone, Serialize)]
pub struct Ownership {
    pub owns_game: bool,
    pub status: &'static str,
    pub checked_at: Option<IsoDate>,
    pub next_check_at: Option<IsoDate>,
}

#[derive(Clone, Serialize)]
pub struct Account {
    pub id: String,
    pub kind: &'static str,
    pub username: String,
    pub uuid: String,
    pub active: bool,
    pub ownership: Option<Ownership>,
    /// The account's current skin (Microsoft accounts; public profile data), for the 3D model.
    pub skin: Option<Skin>,
}

/// A skin texture as the Minecraft profile gives it.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Skin {
    /// `https://textures.minecraft.net/texture/<hash>` (the profile's `http://` made `https://`).
    pub url: String,
    /// The slim (3-pixel arms) model.
    pub slim: bool,
}

impl Skin {
    /// The active skin among a profile's skins (else the first), with an `https` URL.
    pub fn active(skins: &[rusticmc_auth::api::Texture]) -> Option<Skin> {
        let t = skins
            .iter()
            .find(|t| t.state.eq_ignore_ascii_case("ACTIVE"))
            .or_else(|| skins.first())?;
        let url = match t.url.strip_prefix("http://") {
            Some(rest) => format!("https://{rest}"),
            None => t.url.clone(),
        };
        url.starts_with("https://textures.minecraft.net/")
            .then_some(Skin {
                url,
                slim: t
                    .variant
                    .as_deref()
                    .is_some_and(|v| v.eq_ignore_ascii_case("SLIM")),
            })
    }
}

#[derive(Clone, Serialize)]
pub struct MicrosoftLoginStart {
    pub flow_id: String,
    pub verification_uri: String,
    pub user_code: String,
    pub expires_in_s: u64,
}

#[derive(Clone, Serialize)]
pub struct MicrosoftBrowserLoginStart {
    pub flow_id: String,
    pub authorize_url: String,
}

#[derive(Clone, Serialize)]
pub struct MicrosoftLoginPoll {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    /// Why it failed (not in the TS type yet; ignored by older UIs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct RusticSession {
    pub signed_in: bool,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize)]
pub struct FlowArgs {
    pub flow_id: String,
}

#[derive(Deserialize)]
pub struct AccountArgs {
    pub account_id: String,
}

// ---- instances ------------------------------------------------------------------------------

#[derive(Clone, Serialize)]
pub struct HorizonVersion {
    pub id: String,
    pub label: String,
    pub channel: &'static str,
    pub rustic_edition: bool,
    pub released_at: IsoDate,
    pub installed: bool,
    pub recommended: bool,
    pub size_bytes: u64,
}

#[derive(Clone, Serialize)]
pub struct LastServer {
    pub name: String,
    pub address: String,
}

#[derive(Clone, Serialize)]
pub struct InstanceSourceDto {
    pub kind: String,
    pub project_id: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Serialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub icon: InstanceIcon,
    pub horizon_version: String,
    pub horizon_version_label: String,
    pub state: &'static str,
    pub update_available: Option<String>,
    pub mod_count: usize,
    pub resource_pack_count: usize,
    pub shader_count: usize,
    pub datapack_world_count: usize,
    pub playtime_s: u64,
    pub last_played_at: Option<IsoDate>,
    pub last_server: Option<LastServer>,
    pub created_at: IsoDate,
    pub memory_mb: Option<u32>,
    pub game_args: String,
    pub resolution: Option<Resolution>,
    pub folder: String,
    pub source: Option<InstanceSourceDto>,
    pub pinned: bool,
}

#[derive(Deserialize)]
pub struct CreateInstanceRequest {
    pub name: String,
    pub icon: InstanceIcon,
    pub horizon_version: String,
    #[serde(default)]
    pub memory_mb: Option<u32>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(dead_code)] // CurseForge and catalogue imports are not wired yet.
pub enum ImportInstanceRequest {
    Folder {
        path: String,
        #[serde(default)]
        name: Option<String>,
    },
    Mrpack {
        path: String,
        #[serde(default)]
        name: Option<String>,
    },
    CurseforgeZip {
        path: String,
        #[serde(default)]
        name: Option<String>,
    },
    Modpack {
        project_id: String,
        #[serde(default)]
        version_id: Option<String>,
        #[serde(default)]
        name: Option<String>,
    },
}

#[derive(Deserialize, Default)]
pub struct InstancePatchDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<InstanceIcon>,
    /// `null` = back to the default.
    #[serde(default, with = "double_option")]
    pub memory_mb: Option<Option<u32>>,
    #[serde(default)]
    pub game_args: Option<String>,
    #[serde(default, with = "double_option")]
    pub resolution: Option<Option<Resolution>>,
    #[serde(default)]
    pub pinned: Option<bool>,
}

/// Distinguishes a missing field (`None`) from an explicit `null` (`Some(None)`).
mod double_option {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(d).map(Some)
    }
}

#[derive(Deserialize)]
pub struct InstanceUpdateArgs {
    pub id: String,
    pub patch: InstancePatchDto,
}

#[derive(Deserialize)]
pub struct InstanceSetVersionArgs {
    pub id: String,
    pub horizon_version: String,
}

#[derive(Deserialize)]
pub struct InstanceFolderArgs {
    pub id: String,
    pub folder: String,
}

#[derive(Deserialize)]
pub struct InstanceIdArgs {
    pub instance_id: String,
}

#[derive(Deserialize)]
pub struct InstanceContentListArgs {
    pub instance_id: String,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Deserialize)]
pub struct InstanceContentToggleArgs {
    pub instance_id: String,
    pub content_id: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct InstanceContentArgs {
    pub instance_id: String,
    #[serde(default)]
    pub content_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ReorderArgs {
    pub instance_id: String,
    pub ordered_ids: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct InstalledContent {
    pub id: String,
    pub project_id: Option<String>,
    pub kind: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    pub enabled: bool,
    pub source: String,
    pub icon_url: Option<String>,
    pub update_available: Option<String>,
    pub compatible: bool,
    pub incompatibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_id: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct World {
    pub id: String,
    pub name: String,
    pub mode: &'static str,
    pub last_played_at: Option<IsoDate>,
    pub size_bytes: u64,
    pub datapack_count: usize,
}

#[derive(Deserialize, Default)]
pub struct JoinTarget {
    pub address: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct LaunchOptions {
    #[serde(default)]
    pub join: Option<JoinTarget>,
    #[serde(default)]
    pub world_id: Option<String>,
}

#[derive(Deserialize)]
pub struct InstanceLaunchArgs {
    pub instance_id: String,
    #[serde(default)]
    pub options: Option<LaunchOptions>,
}

#[derive(Clone, Serialize)]
pub struct LaunchStatus {
    pub instance_id: String,
    pub phase: &'static str,
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Machine-readable cause of an idle-with-message state (`auth.no_account`, `prepare.failed`...):
    /// the UI words it in the player's language and drops it once it no longer applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<IsoDate>,
}

#[derive(Clone, Serialize)]
pub struct GameLogLine {
    pub instance_id: String,
    pub level: &'static str,
    pub line: String,
}

#[derive(Clone, Serialize)]
pub struct GameExited {
    pub instance_id: String,
    pub code: i32,
    pub crashed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash_report_path: Option<String>,
    pub played_s: u64,
}

// ---- multiplayer ----------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct MultiplayerEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub favorite: bool,
    pub last_joined_at: Option<IsoDate>,
}

#[derive(Deserialize)]
pub struct MultiplayerAddArgs {
    pub name: String,
    pub address: String,
}

#[derive(Deserialize)]
pub struct MultiplayerPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub favorite: Option<bool>,
}

#[derive(Deserialize)]
pub struct MultiplayerUpdateArgs {
    pub id: String,
    pub patch: MultiplayerPatch,
}

#[derive(Deserialize)]
pub struct AddressArgs {
    pub address: String,
}

#[derive(Clone, Serialize)]
pub struct ServerPing {
    pub online: bool,
    pub latency_ms: Option<u64>,
    pub motd: String,
    pub players_online: u64,
    pub players_max: u64,
    pub version: String,
}

// ---- servers --------------------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[allow(dead_code)] // `Cloud`: Rustic Cloud hosting is coming soon.
pub enum ServerHosting {
    Local,
    Vps { connection_id: String, host: String },
    Cloud { region: String },
}

#[derive(Clone, Serialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub icon: InstanceIcon,
    pub engine: &'static str,
    pub horizon_version: String,
    pub horizon_version_label: String,
    pub hosting: ServerHosting,
    pub state: &'static str,
    pub players_online: u64,
    pub max_players: u64,
    pub port: u16,
    pub motd: String,
    pub world_name: String,
    pub memory_mb: u32,
    pub created_at: IsoDate,
    pub last_started_at: Option<IsoDate>,
    pub share_address: Option<String>,
    pub plugin_count: usize,
    pub folder: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorldSetup {
    New {
        seed: Option<String>,
        level_type: String,
        generate_structures: bool,
        gamemode: String,
        difficulty: String,
        hardcore: bool,
    },
    Import {
        path: String,
    },
    FromInstance {
        instance_id: String,
        world_id: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CreateHosting {
    Local,
    Vps { connection_id: String },
}

#[derive(Deserialize)]
pub struct CreateAccess {
    pub relay: bool,
    pub invite: bool,
}

#[derive(Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub icon: InstanceIcon,
    pub engine: String,
    pub horizon_version: String,
    pub hosting: CreateHosting,
    pub world: WorldSetup,
    pub memory_mb: u32,
    pub max_players: u32,
    pub eula_accepted: bool,
    #[serde(default)]
    pub starter_plugins: Vec<String>,
    pub access: CreateAccess,
    #[serde(default)]
    pub start_with_launcher: bool,
}

#[derive(Clone, Serialize)]
pub struct CreateServerResult {
    pub server: Server,
    pub task_id: String,
}

#[derive(Deserialize)]
pub struct ServerDeleteArgs {
    pub id: String,
    pub delete_files: bool,
}

#[derive(Deserialize)]
pub struct ServerIdArgs {
    pub server_id: String,
}

#[derive(Deserialize)]
pub struct ServerCommandArgs {
    pub server_id: String,
    pub line: String,
}

#[derive(Deserialize)]
pub struct ConsoleHistoryArgs {
    pub server_id: String,
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct CompleteArgs {
    pub server_id: String,
    pub partial: String,
}

#[derive(Clone, Serialize)]
pub struct ConsoleLine {
    pub server_id: String,
    pub ts: IsoDate,
    pub level: &'static str,
    pub text: String,
}

#[derive(Clone, Serialize)]
pub struct ServerPlayer {
    pub uuid: String,
    pub name: String,
    pub online: bool,
    pub op: bool,
    pub whitelisted: bool,
    pub banned: bool,
    pub ping_ms: Option<u64>,
    pub joined_at: Option<IsoDate>,
    pub playtime_s: u64,
}

#[derive(Deserialize)]
pub struct PlayerActionArgs {
    pub server_id: String,
    pub player: String,
    pub action: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct WhitelistArgs {
    pub server_id: String,
    pub enabled: bool,
}

#[derive(Clone, Serialize)]
pub struct ConfigField {
    pub key: String,
    pub group: &'static str,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub value: Value,
    pub default: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    pub restart_required: bool,
}

#[derive(Clone, Serialize)]
pub struct ServerConfig {
    pub fields: Vec<ConfigField>,
}

#[derive(Deserialize)]
pub struct ServerConfigSetArgs {
    pub server_id: String,
    pub values: serde_json::Map<String, Value>,
}

#[derive(Clone, Serialize)]
pub struct RawConfig {
    pub path: String,
    pub format: &'static str,
    pub content: String,
}

#[derive(Deserialize)]
pub struct RawConfigSetArgs {
    pub server_id: String,
    pub content: String,
}

#[derive(Clone, Serialize)]
pub struct PluginCapabilities {
    pub network: Value,
    pub filesystem: String,
}

#[derive(Clone, Serialize)]
pub struct ServerPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: String,
    pub authors: Vec<String>,
    pub license: String,
    pub description: String,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub update_available: Option<String>,
    pub capabilities: PluginCapabilities,
    pub project_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PluginInstallArgs {
    pub server_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct PluginToggleArgs {
    pub server_id: String,
    pub plugin_id: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct PluginArgs {
    pub server_id: String,
    pub plugin_id: String,
}

#[derive(Clone, Serialize)]
pub struct ServerDatapack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub description: String,
}

#[derive(Deserialize)]
pub struct DatapackToggleArgs {
    pub server_id: String,
    pub datapack_id: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct DatapackArgs {
    pub server_id: String,
    pub datapack_id: String,
}

#[derive(Clone, Serialize)]
pub struct Backup {
    pub id: String,
    pub created_at: IsoDate,
    pub size_bytes: u64,
    pub kind: &'static str,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct BackupCreateArgs {
    pub server_id: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct BackupArgs {
    pub server_id: String,
    pub backup_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct BackupSchedule {
    pub enabled: bool,
    pub interval_hours: f64,
    pub keep: u32,
}

#[derive(Deserialize)]
pub struct BackupScheduleSetArgs {
    pub server_id: String,
    pub enabled: bool,
    pub interval_hours: f64,
    pub keep: u32,
}

#[derive(Deserialize)]
pub struct StatsHistoryArgs {
    pub server_id: String,
    pub minutes: u64,
}

#[derive(Clone, Serialize)]
pub struct ServerStats {
    pub server_id: String,
    pub ts: IsoDate,
    pub cpu_percent: f64,
    pub ram_used_mb: u64,
    pub ram_max_mb: u64,
    pub tps: f64,
    pub mspt: f64,
    pub players: u64,
}

#[derive(Clone, Serialize)]
pub struct LanAccess {
    pub ip: String,
    pub port: u16,
    pub address: String,
}

#[derive(Clone, Serialize)]
pub struct RelayAccess {
    pub enabled: bool,
    pub hostname: Option<String>,
    pub label: Option<String>,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct InviteAccess {
    pub code: Option<String>,
    pub expires_at: Option<IsoDate>,
}

#[derive(Clone, Serialize)]
pub struct UpnpAccess {
    pub enabled: bool,
    pub status: &'static str,
    pub external_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct ServerAccess {
    pub lan: LanAccess,
    pub relay: RelayAccess,
    pub invite: InviteAccess,
    pub upnp: UpnpAccess,
    pub public_ip: Option<String>,
}

#[derive(Deserialize)]
pub struct RelaySetArgs {
    pub server_id: String,
    pub enabled: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Deserialize)]
pub struct UpnpSetArgs {
    pub server_id: String,
    pub enabled: bool,
}

#[derive(Clone, Serialize)]
pub struct StarterPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: PluginCapabilities,
    pub recommended: bool,
}

// ---- hosting --------------------------------------------------------------------------------

#[derive(Clone, Serialize)]
pub struct HostingProviderInfo {
    pub id: &'static str,
    pub available: bool,
    pub coming_soon: bool,
}

#[derive(Deserialize)]
pub struct WaitlistArgs {
    #[allow(dead_code)] // sent to Rustic Cloud once its client is wired
    pub email: String,
}

// ---- dns ------------------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Money {
    pub amount_minor: u64,
    pub currency: String,
}

#[derive(Clone, Serialize)]
pub struct DnsProviderConnection {
    pub provider: &'static str,
    pub connected: bool,
    pub account_label: Option<String>,
    pub connected_at: Option<IsoDate>,
}

#[derive(Deserialize)]
pub struct ConnectPorkbunArgs {
    pub api_key: String,
    pub secret_api_key: String,
}

#[derive(Deserialize)]
pub struct ProviderArgs {
    pub provider: String,
}

#[derive(Clone, Serialize)]
pub struct OwnedDomain {
    pub domain: String,
    pub expires_at: IsoDate,
    pub auto_renew: bool,
    pub api_access: bool,
}

#[derive(Deserialize)]
pub struct QueryArgs {
    pub query: String,
}

#[derive(Clone, Serialize)]
pub struct DomainAvailability {
    pub domain: String,
    pub available: bool,
    pub premium: bool,
    pub price: Option<Money>,
    pub renewal_price: Option<Money>,
}

#[derive(Deserialize)]
pub struct BuyDomainRequest {
    pub domain: String,
    pub years: u32,
    pub expected_price: Money,
    pub agree_registrar_terms: bool,
    pub authorize_order: bool,
}

#[derive(Clone, Serialize)]
pub struct BuyDomainResult {
    pub order_id: String,
    pub domain: String,
    pub charged: Money,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DnsRecordDto {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub host: String,
    pub fqdn: String,
    pub value: String,
    pub ttl: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Clone, Serialize)]
pub struct RequiredRecords {
    pub fqdn: String,
    pub hosting_mode: &'static str,
    pub records: Vec<DnsRecordDto>,
}

#[derive(Clone, Serialize)]
pub struct RecordVerification {
    pub record_id: String,
    pub status: &'static str,
    pub found: Vec<String>,
    pub resolvers_ok: usize,
    pub resolvers_total: usize,
}

#[derive(Clone, Serialize)]
pub struct DnsVerification {
    pub fqdn: String,
    pub checked_at: IsoDate,
    pub records: Vec<RecordVerification>,
    pub all_ok: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DomainBinding {
    pub server_id: String,
    pub domain: String,
    pub subdomain: Option<String>,
    pub fqdn: String,
    pub provider: String,
    pub status: String,
    pub records: Vec<DnsRecordDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub verified_at: Option<IsoDate>,
}

#[derive(Deserialize)]
pub struct DomainTarget {
    pub server_id: String,
    pub domain: String,
    #[serde(default)]
    pub subdomain: Option<String>,
}

#[cfg(test)]
mod skin_tests {
    use super::Skin;
    use rusticmc_auth::api::Texture;

    fn texture(url: &str, state: &str, variant: Option<&str>) -> Texture {
        Texture {
            id: "t".into(),
            url: url.into(),
            state: state.into(),
            variant: variant.map(Into::into),
            alias: None,
        }
    }

    #[test]
    fn the_active_skin_with_an_https_url_and_its_model() {
        let skins = [
            texture(
                "http://textures.minecraft.net/texture/old",
                "INACTIVE",
                None,
            ),
            texture(
                "http://textures.minecraft.net/texture/abc",
                "ACTIVE",
                Some("SLIM"),
            ),
        ];
        assert_eq!(
            Skin::active(&skins),
            Some(Skin {
                url: "https://textures.minecraft.net/texture/abc".into(),
                slim: true,
            })
        );
        let classic = [texture(
            "https://textures.minecraft.net/texture/x",
            "ACTIVE",
            Some("CLASSIC"),
        )];
        assert!(!Skin::active(&classic).unwrap().slim);
    }

    #[test]
    fn no_skin_or_a_foreign_host_gives_none() {
        assert_eq!(Skin::active(&[]), None);
        let foreign = [texture("https://example.com/skin.png", "ACTIVE", None)];
        assert_eq!(Skin::active(&foreign), None);
    }
}
