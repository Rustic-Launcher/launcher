import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import type { ListingSort, ServerEngine } from '@/cloud/types';
import { shellRoute } from '@/routes/base';

/** The Servers page's tabs (ADR 0158): the player's servers, the directory's front page, the list. */
export type ServersTab = 'mine' | 'discover' | 'browse';

export interface ServersSearch {
  tab?: ServersTab;
  q?: string;
  // Browse filters (a Discover row's "See all" pre-fills them)
  tag?: string;
  language?: string;
  engine?: ServerEngine;
  sort?: Exclude<ListingSort, 'featured'>;
  verified?: boolean;
}

const TABS: ServersTab[] = ['mine', 'discover', 'browse'];
const SORTS = ['trending', 'players', 'new', 'name'] as const;

const str = (v: unknown, max = 100): string | undefined =>
  typeof v === 'string' && v.length <= max ? v : undefined;

/** The search of `/servers`, strictly validated (older `joined`/`multiplayer` tabs are Mine now). */
export function validateServersSearch(s: Record<string, unknown>): ServersSearch {
  const tab = s.tab === 'joined' || s.tab === 'multiplayer' ? 'mine' : s.tab;
  const tag = str(s.tag, 24);
  const language = str(s.language, 2);
  return {
    tab: TABS.includes(tab as ServersTab) ? (tab as ServersTab) : undefined,
    q: str(s.q),
    tag: tag && /^[a-z0-9_-]+$/.test(tag) ? tag : undefined,
    language: language && /^[a-z]{2}$/.test(language) ? language : undefined,
    engine: s.engine === 'horizon' || s.engine === 'rustic' ? s.engine : undefined,
    sort: SORTS.includes(s.sort as (typeof SORTS)[number])
      ? (s.sort as (typeof SORTS)[number])
      : undefined,
    verified: s.verified === true || s.verified === 'true' || s.verified === 1 ? true : undefined,
  };
}

export const serversRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers',
  validateSearch: validateServersSearch,
  component: lazyRouteComponent(() => import('./ServersPage'), 'ServersPage'),
});

export const createServerRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers/new',
  component: lazyRouteComponent(() => import('./CreateServerPage'), 'CreateServerPage'),
});

/** One server of the directory, by slug or id (`rustic://server/<slug>` opens it). */
export const listingRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers/listing/$slug',
  component: lazyRouteComponent(() => import('./directory/ListingPage'), 'ListingPage'),
});

export const serverRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers/$serverId',
  validateSearch: (s: Record<string, unknown>): { tab?: string } => ({
    tab: typeof s.tab === 'string' ? s.tab : undefined,
  }),
  component: lazyRouteComponent(() => import('./ServerPage'), 'ServerPage'),
});

export const serversRoutes = [serversRoute, createServerRoute, listingRoute, serverRoute];
