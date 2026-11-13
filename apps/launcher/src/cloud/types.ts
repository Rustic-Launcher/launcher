/**
 * Rustic Cloud types as the UI consumes them. The backend engineer will generate the real client
 * from services/api/openapi.json; these hand-written types are the expected shapes (see IPC.md,
 * "Cloud API expected by the UI").
 */
import type { CosmeticSlot, CosmeticVisual } from '@rustic/ui/three';
import type { Currency, Money } from '../ipc/types/dns';

export type { Currency, Money, CosmeticSlot, CosmeticVisual };

export type PresenceStatus = 'online' | 'in-game' | 'away' | 'busy' | 'offline';

export interface UserLite {
  id: string;
  username: string;
  display_name: string;
}

export interface Activity {
  kind: 'launcher' | 'singleplayer' | 'server' | 'hosting';
  instance_name?: string;
  server_name?: string;
  server_address?: string;
  /** The friend allows joining (server is reachable for them). */
  joinable: boolean;
  since: string;
}

export interface Friend extends UserLite {
  presence: PresenceStatus;
  activity: Activity | null;
  last_seen_at: string | null;
  favorite: boolean;
}

/** Who a friend request goes to: a known user id, or an exact Rustic or Minecraft name. */
export type FriendTarget = { user_id: string } | { username: string } | { minecraft_name: string };

export type LookupRelation = 'none' | 'friends' | 'outgoing_request' | 'incoming_request';

/** Exact-match lookup shown before sending a friend request (`GET /v1/users/lookup`). */
export interface UserLookup {
  user: UserLite;
  /** Linked Minecraft profile name (hidden for minors unless it is what was searched). */
  minecraft_name: string | null;
  relation: LookupRelation;
  /** Incoming request to accept when `relation` is `incoming_request`. */
  request_id: string | null;
}

export interface FriendSuggestion {
  user: UserLite;
  minecraft_name: string | null;
  /** Same party, or same (shared) server at the same time. */
  reason: 'party' | 'server';
  /** The viewer's own name for the server. */
  server_name: string | null;
  seen_at: string;
}

/** Players met recently (ADR 0145). Empty and `discoverable: false` when opted out or a minor. */
export interface FriendSuggestions {
  discoverable: boolean;
  played_with: FriendSuggestion[];
  crossed: FriendSuggestion[];
}

export interface FriendRequest {
  id: string;
  user: UserLite;
  direction: 'incoming' | 'outgoing';
  created_at: string;
}

export interface Party {
  id: string;
  leader_id: string;
  members: (UserLite & { presence: PresenceStatus })[];
  max_size: number;
  pending_invites: UserLite[];
}

export interface Me extends UserLite {
  email: string;
  created_at: string;
  /** Used for the parental gate and minimum ages (account, chat). */
  age_group: 'child' | 'teen' | 'adult';
  country: string;
  currency: Currency;
  balance: Money | null;
  /** "Appear in suggestions of players I meet", as in effect (ADR 0145). */
  discoverable: boolean;
  /** False for minors: the setting is forced off and cannot be changed. */
  discoverable_allowed: boolean;
}

export interface NewsItem {
  id: string;
  source: 'horizon' | 'rustic';
  kind: 'patch_notes' | 'event' | 'blog';
  title: string;
  summary: string;
  published_at: string;
  url: string;
  /** Hue for the generated cover art (no third-party images). */
  hue: number;
  version?: string;
}

export interface FeaturedServer {
  id: string;
  name: string;
  address: string;
  description: string;
  tags: string[];
  engine: 'horizon' | 'rustic';
  /** Null when nobody measured it (the cloud does not ping featured servers yet). */
  players_online: number | null;
  players_max: number | null;
  language: string;
  verified: boolean;
  hue: number;
}

/** What a listed server runs (ADR 0158). */
export type ServerEngine = 'horizon' | 'rustic';
export type ListingSort = 'trending' | 'players' | 'new' | 'name' | 'featured';

/** The cloud's last server list ping of a listed server (public addresses, every ~10 min). */
export interface ServerListingStatus {
  online: boolean;
  players_online: number | null;
  players_max: number | null;
  motd: string | null;
  version: string | null;
  checked_at: string;
}

/**
 * A server of the directory (`GET /v1/server-listings`, ADR 0158): published by its owner from
 * the website, verified and featured by staff. Lists leave `description` null.
 */
export interface ServerListing {
  id: string;
  slug: string;
  name: string;
  address: string;
  summary: string;
  description: string | null;
  tags: string[];
  languages: string[];
  engine: ServerEngine;
  game_version: string | null;
  requires_rustic_edition: boolean;
  website: string | null;
  discord: string | null;
  icon_url: string | null;
  banner_url: string | null;
  verified: boolean;
  featured: boolean;
  owner: { id: string; username: string; display_name: string } | null;
  status: ServerListingStatus | null;
  /** "Play" presses from launchers over the last 7 days (the trending signal). */
  plays_7d: number;
  created_at: string;
  updated_at: string;
}

export interface ServerListingQuery {
  q?: string;
  tag?: string;
  language?: string;
  engine?: ServerEngine;
  verified?: boolean;
  featured?: boolean;
  sort?: ListingSort;
  offset?: number;
  limit?: number;
}

export interface ServerListingPage {
  items: ServerListing[];
  has_more: boolean;
  total: number;
}

export type Rarity = 'common' | 'rare' | 'epic' | 'legendary';

export interface Cosmetic {
  id: string;
  name: string;
  description: string;
  slot: CosmeticSlot;
  rarity: Rarity;
  visual: CosmeticVisual;
  owned: boolean;
  acquired_at: string | null;
  source: 'purchase' | 'gift' | 'event' | 'default' | null;
}

export type Loadout = Partial<Record<CosmeticSlot, string | null>>;

export interface Outfit {
  id: string;
  name: string;
  loadout: Loadout;
  created_at: string;
}

export interface StoreProduct {
  id: string;
  kind: 'item' | 'bundle';
  name: string;
  description: string;
  /** Cosmetics granted (one for items, several for bundles). Purely visual, never random. */
  cosmetic_ids: string[];
  slot: CosmeticSlot | null;
  rarity: Rarity;
  price: Money;
  /** Bundle: sum of the items bought separately. */
  compare_at: Money | null;
  owned: boolean;
  is_new: boolean;
  available_until: string | null;
  hue: number;
}

export interface StoreCatalog {
  currency: Currency;
  featured: StoreProduct[];
  bundles: StoreProduct[];
  new_items: StoreProduct[];
  products: StoreProduct[];
}

export interface PurchaseRequest {
  product_id: string;
  currency: Currency;
  /** EU: explicit consent to immediate execution of the digital content contract. */
  consent_immediate_execution: true;
  /** EU: acknowledgement that the 14-day withdrawal right is lost once delivered. */
  waiver_withdrawal_right: true;
  parental_pin?: string;
  /** Gift to a friend (user id): granted to them, not to the buyer. */
  gift_to?: string;
}

export interface PurchaseResult {
  order_id: string;
  status: 'completed' | 'parental_approval_required';
  charged: Money;
  receipt_url: string;
  granted_cosmetic_ids: string[];
}

export interface ParentalControls {
  enabled: boolean;
  pin_set: boolean;
  purchases: 'allowed' | 'pin' | 'blocked';
  monthly_limit: Money | null;
  spent_this_month: Money;
  chat: 'everyone' | 'friends' | 'off';
  friend_requests: boolean;
}

export interface AppNotification {
  id: string;
  kind: 'friend_request' | 'party_invite' | 'server' | 'store' | 'system' | 'update';
  title: string;
  body: string;
  created_at: string;
  read: boolean;
  /** Route to open, e.g. "/servers/srv_1". */
  href?: string;
}

export interface ChatMessage {
  id: string;
  from_id: string;
  to_id: string;
  body: string;
  sent_at: string;
  /** Read receipt: when the recipient read it (null while unread). */
  read_at?: string | null;
}

export interface Conversation {
  user: UserLite & { presence: PresenceStatus };
  last_message: ChatMessage | null;
  unread: number;
}

export type RealtimeEvent =
  | { type: 'presence'; friend: Friend }
  | { type: 'friend-request'; request: FriendRequest }
  /** A friendship was made (request accepted) or ended elsewhere: refetch the roster. */
  | { type: 'friends-changed' }
  | { type: 'party'; party: Party | null }
  | { type: 'notification'; notification: AppNotification }
  | { type: 'message'; message: ChatMessage }
  | { type: 'messages-read'; reader_id: string; read_at: string }
  | { type: 'join-server'; address: string };

// ---- content marketplace (ADR 0155) ----

/** The API's content kinds (`native_mod` and both shader formats are their own kinds there). */
export type MarketplaceKind =
  | 'mod'
  | 'native_mod'
  | 'modpack'
  | 'resourcepack'
  | 'shader'
  | 'shaderpack'
  | 'datapack'
  | 'plugin';

/** A Rustic catalogue project as the signed-in pages show it (library, own creations). */
export interface MarketplaceProject {
  id: string;
  slug: string;
  kind: MarketplaceKind;
  name: string;
  summary: string;
  author: string;
  icon_url: string | null;
  /** The first screenshot. */
  banner_url: string | null;
  downloads: number;
  verified: boolean;
  /** Null when free. Paid projects are shown, not sold yet (creator sales are off). */
  price: Money | null;
  visibility: 'public' | 'unlisted' | 'private';
  /** `pending` while a native mod waits for an admin. */
  review: 'not_required' | 'pending' | 'approved' | 'rejected';
  /** Owner view: taken down by a moderator. */
  taken_down: boolean;
  updated_at: string | null;
  latest_version: string | null;
}

export interface LibraryItem {
  project: MarketplaceProject;
  source: 'free' | 'purchase' | 'grant';
  added_at: string;
}

/** Report reasons the API takes (`POST /v1/content/projects/{id}/reports`). */
export type ContentReportReason =
  | 'spam'
  | 'harassment'
  | 'inappropriate_content'
  | 'cheating'
  | 'impersonation'
  | 'copyright'
  | 'malware'
  | 'other';
