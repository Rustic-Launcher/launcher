import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { rootRoute } from '@/routes/base';

export const onboardingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/onboarding',
  component: lazyRouteComponent(() => import('./OnboardingPage'), 'OnboardingPage'),
});
