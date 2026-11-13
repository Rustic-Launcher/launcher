/**
 * The game's locales for the launcher and the website (ADR 0161): the registry, which bundle a
 * locale reads, browser negotiation, search, and the flags (drawn from the same shapes as in
 * game, loaded on demand).
 */
import { COMPLETE, LOCALES, PRIMARY, type LocaleInfo } from './registry';

export * from './registry';
export * from './validate';

const BY_CODE = new Map(LOCALES.map((l) => [l.code, l]));

/** The locale with this game code (case-insensitive). */
export function locale(code: string | null | undefined): LocaleInfo | undefined {
  return code ? BY_CODE.get(code.toLowerCase()) : undefined;
}

/** Whether `code` is one of the game's locales. */
export function isLocale(code: unknown): code is string {
  return typeof code === 'string' && BY_CODE.has(code.toLowerCase());
}

/** `de-at` → `de-AT`, `zh-hant` → `zh-Hant`: the usual casing of a BCP 47 tag. */
export function canonicalTag(tag: string): string {
  try {
    return Intl.getCanonicalLocales(tag)[0] ?? tag;
  } catch {
    return tag;
  }
}

function base(tag: string): string {
  return tag.split('-')[0]?.toLowerCase() ?? '';
}

/**
 * The bundles a locale may read, most specific first: its tag (`pt-PT`), its language (`pt`),
 * then its registry fallback's (`pt-BR`, `pt`). English is not in the list (it is everyone's
 * last resort).
 */
export function bundleCandidates(code: string): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  let l = locale(code);
  while (l && !seen.has(l.code)) {
    seen.add(l.code);
    const tag = canonicalTag(l.tag);
    const parts = tag.split('-');
    // `ca-ES-valencia` → `ca-ES-valencia`, `ca-ES`, `ca`.
    for (let i = parts.length; i >= 1; i--) {
      const c = parts.slice(0, i).join('-');
      if (!out.includes(c)) out.push(c);
    }
    l = locale(l.fallback);
  }
  return out;
}

/** The bundle `code` is shown in, among `available` (English when none fits). */
export function resolveBundle(code: string, available: readonly string[]): string {
  const have = new Set(available);
  return bundleCandidates(code).find((b) => have.has(b)) ?? 'en';
}

export type TranslationStatus = 'complete' | 'partial' | 'fallback' | 'none';

/**
 * How far `code` is translated: `complete` when it reads a complete bundle of its own language
 * (regional variants included: `de_at`, `en_gb`), `partial` when that bundle exists but is not
 * complete yet, `fallback` when it reads a close language through its registry fallback (`bar` →
 * German, `pt_pt` → Brazilian Portuguese), `none` when it reads English. Dialects (a `fallback`)
 * and tier-3 languages (Pirate Speak, LOLCAT…) never count as translated by their tag alone.
 */
export function translationStatus(
  code: string,
  available: readonly string[],
  complete: readonly string[] = COMPLETE,
): TranslationStatus {
  const l = locale(code);
  const b = resolveBundle(code, available);
  const own = l && !l.fallback && l.tier !== 3;
  if (own && base(b) === base(l.tag)) return complete.includes(b) ? 'complete' : 'partial';
  return b === 'en' ? 'none' : 'fallback';
}

/** The name a bundle's language is shown with (`de` → `Deutsch`, `pt-BR` → `Português (Brasil)`). */
export function bundleName(bundle: string): string {
  const l = locale(PRIMARY[bundle] ?? codeForTag(bundle));
  if (!l) return bundle;
  return bundle.includes('-') && l.region ? `${l.name} (${l.region})` : l.name;
}

/** The game code for a BCP 47 tag (`de-AT` → `de_at`, `de` → `de_de`, `pt` → `pt_br`). */
export function codeForTag(tag: string): string | undefined {
  const t = canonicalTag(tag);
  if (PRIMARY[t]) return PRIMARY[t];
  const exact = LOCALES.find((l) => l.code === t.toLowerCase().replace(/-/g, '_'));
  if (exact) return exact.code;
  const byTag = LOCALES.find((l) => canonicalTag(l.tag) === t && l.tier !== 3);
  if (byTag) return byTag.code;
  const b = base(t);
  if (PRIMARY[b]) return PRIMARY[b];
  return LOCALES.find((l) => base(l.tag) === b && l.tier !== 3)?.code;
}

/**
 * The best of `offered` (bundle tags such as `fr`, `pt-BR`) for a visitor's languages
 * (`navigator.languages`, `Accept-Language` order): an exact tag, else the same language
 * (`pt-PT` → `pt-BR`, `es-MX` → `es`). Null when none matches.
 */
export function negotiate(preferred: readonly string[], offered: readonly string[]): string | null {
  const tags = offered.map((o) => [o, canonicalTag(o)] as const);
  for (const p of preferred) {
    const want = canonicalTag(p);
    const exact = tags.find(([, t]) => t === want);
    if (exact) return exact[0];
    const code = codeForTag(want);
    if (code) {
      const viaCode = resolveBundle(code, offered);
      if (offered.includes(viaCode) && (viaCode !== 'en' || base(want) === 'en')) return viaCode;
    }
    const same = tags.find(([, t]) => base(t) === base(want));
    if (same) return same[0];
  }
  return null;
}

/** Lower case without accents, for search. */
export function fold(s: string): string {
  return s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}

/** Every word a search may match for a locale (its names, region, code and tag). */
export function searchText(l: LocaleInfo): string {
  return fold(`${l.name} ${l.region} ${l.english} ${l.code} ${l.tag}`);
}

/** Whether `l` matches every word of `query`. */
export function matches(l: LocaleInfo, query: string): boolean {
  const hay = searchText(l);
  return fold(query)
    .split(/\s+/)
    .filter(Boolean)
    .every((w) => hay.includes(w));
}

/** Locales sorted by their own name, then region (as the in-game screen does). */
export function sortedLocales(list: readonly LocaleInfo[] = LOCALES): LocaleInfo[] {
  return [...list].sort(
    (a, b) =>
      fold(a.name).localeCompare(fold(b.name)) ||
      fold(a.region).localeCompare(fold(b.region)) ||
      a.code.localeCompare(b.code),
  );
}

/** A flag as plain shapes in a 30 × 20 box (`flags.json`, generated from the game's flags). */
export type FlagShape =
  | ['r', number, number, number, number, string]
  | ['c', number, number, number, string]
  | ['p', string, string]
  | ['l', number, number, number, number, number, string]
  | ['s', number, number, number, number, FlagShape[]];
export interface FlagDesign {
  s: FlagShape[];
  /** Turned over (Upside Down English). */
  f?: 1;
}
export interface FlagData {
  size: [number, number];
  codes: string[];
  flags: Record<string, FlagDesign | null>;
}

let flagData: FlagData | null = null;
let flagPromise: Promise<FlagData> | null = null;

/** Every flag; its own chunk, fetched once. */
export function loadFlags(): Promise<FlagData> {
  flagPromise ??= import('./flags.json').then((m) => {
    flagData = m.default as unknown as FlagData;
    return flagData;
  });
  return flagPromise;
}

/** The flags if already loaded (else null; see `loadFlags`). */
export function loadedFlags(): FlagData | null {
  return flagData;
}

/** The short text a neutral tile shows for a locale without a flag (`TOK`, `IO`). */
export function tileText(code: string): string {
  return (code.split('_')[0] ?? code).slice(0, 3).toUpperCase();
}
