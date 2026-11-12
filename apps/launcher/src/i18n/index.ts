/**
 * i18n (ADR 0025): i18next + react-i18next, French and English, one JSON namespace per area.
 * Every UI string goes through `t()`; `locales.test.ts` checks both languages have the same keys.
 */
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import enCommon from './locales/en/common.json';
import enContent from './locales/en/content.json';
import enHome from './locales/en/home.json';
import enInstances from './locales/en/instances.json';
import enLocker from './locales/en/locker.json';
import enOnboarding from './locales/en/onboarding.json';
import enServers from './locales/en/servers.json';
import enSettings from './locales/en/settings.json';
import frCommon from './locales/fr/common.json';
import frContent from './locales/fr/content.json';
import frHome from './locales/fr/home.json';
import frInstances from './locales/fr/instances.json';
import frLocker from './locales/fr/locker.json';
import frOnboarding from './locales/fr/onboarding.json';
import frServers from './locales/fr/servers.json';
import frSettings from './locales/fr/settings.json';

export const LANGUAGES = ['en', 'fr'] as const;
export type Language = (typeof LANGUAGES)[number];

export const resources = {
  en: {
    common: enCommon,
    home: enHome,
    instances: enInstances,
    content: enContent,
    servers: enServers,
    locker: enLocker,
    settings: enSettings,
    onboarding: enOnboarding,
  },
  fr: {
    common: frCommon,
    home: frHome,
    instances: frInstances,
    content: frContent,
    servers: frServers,
    locker: frLocker,
    settings: frSettings,
    onboarding: frOnboarding,
  },
} as const;

export const NAMESPACES = Object.keys(resources.en) as (keyof (typeof resources)['en'])[];

export function detectLanguage(): Language {
  try {
    const saved = JSON.parse(window.localStorage.getItem('rustic.prefs') ?? '{}') as {
      state?: { language?: string };
    };
    const l = saved.state?.language;
    if (l === 'en' || l === 'fr') return l;
  } catch {
    /* ignore */
  }
  const nav = typeof navigator !== 'undefined' ? navigator.language.slice(0, 2) : 'en';
  return nav === 'fr' ? 'fr' : 'en';
}

void i18n.use(initReactI18next).init({
  resources,
  lng: detectLanguage(),
  fallbackLng: 'en',
  ns: NAMESPACES,
  defaultNS: 'common',
  interpolation: { escapeValue: false },
  returnNull: false,
});

i18n.on('languageChanged', (lng) => {
  document.documentElement.lang = lng;
});
document.documentElement.lang = i18n.language;

export default i18n;
