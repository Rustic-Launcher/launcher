/**
 * Real Rustic Cloud client over the OpenAPI document (services/api/openapi.json), enabled when
 * `VITE_CLOUD_URL` is set (ADR 0027). Types come from `generated/schema.d.ts`
 * (`pnpm gen:cloud`, openapi-typescript); requests use openapi-fetch.
 *
 * Gaps between the UI's needs and the API (news, featured servers, PIN-based parental gate,
 * friend favourites, waitlist position) fall back to the mock or local storage and are listed in
 * docs/HANDOFF.md for the cloud engineer.
 */
import createClient, { type Middleware } from 'openapi-fetch';
import { invoke, isNative } from '@/ipc';
import type { CloudClient } from './client';
import { CloudError } from './errors';
import type { components, paths } from './generated/schema';
import type {
  AppNotification,
  ChatMessage,
  Conversation,
  Cosmetic,
  CosmeticSlot,
  Currency,
  FeaturedServer,
  Friend,
  FriendRequest,
  Loadout,
  Me,
  NewsItem,
  Outfit,
  Party,
  PresenceStatus,
  RealtimeEvent,
  StoreProduct,
} from './types';

type S = components['schemas'];

/** Consent wording version recorded with each order (docs/LEGAL.md §3). */
export const CONSENT_WORDING_VERSION = '2026-11-10';

export { CloudError };

/** Realtime gateway base (`ws(s)://...`), from VITE_REALTIME_URL; none = presence polling. */
function realtimeUrl(): string | null {
  const u = import.meta.env.VITE_REALTIME_URL as string | undefined;
  return u ? u.replace(/\/$/, '') : null;
}

export function mapMessage(m: S['Message']): ChatMessage {
  return {
    id: m.id,
    from_id: m.sender_id,
    to_id: m.recipient_id,
    body: m.body,
    sent_at: m.created_at,
    read_at: m.read_at ?? null,
  };
}

/** Access token: from the desktop keychain session in Tauri, or VITE_CLOUD_TOKEN in a browser. */
async function accessToken(): Promise<string | null> {
  if (isNative()) {
    try {
      return (await invoke('auth_rustic_access_token')).access_token;
    } catch {
      return null;
    }
  }
  return (import.meta.env.VITE_CLOUD_TOKEN as string | undefined) ?? null;
}

// ---- mapping helpers (pure, unit-tested in http.test.ts) ----

// API slots are the cosmetics format's categories (packages/cosmetics-format); only the
// shoulder companion is spelled differently in the UI.
const API_TO_UI_SLOT: Record<S['CosmeticSlot'], CosmeticSlot> = {
  head: 'head',
  back: 'back',
  top: 'top',
  shoulder_companion: 'shoulder',
  companion: 'companion',
  trail: 'trail',
  aura: 'aura',
  emote: 'emote',
  nametag: 'nametag',
};

const UI_TO_API_SLOT: Record<CosmeticSlot, S['CosmeticSlot']> = {
  head: 'head',
  back: 'back',
  top: 'top',
  shoulder: 'shoulder_companion',
  companion: 'companion',
  trail: 'trail',
  aura: 'aura',
  emote: 'emote',
  nametag: 'nametag',
};

export function mapSlot(slot: S['CosmeticSlot']): CosmeticSlot {
  return API_TO_UI_SLOT[slot];
}

function hueOf(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
  return h;
}

export function mapPresence(p: S['Presence']): {
  presence: PresenceStatus;
  activity: Friend['activity'];
} {
  const status = p.status === 'invisible' ? 'offline' : p.status;
  const a = p.activity ?? null;
  if (!a || status === 'offline') return { presence: status, activity: null };
  const since = p.updated_at;
  if (a.kind === 'playing') {
    const onServer = !!a.server_address;
    return {
      presence: 'in-game',
      activity: {
        kind: onServer ? 'server' : 'singleplayer',
        instance_name: a.instance_name ?? undefined,
        server_name: a.server_name ?? a.server_address ?? undefined,
        server_address: a.server_address ?? undefined,
        joinable: onServer,
        since,
      },
    };
  }
  if (a.kind === 'hosting') {
    return {
      presence: status,
      activity: {
        kind: 'hosting',
        server_name: a.server_name ?? undefined,
        server_address: a.server_address ?? undefined,
        joinable: !!a.server_address,
        since,
      },
    };
  }
  return { presence: status, activity: { kind: 'launcher', joinable: false, since } };
}

const FAV_KEY = 'rustic.cloud.favorites';
function favorites(): Set<string> {
  try {
    return new Set(JSON.parse(window.localStorage.getItem(FAV_KEY) ?? '[]') as string[]);
  } catch {
    return new Set();
  }
}

export function mapFriend(f: S['Friend'], favs: Set<string> = new Set()): Friend {
  const { presence, activity } = mapPresence(f.presence);
  return {
    id: f.user.id,
    username: f.user.username,
    display_name: f.user.display_name,
    presence,
    activity,
    last_seen_at: presence === 'offline' ? f.presence.updated_at : null,
    favorite: favs.has(f.user.id),
  };
}

export function ageGroup(
  dateOfBirth: string | null | undefined,
  now = new Date(),
): Me['age_group'] {
  if (!dateOfBirth) return 'adult';
  const dob = new Date(dateOfBirth);
  let age = now.getFullYear() - dob.getFullYear();
  const m = now.getMonth() - dob.getMonth();
  if (m < 0 || (m === 0 && now.getDate() < dob.getDate())) age--;
  return age < 13 ? 'child' : age < 18 ? 'teen' : 'adult';
}

export function currencyFor(country: string | null | undefined): Currency {
  switch ((country ?? '').toUpperCase()) {
    case 'CH':
    case 'LI':
      return 'CHF';
    case 'GB':
      return 'GBP';
    case 'US':
      return 'USD';
    default:
      return 'EUR';
  }
}

export function mapCosmetic(
  item: S['CosmeticItem'],
  owned: S['InventoryItem'] | undefined,
): Cosmetic {
  const slot = mapSlot(item.slot);
  const hue = hueOf(item.slug);
  return {
    id: item.id,
    name: item.name,
    description: item.description,
    slot,
    rarity: 'common',
    // Real meshes come from asset_ref via the cosmetics pipeline; the preview uses a procedural shape.
    visual: {
      slot,
      shape: item.slug,
      color: `hsl(${hue} 55% 55%)`,
      accent: `hsl(${(hue + 40) % 360} 60% 40%)`,
    },
    owned: !!owned,
    acquired_at: owned?.acquired_at ?? null,
    source: owned
      ? owned.source === 'purchase'
        ? 'purchase'
        : owned.source === 'gift'
          ? 'gift'
          : 'event'
      : null,
  };
}

export function mapProduct(p: S['Product'], currency: Currency): StoreProduct {
  const first = p.items[0];
  return {
    id: p.slug,
    kind: p.kind,
    name: p.name,
    description: p.description,
    cosmetic_ids: p.items.map((i) => i.id),
    slot: p.kind === 'item' && first ? mapSlot(first.slot) : null,
    rarity: 'common',
    price: { amount_minor: p.price?.amount_minor ?? 0, currency: p.price?.currency ?? currency },
    compare_at: null,
    owned: !!p.owned,
    is_new: false,
    available_until: null,
    hue: hueOf(p.slug),
  };
}

export function mapLoadout(e: S['Equipped']): Loadout {
  const out: Loadout = {};
  for (const s of e.items) out[mapSlot(s.slot)] = s.item_id;
  return out;
}

export function toEquippedSlots(l: Loadout): S['EquippedSlot'][] {
  const out: S['EquippedSlot'][] = [];
  for (const [slot, id] of Object.entries(l)) {
    const api = UI_TO_API_SLOT[slot as CosmeticSlot];
    if (api && id) out.push({ slot: api, item_id: id });
  }
  return out;
}

/**
 * Socket frames that change the friend list itself (a request accepted on another device or by
 * the other player, a removal): the launcher refetches the roster and the pending requests.
 */
export function rosterEvent(frame: { type: string }): RealtimeEvent | null {
  return frame.type === 'friend_added' || frame.type === 'friend_removed'
    ? { type: 'friends-changed' }
    : null;
}

/** A published news item (`GET /v1/news`) as the Launchpad shows it. */
export function mapNews(n: S['NewsItem']): NewsItem {
  return {
    id: n.id,
    source: n.source,
    kind: n.kind,
    title: n.title,
    summary: n.summary,
    published_at: n.published_at,
    url: n.url,
    hue: hueOf(n.slug),
    version: n.version ?? undefined,
  };
}

/** A featured server (`GET /v1/featured-servers`); player counts stay null until measured. */
export function mapFeatured(f: S['FeaturedServer']): FeaturedServer {
  return {
    id: f.id,
    name: f.name,
    address: f.address,
    description: f.description,
    tags: f.tags,
    engine: f.engine,
    players_online: f.players_online ?? null,
    players_max: f.players_max ?? null,
    language: f.language,
    verified: f.verified,
    hue: hueOf(f.id),
  };
}

export function mapNotification(n: S['Notification']): AppNotification {
  const payload = (n.payload ?? {}) as Record<string, unknown>;
  const kinds: AppNotification['kind'][] = [
    'friend_request',
    'party_invite',
    'server',
    'store',
    'system',
    'update',
  ];
  const kind = (kinds as string[]).includes(n.kind)
    ? (n.kind as AppNotification['kind'])
    : 'system';
  return {
    id: n.id,
    kind,
    title: typeof payload.title === 'string' ? payload.title : n.kind,
    body: typeof payload.body === 'string' ? payload.body : n.kind,
    created_at: n.created_at,
    read: n.read_at != null,
    href: typeof payload.href === 'string' ? payload.href : undefined,
  };
}

// ---- client ----

export function createHttpCloud(baseUrl: string, fallback: CloudClient): CloudClient {
  const api = createClient<paths>({ baseUrl });
  const auth: Middleware = {
    async onRequest({ request }) {
      const token = await accessToken();
      if (token) request.headers.set('Authorization', `Bearer ${token}`);
      return request;
    },
  };
  api.use(auth);

  async function data<T>(
    p: Promise<{ data?: T; error?: unknown; response: Response }>,
  ): Promise<T> {
    const { data: d, error, response } = await p;
    if (error !== undefined || d === undefined) {
      const body = error as S['ErrorBody'] | undefined;
      throw new CloudError(
        body?.error?.code ?? 'http_error',
        body?.error?.message ?? response.statusText,
        response.status,
      );
    }
    return d;
  }
  async function ok(p: Promise<{ error?: unknown; response: Response }>): Promise<void> {
    const { error, response } = await p;
    if (!response.ok) {
      const body = error as S['ErrorBody'] | undefined;
      throw new CloudError(
        body?.error?.code ?? 'http_error',
        body?.error?.message ?? response.statusText,
        response.status,
      );
    }
  }

  let meCache: S['Me'] | null = null;

  function mapMe(m: S['Me'], balance: Me['balance']): Me {
    return {
      id: m.id,
      username: m.username,
      display_name: m.display_name,
      email: m.email ?? '',
      created_at: m.created_at,
      age_group: ageGroup(m.date_of_birth),
      country: m.country ?? 'FR',
      currency: currencyFor(m.country),
      balance,
      discoverable: m.discoverable ?? false,
      discoverable_allowed: m.capabilities.discoverable_allowed ?? false,
    };
  }
  const slugToId = new Map<string, string>();

  async function getMe(): Promise<S['Me']> {
    meCache ??= await data(api.GET('/v1/me'));
    return meCache;
  }

  async function currentParty(): Promise<S['Party'] | null> {
    const { data: d, response } = await api.GET('/v1/parties/current');
    if (response.status === 404 || !d) return null;
    return d;
  }

  async function toParty(p: S['Party'] | null): Promise<Party | null> {
    if (!p) return null;
    const friends = await client.friends().catch(() => [] as Friend[]);
    const presenceOf = (id: string) => friends.find((f) => f.id === id)?.presence ?? 'online';
    return {
      id: p.id,
      leader_id: p.leader_id,
      max_size: p.max_size,
      members: p.members.map((m) => ({ ...m.user, presence: presenceOf(m.user.id) })),
      pending_invites: [],
    };
  }

  const client: CloudClient = {
    async me() {
      const m = await getMe();
      const wallet = await api.GET('/v1/wallet').catch(() => null);
      const w = wallet?.data?.balances?.[0];
      return mapMe(m, w ? { amount_minor: w.amount_minor, currency: w.currency } : null);
    },
    async setDiscoverable(on) {
      meCache = await data(api.PATCH('/v1/me', { body: { discoverable: on } }));
      return mapMe(meCache, null);
    },
    async lookupUser(by, name) {
      const query = by === 'rustic' ? { username: name } : { minecraft_name: name };
      const r = await data(api.GET('/v1/users/lookup', { params: { query } }));
      return {
        user: r.user,
        minecraft_name: r.minecraft_name ?? null,
        relation: r.relation,
        request_id: r.request_id ?? null,
      };
    },
    async friendSuggestions() {
      const r = await data(api.GET('/v1/friends/suggestions'));
      const map = (s: S['FriendSuggestion']) => ({
        user: s.user,
        minecraft_name: s.minecraft_name ?? null,
        reason: s.reason,
        server_name: s.server_name ?? null,
        seen_at: s.seen_at,
      });
      return {
        discoverable: r.discoverable,
        played_with: r.played_with.map(map),
        crossed: r.crossed.map(map),
      };
    },
    async friends() {
      const favs = favorites();
      return (await data(api.GET('/v1/friends'))).map((f) => mapFriend(f, favs));
    },
    async friendRequests() {
      const r = await data(api.GET('/v1/friends/requests'));
      const map = (
        fr: S['FriendRequest'],
        direction: FriendRequest['direction'],
      ): FriendRequest => ({
        id: fr.id,
        user: direction === 'incoming' ? fr.from : fr.to,
        direction,
        created_at: fr.created_at,
      });
      return [
        ...r.incoming.map((x) => map(x, 'incoming')),
        ...r.outgoing.map((x) => map(x, 'outgoing')),
      ];
    },
    async searchUsers(q) {
      if (!q.trim()) return [];
      return data(api.GET('/v1/users/search', { params: { query: { q } } }));
    },
    async sendFriendRequest(target) {
      const body = typeof target === 'string' ? { username: target } : target;
      const fr = await data(api.POST('/v1/friends/requests', { body }));
      return { id: fr.id, user: fr.to, direction: 'outgoing', created_at: fr.created_at };
    },
    async respondFriendRequest(id, accept) {
      const path = accept
        ? '/v1/friends/requests/{id}/accept'
        : '/v1/friends/requests/{id}/decline';
      await ok(api.POST(path, { params: { path: { id } } }));
    },
    async cancelFriendRequest(id) {
      await ok(api.DELETE('/v1/friends/requests/{id}', { params: { path: { id } } }));
    },
    async removeFriend(id) {
      await ok(api.DELETE('/v1/friends/{user_id}', { params: { path: { user_id: id } } }));
    },
    async setFavoriteFriend(id, favorite) {
      // Not in the API yet: kept per device.
      const favs = favorites();
      if (favorite) favs.add(id);
      else favs.delete(id);
      window.localStorage.setItem(FAV_KEY, JSON.stringify([...favs]));
    },
    async party() {
      return toParty(await currentParty());
    },
    async inviteToParty(userId) {
      let p = await currentParty();
      p ??= await data(api.POST('/v1/parties'));
      await ok(
        api.POST('/v1/parties/{id}/invites', {
          params: { path: { id: p.id } },
          body: { user_id: userId },
        }),
      );
      const party = await toParty(await currentParty());
      return party ?? (await toParty(p))!;
    },
    async leaveParty() {
      const p = await currentParty();
      if (p) await ok(api.POST('/v1/parties/{id}/leave', { params: { path: { id: p.id } } }));
    },
    subscribe(handler: (e: RealtimeEvent) => void) {
      // Realtime gateway (docs/API.md "Realtime WebSocket"): `?access_token=` on the URL, JSON
      // frames tagged by `type`. Without VITE_REALTIME_URL, presence is polled every 30 s.
      const rt = realtimeUrl();
      if (rt) return subscribeSocket(rt, handler);
      let prev = new Map<string, string>();
      const tick = async () => {
        try {
          const list = await client.friends();
          const next = new Map(
            list.map((f) => [f.id, `${f.presence}|${f.activity?.server_address ?? ''}`]),
          );
          for (const f of list)
            if (prev.size && prev.get(f.id) !== next.get(f.id))
              handler({ type: 'presence', friend: f });
          prev = next;
        } catch {
          /* offline: try again next tick */
        }
      };
      void tick();
      const t = setInterval(() => void tick(), 30_000);
      return () => clearInterval(t);
    },

    async conversations(): Promise<Conversation[]> {
      const list = await data(api.GET('/v1/conversations'));
      return list.map((c) => ({
        user: { ...c.peer, presence: mapPresence(c.presence).presence },
        last_message: mapMessage(c.last_message),
        unread: c.unread,
      }));
    },
    async messages(userId) {
      const list = await data(
        api.GET('/v1/messages/{user_id}', { params: { path: { user_id: userId } } }),
      );
      return list.map(mapMessage);
    },
    async sendMessage(userId, body) {
      const m = await data(
        api.POST('/v1/messages/{user_id}', {
          params: { path: { user_id: userId } },
          body: { body },
        }),
      );
      return mapMessage(m);
    },
    async markConversationRead(userId) {
      await ok(api.POST('/v1/messages/{user_id}/read', { params: { path: { user_id: userId } } }));
    },
    // Published by staff from the website back-office (ADR 0140); never the offline samples.
    async news() {
      return (await data(api.GET('/v1/news', { params: { query: { limit: 20 } } }))).map(mapNews);
    },
    async featuredServers() {
      return (await data(api.GET('/v1/featured-servers'))).map(mapFeatured);
    },
    async notifications() {
      return (await data(api.GET('/v1/notifications'))).map(mapNotification);
    },
    async markNotificationsRead(ids) {
      await Promise.all(
        ids.map((id) => ok(api.POST('/v1/notifications/{id}/read', { params: { path: { id } } }))),
      );
    },
    async cosmetics() {
      const inv = await data(api.GET('/v1/inventory'));
      return inv.map((i) => mapCosmetic(i.item, i));
    },
    async loadout() {
      return mapLoadout(await data(api.GET('/v1/cosmetics/equipped')));
    },
    async equip(loadout) {
      return mapLoadout(
        await data(
          api.PUT('/v1/cosmetics/equipped', { body: { items: toEquippedSlots(loadout) } }),
        ),
      );
    },
    async outfits() {
      const list = await data(api.GET('/v1/outfits'));
      return list.map((o): Outfit => ({
        id: o.id,
        name: o.name,
        loadout: mapLoadout({ items: o.items }),
        created_at: o.created_at,
      }));
    },
    async saveOutfit(name, loadout) {
      const o = await data(
        api.POST('/v1/outfits', { body: { name, items: toEquippedSlots(loadout) } }),
      );
      return {
        id: o.id,
        name: o.name,
        loadout: mapLoadout({ items: o.items }),
        created_at: o.created_at,
      };
    },
    async deleteOutfit(id) {
      await ok(api.DELETE('/v1/outfits/{id}', { params: { path: { id } } }));
    },
    async catalog(currency) {
      const me = await getMe().catch(() => null);
      const c = await data(
        api.GET('/v1/store/catalog', {
          params: { query: { currency, country: me?.country ?? undefined } },
        }),
      );
      const products = c.products.map((p) => {
        slugToId.set(p.slug, p.id);
        return mapProduct(p, c.currency);
      });
      return {
        currency: c.currency,
        featured: products.slice(0, 3),
        bundles: products.filter((p) => p.kind === 'bundle'),
        new_items: products.filter((p) => p.kind === 'item').slice(0, 8),
        products,
      };
    },
    async product(id, currency) {
      const me = await getMe().catch(() => null);
      const p = await data(
        api.GET('/v1/store/products/{slug}', {
          params: { path: { slug: id }, query: { currency, country: me?.country ?? undefined } },
        }),
      );
      slugToId.set(p.slug, p.id);
      return mapProduct(p, currency);
    },
    async purchase(req) {
      const me = await getMe();
      const productId = slugToId.get(req.product_id) ?? req.product_id;
      const legal = await data(api.GET('/v1/store/legal'));
      const res = await data(
        // The server owns the consent wording version (docs/LEGAL.md §3).
        api.POST('/v1/store/checkout', {
          body: {
            quote: {
              product_ids: [productId],
              currency: req.currency,
              country: me.country ?? 'FR',
            },
            consent: {
              immediate_execution: req.consent_immediate_execution,
              withdrawal_waiver: req.waiver_withdrawal_right,
              wording_version: legal.consent_wording_version,
            },
          },
        }),
      );
      let order = res.order;
      if (order.status === 'awaiting_guardian') {
        return {
          order_id: order.id,
          status: 'parental_approval_required',
          charged: { amount_minor: order.total_minor, currency: order.currency },
          receipt_url: order.invoice_url ?? '',
          granted_cosmetic_ids: [],
        };
      }
      if (res.checkout_url && order.status === 'pending') {
        // Stripe Checkout opens in the system browser; poll the order until it settles.
        await invoke('app_open_url', { url: res.checkout_url });
        const deadline = Date.now() + 15 * 60_000;
        while (order.status === 'pending' && Date.now() < deadline) {
          await new Promise((r) => setTimeout(r, 3000));
          order = await data(api.GET('/v1/orders/{id}', { params: { path: { id: order.id } } }));
        }
      }
      if (order.status !== 'paid')
        throw new CloudError(`order_${order.status}`, `Order ${order.status}`, 402);
      return {
        order_id: order.id,
        status: 'completed',
        charged: { amount_minor: order.total_minor, currency: order.currency },
        receipt_url: order.invoice_url ?? '',
        granted_cosmetic_ids: order.lines.flatMap((l) => l.item_ids),
      };
    },
    // Parental controls in the API are guardian-linked approvals (no PIN); the PIN gate stays local.
    parental: () => fallback.parental(),
    setParental: (patch) => fallback.setParental(patch),
    verifyParentalPin: (pin) => fallback.verifyParentalPin(pin),
    async identities() {
      const r = await data(api.GET('/v1/me/identities'));
      return {
        has_password: r.has_password,
        identities: r.identities.map((i) => ({
          provider: i.provider,
          email: i.email ?? null,
          linked_at: i.linked_at,
        })),
      };
    },
    async unlinkIdentity(provider) {
      await ok(api.DELETE('/v1/me/identities/{provider}', { params: { path: { provider } } }));
    },
    async exportAccount() {
      const dump = await data(api.GET('/v1/me/export'));
      const url = URL.createObjectURL(
        new Blob([JSON.stringify(dump, null, 2)], { type: 'application/json' }),
      );
      return { url, expires_at: new Date(Date.now() + 3600_000).toISOString() };
    },
    async deleteAccount(confirmUsername) {
      const me = await getMe();
      if (confirmUsername !== me.username)
        throw new CloudError('invalid_argument', 'confirmation', 400);
      const r = await data(api.POST('/v1/me/delete', { body: {} }));
      return {
        scheduled_for: r.scheduled_for ?? new Date(Date.now() + 30 * 86_400_000).toISOString(),
      };
    },
    async joinWaitlist() {
      await data(api.POST('/v1/hosting/waitlist', { body: {} }));
      return { position: 0 };
    },
  };
  /**
   * Realtime socket with reconnection (backoff 1 s → 30 s). Each connection takes a fresh access
   * token; close codes 4001/4002 (session revoked / token expired) just reconnect with a new one.
   */
  function subscribeSocket(base: string, handler: (e: RealtimeEvent) => void): () => void {
    let ws: WebSocket | null = null;
    let stopped = false;
    let retry = 1000;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let beat: ReturnType<typeof setInterval> | undefined;

    const onFrame = async (frame: S['ServerMessage']) => {
      switch (frame.type) {
        case 'hello':
          retry = 1000;
          ws?.send(JSON.stringify({ type: 'heartbeat', status: 'online' }));
          clearInterval(beat);
          beat = setInterval(
            () => ws?.send(JSON.stringify({ type: 'heartbeat', status: 'online' })),
            frame.heartbeat_interval_secs * 1000,
          );
          break;
        case 'message_received':
          handler({ type: 'message', message: mapMessage(frame.message) });
          break;
        case 'messages_read':
          handler({ type: 'messages-read', reader_id: frame.reader_id, read_at: frame.read_at });
          break;
        case 'presence_changed': {
          const friend = (await client.friends().catch(() => [] as Friend[])).find(
            (f) => f.id === frame.user_id,
          );
          if (friend) handler({ type: 'presence', friend });
          break;
        }
        case 'friend_request_received':
          handler({
            type: 'friend-request',
            request: {
              id: frame.request.id,
              user: frame.request.from,
              direction: 'incoming',
              created_at: frame.request.created_at,
            },
          });
          break;
        case 'friend_added':
        case 'friend_removed':
          handler(rosterEvent(frame)!);
          break;
        case 'party_updated':
          handler({ type: 'party', party: await toParty(frame.party ?? null) });
          break;
        case 'notification_created':
          handler({ type: 'notification', notification: mapNotification(frame.notification) });
          break;
        case 'join_server':
          handler({ type: 'join-server', address: frame.address });
          break;
        default:
          break;
      }
    };

    const connect = async () => {
      if (stopped) return;
      const token = await accessToken();
      if (!token) {
        timer = setTimeout(() => void connect(), 30_000);
        return;
      }
      ws = new WebSocket(`${base}/v1/ws?access_token=${encodeURIComponent(token)}`);
      ws.onmessage = (ev) => {
        try {
          void onFrame(JSON.parse(String(ev.data)) as S['ServerMessage']);
        } catch {
          /* ignore malformed frames */
        }
      };
      ws.onclose = () => {
        clearInterval(beat);
        if (stopped) return;
        timer = setTimeout(() => void connect(), retry);
        retry = Math.min(retry * 2, 30_000);
      };
    };
    void connect();
    return () => {
      stopped = true;
      clearTimeout(timer);
      clearInterval(beat);
      ws?.close();
    };
  }

  return client;
}
