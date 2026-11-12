/**
 * Route tree roots (ADR 0026: TanStack Router, code-based, hash history, every page lazy).
 * Feature folders create their routes against `shellRoute` in their own `routes.tsx`.
 */
import { createRootRouteWithContext, createRoute, Outlet, redirect } from '@tanstack/react-router';
import type { QueryClient } from '@tanstack/react-query';
import { invoke } from '@/ipc';
import { qk } from '@/queries/core';
import { AppShell } from '@/layout/AppShell';
import { PageSkeleton } from '@/components/PageSkeleton';
import { RouteError } from '@/components/RouteError';

export interface RouterContext {
  queryClient: QueryClient;
}

export const rootRoute = createRootRouteWithContext<RouterContext>()({
  component: Outlet,
  errorComponent: RouteError,
});

/** Everything inside the launcher chrome (rail, bars, friends panel). */
export const shellRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'shell',
  component: AppShell,
  pendingComponent: PageSkeleton,
  beforeLoad: async ({ context }) => {
    const settings = await context.queryClient.ensureQueryData({
      queryKey: qk.settings,
      queryFn: () => invoke('settings_get'),
    });
    if (!settings.onboarding_completed) throw redirect({ to: '/onboarding' });
  },
});
