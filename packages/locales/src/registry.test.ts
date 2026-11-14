import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import flags from './flags.json';
import {
  bundleCandidates,
  bundleName,
  codeForTag,
  COMPLETE,
  isLocale,
  LOCALES,
  locale,
  matches,
  negotiate,
  resolveBundle,
  sortedLocales,
  translationStatus,
  validateBundle,
} from './index';

const GAME_I18N = join(__dirname, '../../../client-mod/rustic-companion/core/i18n');

describe('registry', () => {
  it('lists exactly the game languages the companion knows, once each', () => {
    const codes = LOCALES.map((l) => l.code);
    expect(new Set(codes).size).toBe(codes.length);
    expect([...codes].sort()).toEqual([...flags.codes].sort());
    expect(codes).toHaveLength(143);
  });

  it('gives every locale a valid tag, names and a known fallback', () => {
    for (const l of LOCALES) {
      expect(() => Intl.getCanonicalLocales(l.tag), l.code).not.toThrow();
      expect(l.name.trim(), l.code).not.toBe('');
      expect(l.english.trim(), l.code).not.toBe('');
      if (l.fallback) expect(isLocale(l.fallback), `${l.code} → ${l.fallback}`).toBe(true);
    }
  });

  it('marks the right-to-left scripts', () => {
    const rtl = LOCALES.filter((l) => l.dir === 'rtl').map((l) => l.code);
    expect(rtl.sort()).toEqual(['ar_sa', 'fa_ir', 'he_il', 'yi_de', 'zlm_arab']);
  });

  it('has about thirty tier-1 languages', () => {
    const tier1 = new Set(LOCALES.filter((l) => l.tier === 1).map((l) => l.tag.split('-')[0]));
    expect(tier1.size).toBeGreaterThanOrEqual(28);
    expect(tier1.size).toBeLessThanOrEqual(32);
  });

  it('resolves the bundle of a locale through its language and fallback', () => {
    const have = [...COMPLETE];
    expect(resolveBundle('de_at', have)).toBe('de');
    expect(resolveBundle('pt_pt', have)).toBe('pt-PT');
    expect(resolveBundle('pt_br', have)).toBe('pt-BR');
    expect(resolveBundle('bar', have)).toBe('de');
    expect(resolveBundle('nah', have)).toBe('es');
    expect(resolveBundle('cs_cz', have)).toBe('cs');
    expect(resolveBundle('th_th', have)).toBe('th');
    expect(resolveBundle('ovd', have)).toBe('sv');
    expect(resolveBundle('da_dk', have)).toBe('en');
    expect(resolveBundle('ja_jp', have)).toBe('ja');
    // Hong Kong and Classical Chinese read Traditional Chinese, Rusyn Ukrainian, Brabantian Dutch.
    expect(resolveBundle('zh_hk', have)).toBe('zh-TW');
    expect(resolveBundle('lzh', have)).toBe('zh-TW');
    expect(resolveBundle('zh_cn', have)).toBe('zh-CN');
    expect(resolveBundle('ry_ua', have)).toBe('uk');
    expect(resolveBundle('brb', have)).toBe('nl');
    expect(resolveBundle('en_gb', have)).toBe('en');
    expect(bundleCandidates('val_es')).toEqual(['ca-ES-valencia', 'ca-ES', 'ca']);
    expect(translationStatus('fr_ca', have)).toBe('complete');
    expect(translationStatus('en_au', have)).toBe('complete');
    expect(translationStatus('ja_jp', have)).toBe('complete');
    expect(translationStatus('nl_be', have)).toBe('complete');
    expect(translationStatus('cs_cz', have)).toBe('complete');
    expect(translationStatus('da_dk', have)).toBe('none');
    expect(translationStatus('da_dk', [...have, 'da'])).toBe('partial');
    // Dialects and close varieties read another language; joke languages read English.
    expect(translationStatus('bar', have)).toBe('fallback');
    expect(translationStatus('pt_pt', have)).toBe('complete');
    expect(translationStatus('ovd', have)).toBe('fallback');
    expect(translationStatus('esan', have)).toBe('fallback');
    expect(translationStatus('en_pt', have)).toBe('none');
    expect(translationStatus('val_es', have)).toBe('none');
    expect(bundleName('pt-BR')).toBe('Português (Brasil)');
    expect(bundleName('de')).toBe('Deutsch');
    expect(bundleName('pt-PT')).toBe('Português (Portugal)');
    expect(bundleName('th')).toBe('ไทย');
  });

  it('negotiates browser languages', () => {
    const offered = [...COMPLETE];
    expect(negotiate(['de-CH', 'en'], offered)).toBe('de');
    expect(negotiate(['pt-PT'], offered)).toBe('pt-PT');
    expect(negotiate(['pt-AO'], offered)).toBe('pt-BR');
    expect(negotiate(['vi-VN'], offered)).toBe('vi');
    expect(negotiate(['es-MX', 'en'], offered)).toBe('es');
    expect(negotiate(['da', 'fr-FR'], offered)).toBe('fr');
    expect(negotiate(['cs'], offered)).toBe('cs');
    expect(negotiate(['da'], offered)).toBeNull();
    expect(negotiate(['ja'], offered)).toBe('ja');
    expect(negotiate(['zh-HK'], offered)).toBe('zh-TW');
    expect(negotiate(['zh-CN'], offered)).toBe('zh-CN');
    expect(negotiate(['en-GB'], offered)).toBe('en');
    expect(codeForTag('de')).toBe('de_de');
    expect(codeForTag('de-AT')).toBe('de_at');
    expect(codeForTag('pt')).toBe('pt_br');
    expect(codeForTag('zh-TW')).toBe('zh_tw');
    // Every complete bundle names its main locale (the website menu's flag).
    for (const b of COMPLETE) expect(codeForTag(b), b).toMatch(/^[a-z]{2}_[a-z]{2}$/);
    expect(codeForTag('zh-CN')).toBe('zh_cn');
    expect(codeForTag('ja')).toBe('ja_jp');
    expect(codeForTag('en')).toBe('en_us');
  });

  it('searches names, regions, English names and codes without accents', () => {
    const find = (q: string) => LOCALES.filter((l) => matches(l, q)).map((l) => l.code);
    expect(find('francais canada')).toEqual(['fr_ca']);
    expect(find('german')).toContain('de_de');
    expect(find('pt_br')).toEqual(['pt_br']);
    expect(find('japanese')).toEqual(['ja_jp']);
    expect(sortedLocales()[0]?.code).toBe('af_za');
    expect(locale('DE_DE')?.name).toBe('Deutsch');
  });

  it('keeps the in-game tables covering the codes whose fallback lands on them', () => {
    const files = readdirSync(GAME_I18N).filter((f) => f.endsWith('.json'));
    for (const f of files) {
      const table = JSON.parse(readFileSync(join(GAME_I18N, f), 'utf8')) as { covers?: string[] };
      const code = f.replace(/\.json$/, '');
      for (const c of table.covers ?? []) {
        // `pt` (a language) or a game code whose fallback chain reaches this table.
        if (!isLocale(c)) continue;
        const target = locale(c)?.fallback ?? '';
        const reaches = target.startsWith(code) || locale(target)?.fallback?.startsWith(code);
        expect(reaches, `${f} covers ${c}`).toBe(true);
      }
    }
  });
});

describe('validateBundle', () => {
  const en = {
    a: 'Hello {{name}}',
    files_one: '{{count}} file',
    files_other: '{{count}} files',
    list: ['one', 'two'],
  };

  it('accepts a complete Polish bundle and reports what is missing', () => {
    const pl = {
      a: 'Cześć {{name}}',
      files_one: '1 plik',
      files_few: '{{count}} pliki',
      files_many: '{{count}} plików',
      files_other: '{{count}} pliku',
      list: ['jeden', 'dwa'],
    };
    expect(validateBundle(en, pl, 'pl')).toEqual({ issues: [], missing: [], total: 3 });
    expect(validateBundle(en, { a: 'Hallo {{name}}' }, 'de').missing).toEqual(['list', 'files']);
  });

  it('rejects unknown keys, other placeholders and wrong plural forms', () => {
    const de = {
      a: 'Hallo {{user}}',
      b: 'x',
      files_one: '{{count}} Datei',
      files_many: '{{count}} Dateien',
      files_other: '{{count}} Dateien',
      list: ['eins'],
    };
    const { issues } = validateBundle(en, de, 'de');
    expect(issues).toHaveLength(4);
    expect(issues.join('\n')).toMatch(/a: placeholders/);
    expect(issues.join('\n')).toMatch(/files: plural forms many,one,other, de needs one,other/);
    expect(issues.join('\n')).toMatch(/b: not an English key/);
    expect(issues.join('\n')).toMatch(/list: a list of 2/);
  });
});
