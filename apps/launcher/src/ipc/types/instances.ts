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
  /** Compiling the instance's native mods before it starts (ADR 0157). */
  | 'building'
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
