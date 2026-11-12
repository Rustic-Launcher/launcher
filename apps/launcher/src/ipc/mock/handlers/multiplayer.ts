/** Mock handlers for the game's multiplayer list (servers.dat) and server pings. */
import type { MultiplayerEntry, ServerPing } from '../../types/multiplayer';
import { relayHost } from '@/lib/relay';
import { daysAgo, fail, latency, minutesAgo, sleep, uid, type MockHandlers } from '../runtime';

let entries: MultiplayerEntry[] = [
  {
    id: 'mp_copains',
    name: 'Les Copains SMP',
    address: relayHost('copains'),
    favorite: true,
    last_joined_at: minutesAgo(60 * 20),
  },
  {
    id: 'mp_arena',
    name: 'Arena Nord',
    address: 'arena-nord.fr',
    favorite: true,
    last_joined_at: daysAgo(3),
  },
  {
    id: 'mp_sky',
    name: 'Skyreach',
    address: 'play.skyreach.gg',
    favorite: false,
    last_joined_at: daysAgo(12),
  },
  {
    id: 'mp_guild',
    name: 'Builders Guild',
    address: relayHost('guild'),
    favorite: false,
    last_joined_at: daysAgo(30),
  },
  {
    id: 'mp_lan',
    name: 'Serveur de Tom',
    address: '192.168.1.42:25565',
    favorite: false,
    last_joined_at: daysAgo(45),
  },
  {
    id: 'mp_old',
    name: 'Old Factions',
    address: 'factions.example.net',
    favorite: false,
    last_joined_at: daysAgo(160),
  },
];

type PingSeed = Omit<ServerPing, 'latency_ms'> & { base: number | null };

const pings: Record<string, PingSeed> = {
  [relayHost('copains')]: {
    online: true,
    base: 18,
    motd: 'Bienvenue chez les copains ! Event citrouilles ce samedi',
    players_online: 4,
    players_max: 20,
    version: 'Horizon 0.2.15',
  },
  'arena-nord.fr': {
    online: true,
    base: 24,
    motd: 'Arena Nord · Ranked duels saison 4',
    players_online: 212,
    players_max: 500,
    version: 'Horizon 0.2.15',
  },
  'play.skyreach.gg': {
    online: true,
    base: 96,
    motd: 'Skyreach · co-op skyblock, weekly challenges',
    players_online: 91,
    players_max: 300,
    version: 'Horizon 0.2.14',
  },
  [relayHost('guild')]: {
    online: true,
    base: 41,
    motd: 'Builders Guild · creative plots',
    players_online: 54,
    players_max: 150,
    version: 'Horizon 0.2.15',
  },
  '192.168.1.42:25565': {
    online: false,
    base: null,
    motd: '',
    players_online: 0,
    players_max: 0,
    version: '',
  },
  'factions.example.net': {
    online: false,
    base: null,
    motd: '',
    players_online: 0,
    players_max: 0,
    version: '',
  },
};

export const ADDRESS_RE = /^[a-z0-9.-]+(:\d{1,5})?$/i;

export const multiplayerHandlers: MockHandlers = {
  multiplayer_list: async () => {
    await latency(0.6);
    return structuredClone(entries);
  },
  multiplayer_add: async ({ name, address }) => {
    await latency(0.4);
    if (!name.trim()) fail('invalid_argument', 'name');
    if (!ADDRESS_RE.test(address.trim())) fail('invalid_argument', 'address');
    const e: MultiplayerEntry = {
      id: uid('mp'),
      name: name.trim(),
      address: address.trim(),
      favorite: false,
      last_joined_at: null,
    };
    entries = [...entries, e];
    return structuredClone(e);
  },
  multiplayer_update: async ({ id, patch }) => {
    await latency(0.3);
    const e = entries.find((x) => x.id === id) ?? fail('not_found', id);
    if (patch.address !== undefined && !ADDRESS_RE.test(patch.address.trim()))
      fail('invalid_argument', 'address');
    Object.assign(e, patch);
    return structuredClone(e);
  },
  multiplayer_remove: async ({ id }) => {
    await latency(0.3);
    entries = entries.filter((x) => x.id !== id);
  },
  multiplayer_ping: async ({ address }) => {
    const p = pings[address.trim().toLowerCase()];
    if (!p) {
      // Unknown servers: a plausible answer derived from the address.
      await sleep(400);
      const h = [...address].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 997, 7);
      return {
        online: true,
        latency_ms: 30 + (h % 90),
        motd: 'A Horizon server',
        players_online: h % 40,
        players_max: 100,
        version: 'Horizon 0.2.15',
      };
    }
    await sleep(p.base == null ? 1200 : 150 + p.base * 2);
    return {
      online: p.online,
      latency_ms: p.base == null ? null : Math.round(p.base * (0.85 + Math.random() * 0.3)),
      motd: p.motd,
      players_online: p.players_online,
      players_max: p.players_max,
      version: p.version,
    };
  },
};
