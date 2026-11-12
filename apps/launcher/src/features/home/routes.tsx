import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { shellRoute } from '@/routes/base';

export const homeRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/',
  component: lazyRouteComponent(() => import('./HomePage'), 'HomePage'),
});

export const homeRoutes = [homeRoute];
