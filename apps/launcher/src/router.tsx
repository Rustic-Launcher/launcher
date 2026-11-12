import { createHashHistory, createRouter } from '@tanstack/react-router';
import { PageSkeleton } from '@/components/PageSkeleton';
import { contentRoutes } from '@/features/content/routes';
import { homeRoutes } from '@/features/home/routes';
import { instancesRoutes } from '@/features/instances/routes';
import { lockerRoutes } from '@/features/locker/routes';
import { onboardingRoute } from '@/features/onboarding/routes';
import { serversRoutes } from '@/features/servers/routes';
import { settingsRoutes } from '@/features/settings/routes';
import { socialRoute } from '@/features/social/routes';
import { queryClient } from '@/queries/core';
import { rootRoute, shellRoute } from '@/routes/base';
import { NotFound } from '@/components/NotFound';

const routeTree = rootRoute.addChildren([
  shellRoute.addChildren([
    ...homeRoutes,
    ...instancesRoutes,
    ...contentRoutes,
    ...serversRoutes,
    ...lockerRoutes,
    ...settingsRoutes,
  ]),
  onboardingRoute,
  socialRoute,
]);

export const router = createRouter({
  routeTree,
  // Hash history: works from Tauri's custom protocol and any static server without rewrites.
  history: createHashHistory(),
  context: { queryClient },
  defaultPreload: 'intent',
  defaultPreloadStaleTime: 0,
  defaultPendingComponent: PageSkeleton,
  defaultPendingMs: 120,
  defaultPendingMinMs: 200,
  defaultNotFoundComponent: NotFound,
  scrollRestoration: true,
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
