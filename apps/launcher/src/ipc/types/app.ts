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
