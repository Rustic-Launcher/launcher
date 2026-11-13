import { LanguageSelect, SegmentedControl, type ThemePreference } from '@rustic/ui';
import { Monitor, Moon, Sun } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { usePrefs } from '@/stores/prefs';
import { bundleName, resolveBundle } from '@rustic/locales';
import { BUNDLES, currentLanguage, languageStatus } from '@/i18n';

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

/**
 * The launcher's language: every game locale with its flag, translated ones first (ADR 0161).
 * Applies at once (main.tsx follows the stored choice).
 */
export function LanguageControl() {
  const { t } = useTranslation('settings');
  const language = usePrefs((s) => s.language) ?? currentLanguage();
  const setLanguage = usePrefs((s) => s.setLanguage);
  return (
    <LanguageSelect
      value={language}
      onChange={setLanguage}
      status={languageStatus}
      testId="language-select"
      className="max-w-sm"
      labels={{
        label: t('launcher.language.label'),
        search: t('launcher.language.search'),
        empty: t('launcher.language.empty'),
        translated: t('launcher.language.translated'),
        others: t('launcher.language.others'),
        partial: t('launcher.language.partial'),
        english: t('launcher.language.english'),
        via: (code) =>
          t('launcher.language.via', { lang: bundleName(resolveBundle(code, BUNDLES)) }),
        footer: t('launcher.language.help'),
      }}
    />
  );
}
