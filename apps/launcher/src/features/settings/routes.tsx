import { createRoute, lazyRouteComponent } from '@tanstack/react-router';
import { shellRoute } from '@/routes/base';
import { SETTINGS_ANCHORS, type SettingsAnchor } from './anchors';

export const SETTINGS_SECTIONS = [
  'accounts',
  'game',
  'launcher',
  'appearance',
  'privacy',
  'parental',
  'about',
] as const;
export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

export const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: '/settings',
  validateSearch: (
    s: Record<string, unknown>,
  ): { section?: SettingsSection; focus?: SettingsAnchor } => ({
    section: SETTINGS_SECTIONS.includes(s.section as SettingsSection)
      ? (s.section as SettingsSection)
      : undefined,
    focus: SETTINGS_ANCHORS.includes(s.focus as SettingsAnchor)
      ? (s.focus as SettingsAnchor)
      : undefined,
  }),
  component: lazyRouteComponent(() => import('./SettingsPage'), 'SettingsPage'),
});

export const settingsRoutes = [settingsRoute];
