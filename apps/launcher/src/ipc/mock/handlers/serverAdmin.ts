/** Mock handlers for server management (console, players, config, plugins, backups, stats, access). */
import { RELAY_DOMAIN, relayHost } from '@/lib/relay';
import type { ConsoleLine, ServerPlayer, ServerPlugin } from '../../types/servers';
import { admin, defaultAccess, findServer, renderRawConfig, statSample } from '../data/serverAdmin';
import { serversDb } from '../data/servers';
import { emit, fail, latency, nowIso, sleep, uid, type MockHandlers } from '../runtime';
import { slug } from './servers';

const clone = <T>(v: T): T => structuredClone(v);

function server(id: string) {
  return findServer(id) ?? fail('not_found', `server ${id}`);
}

function log(serverId: string, level: ConsoleLine['level'], text: string) {
  const line: ConsoleLine = { server_id: serverId, ts: nowIso(), level, text };
  const a = admin(serverId);
  a.console.push(line);
  if (a.console.length > 2000) a.console.splice(0, a.console.length - 2000);
  emit('server-console', line);
}

function onlinePlayers(serverId: string): ServerPlayer[] {
  const s = findServer(serverId);
  if (!s || s.state !== 'running') return [];
  return admin(serverId).players.filter((p) => p.online);
}

function syncCount(serverId: string) {
  const s = findServer(serverId);
  if (s) s.players_online = onlinePlayers(serverId).length;
  emit('server-players-changed', { server_id: serverId });
}

export function accessOf(serverId: string) {
  const s = server(serverId);
  return (serversDb.access[serverId] ??= defaultAccess(s));
}

// ---- live activity: stats every 2 s, ambient console lines ----
const CHAT = [
  'quelqu’un a du fer ?',
  'gg',
  'on se retrouve au spawn',
  'nice build!',
  'brb',
  'qui vient au nether ?',
];
let tick = 0;
function liveLoop() {
  tick += 1;
  for (const s of serversDb.servers) {
    if (s.state !== 'running') continue;
    const a = admin(s.id);
    const sample = statSample(s, nowIso(), tick);
    a.stats.push(sample);
    if (a.stats.length > 900) a.stats.splice(0, a.stats.length - 900);
    emit('server-stats', sample);
    const online = onlinePlayers(s.id);
    const r = Math.random();
    if (r < 0.12 && online.length > 0) {
      const p = online[Math.floor(Math.random() * online.length)]!;
      log(s.id, 'chat', `<${p.name}> ${CHAT[Math.floor(Math.random() * CHAT.length)]}`);
    } else if (r < 0.15) {
      const off = a.players.filter((p) => !p.online && !p.banned);
      const pick = off[Math.floor(Math.random() * off.length)];
      if (pick && online.length < s.max_players) {
        pick.online = true;
        pick.joined_at = nowIso();
        pick.ping_ms = 20 + Math.floor(Math.random() * 80);
        log(s.id, 'info', `${pick.name} joined the game`);
        syncCount(s.id);
      }
    } else if (r < 0.165 && online.length > 2) {
      const p = online[online.length - 1]!;
      p.online = false;
      p.ping_ms = null;
      log(s.id, 'info', `${p.name} left the game`);
      syncCount(s.id);
    }
    if (tick % 30 === 0) {
      log(s.id, 'info', 'Saving the game (autosave)');
      log(s.id, 'info', 'Saved the game');
    }
  }
}
if (typeof window !== 'undefined') setInterval(liveLoop, 2000);

const COMMANDS = [
  'help',
  'list',
  'say',
  'msg',
  'time',
  'weather',
  'gamemode',
  'tp',
  'op',
  'deop',
  'kick',
  'ban',
  'pardon',
  'whitelist',
  'tps',
  'save-all',
  'difficulty',
  'rustic',
];
const PLAYER_ARG = new Set(['op', 'deop', 'kick', 'ban', 'pardon', 'tp', 'msg']);
const SUBS: Record<string, string[]> = {
  time: ['set day', 'set night', 'set noon', 'query daytime'],
  weather: ['clear', 'rain', 'thunder'],
  whitelist: ['on', 'off', 'add', 'remove', 'list'],
  difficulty: ['peaceful', 'easy', 'normal', 'hard'],
  rustic: ['plugins', 'reload', 'timings', 'version'],
  gamemode: ['survival', 'creative', 'adventure', 'spectator'],
};

function playerAction(serverId: string, name: string, action: string, reason?: string): boolean {
  const a = admin(serverId);
  const p = a.players.find((x) => x.name.toLowerCase() === name.toLowerCase());
  if (!p) return false;
  switch (action) {
    case 'kick':
      if (!p.online) return false;
      p.online = false;
      p.ping_ms = null;
      log(serverId, 'info', `Kicked ${p.name}: ${reason || 'Kicked by an operator'}`);
      break;
    case 'ban':
      p.banned = true;
      p.online = false;
      p.ping_ms = null;
      log(serverId, 'info', `Banned ${p.name}: ${reason || 'Banned by an operator'}`);
      break;
    case 'pardon':
      p.banned = false;
      log(serverId, 'info', `Unbanned ${p.name}`);
      break;
    case 'op':
      p.op = true;
      log(serverId, 'info', `Made ${p.name} a server operator`);
      break;
    case 'deop':
      p.op = false;
      log(serverId, 'info', `Made ${p.name} no longer a server operator`);
      break;
    case 'whitelist_add':
      p.whitelisted = true;
      log(serverId, 'info', `Added ${p.name} to the whitelist`);
      break;
    case 'whitelist_remove':
      p.whitelisted = false;
      log(serverId, 'info', `Removed ${p.name} from the whitelist`);
      break;
  }
  syncCount(serverId);
  return true;
}

function runCommand(serverId: string, raw: string) {
  const s = server(serverId);
  const line = raw.trim().replace(/^\//, '');
  log(serverId, 'command', `> ${line}`);
  if (s.state !== 'running') {
    log(serverId, 'error', 'The server is not running.');
    return;
  }
  const [cmd = '', ...args] = line.split(/\s+/);
  const a = admin(serverId);
  const online = onlinePlayers(serverId);
  switch (cmd.toLowerCase()) {
    case 'help':
      log(serverId, 'info', `Commands: ${COMMANDS.join(', ')}`);
      break;
    case 'list':
      log(
        serverId,
        'info',
        `There are ${online.length} of a max of ${s.max_players} players online: ${online.map((p) => p.name).join(', ')}`,
      );
      break;
    case 'say':
      log(serverId, 'chat', `[Server] ${args.join(' ')}`);
      break;
    case 'msg':
      log(serverId, 'info', `You whisper to ${args[0] ?? '?'}: ${args.slice(1).join(' ')}`);
      break;
    case 'time': {
      const named: Record<string, number> = { day: 1000, night: 13000, noon: 6000 };
      log(
        serverId,
        'info',
        args[0] === 'query'
          ? 'The time is 6000'
          : `Set the time to ${named[args[1] ?? ''] ?? args[1] ?? 0}`,
      );
      break;
    }
    case 'weather':
      log(serverId, 'info', `Set the weather to ${args[0] ?? 'clear'}`);
      break;
    case 'tps':
      log(
        serverId,
        'info',
        `TPS from last 1m, 5m, 15m: ${a.stats.at(-1)?.tps.toFixed(2) ?? '20.00'}, 20.00, 19.98`,
      );
      break;
    case 'save-all':
      log(serverId, 'info', 'Saving the game (this may take a moment!)');
      log(serverId, 'info', 'Saved the game');
      break;
    case 'difficulty':
      log(serverId, 'info', `The difficulty has been set to ${args[0] ?? 'normal'}`);
      break;
    case 'gamemode':
      log(serverId, 'info', `Set ${args[1] ?? 'own'} game mode to ${args[0] ?? 'survival'}`);
      break;
    case 'tp':
      log(
        serverId,
        'info',
        `Teleported ${args[0] ?? '?'} to ${args.slice(1).join(' ') || 'spawn'}`,
      );
      break;
    case 'op':
    case 'deop':
    case 'kick':
    case 'ban':
    case 'pardon':
      if (!args[0] || !playerAction(serverId, args[0], cmd, args.slice(1).join(' '))) {
        log(serverId, 'error', `No player was found: ${args[0] ?? ''}`);
      }
      break;
    case 'whitelist': {
      const sub = args[0];
      if (sub === 'on' || sub === 'off') {
        a.whitelist_enabled = sub === 'on';
        log(serverId, 'info', `Whitelist is now turned ${sub}`);
      } else if ((sub === 'add' || sub === 'remove') && args[1]) {
        if (!playerAction(serverId, args[1], `whitelist_${sub}`))
          log(serverId, 'error', `No player was found: ${args[1]}`);
      } else {
        const wl = a.players.filter((p) => p.whitelisted);
        log(
          serverId,
          'info',
          `There are ${wl.length} whitelisted players: ${wl.map((p) => p.name).join(', ')}`,
        );
      }
      break;
    }
    case 'rustic':
      if (s.engine !== 'rustic') {
        log(
          serverId,
          'error',
          'Unknown command. This server runs vanilla Horizon (no Rustic plugins).',
        );
      } else if (args[0] === 'plugins') {
        log(
          serverId,
          'info',
          `Plugins (${a.plugins.length}): ${a.plugins.map((p) => `${p.id} ${p.version} [${p.state}]`).join(', ')}`,
        );
      } else if (args[0] === 'timings') {
        log(
          serverId,
          'info',
          'Timings: essentials-lite 0.21 ms/tick, perms 0.03 ms/tick, chat-format 0.01 ms/tick',
        );
      } else {
        log(serverId, 'info', `Rustic Server 0.1.0 (api 0.1) on Horizon ${s.horizon_version}`);
      }
      break;
    default:
      log(serverId, 'error', `Unknown or incomplete command "${cmd}". Type "help" for help.`);
  }
}

function requireRustic(serverId: string) {
  const s = server(serverId);
  if (s.engine !== 'rustic')
    fail('invalid_argument', 'Plugins need the Rustic Server engine', {
      reason: 'needs_rustic_server',
    });
}

function plugin(serverId: string, pluginId: string): ServerPlugin {
  return admin(serverId).plugins.find((p) => p.id === pluginId) ?? fail('not_found', pluginId);
}

/** Completions for a partial console line (full lines). Exported for unit tests. */
export function complete(serverId: string, partial: string): string[] {
  const text = partial.replace(/^\//, '');
  const parts = text.split(' ');
  if (parts.length <= 1)
    return COMMANDS.filter((c) => c.startsWith((parts[0] ?? '').toLowerCase()));
  const [cmd = '', ...rest] = parts;
  const last = rest.at(-1) ?? '';
  const head = [cmd, ...rest.slice(0, -1)].join(' ');
  let pool: string[] = [];
  const names = admin(serverId).players.map((p) => p.name);
  if (PLAYER_ARG.has(cmd) && rest.length === 1) pool = names;
  else if (cmd === 'whitelist' && rest.length === 2 && (rest[0] === 'add' || rest[0] === 'remove'))
    pool = names;
  else if (SUBS[cmd] && rest.length === 1) pool = SUBS[cmd]!;
  return pool
    .filter((o) => o.toLowerCase().startsWith(last.toLowerCase()))
    .map((o) => `${head} ${o}`);
}

export const serverAdminHandlers: MockHandlers = {
  server_command: async ({ server_id, line }) => {
    await latency(0.3);
    runCommand(server_id, line);
  },
  server_console_history: async ({ server_id, limit }) => {
    await latency(0.5);
    server(server_id);
    return clone(admin(server_id).console.slice(-limit));
  },
  server_complete: ({ server_id, partial }) => complete(server_id, partial).slice(0, 12),
  server_players: async ({ server_id }) => {
    await latency(0.6);
    const s = server(server_id);
    return clone(
      admin(server_id).players.map((p) =>
        s.state === 'running' ? p : { ...p, online: false, ping_ms: null },
      ),
    );
  },
  server_player_action: async ({ server_id, player, action, reason }) => {
    await latency(0.5);
    if (!playerAction(server_id, player, action, reason)) fail('not_found', player);
    return clone(admin(server_id).players);
  },
  server_whitelist_set_enabled: async ({ server_id, enabled }) => {
    await latency(0.3);
    const a = admin(server_id);
    a.whitelist_enabled = enabled;
    const f = a.config.find((x) => x.key === 'white-list');
    if (f) f.value = enabled;
    log(server_id, 'info', `Whitelist is now turned ${enabled ? 'on' : 'off'}`);
  },
  server_config_get: async ({ server_id }) => {
    await latency(0.6);
    server(server_id);
    return { fields: clone(admin(server_id).config) };
  },
  server_config_set: async ({ server_id, values }) => {
    await latency();
    const s = server(server_id);
    const a = admin(server_id);
    for (const [k, v] of Object.entries(values)) {
      const f = a.config.find((x) => x.key === k) ?? fail('invalid_argument', `unknown key ${k}`);
      if (f.type === 'integer') {
        const n = Number(v);
        if (!Number.isInteger(n) || (f.min != null && n < f.min) || (f.max != null && n > f.max))
          fail('invalid_argument', k);
        f.value = n;
      } else if (f.type === 'boolean') f.value = Boolean(v);
      else if (f.type === 'enum') {
        if (!f.options?.includes(String(v))) fail('invalid_argument', k);
        f.value = String(v);
      } else f.value = String(v);
      if (k === 'motd') s.motd = String(v);
      if (k === 'max-players') s.max_players = Number(v);
      if (k === 'server-port') s.port = Number(v);
      if (k === 'white-list') a.whitelist_enabled = Boolean(v);
    }
    return { fields: clone(a.config) };
  },
  server_config_raw_get: async ({ server_id }) => {
    await latency(0.5);
    const s = server(server_id);
    return {
      path: `${s.folder}/server.toml`,
      format: 'toml',
      content: renderRawConfig(s, admin(server_id)),
    };
  },
  server_config_raw_set: async ({ server_id, content }) => {
    await latency();
    const s = server(server_id);
    const a = admin(server_id);
    for (const line of content.split('\n')) {
      const m = /^\s*([a-z-]+)\s*=\s*(.+?)\s*$/.exec(line);
      if (!m) continue;
      const f = a.config.find((x) => x.key === m[1]);
      if (!f) continue;
      const v = m[2]!;
      if (f.type === 'integer') {
        if (!/^-?\d+$/.test(v)) fail('invalid_argument', `${f.key}: integer expected`);
        f.value = Number(v);
      } else if (f.type === 'boolean') {
        if (v !== 'true' && v !== 'false')
          fail('invalid_argument', `${f.key}: true or false expected`);
        f.value = v === 'true';
      } else {
        f.value = v.replace(/^"(.*)"$/, '$1');
      }
    }
    const motd = a.config.find((f) => f.key === 'motd');
    if (motd) s.motd = String(motd.value);
    return { path: `${s.folder}/server.toml`, format: 'toml', content: renderRawConfig(s, a) };
  },
  server_plugins_list: async ({ server_id }) => {
    await latency(0.6);
    server(server_id);
    return clone(admin(server_id).plugins);
  },
  server_plugin_install: async ({ server_id, project_id, path }) => {
    requireRustic(server_id);
    await latency(2);
    const id =
      project_id ??
      path
        ?.split(/[\\/]/)
        .pop()
        ?.replace(/\.rplugin$/, '') ??
      uid('plugin');
    const existing = admin(server_id).plugins.find((p) => p.id === id);
    if (existing) return clone(existing);
    const p: ServerPlugin = {
      id,
      name: id
        .replace(/^rp_/, '')
        .replace(/[-_]/g, ' ')
        .replace(/\b\w/g, (c) => c.toUpperCase()),
      version: '1.0.0',
      api: '0.1',
      authors: ['Community'],
      license: 'MIT',
      description: '',
      state: 'enabled',
      update_available: null,
      capabilities: { network: false, filesystem: 'none' },
      project_id: project_id ?? null,
    };
    admin(server_id).plugins.push(p);
    const s = server(server_id);
    s.plugin_count = admin(server_id).plugins.length;
    log(server_id, 'info', `[Rustic] Installed ${p.id} ${p.version}; enabled on the fly`);
    return clone(p);
  },
  server_plugin_set_enabled: async ({ server_id, plugin_id, enabled }) => {
    requireRustic(server_id);
    await latency(0.6);
    const p = plugin(server_id, plugin_id);
    p.state = enabled ? 'enabled' : 'disabled';
    delete p.error;
    log(server_id, 'info', `[Rustic] ${enabled ? 'Enabled' : 'Disabled'} ${p.id} ${p.version}`);
    return clone(p);
  },
  server_plugin_update: async ({ server_id, plugin_id }) => {
    requireRustic(server_id);
    await latency(2);
    const p = plugin(server_id, plugin_id);
    if (p.update_available) {
      p.version = p.update_available;
      p.update_available = null;
      if (p.state === 'errored') {
        p.state = 'enabled';
        delete p.error;
      }
      log(server_id, 'info', `[Rustic] Reloaded ${p.id} ${p.version}`);
    }
    return clone(p);
  },
  server_plugin_remove: async ({ server_id, plugin_id }) => {
    requireRustic(server_id);
    await latency();
    const a = admin(server_id);
    a.plugins = a.plugins.filter((p) => p.id !== plugin_id);
    server(server_id).plugin_count = a.plugins.length;
    log(server_id, 'info', `[Rustic] Removed ${plugin_id}`);
  },
  server_datapacks_list: async ({ server_id }) => {
    await latency(0.5);
    return clone(admin(server_id).datapacks);
  },
  server_datapack_set_enabled: async ({ server_id, datapack_id, enabled }) => {
    await latency(0.4);
    const d =
      admin(server_id).datapacks.find((x) => x.id === datapack_id) ??
      fail('not_found', datapack_id);
    d.enabled = enabled;
    log(
      server_id,
      'info',
      `${enabled ? 'Enabled' : 'Disabled'} data pack ${d.name}; run /reload to apply`,
    );
    return clone(d);
  },
  server_datapack_remove: async ({ server_id, datapack_id }) => {
    await latency();
    const a = admin(server_id);
    a.datapacks = a.datapacks.filter((d) => d.id !== datapack_id);
  },
  server_backups_list: async ({ server_id }) => {
    await latency(0.6);
    return clone(admin(server_id).backups);
  },
  server_backup_create: async ({ server_id, note }) => {
    await latency(4);
    const b = {
      id: uid('bk'),
      created_at: nowIso(),
      size_bytes: 620 * 1024 ** 2 + Math.floor(Math.random() * 30 * 1024 ** 2),
      kind: 'manual' as const,
      note: note?.trim() || null,
    };
    admin(server_id).backups.unshift(b);
    log(server_id, 'info', `Backup ${b.id} created`);
    return clone(b);
  },
  server_backup_restore: async ({ server_id, backup_id }) => {
    const a = admin(server_id);
    const b = a.backups.find((x) => x.id === backup_id) ?? fail('not_found', backup_id);
    await latency(5);
    a.backups.unshift({
      id: uid('bk'),
      created_at: nowIso(),
      size_bytes: b.size_bytes,
      kind: 'pre_restore',
      note: null,
    });
    log(server_id, 'warn', `World restored from the backup of ${b.created_at}`);
  },
  server_backup_delete: async ({ server_id, backup_id }) => {
    await latency(0.5);
    const a = admin(server_id);
    a.backups = a.backups.filter((b) => b.id !== backup_id);
  },
  server_backup_schedule_get: async ({ server_id }) => {
    await latency(0.4);
    return clone(admin(server_id).schedule);
  },
  server_backup_schedule_set: async ({ server_id, enabled, interval_hours, keep }) => {
    await latency(0.5);
    if (interval_hours < 1 || keep < 1) fail('invalid_argument', 'schedule');
    admin(server_id).schedule = { enabled, interval_hours, keep };
    return clone(admin(server_id).schedule);
  },
  server_stats_history: async ({ server_id, minutes }) => {
    await latency(0.5);
    server(server_id);
    const since = Date.now() - minutes * 60_000;
    return clone(admin(server_id).stats.filter((s) => Date.parse(s.ts) >= since));
  },
  server_access_get: async ({ server_id }) => {
    await latency(0.5);
    return clone(accessOf(server_id));
  },
  server_relay_set: async ({ server_id, enabled, label }) => {
    await latency(1.5);
    const s = server(server_id);
    const acc = accessOf(server_id);
    const relaySuffix = `.${RELAY_DOMAIN}`;
    if (!enabled) {
      acc.relay = { enabled: false, hostname: null, label: acc.relay.label, status: 'off' };
      if (s.share_address?.endsWith(relaySuffix))
        s.share_address = serversDb.domains[server_id]?.fqdn ?? null;
      return clone(acc);
    }
    const l = (label ?? acc.relay.label ?? slug(s.name)).toLowerCase();
    if (!/^[a-z0-9](?:[a-z0-9-]{1,30}[a-z0-9])$/.test(l)) fail('invalid_argument', 'label');
    if (['admin', 'www', 'rustic', 'play', 'relay'].includes(l)) fail('conflict', 'label taken');
    const hostname = relayHost(l);
    acc.relay = {
      enabled: true,
      hostname,
      label: l,
      status: s.state === 'running' ? 'online' : 'connecting',
    };
    if (serversDb.domains[server_id]?.status !== 'verified') s.share_address = hostname;
    return clone(acc);
  },
  server_invite_create: async ({ server_id }) => {
    await latency();
    const acc = accessOf(server_id);
    const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
    const part = () =>
      Array.from({ length: 4 }, () => alphabet[Math.floor(Math.random() * alphabet.length)]).join(
        '',
      );
    acc.invite = {
      code: `RX-${part()}-${part()}`,
      expires_at: new Date(Date.now() + 24 * 3600_000).toISOString(),
    };
    return clone(acc);
  },
  server_invite_revoke: async ({ server_id }) => {
    await latency(0.5);
    const acc = accessOf(server_id);
    acc.invite = { code: null, expires_at: null };
    return clone(acc);
  },
  server_upnp_set: async ({ server_id, enabled }) => {
    const s = server(server_id);
    if (s.hosting.mode !== 'local')
      fail('unavailable', 'UPnP only applies to servers hosted on this PC');
    const acc = accessOf(server_id);
    if (!enabled) {
      await latency(0.5);
      acc.upnp = { enabled: false, status: 'off', external_address: null };
      return clone(acc);
    }
    acc.upnp = { enabled: true, status: 'mapping', external_address: null };
    await sleep(1200);
    acc.upnp = { enabled: true, status: 'mapped', external_address: `198.51.100.5:${s.port}` };
    return clone(acc);
  },
};
