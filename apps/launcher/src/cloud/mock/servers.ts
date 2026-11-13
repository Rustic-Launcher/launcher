/**
 * Mock server directory (ADR 0158): a dozen listings with the API's filters, sorts and paging, so
 * the Servers page runs in a browser and the e2e tests have stable data. Addresses of Arena Nord
 * and Skyreach are those of the mock multiplayer list (the "In your list" state).
 */
import type { CloudClient } from '../client';
import { CloudError } from '../errors';
import type {
  ListingSort,
  ServerEngine,
  ServerListing,
  ServerListingPage,
  ServerListingQuery,
} from '../types';
import { daysAgo, latency, minutesAgo } from '../../ipc/mock/runtime';

interface Seed {
  slug: string;
  name: string;
  address: string;
  summary: string;
  tags: string[];
  languages: string[];
  engine: ServerEngine;
  verified?: boolean;
  featured?: number;
  /** Players online, or null when offline / never pinged (`undefined`). */
  online?: number | null;
  max?: number;
  plays: number;
  age: number;
  edition?: boolean;
  description?: string;
}

const SEEDS: Seed[] = [
  {
    slug: 'arena-nord',
    name: 'Arena Nord',
    address: 'arena-nord.fr',
    summary: 'Ranked PvP duels, practice kits and weekly tournaments.',
    tags: ['pvp', 'minigames'],
    languages: ['fr', 'en'],
    engine: 'rustic',
    verified: true,
    featured: 1,
    online: 212,
    max: 500,
    plays: 940,
    age: 210,
    description:
      '# Arena Nord\n- Ranked **1v1** and 2v2 duels\n- Practice kits and replays\n- Weekly tournaments with prizes\n\nRules and ranks on [our site](https://example.org).',
  },
  {
    slug: 'skyreach',
    name: 'Skyreach',
    address: 'play.skyreach.gg',
    summary: 'Co-op skyblock islands with weekly challenges.',
    tags: ['skyblock', 'survival'],
    languages: ['en'],
    engine: 'horizon',
    featured: 2,
    online: 91,
    max: 300,
    plays: 410,
    age: 120,
  },
  {
    slug: 'les-copains-smp',
    name: 'Les Copains SMP',
    address: 'copains-smp.example.org',
    summary: 'Survie détente entre amis, économie joueurs et events le week-end.',
    tags: ['smp', 'survival', 'economy'],
    languages: ['fr'],
    engine: 'rustic',
    verified: true,
    online: 38,
    max: 100,
    plays: 260,
    age: 90,
    edition: true,
  },
  {
    slug: 'builders-guild',
    name: 'Builders Guild',
    address: 'guild.example.org',
    summary: 'Creative plots with WorldEdit-like tools as Rustic plugins.',
    tags: ['creative', 'building'],
    languages: ['en', 'de'],
    engine: 'rustic',
    verified: true,
    featured: 3,
    online: 54,
    max: 150,
    plays: 120,
    age: 300,
  },
  {
    slug: 'mythos-rp',
    name: 'Mythos RP',
    address: 'mythos-rp.example.org',
    summary: 'Medieval roleplay with towns, guilds and a living economy.',
    tags: ['roleplay', 'towny', 'economy'],
    languages: ['fr', 'en'],
    engine: 'rustic',
    online: 77,
    max: 200,
    plays: 180,
    age: 40,
  },
  {
    slug: 'blockparty',
    name: 'BlockParty',
    address: 'play.blockparty.example.org',
    summary: 'Bedwars, skywars and parkour races in short rounds.',
    tags: ['minigames', 'bedwars', 'parkour'],
    languages: ['en', 'es'],
    engine: 'horizon',
    verified: true,
    online: 480,
    max: 1000,
    plays: 1200,
    age: 400,
  },
  {
    slug: 'oneblock-odyssey',
    name: 'OneBlock Odyssey',
    address: 'oneblock.example.org:25570',
    summary: 'Start on one block, unlock biomes and bosses as you mine.',
    tags: ['oneblock', 'skyblock'],
    languages: ['en'],
    engine: 'horizon',
    online: 23,
    max: 80,
    plays: 60,
    age: 6,
  },
  {
    slug: 'terres-sauvages',
    name: 'Terres Sauvages',
    address: 'terres-sauvages.example.org',
    summary: 'Survie hardcore sans téléportation, carte de 20 000 blocs.',
    tags: ['hardcore', 'survival', 'vanilla'],
    languages: ['fr'],
    engine: 'horizon',
    online: 12,
    max: 50,
    plays: 35,
    age: 2,
  },
  {
    slug: 'factions-reborn',
    name: 'Factions Reborn',
    address: 'factions-reborn.example.org',
    summary: 'Classic factions with raids, outposts and seasonal resets.',
    tags: ['factions', 'pvp'],
    languages: ['en', 'pl'],
    engine: 'rustic',
    online: null,
    plays: 15,
    age: 14,
  },
  {
    slug: 'parkour-paradise',
    name: 'Parkour Paradise',
    address: 'parkour.example.org',
    summary: 'Three hundred hand-made parkour maps, from easy to impossible.',
    tags: ['parkour', 'minigames'],
    languages: ['en', 'fr', 'de'],
    engine: 'horizon',
    online: 31,
    max: 120,
    plays: 75,
    age: 1,
  },
  {
    slug: 'prison-break',
    name: 'Prison Break',
    address: 'prison.example.org',
    summary: 'Mine your way out: ranks, gangs and a player market.',
    tags: ['prison', 'economy'],
    languages: ['en'],
    engine: 'rustic',
    online: 44,
    max: 150,
    plays: 50,
    age: 60,
  },
  {
    slug: 'atelier-creatif',
    name: 'L’Atelier Créatif',
    address: 'atelier.example.org',
    summary: 'Parcelles créatives, concours de build mensuels et galerie.',
    tags: ['creative', 'building'],
    languages: ['fr'],
    engine: 'horizon',
    plays: 8,
    age: 4,
  },
];

function toListing(s: Seed, i: number): ServerListing {
  const offline = s.online === null;
  return {
    id: `0190f0aa-0000-7000-8000-${String(i + 1).padStart(12, '0')}`,
    slug: s.slug,
    name: s.name,
    address: s.address,
    summary: s.summary,
    description:
      s.description ??
      `# ${s.name}\n${s.summary}\n\n- Friendly staff\n- Fair play: no hacked clients\n- Discord for events`,
    tags: s.tags,
    languages: s.languages,
    engine: s.engine,
    game_version: '26.3',
    requires_rustic_edition: s.edition ?? false,
    website: 'https://example.org',
    discord: i % 2 ? 'https://discord.gg/example' : null,
    icon_url: null,
    banner_url: null,
    verified: s.verified ?? false,
    featured: s.featured !== undefined,
    owner: { id: `u_owner${i}`, username: `owner${i}`, display_name: `Owner ${i + 1}` },
    status:
      s.online === undefined
        ? null
        : {
            online: !offline,
            players_online: offline ? null : s.online,
            players_max: offline ? null : (s.max ?? 100),
            motd: offline ? null : `${s.name} · ${s.summary}`,
            version: offline ? null : 'Horizon 26.3',
            checked_at: minutesAgo(4),
          },
    plays_7d: s.plays,
    created_at: daysAgo(s.age),
    updated_at: daysAgo(Math.max(0, s.age - 1)),
  };
}

export const listingsDb: ServerListing[] = SEEDS.map(toListing);
const featuredRank = new Map(SEEDS.map((s, i) => [listingsDb[i]!.id, s.featured]));

const playersOf = (l: ServerListing) => (l.status?.online ? (l.status.players_online ?? 0) : -1);

/** The API's trending score (routes/listings.rs). */
export function trendScore(l: ServerListing): number {
  return (
    l.plays_7d * 3 +
    Math.min(Math.max(playersOf(l), 0), 2000) / 10 +
    (l.verified ? 2 : 0) +
    (l.featured ? 1 : 0)
  );
}

/** Filters and sorts like `GET /v1/server-listings` (pure, unit-tested). */
export function queryListings(all: ServerListing[], q: ServerListingQuery): ServerListingPage {
  const text = (q.q ?? '').trim().toLowerCase();
  const sort: ListingSort = q.sort ?? (q.featured ? 'featured' : 'trending');
  const hits = all
    .filter(
      (l) =>
        (!text ||
          l.name.toLowerCase().includes(text) ||
          l.summary.toLowerCase().includes(text) ||
          l.address.includes(text) ||
          l.slug.includes(text) ||
          l.tags.includes(text)) &&
        (!q.tag || l.tags.includes(q.tag)) &&
        (!q.language || l.languages.includes(q.language)) &&
        (!q.engine || l.engine === q.engine) &&
        (!q.verified || l.verified) &&
        (!q.featured || l.featured),
    )
    .sort((a, b) => {
      switch (sort) {
        case 'players':
          return playersOf(b) - playersOf(a) || a.name.localeCompare(b.name);
        case 'new':
          return b.created_at.localeCompare(a.created_at);
        case 'name':
          return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
        case 'featured':
          return (featuredRank.get(a.id) ?? 1e9) - (featuredRank.get(b.id) ?? 1e9);
        default:
          return trendScore(b) - trendScore(a) || b.created_at.localeCompare(a.created_at);
      }
    });
  const offset = Math.max(0, q.offset ?? 0);
  const limit = Math.min(50, Math.max(1, q.limit ?? 24));
  return {
    items: hits.slice(offset, offset + limit).map((l) => ({ ...l, description: null })),
    has_more: hits.length > offset + limit,
    total: hits.length,
  };
}

export const serversMock: Pick<
  CloudClient,
  'serverListings' | 'serverListing' | 'recordServerPlay' | 'reportServerListing'
> = {
  async serverListings(q) {
    await latency(0.5);
    return structuredClone(queryListings(listingsDb, q));
  },
  async serverListing(idOrSlug) {
    await latency(0.4);
    const l = listingsDb.find((x) => x.id === idOrSlug || x.slug === idOrSlug);
    if (!l) throw new CloudError('not_found', 'server not found', 404);
    return structuredClone(l);
  },
  async recordServerPlay(id) {
    const l = listingsDb.find((x) => x.id === id);
    if (l) l.plays_7d += 1;
  },
  async reportServerListing() {
    await latency(0.5);
  },
};
