import { describe, expect, it } from 'vitest';
import { NAMESPACES, resources } from './index';

function keys(obj: unknown, prefix = ''): string[] {
  if (obj === null || typeof obj !== 'object') return [prefix];
  return Object.entries(obj as Record<string, unknown>).flatMap(([k, v]) =>
    keys(v, prefix ? `${prefix}.${k}` : k),
  );
}

/** i18next plural suffixes differ per language (FR has _many in v21+ rules); normalise them. */
function normalise(list: string[]): string[] {
  return [...new Set(list.map((k) => k.replace(/_(zero|one|two|few|many|other)$/, '')))].sort();
}

function placeholders(s: string): string[] {
  return [...s.matchAll(/\{\{\s*(\w+)[^}]*\}\}/g)].map((m) => m[1] ?? '').sort();
}

describe('locales', () => {
  for (const ns of NAMESPACES) {
    it(`fr and en have the same keys in ${ns}`, () => {
      expect(normalise(keys(resources.fr[ns]))).toEqual(normalise(keys(resources.en[ns])));
    });

    it(`fr and en use the same placeholders in ${ns}`, () => {
      const flat = (o: unknown, p = ''): [string, string][] =>
        o && typeof o === 'object'
          ? Object.entries(o as Record<string, unknown>).flatMap(([k, v]) =>
              flat(v, p ? `${p}.${k}` : k),
            )
          : [[p, String(o)]];
      const fr = new Map(flat(resources.fr[ns]));
      for (const [k, v] of flat(resources.en[ns])) {
        const other = fr.get(k);
        if (other === undefined) continue; // plural forms checked above
        // A singular form may say "the last backup" instead of "the last 1 backup".
        const drop = (l: string[]) => (/_(one|zero)$/.test(k) ? l.filter((p) => p !== 'count') : l);
        expect(drop(placeholders(other)), `${ns}:${k}`).toEqual(drop(placeholders(v)));
      }
    });

    it(`${ns} has no empty strings`, () => {
      const flat = (o: unknown): string[] =>
        o && typeof o === 'object'
          ? Object.values(o as Record<string, unknown>).flatMap(flat)
          : [String(o)];
      for (const lang of ['en', 'fr'] as const) {
        expect(flat(resources[lang][ns]).filter((s) => s.trim() === '')).toEqual([]);
      }
    });
  }
});
