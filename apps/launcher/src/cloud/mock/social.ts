import type { CloudClient } from '../client';
import { CloudError } from '../errors';
import type {
  AppNotification,
  ChatMessage,
  Conversation,
  FeaturedServer,
  Friend,
  FriendRequest,
  FriendSuggestion,
  FriendTarget,
  NewsItem,
  Party,
  RealtimeEvent,
  UserLite,
} from '../types';
import { latency, minutesAgo, daysAgo, uid, sleep } from '../../ipc/mock/runtime';
import { mockDiscoverable } from './account';

const clone = <T>(v: T): T => structuredClone(v);

export const friendsDb: Friend[] = [
  {
    id: 'u_lea',
    username: 'lea_builds',
    display_name: 'Léa',
    presence: 'in-game',
    favorite: true,
    last_seen_at: null,
    activity: {
      kind: 'server',
      server_name: 'Les Copains SMP',
      server_address: 'copains.relay.example.invalid',
      instance_name: 'Survie entre amis',
      joinable: true,
      since: minutesAgo(35),
    },
  },
  {
    id: 'u_max',
    username: 'MaxRedstone',
    display_name: 'Max',
    presence: 'online',
    favorite: false,
    last_seen_at: null,
    activity: { kind: 'launcher', joinable: false, since: minutesAgo(5) },
  },
  {
    id: 'u_noah',
    username: 'noah.pvp',
    display_name: 'Noah',
    presence: 'in-game',
    favorite: false,
    last_seen_at: null,
    activity: {
      kind: 'server',
      server_name: 'Arena Nord',
      server_address: 'arena-nord.fr',
      joinable: true,
      since: minutesAgo(12),
    },
  },
  {
    id: 'u_ines',
    username: 'InesCraft',
    display_name: 'Inès',
    presence: 'away',
    favorite: true,
    last_seen_at: null,
    activity: {
      kind: 'singleplayer',
      instance_name: 'Creative Builds',
      joinable: false,
      since: minutesAgo(80),
    },
  },
  {
    id: 'u_tom',
    username: 'tom_h',
    display_name: 'Tom',
    presence: 'busy',
    favorite: false,
    last_seen_at: null,
    activity: {
      kind: 'hosting',
      server_name: 'Tom’s Modded',
      joinable: true,
      server_address: 'toms-modded.relay.example.invalid',
      since: minutesAgo(140),
    },
  },
  {
    id: 'u_emma',
    username: 'emma.w',
    display_name: 'Emma',
    presence: 'offline',
    favorite: false,
    last_seen_at: daysAgo(1),
    activity: null,
  },
  {
    id: 'u_lucas',
    username: 'Lucas_77',
    display_name: 'Lucas',
    presence: 'offline',
    favorite: false,
    last_seen_at: daysAgo(4),
    activity: null,
  },
  {
    id: 'u_jade',
    username: 'jade.dev',
    display_name: 'Jade',
    presence: 'offline',
    favorite: false,
    last_seen_at: minutesAgo(300),
    activity: null,
  },
];

let requestsDb: FriendRequest[] = [
  {
    id: 'fr_1',
    user: { id: 'u_hugo', username: 'HugoMines', display_name: 'Hugo' },
    direction: 'incoming',
    created_at: minutesAgo(50),
  },
  {
    id: 'fr_2',
    user: { id: 'u_chloe', username: 'chloe_art', display_name: 'Chloé' },
    direction: 'outgoing',
    created_at: daysAgo(2),
  },
];

let partyDb: Party | null = {
  id: 'party_1',
  leader_id: 'usr_sam',
  max_size: 8,
  members: [
    { id: 'usr_sam', username: 'samcrafte', display_name: 'Sam', presence: 'online' },
    { id: 'u_lea', username: 'lea_builds', display_name: 'Léa', presence: 'in-game' },
  ],
  pending_invites: [],
};

/** Rustic accounts the mock knows, with their linked Minecraft name (example data). */
interface DirectoryEntry extends UserLite {
  minecraft_name: string | null;
  /** Under 18: their Minecraft name is only echoed when it is what was searched. */
  minor?: boolean;
  /** Blocked (either way) or parental controls forbid requests: both look like `cannot_add_friend`. */
  unreachable?: boolean;
}

const ME_USERNAME = 'samcrafte';
const ME_MINECRAFT = 'SamCrafte';

const directory: DirectoryEntry[] = [
  { id: 'u_hugo', username: 'HugoMines', display_name: 'Hugo', minecraft_name: 'Hugo_Mines' },
  { id: 'u_chloe', username: 'chloe_art', display_name: 'Chloé', minecraft_name: 'ChloeArt' },
  { id: 'u_alex', username: 'alex_rs', display_name: 'Alex', minecraft_name: 'AlexRS' },
  { id: 'u_nina', username: 'NinaSky', display_name: 'Nina', minecraft_name: 'Nina_Sky' },
  { id: 'u_paul', username: 'paul.builds', display_name: 'Paul', minecraft_name: null },
  { id: 'u_rayan', username: 'RayanPvP', display_name: 'Rayan', minecraft_name: 'RayanTheBold' },
  { id: 'u_yanis', username: 'yanis_b', display_name: 'Yanis', minecraft_name: 'YanisBuilds' },
  { id: 'u_sara', username: 'SaraLight', display_name: 'Sara', minecraft_name: 'Sara_Light' },
  { id: 'u_ilyes', username: 'ilyes_mc', display_name: 'Ilyès', minecraft_name: 'Ilyes42' },
  {
    id: 'u_zoe',
    username: 'zoe_crafts',
    display_name: 'Zoé',
    minecraft_name: 'ZoeCrafts',
    minor: true,
  },
  {
    id: 'u_kev',
    username: 'kev_grief',
    display_name: 'Kévin',
    minecraft_name: 'KevGrief',
    unreachable: true,
  },
  {
    id: 'u_milo',
    username: 'milo_jr',
    display_name: 'Milo',
    minecraft_name: 'MiloJr',
    minor: true,
    unreachable: true,
  },
];
const friendMinecraft: Record<string, string> = { u_lea: 'LeaBuilds', u_max: 'MaxRedstone' };

/** Players met recently (ADR 0145), before excluding friends and pending requests. */
const suggestionPool: { played_with: FriendSuggestion[]; crossed: FriendSuggestion[] } = {
  played_with: [
    suggestion('u_alex', 'party', null, minutesAgo(120)),
    suggestion('u_yanis', 'server', 'Les Copains SMP', daysAgo(1)),
    suggestion('u_sara', 'server', 'Arena Nord', daysAgo(3)),
  ],
  crossed: [
    suggestion('u_nina', 'server', 'Skyreach', minutesAgo(300)),
    suggestion('u_ilyes', 'server', 'Arena Nord', daysAgo(2)),
    suggestion('u_paul', 'server', 'Builders Guild', daysAgo(6)),
  ],
};

function suggestion(
  id: string,
  reason: FriendSuggestion['reason'],
  server: string | null,
  seenAt: string,
): FriendSuggestion {
  const d = directory.find((u) => u.id === id)!;
  return {
    user: { id: d.id, username: d.username, display_name: d.display_name },
    minecraft_name: d.minecraft_name,
    reason,
    server_name: server,
    seen_at: seenAt,
  };
}

const cloudError = (code: string, message: string, status: number) =>
  new CloudError(code, message, status);

// Same limit as the API (30 lookups per minute).
let lookupTimes: number[] = [];
function lookupRateLimit() {
  const now = Date.now();
  lookupTimes = lookupTimes.filter((t) => now - t < 60_000);
  if (lookupTimes.length >= 30) throw cloudError('rate_limited', 'too many requests', 429);
  lookupTimes.push(now);
}

const NAME = { rustic: /^[A-Za-z0-9_]{3,20}$/, minecraft: /^[A-Za-z0-9_]{3,16}$/ };

interface Resolved {
  user: UserLite;
  minecraft_name: string | null;
  minor: boolean;
  unreachable: boolean;
  friend: boolean;
}

/** Exact, case-insensitive match over friends and the directory; `null` = no such account. */
function resolve(by: 'rustic' | 'minecraft' | 'id', value: string): Resolved | 'self' | null {
  const v = value.trim().toLowerCase();
  if (
    (by === 'rustic' && v === ME_USERNAME.toLowerCase()) ||
    (by === 'minecraft' && v === ME_MINECRAFT.toLowerCase()) ||
    (by === 'id' && value === ME)
  )
    return 'self';
  for (const f of friendsDb) {
    const mc = friendMinecraft[f.id] ?? null;
    const hit =
      by === 'id'
        ? f.id === value
        : by === 'rustic'
          ? f.username.toLowerCase() === v
          : mc?.toLowerCase() === v;
    if (hit) {
      const user = { id: f.id, username: f.username, display_name: f.display_name };
      return { user, minecraft_name: mc, minor: false, unreachable: false, friend: true };
    }
  }
  const d = directory.find((u) =>
    by === 'id'
      ? u.id === value
      : by === 'rustic'
        ? u.username.toLowerCase() === v
        : u.minecraft_name?.toLowerCase() === v,
  );
  if (!d) return null;
  return {
    user: { id: d.id, username: d.username, display_name: d.display_name },
    minecraft_name: d.minecraft_name,
    minor: !!d.minor,
    unreachable: !!d.unreachable,
    friend: false,
  };
}

const newsDb: NewsItem[] = [
  {
    id: 'n1',
    source: 'rustic',
    kind: 'patch_notes',
    version: '0.1.0',
    title: 'Rustic Launcher 0.1 beta',
    summary: 'Instances, content, servers you host in two clicks, and the Locker.',
    published_at: daysAgo(1),
    url: 'https://rustic.example.invalid/news/launcher-0-1',
    hue: 22,
  },
  {
    id: 'n2',
    source: 'horizon',
    kind: 'patch_notes',
    version: '0.2.15',
    title: 'Horizon 0.2.15',
    summary: 'Faster chunk meshing, shader pack hot reload, and fixes for redstone timing.',
    published_at: daysAgo(1),
    url: 'https://github.com/horizon-rust/horizon/releases',
    hue: 205,
  },
  {
    id: 'n3',
    source: 'rustic',
    kind: 'event',
    title: 'Autumn build contest',
    summary:
      'Build a harvest village on a Rustic Server, win a cosmetic bundle for your whole party.',
    published_at: daysAgo(4),
    url: 'https://rustic.example.invalid/events/autumn',
    hue: 32,
  },
  {
    id: 'n4',
    source: 'horizon',
    kind: 'blog',
    title: 'How Horizon prepares your game files',
    summary: 'Why your own install is the source of truth, and what horizon-prepare does with it.',
    published_at: daysAgo(8),
    url: 'https://github.com/horizon-rust/horizon',
    hue: 260,
  },
];

const featuredDb: FeaturedServer[] = [
  {
    id: 'fs1',
    name: 'Les Copains SMP',
    address: 'copains.relay.example.invalid',
    description: 'Survie détente, économie joueurs, events le week-end.',
    tags: ['survival', 'smp'],
    engine: 'rustic',
    players_online: 38,
    players_max: 100,
    language: 'fr',
    verified: true,
    hue: 140,
  },
  {
    id: 'fs2',
    name: 'Arena Nord',
    address: 'arena-nord.fr',
    description: 'PvP 1v1, ranked duels and practice kits.',
    tags: ['pvp', 'minigames'],
    engine: 'rustic',
    players_online: 212,
    players_max: 500,
    language: 'fr',
    verified: true,
    hue: 4,
  },
  {
    id: 'fs3',
    name: 'Skyreach',
    address: 'play.skyreach.gg',
    description: 'Skyblock islands with co-op and weekly challenges.',
    tags: ['skyblock'],
    engine: 'horizon',
    players_online: 91,
    players_max: 300,
    language: 'en',
    verified: false,
    hue: 195,
  },
  {
    id: 'fs4',
    name: 'Builders Guild',
    address: 'guild.relay.example.invalid',
    description: 'Creative plots, WorldEdit-like tools as Rustic plugins.',
    tags: ['creative'],
    engine: 'rustic',
    players_online: 54,
    players_max: 150,
    language: 'en',
    verified: true,
    hue: 45,
  },
];

let notificationsDb: AppNotification[] = [
  {
    id: 'no1',
    kind: 'friend_request',
    title: 'HugoMines',
    body: 'friend_request',
    created_at: minutesAgo(50),
    read: false,
  },
  {
    id: 'no2',
    kind: 'update',
    title: 'Horizon 0.2.15',
    body: 'update_available',
    created_at: daysAgo(1),
    read: false,
    href: '/instances',
  },
  {
    id: 'no3',
    kind: 'server',
    title: 'Les Copains SMP',
    body: 'backup_done',
    created_at: daysAgo(1),
    read: true,
    href: '/servers',
  },
  {
    id: 'no4',
    kind: 'store',
    title: 'Autumn Lantern',
    body: 'store_new',
    created_at: daysAgo(3),
    read: true,
    href: '/store',
  },
];

const ME = 'usr_sam';
const msg = (from: string, to: string, body: string, minutes: number): ChatMessage => ({
  id: uid('msg'),
  from_id: from,
  to_id: to,
  body,
  sent_at: minutesAgo(minutes),
});
const chatsDb: Record<string, ChatMessage[]> = {
  u_lea: [
    msg('u_lea', ME, 'On lance le donjon ce soir ?', 95),
    msg(ME, 'u_lea', 'Carrément, vers 21h ?', 90),
    msg('u_lea', ME, 'Parfait, je prépare les potions', 88),
    msg('u_lea', ME, 'Tu viens sur Les Copains ? On est 4', 6),
  ],
  u_noah: [msg('u_noah', ME, 'gg pour le duel tout à l’heure', 40)],
  u_ines: [
    msg(ME, 'u_ines', 'Ta ville médiévale est incroyable', 60 * 26),
    msg('u_ines', ME, 'Merci ! Je fais le port demain', 60 * 25),
  ],
};
const unreadDb: Record<string, number> = { u_lea: 2, u_noah: 1 };

const subscribers = new Set<(e: RealtimeEvent) => void>();
function push(e: RealtimeEvent) {
  for (const s of subscribers) s(e);
}

// Presence drifts a little so the panel feels alive.
let presenceTimer: ReturnType<typeof setInterval> | null = null;
function startPresence() {
  if (presenceTimer) return;
  presenceTimer = setInterval(() => {
    const f = friendsDb.find((x) => x.id === 'u_max');
    if (!f) return;
    f.presence = f.presence === 'online' ? 'in-game' : 'online';
    f.activity =
      f.presence === 'in-game'
        ? {
            kind: 'server',
            server_name: 'Skyreach',
            server_address: 'play.skyreach.gg',
            joinable: true,
            since: new Date().toISOString(),
          }
        : { kind: 'launcher', joinable: false, since: new Date().toISOString() };
    push({ type: 'presence', friend: clone(f) });
  }, 45_000);
}

/** Replies the mock friends send next (scripted by the product videos), then 👍. */
const nextReplies: string[] = [];

/**
 * Scripted social activity for the product videos (`e2e/journeys-video.spec.ts`): a friend
 * changes presence, a message arrives, the next replies are set. Browser mock only, exposed as
 * `window.__rusticSocialMock`; the real cloud has no such thing.
 */
export const socialScript = {
  presence(id: string, presence: Friend['presence'], activity: Friend['activity'] = null) {
    const f = friendsDb.find((x) => x.id === id);
    if (!f) return;
    f.presence = presence;
    f.activity = activity && { ...activity, since: new Date().toISOString() };
    f.last_seen_at = presence === 'offline' ? new Date().toISOString() : null;
    push({ type: 'presence', friend: clone(f) });
  },
  receive(fromId: string, body: string) {
    const m: ChatMessage = {
      id: uid('msg'),
      from_id: fromId,
      to_id: ME,
      body,
      sent_at: new Date().toISOString(),
    };
    (chatsDb[fromId] ??= []).push(m);
    unreadDb[fromId] = (unreadDb[fromId] ?? 0) + 1;
    push({ type: 'message', message: clone(m) });
  },
  replies(...bodies: string[]) {
    nextReplies.push(...bodies);
  },
  /** Every friend goes offline (a quiet evening: `e2e/hero.spec.ts`). */
  quiet() {
    for (const f of friendsDb) this.presence(f.id, 'offline');
  },
};
if (typeof window !== 'undefined') {
  (window as unknown as { __rusticSocialMock?: typeof socialScript }).__rusticSocialMock =
    socialScript;
}

export const socialMock: Partial<CloudClient> = {
  async conversations() {
    await latency(0.6);
    const list: Conversation[] = Object.entries(chatsDb).map(([id, msgs]) => {
      const f = friendsDb.find((x) => x.id === id);
      return {
        user: {
          id,
          username: f?.username ?? id,
          display_name: f?.display_name ?? id,
          presence: f?.presence ?? 'offline',
        },
        last_message: msgs[msgs.length - 1] ?? null,
        unread: unreadDb[id] ?? 0,
      };
    });
    return list.sort((a, b) =>
      (b.last_message?.sent_at ?? '').localeCompare(a.last_message?.sent_at ?? ''),
    );
  },
  async messages(userId) {
    await latency(0.5);
    return clone(chatsDb[userId] ?? []);
  },
  async sendMessage(userId, body) {
    await latency(0.3);
    const m: ChatMessage = {
      id: uid('msg'),
      from_id: ME,
      to_id: userId,
      body,
      sent_at: new Date().toISOString(),
    };
    (chatsDb[userId] ??= []).push(m);
    const f = friendsDb.find((x) => x.id === userId);
    if (f && f.presence !== 'offline') {
      void sleep(1800).then(() => {
        const reply: ChatMessage = {
          id: uid('msg'),
          from_id: userId,
          to_id: ME,
          body: nextReplies.shift() ?? '👍',
          sent_at: new Date().toISOString(),
        };
        (chatsDb[userId] ??= []).push(reply);
        push({ type: 'message', message: clone(reply) });
      });
    }
    return clone(m);
  },
  async markConversationRead(userId) {
    await latency(0.2);
    unreadDb[userId] = 0;
  },

  async friends() {
    await latency();
    return clone(friendsDb);
  },
  async friendRequests() {
    await latency(0.6);
    return clone(requestsDb);
  },
  async searchUsers(q) {
    await latency(0.5);
    const s = q.trim().toLowerCase();
    if (!s) return [];
    return directory.filter(
      (u) => u.username.toLowerCase().includes(s) || u.display_name.toLowerCase().includes(s),
    );
  },
  async lookupUser(by, name) {
    await latency(0.6);
    lookupRateLimit();
    const n = name.trim();
    if (!NAME[by].test(n)) throw cloudError('invalid_input', 'invalid name', 400);
    const r = resolve(by, n);
    if (r === null) throw cloudError('not_found', 'user not found', 404);
    if (r === 'self') throw cloudError('cannot_add_self', 'you cannot befriend yourself', 400);
    if (r.unreachable) {
      throw cloudError('cannot_add_friend', 'this user cannot receive friend requests', 403);
    }
    const req = requestsDb.find((x) => x.user.id === r.user.id);
    return {
      user: clone(r.user),
      minecraft_name: by === 'minecraft' || !r.minor ? r.minecraft_name : null,
      relation: r.friend
        ? 'friends'
        : req?.direction === 'incoming'
          ? 'incoming_request'
          : req
            ? 'outgoing_request'
            : 'none',
      request_id: req?.direction === 'incoming' ? req.id : null,
    };
  },
  async friendSuggestions() {
    await latency(0.8);
    if (!mockDiscoverable()) return { discoverable: false, played_with: [], crossed: [] };
    const taken = new Set([...friendsDb.map((f) => f.id), ...requestsDb.map((r) => r.user.id)]);
    const keep = (list: FriendSuggestion[]) => clone(list.filter((s) => !taken.has(s.user.id)));
    return {
      discoverable: true,
      played_with: keep(suggestionPool.played_with),
      crossed: keep(suggestionPool.crossed),
    };
  },
  async sendFriendRequest(target) {
    await latency();
    const t: FriendTarget = typeof target === 'string' ? { username: target } : target;
    const r =
      'user_id' in t
        ? resolve('id', t.user_id)
        : 'username' in t
          ? resolve('rustic', t.username)
          : resolve('minecraft', t.minecraft_name);
    if (r === null) throw cloudError('not_found', 'user not found', 404);
    if (r === 'self') throw cloudError('invalid_input', 'you cannot befriend yourself', 400);
    if (r.unreachable) {
      throw cloudError('cannot_add_friend', 'this user cannot receive friend requests', 403);
    }
    if (r.friend) throw cloudError('already_friends', 'you are already friends', 409);
    const existing = requestsDb.find((x) => x.user.id === r.user.id);
    if (existing?.direction === 'incoming') {
      throw cloudError('incoming_request_exists', 'this user already sent you a request', 409);
    }
    if (existing) throw cloudError('request_pending', 'a request is already pending', 409);
    const user = r.user;
    const r2: FriendRequest = {
      id: uid('fr'),
      user,
      direction: 'outgoing',
      created_at: new Date().toISOString(),
    };
    requestsDb = [r2, ...requestsDb];
    return clone(r2);
  },
  async respondFriendRequest(id, accept) {
    await latency();
    const r = requestsDb.find((x) => x.id === id);
    requestsDb = requestsDb.filter((x) => x.id !== id);
    if (r && accept) {
      friendsDb.unshift({
        ...r.user,
        presence: 'online',
        activity: { kind: 'launcher', joinable: false, since: new Date().toISOString() },
        last_seen_at: null,
        favorite: false,
      });
    }
  },
  async cancelFriendRequest(id) {
    await latency(0.5);
    requestsDb = requestsDb.filter((x) => x.id !== id);
  },
  async removeFriend(id) {
    await latency();
    const i = friendsDb.findIndex((f) => f.id === id);
    if (i >= 0) friendsDb.splice(i, 1);
  },
  async setFavoriteFriend(id, favorite) {
    await latency(0.3);
    const f = friendsDb.find((x) => x.id === id);
    if (f) f.favorite = favorite;
  },
  async party() {
    await latency(0.5);
    return clone(partyDb);
  },
  async inviteToParty(userId) {
    await latency();
    const f = friendsDb.find((x) => x.id === userId);
    if (!partyDb) {
      partyDb = {
        id: uid('party'),
        leader_id: 'usr_sam',
        max_size: 8,
        members: [
          { id: 'usr_sam', username: 'samcrafte', display_name: 'Sam', presence: 'online' },
        ],
        pending_invites: [],
      };
    }
    if (
      f &&
      !partyDb.pending_invites.some((u) => u.id === f.id) &&
      !partyDb.members.some((m) => m.id === f.id)
    ) {
      partyDb.pending_invites.push({
        id: f.id,
        username: f.username,
        display_name: f.display_name,
      });
      // The friend accepts after a moment when online.
      if (f.presence !== 'offline') {
        void sleep(2500).then(() => {
          if (!partyDb) return;
          partyDb.pending_invites = partyDb.pending_invites.filter((u) => u.id !== f.id);
          partyDb.members.push({
            id: f.id,
            username: f.username,
            display_name: f.display_name,
            presence: f.presence,
          });
          push({ type: 'party', party: clone(partyDb) });
        });
      }
    }
    return clone(partyDb);
  },
  async leaveParty() {
    await latency();
    partyDb = null;
    push({ type: 'party', party: null });
  },
  subscribe(handler) {
    subscribers.add(handler);
    startPresence();
    return () => subscribers.delete(handler);
  },
  async news() {
    await latency(1.2);
    return clone(newsDb);
  },
  async featuredServers() {
    await latency(1.1);
    return clone(featuredDb);
  },
  async notifications() {
    await latency(0.6);
    return clone(notificationsDb);
  },
  async markNotificationsRead(ids) {
    await latency(0.3);
    notificationsDb = notificationsDb.map((n) => (ids.includes(n.id) ? { ...n, read: true } : n));
  },
};
