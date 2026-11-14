import { COMPLETE, LOCALES, translationStatus, validateBundle } from '@rustic/locales';
import { describe, expect, it } from 'vitest';
import { BUNDLES, bundleChain, NAMESPACES, normaliseLanguage, resources } from './index';

type Json = Record<string, unknown>;
const FILES = import.meta.glob<Json>('./locales/*/*.json', { eager: true, import: 'default' });

function bundle(name: string): Partial<Record<string, Json>> {
  const out: Partial<Record<string, Json>> = {};
  for (const [p, res] of Object.entries(FILES)) {
    const [, , b, file] = p.split('/');
    if (b === name && file) out[file.replace(/\.json$/, '')] = res;
  }
  return out;
}

describe('locales', () => {
  it('finds every bundle on disk, English first', () => {
    expect(BUNDLES[0]).toBe('en');
    for (const b of COMPLETE) expect(BUNDLES, b).toContain(b);
  });

  for (const b of BUNDLES.filter((x) => x !== 'en')) {
    const files = bundle(b);
    const complete = (COMPLETE as readonly string[]).includes(b);
    it(`${b} is valid${complete ? ' and complete' : ''}`, () => {
      // Only namespaces English has.
      for (const ns of Object.keys(files)) expect(NAMESPACES, `${b}/${ns}.json`).toContain(ns);
      for (const ns of NAMESPACES) {
        const report = validateBundle(resources.en[ns], files[ns] ?? {}, b);
        expect(report.issues, `${b}/${ns}`).toEqual([]);
        if (complete) expect(report.missing, `${b}/${ns}`).toEqual([]);
      }
    });
  }

  it('English is valid against itself, with no empty string', () => {
    for (const ns of NAMESPACES) {
      expect(validateBundle(resources.en[ns], resources.en[ns], 'en').issues).toEqual([]);
    }
  });

  it('reads old settings and resolves every game locale', () => {
    expect(normaliseLanguage('fr')).toBe('fr_fr');
    expect(normaliseLanguage('en')).toBe('en_us');
    expect(normaliseLanguage('de_AT')).toBe('de_at');
    expect(normaliseLanguage('xx')).toBeNull();
    for (const l of LOCALES) {
      expect(['complete', 'partial', 'fallback', 'none']).toContain(
        translationStatus(l.code, BUNDLES),
      );
    }
    expect(bundleChain('pt_pt')).toEqual(['pt-PT', 'en']);
    expect(bundleChain('pt_br')).toEqual(['pt-BR', 'en']);
    expect(bundleChain('de_ch')).toEqual(['de', 'en']);
    expect(bundleChain('ja_jp')).toEqual(['ja', 'en']);
    // Hong Kong and Classical Chinese read Traditional Chinese; Galician-Ukrainian dialects Ukrainian.
    expect(bundleChain('zh_hk')).toEqual(['zh-TW', 'en']);
    expect(bundleChain('lzh')).toEqual(['zh-TW', 'en']);
    expect(bundleChain('zh_cn')).toEqual(['zh-CN', 'en']);
    expect(bundleChain('nl_be')).toEqual(['nl', 'en']);
    expect(bundleChain('cs_cz')).toEqual(['cs', 'en']);
    expect(bundleChain('th_th')).toEqual(['th', 'en']);
    // Elfdalian reads Swedish; Danish has no translation yet.
    expect(bundleChain('ovd')).toEqual(['sv', 'en']);
    expect(bundleChain('da_dk')).toEqual(['en']);
    expect(bundleChain('en_gb')).toEqual(['en']);
  });
});
