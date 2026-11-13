/**
 * The IPC contract: every Tauri command and event, with TS types. Source of truth for
 * launcher/docs/IPC.md. Convention: each command takes ONE argument named `args` (a struct, fields
 * snake_case) or none; Rust side: `#[tauri::command] fn name(args: NameArgs) -> Result<T, IpcError>`.
 */
import type {
  AppInfo,
  DownloadTask,
  GameInstallDetection,
  GamePreparation,
  JavaRuntime,
  LauncherUpdate,
  PickPathRequest,
  PrepareProgress,
  RusticServerStatus,
  Settings,
  SettingsPatch,
  SystemInfo,
} from './types/app';
import type {
  Account,
  PageDeepLink,
  DeepLinkEvent,
  MicrosoftBrowserLoginStart,
  MicrosoftLoginPoll,
  MicrosoftLoginStart,
  RusticLoginResult,
  RusticSession,
  RusticSignupRequest,
  RusticSignupResult,
} from './types/auth';
import type {
  ContentCategory,
  ContentDetail,
  ContentKind,
  ContentSearchQuery,
  ContentSearchResult,
  InstallRequest,
  InstallResult,
  InstallTarget,
} from './types/content';
import type {
  BuyDomainRequest,
  BuyDomainResult,
  DnsProviderConnection,
  DnsVerification,
  DomainAvailability,
  DomainBinding,
  DomainTarget,
  OwnedDomain,
  RequiredRecords,
} from './types/dns';
import type {
  GeneratedSshKey,
  HostingInstallProgress,
  HostingProviderInfo,
  VpsConnection,
  VpsConnectionRequest,
  VpsInstallRequest,
  VpsResources,
  VpsTestResult,
} from './types/hosting';
import type {
  CreateInstanceRequest,
  GameExited,
  GameLogLine,
  HorizonVersion,
  ImportInstanceRequest,
  Instance,
  InstalledContent,
  InstanceFolder,
  InstancePatch,
  LaunchOptions,
  LaunchStatus,
  World,
} from './types/instances';
import type { MultiplayerEntry, ServerPing } from './types/multiplayer';
import type {
  NativeBuildProgress,
  NativeCacheInfo,
  NativeOverview,
  NativeStatus,
} from './types/native';
import type {
  Backup,
  BackupSchedule,
  ConsoleLine,
  CreateServerRequest,
  CreateServerResult,
  PlayerAction,
  RawConfig,
  Server,
  ServerAccess,
  ServerConfig,
  ServerDatapack,
  ServerPlayer,
  ServerPlugin,
  ServerState,
  ServerStats,
  StarterPlugin,
} from './types/servers';

type Id = { id: string };
type ServerId = { server_id: string };
type InstanceId = { instance_id: string };

/** name -> [args, result]. `void` args = command takes no argument. */
export interface IpcCommands {
  // ---- app / system ----
  app_info: [void, AppInfo];
  system_info: [void, SystemInfo];
  settings_get: [void, Settings];
  settings_set: [SettingsPatch, Settings];
  java_runtimes_list: [void, JavaRuntime[]];
  /** The `rustic-server` new Rustic Servers run, where it comes from and its version line. */
  rustic_server_status: [void, RusticServerStatus];
  /** Sets the manual override after checking the file answers `--rustic-version` (`invalid_argument`, reason `server.not_rustic_server`, otherwise); `path: null` clears it (automatic). */
  rustic_server_set: [{ path: string | null }, RusticServerStatus];
  app_open_path: [{ path: string }, void];
  app_open_url: [{ url: string }, void];
  /** The `rustic://content/...` or `rustic://server/...` link that started (or reached) the launcher before the UI listened, taken once (null when none). */
  app_take_deep_link: [void, PageDeepLink | null];
  /** A player's skin URL (Mojang's session service, cached); `null` without one. */
  player_skin_url: [{ uuid: string }, string | null];
  app_pick_path: [PickPathRequest, string | null];
  app_check_update: [void, LauncherUpdate];
  app_install_update: [void, void];
  downloads_list: [void, DownloadTask[]];
  download_cancel: [Id, void];
  game_detect_install: [void, GameInstallDetection];
  game_prepare_status: [void, GamePreparation];
  game_prepare: [{ source_path: string | null }, { task_id: string }];

  // ---- auth (game accounts; Rustic identity is cloud + this session mirror) ----
  auth_accounts_list: [void, Account[]];
  auth_microsoft_login_start: [void, MicrosoftLoginStart];
  /** Opens the system browser (auth code + PKCE, loopback); poll with `auth_microsoft_login_poll`. */
  auth_microsoft_browser_login_start: [void, MicrosoftBrowserLoginStart];
  auth_microsoft_login_poll: [{ flow_id: string }, MicrosoftLoginPoll];
  auth_ownership_check: [{ account_id: string }, Account];
  auth_set_active: [{ account_id: string }, Account[]];
  auth_logout: [{ account_id: string }, Account[]];
  auth_rustic_session: [void, RusticSession];
  auth_rustic_login: [void, RusticSession];
  auth_rustic_logout: [void, RusticSession];
  /** Creates a Rustic account; the cloud mails a verification link. */
  auth_rustic_signup: [RusticSignupRequest, RusticSignupResult];
  /** Email + password sign-in (tokens to the OS keychain); `mfa_required` asks for a TOTP code. */
  auth_rustic_login_password: [{ email: string; password: string }, RusticLoginResult];
  auth_rustic_login_mfa: [{ code: string }, RusticSession];
  auth_rustic_resend_verification: [{ email: string }, void];
  /** Short-lived Rustic Cloud access token for the webview (refresh token stays in the keychain). */
  auth_rustic_access_token: [void, { access_token: string; expires_at: string }];

  // ---- Horizon versions & instances ----
  horizon_versions_list: [void, HorizonVersion[]];
  instances_list: [void, Instance[]];
  instance_get: [Id, Instance];
  instance_create: [CreateInstanceRequest, Instance];
  instance_import: [ImportInstanceRequest, { instance: Instance; task_id: string }];
  instance_update: [Id & { patch: InstancePatch }, Instance];
  instance_delete: [Id, void];
  instance_duplicate: [Id, Instance];
  instance_set_version: [Id & { horizon_version: string }, { task_id: string }];
  instance_open_folder: [Id & { folder: InstanceFolder }, void];
  instance_worlds_list: [InstanceId, World[]];
  instance_content_list: [InstanceId & { kind?: ContentKind }, InstalledContent[]];
  instance_content_set_enabled: [
    InstanceId & { content_id: string; enabled: boolean },
    InstalledContent,
  ];
  instance_content_remove: [InstanceId & { content_id: string }, void];
  instance_content_update: [InstanceId & { content_id?: string }, { task_id: string }];
  instance_resource_packs_reorder: [InstanceId & { ordered_ids: string[] }, InstalledContent[]];

  // ---- launch ----
  instance_launch: [InstanceId & { options?: LaunchOptions }, LaunchStatus];
  instance_launch_cancel: [InstanceId, LaunchStatus];
  instance_stop: [InstanceId, LaunchStatus];
  launch_status: [void, LaunchStatus[]];

  // ---- content (Horizon registry, Modrinth, Rustic) ----
  content_search: [ContentSearchQuery, ContentSearchResult];
  content_get: [{ project_id: string; target?: InstallTarget }, ContentDetail];
  content_categories: [{ kind: ContentKind }, ContentCategory[]];
  /** A native mod needs the trust notice accepted for the instance first (`unauthorized`, reason `native.consent_required`), Horizon's registry or Rustic's marketplace as source (`native.source`) and a Rustic edition instance (`native.not_edition`). */
  content_install: [InstallRequest, InstallResult];

  // ---- native mods (compiled on the player's PC, ADR 0157) ----
  /** An instance's native mods, its build, the trust notice and Rust. */
  native_status: [InstanceId, NativeStatus];
  /** Every instance with native mods, read from disk only (no Rust detection): the Profiles page's cards (ADR 0159). Instances without native mods are left out. */
  native_overview: [void, NativeOverview[]];
  /** The player's click on the trust notice: `accept: false` withdraws it. */
  native_consent: [InstanceId & { accept: boolean }, NativeStatus];
  /** Installs the launcher's own Rust (rustup-init, checked against its published SHA-256) as a task (`native-toolchain`). */
  native_toolchain_install: [void, { task_id: string }];
  /** Compiles the instance's client with its native mods as a task (`native-build`, cancellable with `download_cancel`); refuses with `native.consent_required`, `native.toolchain_missing`, `native.not_edition`, `native.running`. */
  native_build: [InstanceId, { task_id: string }];
  native_cache_info: [void, NativeCacheInfo];
  /** Clears the compile cache; the bytes freed (`conflict`, `native.busy`, while a build runs). */
  native_cache_clear: [void, number];
  native_cache_set_limit: [{ cache_limit_gib: number }, NativeCacheInfo];

  // ---- multiplayer list ----
  multiplayer_list: [void, MultiplayerEntry[]];
  multiplayer_add: [{ name: string; address: string }, MultiplayerEntry];
  multiplayer_update: [
    Id & { patch: Partial<Pick<MultiplayerEntry, 'name' | 'address' | 'favorite'>> },
    MultiplayerEntry,
  ];
  multiplayer_remove: [Id, void];
  multiplayer_ping: [{ address: string }, ServerPing];

  // ---- servers (local and remote, same commands) ----
  servers_list: [void, Server[]];
  server_get: [Id, Server];
  server_create: [CreateServerRequest, CreateServerResult];
  server_delete: [Id & { delete_files: boolean }, void];
  server_start: [Id, Server];
  server_stop: [Id, Server];
  server_restart: [Id, Server];
  server_command: [ServerId & { line: string }, void];
  server_console_history: [ServerId & { limit: number }, ConsoleLine[]];
  server_complete: [ServerId & { partial: string }, string[]];
  server_players: [ServerId, ServerPlayer[]];
  server_player_action: [
    ServerId & { player: string; action: PlayerAction; reason?: string },
    ServerPlayer[],
  ];
  server_whitelist_set_enabled: [ServerId & { enabled: boolean }, void];
  server_config_get: [ServerId, ServerConfig];
  server_config_set: [
    ServerId & { values: Record<string, string | number | boolean> },
    ServerConfig,
  ];
  server_config_raw_get: [ServerId, RawConfig];
  server_config_raw_set: [ServerId & { content: string }, RawConfig];
  server_plugins_list: [ServerId, ServerPlugin[]];
  server_plugin_install: [
    ServerId & { project_id?: string; version_id?: string; path?: string },
    ServerPlugin,
  ];
  server_plugin_set_enabled: [ServerId & { plugin_id: string; enabled: boolean }, ServerPlugin];
  server_plugin_update: [ServerId & { plugin_id: string }, ServerPlugin];
  server_plugin_remove: [ServerId & { plugin_id: string }, void];
  server_datapacks_list: [ServerId, ServerDatapack[]];
  server_datapack_set_enabled: [
    ServerId & { datapack_id: string; enabled: boolean },
    ServerDatapack,
  ];
  server_datapack_remove: [ServerId & { datapack_id: string }, void];
  server_backups_list: [ServerId, Backup[]];
  server_backup_create: [ServerId & { note?: string }, Backup];
  server_backup_restore: [ServerId & { backup_id: string }, void];
  server_backup_delete: [ServerId & { backup_id: string }, void];
  server_backup_schedule_get: [ServerId, BackupSchedule];
  server_backup_schedule_set: [ServerId & BackupSchedule, BackupSchedule];
  server_stats_history: [ServerId & { minutes: number }, ServerStats[]];
  server_access_get: [ServerId, ServerAccess];
  server_relay_set: [ServerId & { enabled: boolean; label?: string | null }, ServerAccess];
  server_invite_create: [ServerId, ServerAccess];
  server_invite_revoke: [ServerId, ServerAccess];
  server_upnp_set: [ServerId & { enabled: boolean }, ServerAccess];

  // ---- hosting (Rustic Cloud placeholder, VPS over SSH/SFTP) ----
  hosting_providers: [void, HostingProviderInfo[]];
  hosting_waitlist_join: [{ email: string }, { position: number }];
  hosting_vps_test: [VpsConnectionRequest, VpsTestResult];
  hosting_vps_trust: [
    VpsConnectionRequest & { fingerprint_sha256: string; label?: string },
    VpsConnection,
  ];
  hosting_vps_list: [void, VpsConnection[]];
  hosting_vps_remove: [Id, void];
  hosting_vps_check: [{ connection_id: string }, VpsResources];
  hosting_vps_install: [VpsInstallRequest, CreateServerResult];
  hosting_vps_generate_key: [{ label: string }, GeneratedSshKey];
  servers_starter_plugins: [void, StarterPlugin[]];

  // ---- domains (Porkbun API with the user's keys, or manual for any registrar) ----
  dns_porkbun_status: [void, DnsProviderConnection];
  dns_connect_porkbun: [{ api_key: string; secret_api_key: string }, DnsProviderConnection];
  dns_disconnect: [{ provider: 'porkbun' }, DnsProviderConnection];
  dns_list_domains: [{ provider: 'porkbun' }, OwnedDomain[]];
  dns_check_domain: [{ query: string }, DomainAvailability[]];
  dns_buy_domain: [BuyDomainRequest, BuyDomainResult];
  dns_configure_server: [DomainTarget, DomainBinding];
  dns_status: [ServerId, DomainBinding | null];
  dns_required_records: [DomainTarget, RequiredRecords];
  dns_verify: [DomainTarget, DnsVerification];
  dns_remove_binding: [ServerId, void];
}

/** event name -> payload */
export interface IpcEvents {
  'download-progress': DownloadTask;
  'downloads-changed': DownloadTask[];
  'prepare-progress': PrepareProgress;
  'launch-state': LaunchStatus;
  'game-log': GameLogLine;
  'game-exited': GameExited;
  'instances-changed': Instance[];
  'server-state': { server_id: string; state: ServerState };
  'server-console': ConsoleLine;
  'server-stats': ServerStats;
  'server-players-changed': { server_id: string };
  'servers-changed': Server[];
  'hosting-install': HostingInstallProgress;
  'dns-status': DomainBinding;
  'accounts-changed': Account[];
  'deep-link': DeepLinkEvent;
  /** Each step of a native build (also mirrored in its task's fraction and `detail`). */
  'native-build-progress': NativeBuildProgress;
  /** A native build or Rust's installation ended: statuses may have changed. */
  'native-changed': { instance_id?: string };
}

export type CommandName = keyof IpcCommands;
export type CommandArgs<K extends CommandName> = IpcCommands[K][0];
export type CommandResult<K extends CommandName> = IpcCommands[K][1];
export type EventName = keyof IpcEvents;
