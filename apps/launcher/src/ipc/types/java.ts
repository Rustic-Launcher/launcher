/**
 * The Java Edition launcher and the Bedrock panel (ADR 0172): Mojang's versions, Java profiles
 * (kept apart from Rustic instances), installs, launch state, logs, Modrinth mods for a profile's
 * loader, and Bedrock Edition on Windows. Times are Unix seconds.
 */

export type JavaLoader = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge';

export type JavaVersionKind = 'release' | 'snapshot' | 'old_beta' | 'old_alpha';

/** A version of Mojang's manifest. */
export interface JavaGameVersion {
  id: string;
  kind: JavaVersionKind;
  release_time: string;
  /** Its version file is in the launcher's game folder. */
  installed: boolean;
}

export interface JavaLoaderVersion {
  version: string;
  /** Fabric's `stable`, Forge's recommended build, NeoForge without `-beta`. */
  stable: boolean;
}

export interface JavaResolution {
  width: number;
  height: number;
}

export interface JavaProfile {
  id: string;
  name: string;
  game_version: string;
  loader: JavaLoader;
  /** None: the newest stable build at install time. */
  loader_version: string | null;
  /** The version file the profile starts, once installed (`fabric-loader-0.17.2-1.21.1`). */
  version_id: string | null;
  memory_min_mb: number;
  memory_max_mb: number;
  resolution: JavaResolution | null;
  fullscreen: boolean;
  /** A Java executable or home; none: Mojang's runtime for the version. */
  java_path: string | null;
  jvm_args: string[];
  /** A folder of the player's choice; none: the launcher's own per-profile folder. */
  game_dir: string | null;
  created_at: number;
  last_played_at: number | null;
  /** Seconds played. */
  play_time: number;
  /** The resolved game folder. */
  game_dir_path: string;
  installed: boolean;
  running: boolean;
}

export interface NewJavaProfile {
  name: string;
  game_version: string;
  loader: JavaLoader;
  loader_version?: string | null;
  memory_max_mb?: number;
}

/** Fields to change; `null` clears a nullable one. */
export interface JavaProfilePatch {
  name?: string;
  game_version?: string;
  loader?: JavaLoader;
  loader_version?: string | null;
  memory_min_mb?: number;
  memory_max_mb?: number;
  resolution?: JavaResolution | null;
  fullscreen?: boolean;
  java_path?: string | null;
  jvm_args?: string[];
  game_dir?: string | null;
}

export type JavaInstallStep = 'metadata' | 'loader' | 'libraries' | 'assets' | 'java';

export type JavaRunPhase = 'idle' | 'installing' | 'launching' | 'running' | 'crashed';

/** What a profile is doing (`java-state`). */
export interface JavaRunState {
  profile_id: string;
  state: JavaRunPhase;
  step?: JavaInstallStep;
  /** 0..1 while installing. */
  progress?: number;
  task_id?: string;
  /** The last failure, in English. */
  message?: string;
  /** Its stable code: `auth.not_owned`, `auth.no_account`, `java.no_runtime`, `java.installer`... */
  reason?: string;
  started_at?: number;
  exit_code?: number;
  crash_report?: string;
  /** Started offline: no network, the owner's last check is recent enough. */
  offline: boolean;
}

export type JavaLogLevel = 'info' | 'warn' | 'error' | 'debug';

/** One line of a profile's output (`java-log`). */
export interface JavaLogLine {
  profile_id: string;
  level: JavaLogLevel;
  thread: string | null;
  text: string;
  time: number;
}

export interface JavaModHit {
  project_id: string;
  slug: string;
  name: string;
  summary: string;
  author: string;
  icon_url: string | null;
  downloads: number;
  /** Fair in single player, not on every server: installable with a notice. */
  flagged: boolean;
  installed: boolean;
}

export interface JavaModSearch {
  hits: JavaModHit[];
  total: number;
}

export interface JavaModFile {
  file_name: string;
  enabled: boolean;
  size: number;
  project_id?: string;
}

export type JavaFolder = 'game' | 'mods' | 'logs' | 'crash_reports' | 'screenshots';

export interface BedrockStatus {
  /** Windows: Bedrock Edition exists for this system. */
  supported: boolean;
  installed: boolean;
  version: string | null;
}
