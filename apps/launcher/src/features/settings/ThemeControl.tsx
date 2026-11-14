import { LanguageSelect, SegmentedControl, type ThemePreference } from '@rustic/ui';
import { Monitor, Moon, Sun } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { usePrefs } from '@/stores/prefs';
import { bundleName, resolveBundle } from '@rustic/locales';
import { cloud } from '@/cloud/client';
import { BUNDLES, currentLanguage, languageStatus, type Language } from '@/i18n';

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
  const setPref = usePrefs((s) => s.setLanguage);
  const setLanguage = (code: Language) => {
    setPref(code);
    // The account language is shared with the website (ADR 0168): best effort, signed in only.
    void syncAccountLocale(code);
  };
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

/** Saves the launcher's language on the Rustic account (the website's bundle tag: `de`, `pt-BR`). */
export async function syncAccountLocale(code: Language): Promise<void> {
  try {
    await (await cloud()).setAccountLocale(resolveBundle(code, BUNDLES));
  } catch {
    // Not signed in to Rustic, or offline: the launcher's own choice stays.
  }
}
