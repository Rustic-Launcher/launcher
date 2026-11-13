/**
 * Rustic Cloud client facade. With `VITE_CLOUD_URL` set, the real client over the generated OpenAPI
 * types (`./http`) is used; otherwise (default) the mock (`src/cloud/mock`). The interface below is what pages depend on, so swapping the implementation
 * is local to this file.
 */
import type {
  AppNotification,
  ChatMessage,
  ContentReportReason,
  Conversation,
  Cosmetic,
  Currency,
  FeaturedServer,
  Friend,
  FriendRequest,
  FriendSuggestions,
  FriendTarget,
  LibraryItem,
  Loadout,
  MarketplaceProject,
  Me,
  NewsItem,
  Outfit,
  ParentalControls,
  Party,
  PurchaseRequest,
  PurchaseResult,
  RealtimeEvent,
  ServerListing,
  ServerListingPage,
  ServerListingQuery,
  StoreCatalog,
  StoreProduct,
  UserLite,
  UserLookup,
} from './types';

export interface CloudClient {
  me(): Promise<Me>;
  // social
  friends(): Promise<Friend[]>;
  friendRequests(): Promise<FriendRequest[]>;
  searchUsers(query: string): Promise<UserLite[]>;
  /** Exact Rustic or Minecraft name; errors carry the API codes (`not_found`, `cannot_add_friend`...). */
  lookupUser(by: 'rustic' | 'minecraft', name: string): Promise<UserLookup>;
  /** A bare string is a Rustic username. */
  sendFriendRequest(target: string | FriendTarget): Promise<FriendRequest>;
  /** Players met recently (ADR 0145). */
  friendSuggestions(): Promise<FriendSuggestions>;
  /** "Appear in suggestions of players I meet" (refused for minors). */
  setDiscoverable(on: boolean): Promise<Me>;
  respondFriendRequest(id: string, accept: boolean): Promise<void>;
  cancelFriendRequest(id: string): Promise<void>;
  removeFriend(id: string): Promise<void>;
  setFavoriteFriend(id: string, favorite: boolean): Promise<void>;
  party(): Promise<Party | null>;
  inviteToParty(userId: string): Promise<Party>;
  leaveParty(): Promise<void>;
  subscribe(handler: (e: RealtimeEvent) => void): () => void;
  // chats (Social window)
  conversations(): Promise<Conversation[]>;
  messages(userId: string): Promise<ChatMessage[]>;
  sendMessage(userId: string, body: string): Promise<ChatMessage>;
  markConversationRead(userId: string): Promise<void>;
  // content feeds
  news(): Promise<NewsItem[]>;
  featuredServers(): Promise<FeaturedServer[]>;
  // server directory (public; ADR 0158)
  /** `GET /v1/server-listings`: search, filters, sorts, paging. */
  serverListings(query: ServerListingQuery): Promise<ServerListingPage>;
  /** `GET /v1/server-listings/{id}`: one server by id or slug, with its description. */
  serverListing(idOrSlug: string): Promise<ServerListing>;
  /** `POST /v1/server-listings/{id}/plays`: one anonymous "Play" press (the trending signal). */
  recordServerPlay(id: string): Promise<void>;
  /** `POST /v1/server-listings/{id}/reports` (signed in). */
  reportServerListing(id: string, reason: ContentReportReason, details?: string): Promise<void>;
  notifications(): Promise<AppNotification[]>;
  markNotificationsRead(ids: string[]): Promise<void>;
  // cosmetics & store
  cosmetics(): Promise<Cosmetic[]>;
  loadout(): Promise<Loadout>;
  equip(loadout: Loadout): Promise<Loadout>;
  outfits(): Promise<Outfit[]>;
  saveOutfit(name: string, loadout: Loadout): Promise<Outfit>;
  deleteOutfit(id: string): Promise<void>;
  catalog(currency: Currency): Promise<StoreCatalog>;
  product(id: string, currency: Currency): Promise<StoreProduct>;
  purchase(req: PurchaseRequest): Promise<PurchaseResult>;
  // content marketplace (signed in; ADR 0155)
  /** `GET /v1/me/library`: free projects added, purchases and grants. */
  contentLibrary(): Promise<LibraryItem[]>;
  /** `PUT /v1/me/library/{id}`: a free project (paid ones answer `paid_content`). */
  libraryAdd(projectId: string): Promise<void>;
  libraryRemove(projectId: string): Promise<void>;
  /** `GET /v1/me/content`: the projects this account publishes, whatever their state. */
  myContent(): Promise<MarketplaceProject[]>;
  reportContent(projectId: string, reason: ContentReportReason, details?: string): Promise<void>;
  // account & compliance
  parental(): Promise<ParentalControls>;
  setParental(patch: Partial<ParentalControls> & { pin?: string }): Promise<ParentalControls>;
  verifyParentalPin(pin: string): Promise<boolean>;
  /** Sign-in methods on the Rustic account (Microsoft identity, password). */
  identities(): Promise<LinkedIdentities>;
  /** Unlinks a sign-in provider; refused (`last_sign_in_method`) when it is the only way in. */
  unlinkIdentity(provider: 'microsoft'): Promise<void>;
  exportAccount(): Promise<{ url: string; expires_at: string }>;
  deleteAccount(confirmUsername: string): Promise<{ scheduled_for: string }>;
  joinWaitlist(email: string): Promise<{ position: number }>;
}

export interface LinkedIdentities {
  has_password: boolean;
  identities: { provider: string; email: string | null; linked_at: string }[];
}

let client: Promise<CloudClient> | null = null;

export function cloud(): Promise<CloudClient> {
  if (!client) {
    const base = import.meta.env.VITE_CLOUD_URL as string | undefined;
    const mock = import('./mock').then((m) => m.createMockCloud());
    client = base
      ? Promise.all([import('./http'), mock]).then(([h, fallback]) =>
          h.createHttpCloud(base, fallback),
        )
      : mock;
  }
  return client;
}

/** Override (tests). */
export function setCloud(c: CloudClient): void {
  client = Promise.resolve(c);
}
