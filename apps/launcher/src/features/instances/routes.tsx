import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { shellRoute } from '@/routes/base';
import { PROFILE_SORTS, type ProfileSort, type ProfileSource } from './logic';

/** The search of `/instances` (ADR 0159): the sheet, the search text, the sort and the filters. */
export interface InstancesSearch {
  /** `1` opens the new-profile sheet on Create, `import` on Import. */
  new?: string;
  /** Older links: scroll to this profile. */
  selected?: string;
  q?: string;
  sort?: ProfileSort;
  source?: ProfileSource;
  /** A Horizon build id (`v0.2.15`, `nightly-…`). */
  version?: string;
  edition?: 'rustic' | 'horizon';
  native?: boolean;
}

const str = (v: unknown, max = 100): string | undefined =>
  typeof v === 'string' && v.length > 0 && v.length <= max ? v : undefined;

/** Strict: anything unknown is dropped, so a stale or hand-written link still opens the page. */
export function validateInstancesSearch(s: Record<string, unknown>): InstancesSearch {
  const version = str(s.version, 64);
  return {
    // Hash URLs parse `?new=1` as a number: accept both (deep links, the rail's + button).
    new: s.new === undefined || s.new === null ? undefined : String(s.new),
    selected: str(s.selected),
    q: str(s.q),
    sort: PROFILE_SORTS.includes(s.sort as ProfileSort) ? (s.sort as ProfileSort) : undefined,
    source: s.source === 'custom' || s.source === 'modpack' ? s.source : undefined,
    version: version && /^[a-z0-9._-]+$/i.test(version) ? version : undefined,
    edition: s.edition === 'rustic' || s.edition === 'horizon' ? s.edition : undefined,
    native: s.native === true || s.native === 'true' || s.native === 1 ? true : undefined,
  };
}

export const instancesRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/instances',
  validateSearch: validateInstancesSearch,
  component: lazyRouteComponent(() => import('./InstancesPage'), 'InstancesPage'),
});

export const instanceRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/instances/$instanceId',
  validateSearch: (s: Record<string, unknown>): { tab?: string } => ({
    tab: typeof s.tab === 'string' ? s.tab : undefined,
  }),
  component: lazyRouteComponent(() => import('./InstanceDetailPage'), 'InstanceDetailPage'),
});

export const instancesRoutes = [instancesRoute, instanceRoute];
