/**
 * i18n (ADR 0025, ADR 0161): i18next + react-i18next, one JSON namespace per area. The language
 * setting is one of the game's 143 locales (`de_de`, `pt_br`, `ja_jp`…, `@rustic/locales`); it reads
 * the best bundle of its chain (`de_at` → `de`, `pt_pt` → `pt-BR`) and English for any text the
 * bundle lacks. English ships with the app; every other bundle is its own chunk, loaded when
 * chosen. `locales.test.ts` checks English and French are complete and every other bundle valid.
 */
import {
  bundleCandidates,
  codeForTag,
  isLocale,
  locale,
  negotiate,
  PRIMARY,
  resolveBundle,
  translationStatus,
  type TranslationStatus,
} from '@rustic/locales';
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

/** A game locale code (`en_us`, `fr_fr`, `de_at`…). */
export type Language = string;

export const DEFAULT_LANGUAGE: Language = 'en_us';

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
} as const;

export const NAMESPACES = Object.keys(resources.en) as (keyof (typeof resources)['en'])[];

type Json = { default: Record<string, unknown> };
/** Every namespace file of every bundle but English, each its own chunk. */
const FILES = import.meta.glob<Json>(['./locales/*/*.json', '!./locales/en/*.json']);

const bundleOf = (path: string) => path.split('/')[2] ?? '';

/** Bundles that exist (`en`, `fr`, `de`, `pt-BR`…). */
export const BUNDLES: readonly string[] = [
  'en',
  ...new Set(Object.keys(FILES).map(bundleOf).filter(Boolean)),
];

/** How far a locale is translated in the launcher. */
export function languageStatus(code: Language): TranslationStatus {
  return translationStatus(code, BUNDLES);
}

/** A stored setting of an older version (`en`, `fr`) as a game locale. */
export function normaliseLanguage(value: unknown): Language | null {
  if (typeof value !== 'string' || !value) return null;
  if (isLocale(value)) return value.toLowerCase();
  return PRIMARY[value] ?? codeForTag(value) ?? null;
}

/** The saved choice, else the system's languages matched to a translated bundle. */
export function detectLanguage(): Language {
  try {
    const saved = JSON.parse(window.localStorage.getItem('rustic.prefs') ?? '{}') as {
      state?: { language?: string };
    };
    const l = normaliseLanguage(saved.state?.language);
    if (l) return l;
  } catch {
    /* ignore */
  }
  const nav =
    typeof navigator !== 'undefined' ? [...(navigator.languages ?? [navigator.language])] : [];
  const bundle = negotiate(nav, BUNDLES);
  if (!bundle) return DEFAULT_LANGUAGE;
  // The visitor's own region when it reads this bundle (`de-AT` stays Austrian).
  const exact = nav
    .map((t) => codeForTag(t))
    .find((c) => c && resolveBundle(c, BUNDLES) === bundle);
  return exact ?? PRIMARY[bundle] ?? DEFAULT_LANGUAGE;
}

const loaded = new Set<string>(['en']);

async function loadBundle(bundle: string) {
  if (loaded.has(bundle)) return;
  const files = Object.entries(FILES).filter(([p]) => bundleOf(p) === bundle);
  await Promise.all(
    files.map(async ([p, load]) => {
      const ns = p.split('/')[3]?.replace(/\.json$/, '') ?? '';
      const { default: res } = await load();
      i18n.addResourceBundle(bundle, ns, res, true, true);
    }),
  );
  loaded.add(bundle);
}

/** The bundles game locale `code` reads, most specific first, English last. */
export function bundleChain(code: Language): string[] {
  return [...bundleCandidates(code).filter((b) => b !== 'en' && BUNDLES.includes(b)), 'en'];
}

/** i18next's fallback for each tag in use: the locale's bundles, then English. */
const fallbacks = new Map<string, string[]>();
let current: Language = DEFAULT_LANGUAGE;

/** The game locale the launcher is shown in (`i18n.language` is its BCP 47 tag). */
export function currentLanguage(): Language {
  return current;
}

/**
 * Switches the launcher to game locale `code`: loads its bundles, then sets i18next's language to
 * its BCP 47 tag (so dates and numbers follow the region) and the page's `lang` / `dir`.
 */
export async function changeLanguage(code: Language): Promise<void> {
  const l = locale(code) ?? locale(DEFAULT_LANGUAGE);
  const bundles = bundleChain(l?.code ?? DEFAULT_LANGUAGE).filter((b) => b !== 'en');
  await Promise.all(bundles.map(loadBundle));
  const tag = l?.tag ?? 'en';
  current = l?.code ?? DEFAULT_LANGUAGE;
  fallbacks.set(tag, [...bundles, 'en']);
  await i18n.changeLanguage(tag);
  document.documentElement.lang = tag;
  document.documentElement.dir = l?.dir ?? 'ltr';
}

void i18n.use(initReactI18next).init({
  resources,
  lng: 'en',
  fallbackLng: (code: string) => fallbacks.get(code) ?? ['en'],
  ns: NAMESPACES,
  defaultNS: 'common',
  interpolation: { escapeValue: false },
  returnNull: false,
});

document.documentElement.lang = 'en';

/** Resolves once the starting language is loaded: render after it (main.tsx). */
export const i18nReady: Promise<void> = changeLanguage(detectLanguage()).catch(() => undefined);

export default i18n;
