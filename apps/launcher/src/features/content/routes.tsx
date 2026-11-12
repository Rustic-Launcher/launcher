import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import type { ContentKind } from '@/ipc/types/content';
import { shellRoute } from '@/routes/base';

export interface ContentSearch {
  kind?: ContentKind;
  q?: string;
  /** Target: `instance:<id>` or `server:<id>`. */
  target?: string;
}

const KINDS: ContentKind[] = ['mod', 'modpack', 'resourcepack', 'shader', 'datapack', 'plugin'];

export const contentRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/content',
  validateSearch: (s: Record<string, unknown>): ContentSearch => ({
    kind: KINDS.includes(s.kind as ContentKind) ? (s.kind as ContentKind) : undefined,
    q: typeof s.q === 'string' ? s.q : undefined,
    target: typeof s.target === 'string' ? s.target : undefined,
  }),
  component: lazyRouteComponent(() => import('./ContentPage'), 'ContentPage'),
});

export const contentDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/content/$projectId',
  validateSearch: (s: Record<string, unknown>): { target?: string } => ({
    target: typeof s.target === 'string' ? s.target : undefined,
  }),
  component: lazyRouteComponent(() => import('./ContentDetailPage'), 'ContentDetailPage'),
});

export const contentRoutes = [contentRoute, contentDetailRoute];
