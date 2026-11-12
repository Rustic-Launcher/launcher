import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { shellRoute } from '@/routes/base';

export const serversRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers',
  validateSearch: (s: Record<string, unknown>): { tab?: string } => ({
    tab: typeof s.tab === 'string' ? s.tab : undefined,
  }),
  component: lazyRouteComponent(() => import('./ServersPage'), 'ServersPage'),
});

export const createServerRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers/new',
  component: lazyRouteComponent(() => import('./CreateServerPage'), 'CreateServerPage'),
});

export const serverRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/servers/$serverId',
  validateSearch: (s: Record<string, unknown>): { tab?: string } => ({
    tab: typeof s.tab === 'string' ? s.tab : undefined,
  }),
  component: lazyRouteComponent(() => import('./ServerPage'), 'ServerPage'),
});

export const serversRoutes = [serversRoute, createServerRoute, serverRoute];
