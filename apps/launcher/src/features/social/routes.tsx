import { createRoute } from '@tanstack/react-router';
import type { SocialTab } from '@/stores/ui';
import { rootRoute } from '@/routes/base';
import { SocialWindowPage } from './SocialHosts';

/** `/social`: the detachable Social window's content (no launcher chrome). */
export const socialRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/social',
  validateSearch: (s: Record<string, unknown>): { tab?: SocialTab; with?: string } => ({
    tab: s.tab === 'chats' || s.tab === 'friends' ? s.tab : undefined,
    with: typeof s.with === 'string' ? s.with : undefined,
  }),
  component: function SocialRouteComponent() {
    const search = socialRoute.useSearch();
    return <SocialWindowPage tab={search.tab} withUser={search.with} />;
  },
});
