import { describe, expect, it } from 'vitest';
import { formatDate, formatDuration, formatMoney, formatRelative, shortVersion } from './format';

describe('format', () => {
  it('formats money per locale', () => {
    expect(formatMoney({ amount_minor: 499, currency: 'EUR' }, 'en')).toBe('€4.99');
    expect(formatMoney({ amount_minor: 499, currency: 'EUR' }, 'fr').replace(/\s/g, ' ')).toBe(
      '4,99 €',
    );
    expect(formatMoney({ amount_minor: 1200, currency: 'CHF' }, 'en')).toContain('12.00');
  });
  it('formats durations', () => {
    expect(formatDuration(45 * 60, 'en')).toBe('45m');
    expect(formatDuration(2 * 3600 + 5 * 60, 'en')).toBe('2h 5m');
    expect(formatDuration(312 * 3600 + 25 * 60, 'en')).toBe('312h');
    // Regression (QA): a 'now' sampled just before the start gave "-1h -1m".
    expect(formatDuration(-10, 'en')).toBe('0m');
    expect(formatDuration(Number.NaN, 'en')).toBe('0m');
  });
  it('formats relative times', () => {
    const now = Date.parse('2026-11-10T12:00:00Z');
    expect(formatRelative('2026-11-10T09:00:00Z', 'en', now)).toBe('3 hours ago');
    expect(formatRelative('2026-11-09T12:00:00Z', 'fr', now)).toBe('hier');
  });
  it('shortens Horizon versions', () => {
    expect(shortVersion('v0.2.14')).toBe('0.2.14');
    expect(shortVersion('nightly-2026-11-08-a84f637')).toBe('Nightly 2026-11-08 · a84f637');
  });
});

describe('format robustness (real data)', () => {
  it('returns an empty string for missing or invalid dates instead of throwing', () => {
    expect(formatRelative('', 'en')).toBe('');
    expect(formatRelative(null, 'en')).toBe('');
    expect(formatRelative('not a date', 'fr')).toBe('');
    expect(formatDate(undefined, 'en')).toBe('');
    expect(formatRelative('2026-11-09', 'en', Date.parse('2026-11-10T00:00:00Z'))).toBe(
      'yesterday',
    );
  });
});
