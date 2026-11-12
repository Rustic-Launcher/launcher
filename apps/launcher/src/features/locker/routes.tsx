import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { shellRoute } from '@/routes/base';

export const lockerRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/locker',
  component: lazyRouteComponent(() => import('./LockerPage'), 'LockerPage'),
});

export const lockerRoutes = [lockerRoute];
