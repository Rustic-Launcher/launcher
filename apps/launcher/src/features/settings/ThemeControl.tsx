import { SegmentedControl, type ThemePreference } from '@rustic/ui';
import { Monitor, Moon, Sun } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { Language } from '@/i18n';
import { usePrefs } from '@/stores/prefs';
import i18n from '@/i18n';

export function ThemeControl() {
  const { t } = useTranslation('settings');
  const theme = usePrefs((s) => s.theme);
  const setTheme = usePrefs((s) => s.setTheme);
  return (
    <SegmentedControl
      className="isolate"
      label={t('appearance.theme.label')}
      value={theme}
      onValueChange={(v) => setTheme(v as ThemePreference)}
      options={[
        { value: 'dark', label: t('appearance.theme.dark'), icon: <Moon /> },
        { value: 'light', label: t('appearance.theme.light'), icon: <Sun /> },
        { value: 'system', label: t('appearance.theme.system'), icon: <Monitor /> },
      ]}
    />
  );
}

export function LanguageControl() {
  const { t } = useTranslation('settings');
  const language = usePrefs((s) => s.language) ?? (i18n.language as Language);
  const setLanguage = usePrefs((s) => s.setLanguage);
  return (
    <SegmentedControl
      className="isolate"
      label={t('launcher.language.label')}
      value={language}
      onValueChange={(v) => setLanguage(v as Language)}
      options={[
        { value: 'en', label: t('languages.en') },
        { value: 'fr', label: t('languages.fr') },
      ]}
    />
  );
}
