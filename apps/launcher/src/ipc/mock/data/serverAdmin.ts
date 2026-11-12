/** Per-server mock state for the management page, created lazily for any server id. */
import type {
  Backup,
  BackupSchedule,
  ConfigField,
  ConsoleLine,
  Server,
  ServerAccess,
  ServerDatapack,
  ServerPlayer,
  ServerPlugin,
  ServerStats,
} from '../../types/servers';
import { RELAY_DOMAIN } from '@/lib/relay';
import { daysAgo, minutesAgo } from '../runtime';
import { serversDb } from './servers';

export interface AdminState {
  players: ServerPlayer[];
  whitelist_enabled: boolean;
  plugins: ServerPlugin[];
  datapacks: ServerDatapack[];
  backups: Backup[];
  schedule: BackupSchedule;
  config: ConfigField[];
  raw_extra: string;
  console: ConsoleLine[];
  stats: ServerStats[];
}

const PLAYER_NAMES = [
  'SamCrafte',
  'lea_builds',
  'noah.pvp',
  'MaxRedstone',
  'InesCraft',
  'tom_h',
  'HugoMines',
  'chloe_art',
];

function uuidFor(name: string): string {
  let h = 0;
  for (const c of name) h = (h * 33 + c.charCodeAt(0)) >>> 0;
  const hex = (h.toString(16) + '0'.repeat(32)).slice(0, 32);
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-4${hex.slice(13, 16)}-a${hex.slice(17, 20)}-${hex.slice(20, 32)}`;
}

function makePlayers(server: Server): ServerPlayer[] {
  const online = server.state === 'running' ? server.players_online : 0;
  return PLAYER_NAMES.slice(0, Math.max(online + 2, 4)).map((name, i) => ({
    uuid: uuidFor(name),
    name,
    online: i < online,
    op: i === 0,
    whitelisted: i < 6,
    banned: false,
    ping_ms: i < online ? 18 + ((i * 37) % 90) : null,
    joined_at: i < online ? minutesAgo(5 + i * 11) : null,
    playtime_s: (40 - i * 4) * 3600 + i * 600,
  }));
}

const RUSTIC_PLUGINS: ServerPlugin[] = [
  {
    id: 'essentials-lite',
    name: 'Essentials Lite',
    version: '0.3.1',
    api: '0.1',
    authors: ['Rustic'],
    license: 'MIT OR Apache-2.0',
    description: 'Homes, spawn, warps, /tpa and the everyday commands.',
    state: 'enabled',
    update_available: '0.3.2',
    capabilities: { network: false, filesystem: 'data-folder' },
    project_id: 'rp_essentials',
  },
  {
    id: 'perms',
    name: 'Perms',
    version: '0.2.0',
    api: '0.1',
    authors: ['Rustic'],
    license: 'MIT OR Apache-2.0',
    description: 'Groups, inheritance and wildcard permissions from a TOML file.',
    state: 'enabled',
    update_available: null,
    capabilities: { network: false, filesystem: 'data-folder' },
    project_id: 'rp_perms',
  },
  {
    id: 'chat-format',
    name: 'Chat Format',
    version: '0.1.4',
    api: '0.1',
    authors: ['Rustic'],
    license: 'MIT OR Apache-2.0',
    description: 'Prefixes, colours and mentions in chat.',
    state: 'enabled',
    update_available: null,
    capabilities: { network: false, filesystem: 'none' },
    project_id: 'rp_chat',
  },
  {
    id: 'menus',
    name: 'Menus',
    version: '0.1.0',
    api: '0.1',
    authors: ['Rustic'],
    license: 'MIT OR Apache-2.0',
    description: 'Chest-style menus defined in config.',
    state: 'disabled',
    update_available: null,
    capabilities: { network: false, filesystem: 'data-folder' },
    project_id: 'rp_menus',
  },
  {
    id: 'mini-game',
    name: 'Mini Game',
    version: '0.1.2',
    api: '0.1',
    authors: ['Rustic', 'lea_builds'],
    license: 'MIT',
    description: 'Spleef-style arena with queues and scoreboards.',
    state: 'errored',
    error: 'Plugin trapped in on_enable (fuel exhausted); disabled, the server goes on.',
    update_available: '0.1.3',
    capabilities: { network: ['api.rustic.example.invalid'], filesystem: 'data-folder' },
    project_id: 'rp_minigame',
  },
];

function makeConfig(server: Server): ConfigField[] {
  return [
    {
      key: 'motd',
      group: 'general',
      type: 'string',
      value: server.motd,
      default: 'A Horizon server',
      restart_required: false,
    },
    {
      key: 'max-players',
      group: 'players',
      type: 'integer',
      value: server.max_players,
      default: 20,
      min: 1,
      max: 500,
      restart_required: true,
    },
    {
      key: 'white-list',
      group: 'players',
      type: 'boolean',
      value: false,
      default: false,
      restart_required: false,
    },
    {
      key: 'online-mode',
      group: 'players',
      type: 'boolean',
      value: true,
      default: true,
      restart_required: true,
    },
    {
      key: 'pvp',
      group: 'players',
      type: 'boolean',
      value: true,
      default: true,
      restart_required: false,
    },
    {
      key: 'gamemode',
      group: 'world',
      type: 'enum',
      value: 'survival',
      default: 'survival',
      options: ['survival', 'creative', 'adventure', 'spectator'],
      restart_required: false,
    },
    {
      key: 'difficulty',
      group: 'world',
      type: 'enum',
      value: 'normal',
      default: 'easy',
      options: ['peaceful', 'easy', 'normal', 'hard'],
      restart_required: false,
    },
    {
      key: 'level-seed',
      group: 'world',
      type: 'string',
      value: '',
      default: '',
      restart_required: true,
    },
    {
      key: 'spawn-protection',
      group: 'world',
      type: 'integer',
      value: 16,
      default: 16,
      min: 0,
      max: 256,
      restart_required: false,
    },
    {
      key: 'server-port',
      group: 'network',
      type: 'integer',
      value: server.port,
      default: 25565,
      min: 1024,
      max: 65535,
      restart_required: true,
    },
    {
      key: 'enable-rcon',
      group: 'network',
      type: 'boolean',
      value: false,
      default: false,
      restart_required: true,
    },
    {
      key: 'view-distance',
      group: 'performance',
      type: 'integer',
      value: 10,
      default: 10,
      min: 2,
      max: 32,
      restart_required: false,
    },
    {
      key: 'simulation-distance',
      group: 'performance',
      type: 'integer',
      value: 8,
      default: 10,
      min: 2,
      max: 32,
      restart_required: false,
    },
  ];
}

function makeStats(server: Server): ServerStats[] {
  const out: ServerStats[] = [];
  if (server.state !== 'running') return out;
  const now = Date.now();
  for (let i = 60; i > 0; i--) {
    out.push(statSample(server, new Date(now - i * 2000).toISOString(), i));
  }
  return out;
}

export function statSample(server: Server, ts: string, seed = Date.now() / 2000): ServerStats {
  const wave = Math.sin(seed / 6);
  const mspt = Math.max(4, 14 + wave * 6 + Math.random() * 5 + (Math.random() < 0.05 ? 25 : 0));
  return {
    server_id: server.id,
    ts,
    cpu_percent: Math.round((22 + wave * 9 + Math.random() * 8) * 10) / 10,
    ram_used_mb: Math.round(server.memory_mb * (0.52 + wave * 0.06) + Math.random() * 120),
    ram_max_mb: server.memory_mb,
    tps: Math.round(Math.min(20, 1000 / Math.max(50, mspt)) * 100) / 100,
    mspt: Math.round(mspt * 10) / 10,
    players: server.players_online,
  };
}

function makeConsole(server: Server): ConsoleLine[] {
  const base = Date.now() - 6 * 60_000;
  const at = (s: number) => new Date(base + s * 1000).toISOString();
  const l = (s: number, level: ConsoleLine['level'], text: string): ConsoleLine => ({
    server_id: server.id,
    ts: at(s),
    level,
    text,
  });
  const lines: ConsoleLine[] = [
    l(
      0,
      'info',
      `Starting ${server.engine === 'rustic' ? 'rustic-server' : 'horizon'} ${server.horizon_version} on port ${server.port}`,
    ),
    l(1, 'info', `Preparing level "${server.world_name}"`),
    l(3, 'info', 'Preparing spawn area: 100%'),
  ];
  if (server.engine === 'rustic') {
    lines.push(
      l(4, 'info', '[Rustic] Loading 5 plugins (api 0.1)'),
      l(4, 'info', '[Rustic] Enabled essentials-lite 0.3.1, perms 0.2.0, chat-format 0.1.4'),
      l(5, 'warn', '[Rustic] mini-game 0.1.2 trapped in on_enable (fuel exhausted): disabled'),
    );
  }
  lines.push(
    l(6, 'info', 'Done (5.812s)! For help, type "help"'),
    l(40, 'info', 'lea_builds joined the game'),
    l(52, 'chat', '<lea_builds> salut tout le monde'),
    l(80, 'info', 'noah.pvp joined the game'),
    l(200, 'info', 'Saving the game (autosave)'),
    l(201, 'info', 'Saved the game'),
  );
  return lines;
}

const state = new Map<string, AdminState>();

export function findServer(id: string): Server | undefined {
  return serversDb.servers.find((s) => s.id === id);
}

export function admin(serverId: string): AdminState {
  let s = state.get(serverId);
  if (s) return s;
  const server = findServer(serverId);
  const sv: Server =
    server ??
    ({
      id: serverId,
      engine: 'horizon',
      state: 'stopped',
      players_online: 0,
      max_players: 20,
      port: 25565,
      motd: '',
      memory_mb: 2048,
      world_name: 'world',
      horizon_version: 'v0.2.15',
    } as Server);
  s = {
    players: makePlayers(sv),
    whitelist_enabled: false,
    plugins: sv.engine === 'rustic' ? structuredClone(RUSTIC_PLUGINS) : [],
    datapacks: [
      {
        id: 'dp_vanilla_tweaks',
        name: 'Gentle Tweaks',
        version: '1.2.0',
        enabled: true,
        description: 'Small quality-of-life recipes.',
      },
      {
        id: 'dp_graves',
        name: 'Graves',
        version: '2.0.1',
        enabled: true,
        description: 'Keep your items in a grave when you die.',
      },
      {
        id: 'dp_hardmobs',
        name: 'Harder Nights',
        version: '0.4.0',
        enabled: false,
        description: 'Tougher mobs after dark.',
      },
    ],
    backups: [
      {
        id: 'bk_3',
        created_at: daysAgo(0.2),
        size_bytes: 612 * 1024 ** 2,
        kind: 'scheduled',
        note: null,
      },
      {
        id: 'bk_2',
        created_at: daysAgo(1.2),
        size_bytes: 598 * 1024 ** 2,
        kind: 'scheduled',
        note: null,
      },
      {
        id: 'bk_1',
        created_at: daysAgo(4),
        size_bytes: 571 * 1024 ** 2,
        kind: 'manual',
        note: 'Avant la mise à jour 0.2.15',
      },
    ],
    schedule: { enabled: true, interval_hours: 24, keep: 7 },
    config: makeConfig(sv),
    raw_extra: '',
    console: makeConsole(sv),
    stats: makeStats(sv),
  };
  state.set(serverId, s);
  return s;
}

/** server.toml content rendered from the config fields. */
export function renderRawConfig(server: Server, a: AdminState): string {
  const val = (f: ConfigField) =>
    typeof f.value === 'string' ? JSON.stringify(f.value) : String(f.value);
  const groups = ['general', 'world', 'players', 'network', 'performance'] as const;
  const body = groups
    .map(
      (g) =>
        `[${g}]\n${a.config
          .filter((f) => f.group === g)
          .map((f) => `${f.key} = ${val(f)}`)
          .join('\n')}`,
    )
    .join('\n\n');
  const engine =
    server.engine === 'rustic'
      ? `\n\n[engine]\n# Rustic Server engine under Horizon's supervisor\npath = "./rustic-server"`
      : '';
  return `# ${server.name} — managed by Rustic Launcher\n\n${body}${engine}\n${a.raw_extra}`;
}

export function defaultAccess(server: Server): ServerAccess {
  const host = server.hosting.mode === 'vps' ? server.hosting.host : '192.168.1.42';
  const relayOn = server.share_address?.endsWith(`.${RELAY_DOMAIN}`) ?? false;
  return {
    lan: { ip: host, port: server.port, address: `${host}:${server.port}` },
    relay: {
      enabled: relayOn,
      hostname: relayOn ? server.share_address : null,
      label: relayOn ? (server.share_address?.split('.')[0] ?? null) : null,
      status: relayOn ? (server.state === 'running' ? 'online' : 'connecting') : 'off',
    },
    invite: { code: null, expires_at: null },
    upnp: { enabled: false, status: 'off', external_address: null },
    public_ip: server.hosting.mode === 'vps' ? server.hosting.host : null,
  };
}
