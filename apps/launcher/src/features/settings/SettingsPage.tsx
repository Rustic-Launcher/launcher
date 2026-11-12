import { cn } from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  Baby,
  Gamepad2,
  Info,
  Palette,
  Rocket,
  ShieldCheck,
  UserRound,
  type LucideIcon,
} from 'lucide-react';
import { useRef, type KeyboardEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Page, PageHeader } from '@/components/Page';
import { SaveIndicator } from './components';
import { settingsRoute, SETTINGS_SECTIONS, type SettingsSection } from './routes';
import { AboutSection } from './sections/AboutSection';
import { AccountsSection } from './sections/AccountsSection';
import { AppearanceSection } from './sections/AppearanceSection';
import { GameSection } from './sections/GameSection';
import { LauncherSection } from './sections/LauncherSection';
import { ParentalSection } from './sections/ParentalSection';
import { PrivacySection } from './sections/PrivacySection';

const ICONS: Record<SettingsSection, LucideIcon> = {
  accounts: UserRound,
  game: Gamepad2,
  launcher: Rocket,
  appearance: Palette,
  privacy: ShieldCheck,
  parental: Baby,
  about: Info,
};

const VIEWS: Record<SettingsSection, () => ReactNode> = {
  accounts: AccountsSection,
  game: GameSection,
  launcher: LauncherSection,
  appearance: AppearanceSection,
  privacy: PrivacySection,
  parental: ParentalSection,
  about: AboutSection,
};

export function SettingsPage() {
  const { t } = useTranslation('settings');
  const { section = 'accounts' } = settingsRoute.useSearch();
  const navigate = useNavigate();
  const navRef = useRef<HTMLElement>(null);
  const View = VIEWS[section];

  // Up/Down move between sections (roving focus on the sub-navigation).
  const onKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    const i = SETTINGS_SECTIONS.indexOf(section);
    let next = -1;
    if (e.key === 'ArrowDown') next = (i + 1) % SETTINGS_SECTIONS.length;
    if (e.key === 'ArrowUp') next = (i - 1 + SETTINGS_SECTIONS.length) % SETTINGS_SECTIONS.length;
    if (next < 0) return;
    e.preventDefault();
    const id = SETTINGS_SECTIONS[next]!;
    void navigate({ to: '/settings', search: { section: id }, replace: true }).then(() =>
      navRef.current?.querySelector<HTMLAnchorElement>(`[data-section="${id}"]`)?.focus(),
    );
  };

  return (
    <Page>
      <PageHeader title={t('heading')} description={t('subtitle')} actions={<SaveIndicator />} />
      <div className="grid grid-cols-[220px_minmax(0,1fr)] items-start gap-8">
        <nav
          ref={navRef}
          aria-label={t('sectionsLabel')}
          onKeyDown={onKeyDown}
          className="sticky top-6"
        >
          <ul className="flex flex-col gap-0.5">
            {SETTINGS_SECTIONS.map((id) => {
              const Icon = ICONS[id];
              const active = id === section;
              return (
                <li key={id}>
                  <Link
                    to="/settings"
                    search={{ section: id }}
                    replace
                    data-section={id}
                    data-testid={`settings-nav-${id}`}
                    aria-current={active ? 'page' : undefined}
                    tabIndex={active ? 0 : -1}
                    className={cn(
                      'rx-focus-ring flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                      active
                        ? 'bg-accent-soft text-accent-text'
                        : 'text-fg-muted hover:bg-surface-2 hover:text-fg',
                    )}
                  >
                    <Icon className="size-4" aria-hidden />
                    {t(`sections.${id}`)}
                  </Link>
                </li>
              );
            })}
          </ul>
        </nav>
        <div className="flex min-w-0 flex-col gap-4">
          <h2 className="text-lg font-semibold">{t(`sections.${section}`)}</h2>
          <View key={section} />
        </div>
      </div>
    </Page>
  );
}
