/**
 * Checks a translation bundle against its English source (ADR 0161, docs/TRANSLATING.md): known
 * keys only, the same placeholders, arrays of the same length, no empty string, and exactly the
 * plural categories of the language (`Intl.PluralRules`, i18next v4 suffixes `_one`, `_few`...).
 * Self-contained (no import), so `scripts/i18n/i18n.mjs` runs it with Node's type stripping too.
 */

export type Tree = { [key: string]: unknown };

export interface BundleReport {
  /** Problems that make the bundle wrong (whatever its coverage). */
  issues: string[];
  /** English keys the bundle lacks (plural keys counted once, by their base). */
  missing: string[];
  /** English keys, plural ones counted once. */
  total: number;
}

export interface ValidateOptions {
  /** Placeholder syntax: `{{name}}` (i18next, the default) or `{name}` (in game). */
  placeholders?: 'i18next' | 'braces';
  /** Keys that are not texts to translate (in-game `covers`, `language`...), by prefix. */
  ignore?: string[];
}

const PLURAL = /_(zero|one|two|few|many|other)$/;

/** Every leaf of a tree as `a.b.c` → value; arrays are leaves too (`list` → `['a','b']`). */
export function flatten(tree: unknown, prefix = '', out = new Map<string, unknown>()) {
  if (Array.isArray(tree)) {
    out.set(prefix, tree);
    tree.forEach((v, i) => flatten(v, prefix ? `${prefix}.${i}` : String(i), out));
  } else if (tree && typeof tree === 'object') {
    for (const [k, v] of Object.entries(tree as Tree))
      flatten(v, prefix ? `${prefix}.${k}` : k, out);
  } else if (prefix) {
    out.set(prefix, tree);
  }
  return out;
}

/** The plural categories of `lang` (`pl` → few, many, one, other), sorted. */
export function pluralCategories(lang: string): string[] {
  try {
    return [...new Intl.PluralRules(lang).resolvedOptions().pluralCategories].sort();
  } catch {
    return ['one', 'other'];
  }
}

function names(text: string, kind: 'i18next' | 'braces'): string[] {
  const re = kind === 'i18next' ? /\{\{\s*([\w.]+)[^}]*\}\}/g : /\{(\w+)\}/g;
  return [...text.matchAll(re)].map((m) => m[1] ?? '').sort();
}

const same = (a: string[], b: string[]) => a.length === b.length && a.every((x, i) => x === b[i]);

/** Checks `bundle` (in language `lang`) against `english`. */
export function validateBundle(
  english: unknown,
  bundle: unknown,
  lang: string,
  options: ValidateOptions = {},
): BundleReport {
  const kind = options.placeholders ?? 'i18next';
  const ignored = (k: string) =>
    (options.ignore ?? []).some((p) => k === p || k.startsWith(`${p}.`));
  const en = flatten(english);
  const tr = flatten(bundle);
  const issues: string[] = [];
  const missing: string[] = [];
  const cats = pluralCategories(lang);

  // English plural keys by base, with their categories.
  const enPlurals = new Map<string, Set<string>>();
  for (const k of en.keys()) {
    const m = PLURAL.exec(k);
    if (m) {
      const b = k.slice(0, m.index);
      enPlurals.set(b, (enPlurals.get(b) ?? new Set()).add(m[1]!));
    }
  }

  let total = 0;
  for (const [k, v] of en) {
    if (ignored(k) || PLURAL.test(k)) continue;
    // Items of an array are checked with the array.
    if (/\.\d+$/.test(k) && Array.isArray(en.get(k.replace(/\.\d+$/, '')))) {
      const t = tr.get(k);
      if (t !== undefined && typeof v === 'string' && typeof t === 'string') {
        if (!t.trim()) issues.push(`${k}: empty`);
        else if (!same(names(t, kind), names(v, kind))) issues.push(`${k}: placeholders differ`);
      }
      continue;
    }
    total++;
    const t = tr.get(k);
    if (t === undefined) {
      missing.push(k);
      continue;
    }
    if (Array.isArray(v)) {
      if (!Array.isArray(t) || t.length !== v.length)
        issues.push(`${k}: a list of ${v.length} items in English`);
      continue;
    }
    if (typeof t !== 'string' || !t.trim()) {
      issues.push(`${k}: empty or not a text`);
      continue;
    }
    if (!same(names(t, kind), names(String(v), kind)))
      issues.push(
        `${k}: placeholders ${names(t, kind).join(',') || 'none'} ≠ English ${names(String(v), kind).join(',') || 'none'}`,
      );
  }

  for (const [b, enCats] of enPlurals) {
    if (ignored(b)) continue;
    total++;
    const want = [...new Set([...cats, ...(enCats.has('zero') ? ['zero'] : [])])].sort();
    const have = cats
      .concat(['zero', 'one', 'two', 'few', 'many', 'other'])
      .filter((c, i, a) => a.indexOf(c) === i && tr.has(`${b}_${c}`))
      .sort();
    if (!have.length) {
      missing.push(b);
      continue;
    }
    if (!same(have, want))
      issues.push(`${b}: plural forms ${have.join(',')}, ${lang} needs ${want.join(',')}`);
    const enOther = names(String(en.get(`${b}_other`) ?? ''), kind);
    for (const c of have) {
      const got = names(String(tr.get(`${b}_${c}`) ?? ''), kind);
      const extra = got.filter((n) => !enOther.includes(n));
      // A singular form may say "the last backup" rather than "the last 1 backup".
      const optional = c === 'one' || c === 'zero' || c === 'two' ? ['count', 'n'] : [];
      const lacking = enOther.filter((n) => !got.includes(n) && !optional.includes(n));
      if (extra.length || lacking.length)
        issues.push(`${b}_${c}: placeholders differ from English`);
      if (!String(tr.get(`${b}_${c}`) ?? '').trim()) issues.push(`${b}_${c}: empty`);
    }
  }

  for (const k of tr.keys()) {
    if (ignored(k)) continue;
    const m = PLURAL.exec(k);
    const known = en.has(k) || (m && enPlurals.has(k.slice(0, m.index)));
    if (!known) issues.push(`${k}: not an English key`);
  }
  return { issues, missing, total };
}
