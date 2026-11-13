import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import type { ContentKind, ContentSort, ContentSource, PriceFilter } from '@/ipc/types/content';
import { shellRoute } from '@/routes/base';

/** Explore's three tabs (ADR 0156). */
export type ContentTab = 'discover' | 'browse' | 'mine';

export interface ContentSearch {
  kind?: ContentKind;
  q?: string;
  /** Target: `instance:<id>` or `server:<id>`. */
  target?: string;
  /** Default: Discover, or Browse when a query is given. */
  tab?: ContentTab;
  // Browse filters (a "See all" pre-fills them)
  sort?: ContentSort;
  /** Comma-separated sources; default all. */
  sources?: string;
  category?: string;
  verified?: boolean;
  price?: PriceFilter;
}

const KINDS: ContentKind[] = ['mod', 'modpack', 'resourcepack', 'shader', 'datapack', 'plugin'];
const TABS: ContentTab[] = ['discover', 'browse', 'mine'];
const SORTS: ContentSort[] = ['relevance', 'downloads', 'updated', 'newest'];
const SOURCES: ContentSource[] = ['horizon', 'modrinth', 'rustic'];

const str = (v: unknown, max = 200): string | undefined =>
  typeof v === 'string' && v.length <= max ? v : undefined;
const flag = (v: unknown): boolean | undefined =>
  v === true || v === 'true' || v === '1' || v === 1 ? true : undefined;

/** The sources of a `sources=` value (unknown ones dropped; none = all). */
export function parseSources(v: string | undefined): ContentSource[] {
  const list = (v ?? '')
    .split(',')
    .filter((s): s is ContentSource => SOURCES.includes(s as ContentSource));
  return list.length ? [...new Set(list)] : [...SOURCES];
}

export function formatSources(list: ContentSource[]): string | undefined {
  return list.length === 0 || list.length === SOURCES.length
    ? undefined
    : SOURCES.filter((s) => list.includes(s)).join(',');
}

export function validateContentSearch(s: Record<string, unknown>): ContentSearch {
  const category = str(s.category, 64);
  return {
    kind: KINDS.includes(s.kind as ContentKind) ? (s.kind as ContentKind) : undefined,
    q: str(s.q),
    target: str(s.target),
    tab: TABS.includes(s.tab as ContentTab) ? (s.tab as ContentTab) : undefined,
    sort: SORTS.includes(s.sort as ContentSort) ? (s.sort as ContentSort) : undefined,
    sources: formatSources(parseSources(str(s.sources))),
    category: category && /^[a-z0-9_-]+$/.test(category) ? category : undefined,
    verified: flag(s.verified),
    price: s.price === 'free' || s.price === 'paid' ? s.price : undefined,
  };
}

export const contentRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/content',
  validateSearch: validateContentSearch,
  component: lazyRouteComponent(() => import('./ContentPage'), 'ContentPage'),
});

export interface ContentDetailSearch {
  target?: string;
  /** A version to preselect (`<project>@<version>`, from an install link). */
  version?: string;
  /** Opened by an install link: show the install prompt. */
  install?: boolean;
}

export const contentDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/content/$projectId',
  validateSearch: (s: Record<string, unknown>): ContentDetailSearch => ({
    target: str(s.target),
    version: str(s.version, 140),
    install: flag(s.install),
  }),
  component: lazyRouteComponent(() => import('./ContentDetailPage'), 'ContentDetailPage'),
});

export const contentRoutes = [contentRoute, contentDetailRoute];
