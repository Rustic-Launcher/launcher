import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { shellRoute } from '@/routes/base';

export const instancesRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/instances',
  validateSearch: (s: Record<string, unknown>): { new?: string; selected?: string } => ({
    // Hash URLs parse `?new=1` as a number: accept both (deep links, the rail's + button).
    new: s.new === undefined || s.new === null ? undefined : String(s.new),
    selected: typeof s.selected === 'string' ? s.selected : undefined,
  }),
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
