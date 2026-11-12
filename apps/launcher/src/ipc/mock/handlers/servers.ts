/** Mock handlers for the servers area: list, create (local), lifecycle. */
import type { Server, ServerState } from '../../types/servers';
import { horizonVersions } from '../data/core';
import { serversDb } from '../data/servers';
import { startDownload } from '../downloads';
import { relayHost } from '@/lib/relay';
import { rusticServerStatus } from './core';
import type { StarterPlugin } from '../../types/servers';
import { emit, fail, latency, nowIso, sleep, SPEED, uid, type MockHandlers } from '../runtime';

const clone = <T>(v: T): T => structuredClone(v);

export function findServer(id: string): Server {
  return serversDb.servers.find((s) => s.id === id) ?? fail('not_found', `server ${id}`);
}

export function serversChanged(): void {
  emit('servers-changed', clone(serversDb.servers));
}

export function setServerState(s: Server, state: ServerState): void {
  s.state = state;
  emit('server-state', { server_id: s.id, state });
  serversChanged();
}

export function versionText(id: string): string {
  const v = horizonVersions.find((x) => x.id === id);
  return v ? v.label.replace(' (Rustic edition)', '') : id;
}

export function slug(name: string): string {
  return name
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

export const STARTER_PLUGINS: StarterPlugin[] = [
  {
    id: 'essentials-lite',
    name: 'Essentials Lite',
    version: '0.1.0',
    description: 'Homes, spawn, warps, tpa, kits.',
    capabilities: { network: false, filesystem: 'data-folder' },
    recommended: true,
  },
  {
    id: 'perms',
    name: 'Perms',
    version: '0.1.0',
    description: 'Groups and permissions from a TOML file.',
    capabilities: { network: false, filesystem: 'data-folder' },
    recommended: true,
  },
  {
    id: 'chat-format',
    name: 'Chat Format',
    version: '0.1.0',
    description: 'Chat prefixes, colours and mentions.',
    capabilities: { network: false, filesystem: 'none' },
    recommended: true,
  },
  {
    id: 'menus',
    name: 'Menus',
    version: '0.1.0',
    description: 'Inventory menus for server shortcuts.',
    capabilities: { network: false, filesystem: 'data-folder' },
    recommended: false,
  },
  {
    id: 'mini-game',
    name: 'Mini-game',
    version: '0.1.0',
    description: 'A small arena game to try plugins.',
    capabilities: { network: false, filesystem: 'none' },
    recommended: false,
  },
];

const WORDS_A = ['brave', 'amber', 'quiet', 'swift', 'lucky', 'misty', 'ruby', 'sunny'];
const WORDS_B = ['otter', 'fox', 'heron', 'lynx', 'badger', 'wren', 'marten', 'owl'];

/** Temporary relay label like "brave-otter-4821". */
export function relayLabel(): string {
  const r = (n: number) => Math.floor(Math.random() * n);
  return `${WORDS_A[r(WORDS_A.length)]}-${WORDS_B[r(WORDS_B.length)]}-${1000 + r(9000)}`;
}

const step = (ms: number) => sleep(ms / Math.max(0.1, SPEED));

async function start(s: Server): Promise<void> {
  setServerState(s, 'starting');
  await step(1600);
  if (s.state !== 'starting') return;
  s.last_started_at = nowIso();
  s.players_online = s.hosting.mode === 'vps' ? 3 : 0;
  setServerState(s, 'running');
  // A friend joins a bit later.
  void step(4000).then(() => {
    if (s.state === 'running') {
      s.players_online = Math.min(s.max_players, s.players_online + 1);
      emit('server-players-changed', { server_id: s.id });
      serversChanged();
    }
  });
}

async function stop(s: Server): Promise<void> {
  setServerState(s, 'stopping');
  await step(1200);
  s.players_online = 0;
  setServerState(s, 'stopped');
}

export function worldName(req: { world: { kind: string; path?: string } }): string {
  if (req.world.kind === 'import' && req.world.path)
    return req.world.path.split(/[\\/]/).pop() || 'world';
  return 'world';
}

export const serversHandlers: MockHandlers = {
  servers_starter_plugins: async () => {
    await latency(0.5);
    return structuredClone(STARTER_PLUGINS);
  },
  servers_list: async () => {
    await latency();
    return clone(serversDb.servers);
  },
  server_get: async ({ id }) => {
    await latency(0.5);
    return clone(findServer(id));
  },
  server_create: async (req) => {
    await latency(1.5);
    if (!req.name.trim()) fail('invalid_argument', 'name');
    if (!req.eula_accepted) fail('invalid_argument', 'eula');
    if (req.hosting.mode !== 'local')
      fail('invalid_argument', 'use hosting_vps_install for remote servers');
    if (req.engine === 'rustic' && !rusticServerStatus().path)
      fail(
        'unavailable',
        'The rustic-server program was not found. Choose it in Settings > Game and instances > Rustic Server, or install a Rustic edition build that ships it.',
        { reason: 'server.no_rustic_server' },
      );
    const usedPorts = serversDb.servers.map((s) => s.port);
    let port = 25565;
    while (usedPorts.includes(port)) port++;
    const server: Server = {
      id: uid('srv'),
      name: req.name.trim(),
      icon: req.icon,
      engine: req.engine,
      horizon_version: req.horizon_version,
      horizon_version_label: versionText(req.horizon_version),
      hosting: { mode: 'local' },
      state: 'installing',
      players_online: 0,
      max_players: req.max_players,
      port,
      motd:
        req.engine === 'rustic'
          ? `${req.name.trim()} · Rustic Server`
          : `${req.name.trim()} · Horizon`,
      world_name: worldName(req),
      memory_mb: req.memory_mb,
      created_at: nowIso(),
      last_started_at: null,
      share_address: req.access?.relay ? relayHost(relayLabel()) : null,
      plugin_count: req.engine === 'rustic' ? (req.starter_plugins?.length ?? 0) : 0,
      folder: `C:/Users/Sam/AppData/Roaming/RusticLauncher/servers/${slug(req.name)}`,
    };
    serversDb.servers.push(server);
    serversChanged();
    const task_id = startDownload({
      kind: 'server',
      label:
        req.engine === 'rustic'
          ? `Rustic Server · ${versionText(req.horizon_version)}`
          : versionText(req.horizon_version),
      server_id: server.id,
      total_bytes: (req.engine === 'rustic' ? 96 : 84) * 1024 ** 2,
      duration_ms: 3500,
      onDone: () => setServerState(server, 'stopped'),
      onCancel: () => {
        serversDb.servers = serversDb.servers.filter((s) => s.id !== server.id);
        serversChanged();
      },
    });
    return { server: clone(server), task_id };
  },
  server_delete: async ({ id }) => {
    await latency();
    findServer(id);
    serversDb.servers = serversDb.servers.filter((s) => s.id !== id);
    delete serversDb.access[id];
    delete serversDb.domains[id];
    serversChanged();
  },
  server_start: async ({ id }) => {
    await latency(0.4);
    const s = findServer(id);
    if (s.state === 'installing') fail('conflict', 'server is still installing');
    if (s.state !== 'running' && s.state !== 'starting') void start(s);
    return clone(s);
  },
  server_stop: async ({ id }) => {
    await latency(0.4);
    const s = findServer(id);
    if (s.state === 'running' || s.state === 'starting') void stop(s);
    return clone(s);
  },
  server_restart: async ({ id }) => {
    await latency(0.4);
    const s = findServer(id);
    void (async () => {
      if (s.state === 'running') await stop(s);
      await start(s);
    })();
    return clone(s);
  },
};
