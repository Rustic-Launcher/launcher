# Rustic Launcher IPC (UI ↔ Tauri)

Owners: frontend (TypeScript client + browser mock, ADR 0024) and desktop (Rust implementation in
`apps/launcher/src-tauri`). **Generated** from `apps/launcher/src/ipc/commands.ts` and
`src/ipc/types/*.ts` by `pnpm --filter @rustic/launcher gen:ipc-doc`: edit the TypeScript, then regenerate.

119 commands, 16 events.

## Conventions

- **Commands** are `snake_case`. Each takes **one argument named `args`** (a struct whose fields are
  `snake_case`) or no argument at all. The TS client calls `invoke(name, { args })`. Rust side:

  ```rust
  #[derive(serde::Deserialize)]
  pub struct ServerCommandArgs { pub server_id: String, pub line: String }

  #[tauri::command]
  async fn server_command(args: ServerCommandArgs, state: tauri::State<'_, App>) -> Result<(), IpcError> { … }
  ```

  Argument struct names in this document: `<CommandInPascalCase>Args` (e.g. `ServerCommandArgs`). Results and
  payloads serialise with serde defaults (`snake_case` fields, `#[serde(tag = "kind")]` /
  `#[serde(tag = "mode")]` / `#[serde(tag = "status")]` for the tagged unions shown in the types below,
  `#[serde(rename_all = "snake_case")]` for string enums; event names and the `in-game` presence are kebab-case).
- **Errors**: every command returns `Result<T, IpcError>`; `IpcError = { code, message, details? }` with `code` one of
  `not_found | invalid_argument | conflict | cancelled | network | unauthorized | ownership_required |
  parental_block | unavailable | internal`. Messages are developer-facing; the UI localises by `code`.
- **Events** are `kebab-case`, emitted with `app.emit(name, payload)` to all windows.
- **Long work** (downloads, installs, preparation, VPS install, DNS propagation) returns immediately with a `task_id`
  and reports through events (`download-progress`, `downloads-changed`, `prepare-progress`, `hosting-install`,
  `dns-status`); `download_cancel` cancels by task id.
- **Secrets** (Porkbun keys, SSH passwords/passphrases) travel only inside IPC (never on a command line) and are
  stored in the OS keychain by the backend; they are never returned to the UI (only masked labels).
- Dates are ISO-8601 UTC strings; sizes in bytes; memory in MiB; money in minor units with an ISO currency.
- The browser mock (`src/ipc/mock`) implements every command below with realistic data so the UI runs without Tauri.

Helper aliases used in the tables:

```ts
type Id = { id: string };
type ServerId = { server_id: string };
type InstanceId = { instance_id: string };
```

## Commands

### App / system

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `app_info` | — | `AppInfo` |
| `system_info` | — | `SystemInfo` |
| `settings_get` | — | `Settings` |
| `settings_set` | `SettingsPatch` | `Settings` |
| `java_runtimes_list` | — | `JavaRuntime[]` |
| `rustic_server_status`<br>The `rustic-server` new Rustic Servers run, where it comes from and its version line. | — | `RusticServerStatus` |
| `rustic_server_set`<br>Sets the manual override after checking the file answers `--rustic-version` (`invalid_argument`, reason `server.not_rustic_server`, otherwise); `path: null` clears it (automatic). | `{ path: string \| null }` | `RusticServerStatus` |
| `app_open_path` | `{ path: string }` | `void` |
| `app_open_url` | `{ url: string }` | `void` |
| `app_pick_path` | `PickPathRequest` | `string \| null` |
| `app_check_update` | — | `LauncherUpdate` |
| `app_install_update` | — | `void` |
| `downloads_list` | — | `DownloadTask[]` |
| `download_cancel` | `Id` | `void` |
| `game_detect_install` | — | `GameInstallDetection` |
| `game_prepare_status` | — | `GamePreparation` |
| `game_prepare` | `{ source_path: string \| null }` | `{ task_id: string }` |

### Auth (game accounts; Rustic identity is cloud + this session mirror)

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `auth_accounts_list` | — | `Account[]` |
| `auth_microsoft_login_start` | — | `MicrosoftLoginStart` |
| `auth_microsoft_browser_login_start`<br>Opens the system browser (auth code + PKCE, loopback); poll with `auth_microsoft_login_poll`. | — | `MicrosoftBrowserLoginStart` |
| `auth_microsoft_login_poll` | `{ flow_id: string }` | `MicrosoftLoginPoll` |
| `auth_ownership_check` | `{ account_id: string }` | `Account` |
| `auth_set_active` | `{ account_id: string }` | `Account[]` |
| `auth_logout` | `{ account_id: string }` | `Account[]` |
| `auth_rustic_session` | — | `RusticSession` |
| `auth_rustic_login` | — | `RusticSession` |
| `auth_rustic_logout` | — | `RusticSession` |
| `auth_rustic_signup`<br>Creates a Rustic account; the cloud mails a verification link. | `RusticSignupRequest` | `RusticSignupResult` |
| `auth_rustic_login_password`<br>Email + password sign-in (tokens to the OS keychain); `mfa_required` asks for a TOTP code. | `{ email: string; password: string }` | `RusticLoginResult` |
| `auth_rustic_login_mfa` | `{ code: string }` | `RusticSession` |
| `auth_rustic_resend_verification` | `{ email: string }` | `void` |
| `auth_rustic_access_token`<br>Short-lived Rustic Cloud access token for the webview (refresh token stays in the keychain). | — | `{ access_token: string; expires_at: string }` |

### Horizon versions & instances

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `horizon_versions_list` | — | `HorizonVersion[]` |
| `instances_list` | — | `Instance[]` |
| `instance_get` | `Id` | `Instance` |
| `instance_create` | `CreateInstanceRequest` | `Instance` |
| `instance_import` | `ImportInstanceRequest` | `{ instance: Instance; task_id: string }` |
| `instance_update` | `Id & { patch: InstancePatch }` | `Instance` |
| `instance_delete` | `Id` | `void` |
| `instance_duplicate` | `Id` | `Instance` |
| `instance_set_version` | `Id & { horizon_version: string }` | `{ task_id: string }` |
| `instance_open_folder` | `Id & { folder: InstanceFolder }` | `void` |
| `instance_worlds_list` | `InstanceId` | `World[]` |
| `instance_content_list` | `InstanceId & { kind?: ContentKind }` | `InstalledContent[]` |
| `instance_content_set_enabled` | `InstanceId & { content_id: string; enabled: boolean }` | `InstalledContent,` |
| `instance_content_remove` | `InstanceId & { content_id: string }` | `void` |
| `instance_content_update` | `InstanceId & { content_id?: string }` | `{ task_id: string }` |
| `instance_resource_packs_reorder` | `InstanceId & { ordered_ids: string[] }` | `InstalledContent[]` |

### Launch

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `instance_launch` | `InstanceId & { options?: LaunchOptions }` | `LaunchStatus` |
| `instance_launch_cancel` | `InstanceId` | `LaunchStatus` |
| `instance_stop` | `InstanceId` | `LaunchStatus` |
| `launch_status` | — | `LaunchStatus[]` |

### Content (Horizon registry, Modrinth, Rustic)

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `content_search` | `ContentSearchQuery` | `ContentSearchResult` |
| `content_get` | `{ project_id: string; target?: InstallTarget }` | `ContentDetail` |
| `content_categories` | `{ kind: ContentKind }` | `ContentCategory[]` |
| `content_install` | `InstallRequest` | `InstallResult` |

### Multiplayer list

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `multiplayer_list` | — | `MultiplayerEntry[]` |
| `multiplayer_add` | `{ name: string; address: string }` | `MultiplayerEntry` |
| `multiplayer_update` | `Id & { patch: Partial<Pick<MultiplayerEntry, 'name' \| 'address' \| 'favorite'>> }` | `MultiplayerEntry,` |
| `multiplayer_remove` | `Id` | `void` |
| `multiplayer_ping` | `{ address: string }` | `ServerPing` |

### Servers (local and remote, same commands)

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `servers_list` | — | `Server[]` |
| `server_get` | `Id` | `Server` |
| `server_create` | `CreateServerRequest` | `CreateServerResult` |
| `server_delete` | `Id & { delete_files: boolean }` | `void` |
| `server_start` | `Id` | `Server` |
| `server_stop` | `Id` | `Server` |
| `server_restart` | `Id` | `Server` |
| `server_command` | `ServerId & { line: string }` | `void` |
| `server_console_history` | `ServerId & { limit: number }` | `ConsoleLine[]` |
| `server_complete` | `ServerId & { partial: string }` | `string[]` |
| `server_players` | `ServerId` | `ServerPlayer[]` |
| `server_player_action` | `ServerId & { player: string; action: PlayerAction; reason?: string }` | `ServerPlayer[],` |
| `server_whitelist_set_enabled` | `ServerId & { enabled: boolean }` | `void` |
| `server_config_get` | `ServerId` | `ServerConfig` |
| `server_config_set` | `ServerId & { values: Record<string, string \| number \| boolean> }` | `ServerConfig,` |
| `server_config_raw_get` | `ServerId` | `RawConfig` |
| `server_config_raw_set` | `ServerId & { content: string }` | `RawConfig` |
| `server_plugins_list` | `ServerId` | `ServerPlugin[]` |
| `server_plugin_install` | `ServerId & { project_id?: string; version_id?: string; path?: string }` | `ServerPlugin,` |
| `server_plugin_set_enabled` | `ServerId & { plugin_id: string; enabled: boolean }` | `ServerPlugin` |
| `server_plugin_update` | `ServerId & { plugin_id: string }` | `ServerPlugin` |
| `server_plugin_remove` | `ServerId & { plugin_id: string }` | `void` |
| `server_datapacks_list` | `ServerId` | `ServerDatapack[]` |
| `server_datapack_set_enabled` | `ServerId & { datapack_id: string; enabled: boolean }` | `ServerDatapack,` |
| `server_datapack_remove` | `ServerId & { datapack_id: string }` | `void` |
| `server_backups_list` | `ServerId` | `Backup[]` |
| `server_backup_create` | `ServerId & { note?: string }` | `Backup` |
| `server_backup_restore` | `ServerId & { backup_id: string }` | `void` |
| `server_backup_delete` | `ServerId & { backup_id: string }` | `void` |
| `server_backup_schedule_get` | `ServerId` | `BackupSchedule` |
| `server_backup_schedule_set` | `ServerId & BackupSchedule` | `BackupSchedule` |
| `server_stats_history` | `ServerId & { minutes: number }` | `ServerStats[]` |
| `server_access_get` | `ServerId` | `ServerAccess` |
| `server_relay_set` | `ServerId & { enabled: boolean; label?: string \| null }` | `ServerAccess` |
| `server_invite_create` | `ServerId` | `ServerAccess` |
| `server_invite_revoke` | `ServerId` | `ServerAccess` |
| `server_upnp_set` | `ServerId & { enabled: boolean }` | `ServerAccess` |

### Hosting (Rustic Cloud placeholder, VPS over SSH/SFTP)

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `hosting_providers` | — | `HostingProviderInfo[]` |
| `hosting_waitlist_join` | `{ email: string }` | `{ position: number }` |
| `hosting_vps_test` | `VpsConnectionRequest` | `VpsTestResult` |
| `hosting_vps_trust` | `VpsConnectionRequest & { fingerprint_sha256: string; label?: string }` | `VpsConnection,` |
| `hosting_vps_list` | — | `VpsConnection[]` |
| `hosting_vps_remove` | `Id` | `void` |
| `hosting_vps_check` | `{ connection_id: string }` | `VpsResources` |
| `hosting_vps_install` | `VpsInstallRequest` | `CreateServerResult` |
| `hosting_vps_generate_key` | `{ label: string }` | `GeneratedSshKey` |
| `servers_starter_plugins` | — | `StarterPlugin[]` |

### Domains (Porkbun API with the user's keys, or manual for any registrar)

| Command | `args` (Rust: `<Name>Args`) | Result |
|---|---|---|
| `dns_porkbun_status` | — | `DnsProviderConnection` |
| `dns_connect_porkbun` | `{ api_key: string; secret_api_key: string }` | `DnsProviderConnection` |
| `dns_disconnect` | `{ provider: 'porkbun' }` | `DnsProviderConnection` |
| `dns_list_domains` | `{ provider: 'porkbun' }` | `OwnedDomain[]` |
| `dns_check_domain` | `{ query: string }` | `DomainAvailability[]` |
| `dns_buy_domain` | `BuyDomainRequest` | `BuyDomainResult` |
| `dns_configure_server` | `DomainTarget` | `DomainBinding` |
| `dns_status` | `ServerId` | `DomainBinding \| null` |
| `dns_required_records` | `DomainTarget` | `RequiredRecords` |
| `dns_verify` | `DomainTarget` | `DnsVerification` |
| `dns_remove_binding` | `ServerId` | `void` |

## Events

| Event | Payload |
|---|---|
| `download-progress` | `DownloadTask` |
| `downloads-changed` | `DownloadTask[]` |
| `prepare-progress` | `PrepareProgress` |
| `launch-state` | `LaunchStatus` |
| `game-log` | `GameLogLine` |
| `game-exited` | `GameExited` |
| `instances-changed` | `Instance[]` |
| `server-state` | `{ server_id: string; state: ServerState }` |
| `server-console` | `ConsoleLine` |
| `server-stats` | `ServerStats` |
| `server-players-changed` | `{ server_id: string }` |
| `servers-changed` | `Server[]` |
| `hosting-install` | `HostingInstallProgress` |
| `dns-status` | `DomainBinding` |
| `accounts-changed` | `Account[]` |
| `deep-link` | `DeepLinkEvent` |

## Types

Verbatim from `apps/launcher/src/ipc/types/`. Rust mirrors use the same field names.

### app.ts

```ts
/** App, system, settings, downloads, game preparation. Mirrors launcher/docs/IPC.md. */

/** ISO-8601 date-time string (UTC). */
export type IsoDate = string;

/** Every command rejects with this shape (serialised `rusticmc_launcher_core::IpcError`). */
export interface IpcError {
  code:
    | 'not_found'
    | 'invalid_argument'
    | 'conflict'
    | 'cancelled'
    | 'network'
    | 'unauthorized'
    | 'ownership_required'
    | 'parental_block'
    | 'unavailable'
    | 'internal';
  message: string;
  details?: Record<string, unknown>;
}

export interface AppInfo {
  version: string;
  channel: 'stable' | 'beta';
  os: 'windows' | 'macos' | 'linux';
  arch: string;
  /** False in the browser mock. */
  native: boolean;
  data_dir: string;
}

export interface SystemInfo {
  total_memory_mb: number;
  available_memory_mb: number;
  cpu_model: string;
  cpu_threads: number;
}

export interface JavaRuntime {
  id: string;
  path: string;
  version: string;
  vendor: string;
  source: 'bundled' | 'system' | 'custom';
}

export interface GameSettings {
  /** Default RAM for instances without their own value. */
  default_memory_mb: number;
  /** Java runtime used ONLY to prepare game files from the user's own install (horizon-prepare). */
  prepare_java_id: string | null;
  prepare_java_custom_path: string | null;
  game_dir: string;
  instances_dir: string;
  servers_dir: string;
  default_resolution: { width: number; height: number; fullscreen: boolean };
}

export interface LauncherSettings {
  start_on_boot: boolean;
  start_minimized: boolean;
  on_game_launch: 'keep_open' | 'minimize' | 'hide' | 'close';
  on_close: 'quit' | 'tray';
  auto_update: boolean;
  concurrent_downloads: number;
  hardware_acceleration: boolean;
}

export interface PrivacySettings {
  /** Off by default; consent asked during onboarding (GDPR). */
  telemetry: boolean;
  crash_reports: boolean;
  /** Anonymous performance reports (frame and load times). Off by default. */
  performance_reports?: boolean;
  share_activity_with_friends: boolean;
  friend_requests_from: 'everyone' | 'friends_of_friends' | 'nobody';
  /** Discord Rich Presence ("Playing Rustic" in the player's Discord profile). On by default. */
  discord_presence?: boolean;
  /** The server's address in that presence; private addresses are never shown. On by default. */
  discord_show_server?: boolean;
}

export interface Settings {
  game: GameSettings;
  launcher: LauncherSettings;
  privacy: PrivacySettings;
  onboarding_completed: boolean;
}

export type SettingsPatch = {
  game?: Partial<GameSettings>;
  launcher?: Partial<LauncherSettings>;
  privacy?: Partial<PrivacySettings>;
  onboarding_completed?: boolean;
};

export type DownloadKind =
  'game' | 'content' | 'modpack' | 'server' | 'java' | 'update' | 'prepare';

export interface DownloadTask {
  id: string;
  kind: DownloadKind;
  /** Already localised by the UI from `label_key` + `label_args` when present. */
  label: string;
  instance_id?: string;
  server_id?: string;
  received_bytes: number;
  total_bytes: number | null;
  speed_bps: number;
  state: 'queued' | 'running' | 'paused' | 'done' | 'failed' | 'cancelled';
  error?: string;
}

export interface GameInstallDetection {
  /** A legitimate install of the game was found on this computer. */
  found: boolean;
  path: string | null;
  versions: string[];
}

export interface GamePreparation {
  state: 'missing' | 'preparing' | 'ready' | 'outdated';
  game_version: string;
  prepared_at: IsoDate | null;
  source_path: string | null;
}

export interface PrepareProgress {
  step: 'detect' | 'download' | 'verify' | 'compile_exporters' | 'export' | 'index' | 'done';
  progress: number;
  message?: string;
}

export interface LauncherUpdate {
  available: boolean;
  version: string | null;
  notes: string | null;
}

export interface PickPathRequest {
  kind: 'file' | 'folder';
  title?: string;
  filters?: { name: string; extensions: string[] }[];
}

/**
 * Where the `rustic-server` program was found (launcher ADR 0152), in lookup order: the server's
 * own engine, the manual override (Settings), the build folder, beside the launcher, the PATH.
 */
export type RusticServerOrigin = 'server' | 'configured' | 'build' | 'beside_launcher' | 'path';

/** The `rustic-server` new Rustic Servers of the default build run (Settings > Game). */
export interface RusticServerStatus {
  /** The program found, or null when there is none (creating a Rustic Server then fails). */
  path: string | null;
  origin: RusticServerOrigin | null;
  /** The manual override in launcher.toml, even when that file is gone. */
  configured: string | null;
  /** The build whose folder was searched (the default build). */
  build: string | null;
  /** Its `--rustic-version` line, e.g. "Rustic 0.1.0 (rustic:api 0.1) on Horizon (Minecraft 26.3)". */
  version: string | null;
}
```

### auth.ts

```ts
import type { IsoDate } from './app';

export interface Ownership {
  /** Result of the last Microsoft entitlement check. */
  owns_game: boolean;
  status: 'valid' | 'expired' | 'failed' | 'never_checked';
  checked_at: IsoDate | null;
  /** Offline profiles stop working after this date without a successful revalidation. */
  next_check_at: IsoDate | null;
}

export interface Account {
  id: string;
  /** Microsoft accounts only: offline play is the same account under its official name (ADR 0146). */
  kind: 'microsoft';
  username: string;
  /** Game profile UUID. */
  uuid: string;
  active: boolean;
  /** The last ownership check; offline play stays open until `next_check_at`. */
  ownership: Ownership | null;
  /** Microsoft accounts: the profile's current skin (https://textures.minecraft.net/...). */
  skin?: { url: string; slim: boolean } | null;
}

export interface MicrosoftLoginStart {
  flow_id: string;
  verification_uri: string;
  user_code: string;
  expires_in_s: number;
}

export interface MicrosoftLoginPoll {
  status: 'pending' | 'success' | 'expired' | 'denied' | 'no_ownership';
  account?: Account;
  /** Human-readable failure detail (developer-facing). */
  error?: string;
}

/** Browser sign-in (authorization code + PKCE, loopback redirect); polled like the device code. */
export interface MicrosoftBrowserLoginStart {
  flow_id: string;
  authorize_url: string;
}

/** Payload of the `deep-link` event (`rustic://join/<target>`, `rustic://auth?ticket=…`). */
export type DeepLinkEvent =
  | { action: 'join'; target: string; url: string }
  | { action: 'auth'; ok: boolean; error?: string | null };

/** Rustic platform identity (friends, cosmetics, purchases). Never replaces the game licence. */
export interface RusticSession {
  signed_in: boolean;
  user_id: string | null;
  username: string | null;
  email: string | null;
}

export interface RusticSignupRequest {
  email: string;
  password: string;
  username: string;
  /** YYYY-MM-DD */
  date_of_birth: string;
  /** ISO 3166-1 alpha-2 */
  country: string;
  locale?: string | null;
  /** "I accept the privacy policy" (required, versioned by the backend). */
  accept_privacy: boolean;
}

export interface RusticSignupResult {
  email_verification_required: boolean;
  guardian_required: boolean;
}

export interface RusticLoginResult {
  mfa_required: boolean;
  session: RusticSession;
}
```

### content.ts

```ts
import type { IsoDate } from './app';

export type ContentKind = 'mod' | 'modpack' | 'resourcepack' | 'shader' | 'datapack' | 'plugin';
export type ContentSource = 'horizon' | 'modrinth' | 'rustic';

export type IncompatibilityReason =
  | 'horizon_version'
  | 'java_only'
  | 'side'
  | 'conflict'
  | 'unsupported_format'
  | 'needs_rustic_server'
  | 'not_fair_play';

export type Compatibility =
  | {
      status: 'compatible';
      /** The project ships a build made for the target's Horizon version. */
      builds_on_horizon: boolean;
    }
  | { status: 'incompatible'; reason: IncompatibilityReason; detail: string }
  | { status: 'unknown' };

/** Horizon has two shader formats; the UI shows which and its real support status. */
export type ShaderFormat = 'horizon' | 'shaders_dir';

export interface ContentSummary {
  id: string;
  slug: string;
  kind: ContentKind;
  source: ContentSource;
  name: string;
  summary: string;
  author: string;
  icon_url: string | null;
  downloads: number;
  follows: number;
  updated_at: IsoDate;
  categories: string[];
  compatibility: Compatibility;
  /** Instance or server ids where this project is installed. */
  installed_in: string[];
  featured: boolean;
  shader_format?: ShaderFormat;
}

export interface ContentVersion {
  id: string;
  version: string;
  channel: 'release' | 'beta' | 'alpha';
  horizon_versions: string[];
  published_at: IsoDate;
  downloads: number;
  size_bytes: number;
  compatibility: Compatibility;
}

export interface ContentDependency {
  project_id: string;
  name: string;
  relation: 'required' | 'optional' | 'incompatible';
  installed: boolean;
}

export interface ContentDetail extends ContentSummary {
  description_md: string;
  license: string;
  side: 'client' | 'server' | 'both';
  gallery: { url: string | null; title: string; hue: number }[];
  versions: ContentVersion[];
  dependencies: ContentDependency[];
  changelog: { version: string; published_at: IsoDate; notes: string }[];
  links: { source?: string; issues?: string; website?: string };
}

export interface ContentSearchQuery {
  query: string;
  kind: ContentKind;
  sources: ContentSource[];
  /** Compatibility is computed against this instance (or server for plugins). */
  target?: InstallTarget;
  compatible_only: boolean;
  category?: string;
  sort: 'relevance' | 'downloads' | 'updated' | 'newest';
  offset: number;
  limit: number;
}

export interface ContentSearchResult {
  hits: ContentSummary[];
  total: number;
  offset: number;
  limit: number;
  /** Whether a next page has results (sources page differently, so `total` alone can't tell). */
  has_more?: boolean;
}

export type InstallTarget =
  | { kind: 'instance'; instance_id: string }
  | { kind: 'server'; server_id: string }
  | { kind: 'world'; instance_id: string; world_id: string }
  | { kind: 'server_world'; server_id: string };

export interface InstallRequest {
  project_id: string;
  version_id?: string;
  target: InstallTarget;
  with_dependencies: boolean;
}

export interface InstallResult {
  task_id: string;
  installed: { project_id: string; name: string; version: string }[];
}

export interface ContentCategory {
  id: string;
  kind: ContentKind;
}
```

### dns.ts

```ts
import type { IsoDate } from './app';

export type Currency = 'EUR' | 'USD' | 'GBP' | 'CHF';

export interface Money {
  /** Minor units (cents). */
  amount_minor: number;
  currency: Currency;
}

export interface DnsProviderConnection {
  provider: 'porkbun';
  connected: boolean;
  /** e.g. masked API key "pk1_…3f9a". */
  account_label: string | null;
  connected_at: IsoDate | null;
}

export interface OwnedDomain {
  domain: string;
  expires_at: IsoDate;
  auto_renew: boolean;
  /** Porkbun's API access must be enabled per domain. */
  api_access: boolean;
}

export interface DomainAvailability {
  domain: string;
  available: boolean;
  premium: boolean;
  /** First-year price, as quoted by the registrar. */
  price: Money | null;
  renewal_price: Money | null;
}

export interface BuyDomainRequest {
  domain: string;
  years: number;
  /** The exact price shown to the user; the backend refuses if the quote changed. */
  expected_price: Money;
  /** Explicit tick: Porkbun's registration agreement and terms (LEGAL.md, Porkbun). */
  agree_registrar_terms: true;
  /** Explicit tick: Rustic only sends this order to Porkbun on the user's behalf. */
  authorize_order: true;
}

export interface BuyDomainResult {
  order_id: string;
  domain: string;
  charged: Money;
}

export type DnsRecordType = 'CNAME' | 'SRV' | 'A' | 'AAAA' | 'TXT';

export interface DnsRecord {
  id: string;
  type: DnsRecordType;
  /** Host part as most registrars expect it ("play", "_minecraft._tcp.play", "@"). */
  host: string;
  fqdn: string;
  value: string;
  ttl: number;
  priority?: number;
  weight?: number;
  port?: number;
  purpose: 'routing' | 'srv' | 'ownership';
  /** Not required for the address to work (e.g. SRV next to a CNAME on the default port). */
  optional?: boolean;
}

export interface RequiredRecords {
  fqdn: string;
  hosting_mode: 'local' | 'vps' | 'cloud';
  records: DnsRecord[];
}

export type RecordCheckStatus = 'missing' | 'wrong_value' | 'propagating' | 'ok';

export interface DnsVerification {
  fqdn: string;
  checked_at: IsoDate;
  records: {
    record_id: string;
    status: RecordCheckStatus;
    found: string[];
    /** Public resolvers that already return the expected value (e.g. 1 of 3 while propagating). */
    resolvers_ok?: number;
    resolvers_total?: number;
  }[];
  all_ok: boolean;
}

export interface DomainBinding {
  server_id: string;
  domain: string;
  subdomain: string | null;
  fqdn: string;
  provider: 'porkbun' | 'manual';
  status: 'pending' | 'propagating' | 'verified' | 'error';
  records: DnsRecord[];
  error?: string;
  verified_at: IsoDate | null;
}

export interface DomainTarget {
  server_id: string;
  domain: string;
  subdomain?: string | null;
}
```

### hosting.ts

```ts
import type { IsoDate } from './app';
import type { CreateServerRequest } from './servers';

export interface HostingProviderInfo {
  id: 'local' | 'rustic_cloud' | 'vps';
  available: boolean;
  coming_soon: boolean;
}

export type VpsAuth =
  { kind: 'key'; key_path: string; passphrase?: string } | { kind: 'password'; password: string };

/** A key pair generated by the launcher for a VPS. The private key stays in the OS keychain:
 * `key_path` is a `keychain:<id>` reference to pass back as `VpsAuth.key_path`, never a file. */
export interface GeneratedSshKey {
  key_path: string;
  public_key: string;
  /** One line to paste in the VPS console to authorise the key. */
  authorize_command: string;
}

/** Secrets travel over IPC (in-process) only and are stored in the OS keychain by the backend. */
export interface VpsConnectionRequest {
  host: string;
  port: number;
  username: string;
  auth: VpsAuth;
}

export interface VpsTestResult {
  reachable: boolean;
  latency_ms: number | null;
  /** Host key to confirm before anything else happens (TOFU). */
  fingerprint: { algorithm: string; sha256: string } | null;
  /** Already trusted by a previous confirmation. */
  known: boolean;
  auth_ok: boolean;
  error?: string;
}

export interface VpsConnection {
  id: string;
  label: string;
  host: string;
  port: number;
  username: string;
  fingerprint_sha256: string;
  added_at: IsoDate;
}

export interface VpsResources {
  os: string;
  arch: string;
  cpu_cores: number;
  memory_mb: number;
  disk_free_mb: number;
  has_systemd: boolean;
  public_ip: string | null;
  requirements_met: boolean;
  problems: { code: 'memory' | 'disk' | 'arch' | 'os' | 'systemd' | 'sudo'; detail: string }[];
}

export interface VpsInstallRequest {
  connection_id: string;
  server: CreateServerRequest;
  /** The user allowed opening the game port in the VPS firewall (ufw/firewalld). Default false. */
  open_firewall?: boolean;
}

export type InstallStepId =
  'connect' | 'check' | 'user' | 'download' | 'engine' | 'service' | 'firewall' | 'world' | 'start';

export interface InstallStep {
  id: InstallStepId;
  state: 'pending' | 'running' | 'done' | 'failed' | 'skipped';
  detail?: string;
}

/** Payload of the `hosting-install` event. */
export interface HostingInstallProgress {
  task_id: string;
  server_id: string;
  steps: InstallStep[];
  log?: string;
  done: boolean;
  error?: string;
}
```

### instances.ts

```ts
import type { IsoDate } from './app';
import type { ContentKind, ContentSource } from './content';

export interface HorizonVersion {
  /** `v0.2.14` or `nightly-2026-11-08-a84f637`. */
  id: string;
  /** e.g. "Horizon 0.2.14 (Rustic edition)". */
  label: string;
  channel: 'release' | 'nightly';
  /** Prebuilt Horizon client with the Rustic companion (cosmetics, friends) built in. */
  rustic_edition: boolean;
  released_at: IsoDate;
  installed: boolean;
  recommended: boolean;
  size_bytes: number;
}

export type InstanceIcon =
  { kind: 'preset'; preset: InstanceIconPreset; color: string } | { kind: 'image'; url: string };

export type InstanceIconPreset =
  | 'grass'
  | 'sword'
  | 'pickaxe'
  | 'castle'
  | 'rocket'
  | 'gem'
  | 'flame'
  | 'leaf'
  | 'star'
  | 'compass';

export type InstanceState = 'ready' | 'needs_update' | 'installing' | 'running' | 'broken';

export interface Instance {
  id: string;
  name: string;
  icon: InstanceIcon;
  horizon_version: string;
  horizon_version_label: string;
  state: InstanceState;
  /** Newer Horizon version available, if any. */
  update_available: string | null;
  mod_count: number;
  resource_pack_count: number;
  shader_count: number;
  datapack_world_count: number;
  playtime_s: number;
  last_played_at: IsoDate | null;
  last_server: { name: string; address: string } | null;
  created_at: IsoDate;
  /** Null = use the default from settings. */
  memory_mb: number | null;
  game_args: string;
  resolution: { width: number; height: number; fullscreen: boolean } | null;
  folder: string;
  source: { kind: 'modpack'; project_id: string; name: string; version: string } | null;
  pinned: boolean;
}

export interface CreateInstanceRequest {
  name: string;
  icon: InstanceIcon;
  horizon_version: string;
  memory_mb?: number | null;
}

export type ImportInstanceRequest =
  | { kind: 'folder' | 'mrpack' | 'curseforge_zip'; path: string; name?: string }
  | { kind: 'modpack'; project_id: string; version_id?: string; name?: string };

export type InstancePatch = Partial<
  Pick<Instance, 'name' | 'icon' | 'memory_mb' | 'game_args' | 'resolution' | 'pinned'>
>;

export type InstanceFolder =
  'root' | 'mods' | 'resourcepacks' | 'shaderpacks' | 'saves' | 'screenshots' | 'logs';

export interface InstalledContent {
  id: string;
  project_id: string | null;
  kind: ContentKind;
  name: string;
  version: string;
  filename: string;
  enabled: boolean;
  source: ContentSource | 'local';
  icon_url: string | null;
  update_available: string | null;
  compatible: boolean;
  incompatibility: string | null;
  /** Resource packs: load order, 0 = highest priority. */
  order?: number;
  /** Datapacks: world they are installed in. */
  world_id?: string;
}

export interface World {
  id: string;
  name: string;
  mode: 'survival' | 'creative' | 'adventure' | 'hardcore' | 'spectator';
  last_played_at: IsoDate | null;
  size_bytes: number;
  datapack_count: number;
}

export interface LaunchOptions {
  /** Join a server right after start. */
  join?: { address: string; name?: string };
  /** Open a single-player world. */
  world_id?: string;
}

export type LaunchPhase =
  | 'idle'
  | 'checking'
  | 'downloading'
  | 'preparing'
  | 'launching'
  | 'running'
  | 'stopping'
  | 'crashed';

export interface LaunchStatus {
  instance_id: string;
  phase: LaunchPhase;
  /** 0..1 or null when indeterminate. */
  progress: number | null;
  message?: string;
  /** Stable cause of an idle-with-message state (`auth.no_account`, `prepare.failed`...). */
  reason?: string;
  started_at?: IsoDate;
}

export interface GameLogLine {
  instance_id: string;
  level: 'info' | 'warn' | 'error';
  line: string;
}

export interface GameExited {
  instance_id: string;
  code: number;
  crashed: boolean;
  crash_report_path?: string;
  played_s: number;
}
```

### multiplayer.ts

```ts
import type { IsoDate } from './app';

export interface ServerPing {
  online: boolean;
  latency_ms: number | null;
  motd: string;
  players_online: number;
  players_max: number;
  version: string;
}

/** An entry of the game's multiplayer list (servers.dat), shared by all instances. */
export interface MultiplayerEntry {
  id: string;
  name: string;
  address: string;
  favorite: boolean;
  last_joined_at: IsoDate | null;
}
```

### servers.ts

```ts
import type { IsoDate } from './app';
import type { InstanceIcon } from './instances';

/** `rustic`: the `rustic-server` engine under Horizon's `horizon-server` supervisor (plugins). */
export type ServerEngine = 'horizon' | 'rustic';

export type ServerHosting =
  | { mode: 'local' }
  | { mode: 'vps'; connection_id: string; host: string }
  | { mode: 'cloud'; region: string };

export type ServerState =
  'stopped' | 'starting' | 'running' | 'stopping' | 'crashed' | 'installing' | 'unreachable';

export interface Server {
  id: string;
  name: string;
  icon: InstanceIcon;
  engine: ServerEngine;
  horizon_version: string;
  horizon_version_label: string;
  hosting: ServerHosting;
  state: ServerState;
  players_online: number;
  max_players: number;
  port: number;
  motd: string;
  world_name: string;
  memory_mb: number;
  created_at: IsoDate;
  last_started_at: IsoDate | null;
  /** Best address to share, e.g. relay hostname or domain. */
  share_address: string | null;
  plugin_count: number;
  folder: string;
}

export type WorldSetup =
  | {
      kind: 'new';
      seed: string | null;
      level_type: 'default' | 'flat' | 'large_biomes' | 'amplified';
      generate_structures: boolean;
      gamemode: 'survival' | 'creative' | 'adventure';
      difficulty: 'peaceful' | 'easy' | 'normal' | 'hard';
      hardcore: boolean;
    }
  | { kind: 'import'; path: string }
  | { kind: 'from_instance'; instance_id: string; world_id: string };

export interface CreateServerRequest {
  name: string;
  icon: InstanceIcon;
  engine: ServerEngine;
  horizon_version: string;
  hosting: { mode: 'local' } | { mode: 'vps'; connection_id: string };
  world: WorldSetup;
  memory_mb: number;
  max_players: number;
  /** The user accepted the game's EULA themselves (checkbox in the wizard). */
  eula_accepted: boolean;
  /** Official `.rplugin` ids to install first (Rustic Server only). */
  starter_plugins: string[];
  /** Reachability chosen in the wizard's Access step (LAN is always on). */
  access: { relay: boolean; invite: boolean };
  /** Local servers: start the server when the launcher starts. */
  start_with_launcher: boolean;
}

/** Official plugin offered as a starter in the wizard. */
export interface StarterPlugin {
  id: string;
  name: string;
  version: string;
  description: string;
  capabilities: { network: false | string[]; filesystem: 'none' | 'data-folder' };
  recommended: boolean;
}

export interface CreateServerResult {
  server: Server;
  /** Download/install task (local) or hosting install task (VPS). */
  task_id: string;
}

export interface ConsoleLine {
  server_id: string;
  ts: IsoDate;
  level: 'info' | 'warn' | 'error' | 'command' | 'chat';
  text: string;
}

export interface ServerPlayer {
  uuid: string;
  name: string;
  online: boolean;
  op: boolean;
  whitelisted: boolean;
  banned: boolean;
  ping_ms: number | null;
  joined_at: IsoDate | null;
  playtime_s: number;
}

export type PlayerAction =
  'kick' | 'ban' | 'pardon' | 'op' | 'deop' | 'whitelist_add' | 'whitelist_remove';

export interface ConfigField {
  key: string;
  group: 'general' | 'world' | 'players' | 'network' | 'performance';
  type: 'string' | 'integer' | 'boolean' | 'enum';
  value: string | number | boolean;
  default: string | number | boolean;
  options?: string[];
  min?: number;
  max?: number;
  /** Changing it requires a restart. */
  restart_required: boolean;
}

export interface ServerConfig {
  fields: ConfigField[];
}

export interface RawConfig {
  path: string;
  format: 'toml' | 'properties';
  content: string;
}

/** A `.rplugin` installed in `<server>/plugins/` (Rustic Server only). */
export interface ServerPlugin {
  id: string;
  name: string;
  version: string;
  api: string;
  authors: string[];
  license: string;
  description: string;
  state: 'enabled' | 'disabled' | 'errored';
  error?: string;
  update_available: string | null;
  capabilities: { network: false | string[]; filesystem: 'none' | 'data-folder' };
  project_id: string | null;
}

export interface ServerDatapack {
  id: string;
  name: string;
  version: string;
  enabled: boolean;
  description: string;
}

export interface Backup {
  id: string;
  created_at: IsoDate;
  size_bytes: number;
  kind: 'manual' | 'scheduled' | 'pre_restore';
  note: string | null;
}

export interface BackupSchedule {
  enabled: boolean;
  interval_hours: number;
  keep: number;
}

export interface ServerStats {
  server_id: string;
  ts: IsoDate;
  cpu_percent: number;
  ram_used_mb: number;
  ram_max_mb: number;
  tps: number;
  mspt: number;
  players: number;
}

export interface ServerAccess {
  lan: { ip: string; port: number; address: string };
  /** Free temporary hostname on the Rustic Relay (optional, configurable label). */
  relay: {
    enabled: boolean;
    hostname: string | null;
    label: string | null;
    status: 'off' | 'connecting' | 'online' | 'error';
    error?: string;
  };
  /** Invite code in the spirit of Horizon's Play with Friends. */
  invite: { code: string | null; expires_at: IsoDate | null };
  /** UPnP port mapping (advanced, exposes the public IP). */
  upnp: {
    enabled: boolean;
    status: 'off' | 'mapping' | 'mapped' | 'failed';
    external_address: string | null;
    error?: string;
  };
  /** Public IPv4/IPv6 of a VPS (hosting mode vps). */
  public_ip: string | null;
}
```

## Appendix: Rustic Cloud API expected by the UI (not IPC)

Pages use the `CloudClient` interface (`apps/launcher/src/cloud/client.ts`, ADR 0027). Default build: mock. With
`VITE_CLOUD_URL` set, `src/cloud/http.ts` implements it over `services/api/openapi.json` (types generated by
`pnpm --filter @rustic/launcher gen:cloud`, openapi-typescript; requests via openapi-fetch; bearer token from the
`auth_rustic_access_token` command). Gaps between this interface and the API are tracked in HANDOFF.md.

```ts
export interface CloudClient {
  me(): Promise<Me>;
  // social
  friends(): Promise<Friend[]>;
  friendRequests(): Promise<FriendRequest[]>;
  searchUsers(query: string): Promise<UserLite[]>;
  /** Exact Rustic or Minecraft name; errors carry the API codes (`not_found`, `cannot_add_friend`...). */
  lookupUser(by: 'rustic' | 'minecraft', name: string): Promise<UserLookup>;
  /** A bare string is a Rustic username. */
  sendFriendRequest(target: string | FriendTarget): Promise<FriendRequest>;
  /** Players met recently (ADR 0145). */
  friendSuggestions(): Promise<FriendSuggestions>;
  /** "Appear in suggestions of players I meet" (refused for minors). */
  setDiscoverable(on: boolean): Promise<Me>;
  respondFriendRequest(id: string, accept: boolean): Promise<void>;
  cancelFriendRequest(id: string): Promise<void>;
  removeFriend(id: string): Promise<void>;
  setFavoriteFriend(id: string, favorite: boolean): Promise<void>;
  party(): Promise<Party | null>;
  inviteToParty(userId: string): Promise<Party>;
  leaveParty(): Promise<void>;
  subscribe(handler: (e: RealtimeEvent) => void): () => void;
  // chats (Social window)
  conversations(): Promise<Conversation[]>;
  messages(userId: string): Promise<ChatMessage[]>;
  sendMessage(userId: string, body: string): Promise<ChatMessage>;
  markConversationRead(userId: string): Promise<void>;
  // content feeds
  news(): Promise<NewsItem[]>;
  featuredServers(): Promise<FeaturedServer[]>;
  notifications(): Promise<AppNotification[]>;
  markNotificationsRead(ids: string[]): Promise<void>;
  // cosmetics & store
  cosmetics(): Promise<Cosmetic[]>;
  loadout(): Promise<Loadout>;
  equip(loadout: Loadout): Promise<Loadout>;
  outfits(): Promise<Outfit[]>;
  saveOutfit(name: string, loadout: Loadout): Promise<Outfit>;
  deleteOutfit(id: string): Promise<void>;
  catalog(currency: Currency): Promise<StoreCatalog>;
  product(id: string, currency: Currency): Promise<StoreProduct>;
  purchase(req: PurchaseRequest): Promise<PurchaseResult>;
  // account & compliance
  parental(): Promise<ParentalControls>;
  setParental(patch: Partial<ParentalControls> & { pin?: string }): Promise<ParentalControls>;
  verifyParentalPin(pin: string): Promise<boolean>;
  /** Sign-in methods on the Rustic account (Microsoft identity, password). */
  identities(): Promise<LinkedIdentities>;
  /** Unlinks a sign-in provider; refused (`last_sign_in_method`) when it is the only way in. */
  unlinkIdentity(provider: 'microsoft'): Promise<void>;
  exportAccount(): Promise<{ url: string; expires_at: string }>;
  deleteAccount(confirmUsername: string): Promise<{ scheduled_for: string }>;
  joinWaitlist(email: string): Promise<{ position: number }>;
}
```

UI-side shapes (`apps/launcher/src/cloud/types.ts`):

```ts
/**
 * Rustic Cloud types as the UI consumes them. The backend engineer will generate the real client
 * from services/api/openapi.json; these hand-written types are the expected shapes (see IPC.md,
 * "Cloud API expected by the UI").
 */
import type { CosmeticSlot, CosmeticVisual } from '@rustic/ui/three';
import type { Currency, Money } from '../ipc/types/dns';

export type { Currency, Money, CosmeticSlot, CosmeticVisual };

export type PresenceStatus = 'online' | 'in-game' | 'away' | 'busy' | 'offline';

export interface UserLite {
  id: string;
  username: string;
  display_name: string;
}

export interface Activity {
  kind: 'launcher' | 'singleplayer' | 'server' | 'hosting';
  instance_name?: string;
  server_name?: string;
  server_address?: string;
  /** The friend allows joining (server is reachable for them). */
  joinable: boolean;
  since: string;
}

export interface Friend extends UserLite {
  presence: PresenceStatus;
  activity: Activity | null;
  last_seen_at: string | null;
  favorite: boolean;
}

/** Who a friend request goes to: a known user id, or an exact Rustic or Minecraft name. */
export type FriendTarget = { user_id: string } | { username: string } | { minecraft_name: string };

export type LookupRelation = 'none' | 'friends' | 'outgoing_request' | 'incoming_request';

/** Exact-match lookup shown before sending a friend request (`GET /v1/users/lookup`). */
export interface UserLookup {
  user: UserLite;
  /** Linked Minecraft profile name (hidden for minors unless it is what was searched). */
  minecraft_name: string | null;
  relation: LookupRelation;
  /** Incoming request to accept when `relation` is `incoming_request`. */
  request_id: string | null;
}

export interface FriendSuggestion {
  user: UserLite;
  minecraft_name: string | null;
  /** Same party, or same (shared) server at the same time. */
  reason: 'party' | 'server';
  /** The viewer's own name for the server. */
  server_name: string | null;
  seen_at: string;
}

/** Players met recently (ADR 0145). Empty and `discoverable: false` when opted out or a minor. */
export interface FriendSuggestions {
  discoverable: boolean;
  played_with: FriendSuggestion[];
  crossed: FriendSuggestion[];
}

export interface FriendRequest {
  id: string;
  user: UserLite;
  direction: 'incoming' | 'outgoing';
  created_at: string;
}

export interface Party {
  id: string;
  leader_id: string;
  members: (UserLite & { presence: PresenceStatus })[];
  max_size: number;
  pending_invites: UserLite[];
}

export interface Me extends UserLite {
  email: string;
  created_at: string;
  /** Used for the parental gate and minimum ages (account, chat). */
  age_group: 'child' | 'teen' | 'adult';
  country: string;
  currency: Currency;
  balance: Money | null;
  /** "Appear in suggestions of players I meet", as in effect (ADR 0145). */
  discoverable: boolean;
  /** False for minors: the setting is forced off and cannot be changed. */
  discoverable_allowed: boolean;
}

export interface NewsItem {
  id: string;
  source: 'horizon' | 'rustic';
  kind: 'patch_notes' | 'event' | 'blog';
  title: string;
  summary: string;
  published_at: string;
  url: string;
  /** Hue for the generated cover art (no third-party images). */
  hue: number;
  version?: string;
}

export interface FeaturedServer {
  id: string;
  name: string;
  address: string;
  description: string;
  tags: string[];
  engine: 'horizon' | 'rustic';
  /** Null when nobody measured it (the cloud does not ping featured servers yet). */
  players_online: number | null;
  players_max: number | null;
  language: string;
  verified: boolean;
  hue: number;
}

export type Rarity = 'common' | 'rare' | 'epic' | 'legendary';

export interface Cosmetic {
  id: string;
  name: string;
  description: string;
  slot: CosmeticSlot;
  rarity: Rarity;
  visual: CosmeticVisual;
  owned: boolean;
  acquired_at: string | null;
  source: 'purchase' | 'gift' | 'event' | 'default' | null;
}

export type Loadout = Partial<Record<CosmeticSlot, string | null>>;

export interface Outfit {
  id: string;
  name: string;
  loadout: Loadout;
  created_at: string;
}

export interface StoreProduct {
  id: string;
  kind: 'item' | 'bundle';
  name: string;
  description: string;
  /** Cosmetics granted (one for items, several for bundles). Purely visual, never random. */
  cosmetic_ids: string[];
  slot: CosmeticSlot | null;
  rarity: Rarity;
  price: Money;
  /** Bundle: sum of the items bought separately. */
  compare_at: Money | null;
  owned: boolean;
  is_new: boolean;
  available_until: string | null;
  hue: number;
}

export interface StoreCatalog {
  currency: Currency;
  featured: StoreProduct[];
  bundles: StoreProduct[];
  new_items: StoreProduct[];
  products: StoreProduct[];
}

export interface PurchaseRequest {
  product_id: string;
  currency: Currency;
  /** EU: explicit consent to immediate execution of the digital content contract. */
  consent_immediate_execution: true;
  /** EU: acknowledgement that the 14-day withdrawal right is lost once delivered. */
  waiver_withdrawal_right: true;
  parental_pin?: string;
  /** Gift to a friend (user id): granted to them, not to the buyer. */
  gift_to?: string;
}

export interface PurchaseResult {
  order_id: string;
  status: 'completed' | 'parental_approval_required';
  charged: Money;
  receipt_url: string;
  granted_cosmetic_ids: string[];
}

export interface ParentalControls {
  enabled: boolean;
  pin_set: boolean;
  purchases: 'allowed' | 'pin' | 'blocked';
  monthly_limit: Money | null;
  spent_this_month: Money;
  chat: 'everyone' | 'friends' | 'off';
  friend_requests: boolean;
}

export interface AppNotification {
  id: string;
  kind: 'friend_request' | 'party_invite' | 'server' | 'store' | 'system' | 'update';
  title: string;
  body: string;
  created_at: string;
  read: boolean;
  /** Route to open, e.g. "/servers/srv_1". */
  href?: string;
}

export interface ChatMessage {
  id: string;
  from_id: string;
  to_id: string;
  body: string;
  sent_at: string;
  /** Read receipt: when the recipient read it (null while unread). */
  read_at?: string | null;
}

export interface Conversation {
  user: UserLite & { presence: PresenceStatus };
  last_message: ChatMessage | null;
  unread: number;
}

export type RealtimeEvent =
  | { type: 'presence'; friend: Friend }
  | { type: 'friend-request'; request: FriendRequest }
  /** A friendship was made (request accepted) or ended elsewhere: refetch the roster. */
  | { type: 'friends-changed' }
  | { type: 'party'; party: Party | null }
  | { type: 'notification'; notification: AppNotification }
  | { type: 'message'; message: ChatMessage }
  | { type: 'messages-read'; reader_id: string; read_at: string }
  | { type: 'join-server'; address: string };
```
