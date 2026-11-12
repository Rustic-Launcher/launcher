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
