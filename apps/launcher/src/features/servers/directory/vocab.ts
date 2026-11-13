/** The server directory's vocabulary (ADR 0158): tags and languages offered as filters. */

/** The directory's tags (the API's catalog repeats them). */
export const TAGS = [
  'survival',
  'smp',
  'creative',
  'minigames',
  'pvp',
  'skyblock',
  'oneblock',
  'roleplay',
  'factions',
  'prison',
  'parkour',
  'adventure',
  'building',
  'economy',
  'modded',
  'anarchy',
  'hardcore',
  'bedwars',
  'towny',
  'vanilla',
] as const;

/** Languages offered as filters (any two-letter code from a link works too). */
export const LANGUAGES = ['fr', 'en', 'de', 'es', 'it', 'pt', 'nl', 'pl', 'ru', 'tr'] as const;

export function languageName(code: string, lang: string): string {
  try {
    const n = new Intl.DisplayNames([lang], { type: 'language' }).of(code);
    return n && n !== code ? n : code.toUpperCase();
  } catch {
    return code.toUpperCase();
  }
}
