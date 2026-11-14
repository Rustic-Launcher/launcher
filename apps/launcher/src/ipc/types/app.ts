/** App, system, settings, downloads, game preparation. Mirrors launcher/docs/IPC.md. */
import type { LaunchOptions } from './instances';

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
  | 'game'
  | 'content'
  | 'modpack'
  | 'server'
  | 'java'
  | 'update'
  | 'prepare'
  | 'native-build'
  | 'native-toolchain';

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
  /** What a long task is doing now (a native build's step), in English. */
  detail?: string;
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

/** A newer launcher or Rustic edition build (ADR 0162). */
export interface UpdateItem {
  version: string;
  notes: string | null;
  size: number | null;
  published_at: IsoDate | null;
}

/**
 * The update service (ADR 0162): checks at start, before Play, when a game exits, every few hours
 * and on demand; installs without a prompt when `auto` is on, never while a game runs.
 */
export interface UpdateStatus {
  phase:
    | 'idle'
    | 'checking'
    /** Found; automatic updates are off (or a failed install waits): "Update and restart". */
    | 'available'
    /** Found while a game runs: applied as soon as it exits. */
    | 'deferred'
    | 'downloading'
    | 'installing'
    | 'restarting'
    /** The last check or install failed; retried later, playing is never blocked. */
    | 'error';
  current_version: string;
  channel: 'stable' | 'beta';
  /** Settings > Launcher > "Install updates automatically". */
  auto: boolean;
  last_check: IsoDate | null;
  launcher: UpdateItem | null;
  edition: UpdateItem | null;
  /** The newest installed Rustic edition build. */
  edition_current: string | null;
  /** What `downloading`/`installing` works on. */
  working_on: 'launcher' | 'edition' | null;
  /** 0..1 when known. */
  progress: number | null;
  error: string | null;
  /** This launcher was just updated from that version (shown once). */
  updated_from: string | null;
  /** A launch interrupted by the update, resumed after the restart. */
  resume_instance: string | null;
}

/** A launch an update restart interrupted (`app_take_pending_launch`). */
export interface PendingLaunch {
  instance_id: string;
  options: LaunchOptions | null;
  /** Unix seconds. */
  created_at: number;
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
