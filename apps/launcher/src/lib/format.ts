/** Locale-aware formatting helpers (pure; pass the i18n language). */
import type { Money } from '@/ipc/types/dns';

export function formatNumber(n: number, lang: string, opts?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(lang, opts).format(n);
}

export function formatCompact(n: number, lang: string): string {
  return new Intl.NumberFormat(lang, { notation: 'compact', maximumFractionDigits: 1 }).format(n);
}

export function formatMoney(m: Money, lang: string): string {
  return new Intl.NumberFormat(lang, { style: 'currency', currency: m.currency }).format(
    m.amount_minor / 100,
  );
}

export function formatPercent(p: number, lang: string): string {
  return new Intl.NumberFormat(lang, { style: 'percent', maximumFractionDigits: 0 }).format(p);
}

/** "312 h 25 min" style durations. */
export function formatDuration(seconds: number, lang: string): string {
  // A clock read a little before the start (a ticking `now`) must not render "-1h -1m".
  seconds = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const unit = (value: number, u: 'hour' | 'minute') =>
    new Intl.NumberFormat(lang, { style: 'unit', unit: u, unitDisplay: 'narrow' }).format(value);
  if (h === 0) return unit(m, 'minute');
  if (m === 0 || h >= 100) return unit(h, 'hour');
  return `${unit(h, 'hour')} ${unit(m, 'minute')}`;
}

const DIVISIONS: [number, Intl.RelativeTimeFormatUnit][] = [
  [60, 'second'],
  [60, 'minute'],
  [24, 'hour'],
  [7, 'day'],
  [4.34524, 'week'],
  [12, 'month'],
  [Number.POSITIVE_INFINITY, 'year'],
];

/** "il y a 3 heures" / "3 hours ago". */
export function formatRelative(
  iso: string | null | undefined,
  lang: string,
  now = Date.now(),
): string {
  const at = iso ? new Date(iso).getTime() : Number.NaN;
  // Real sources sometimes send no or odd dates: never let one break a page.
  if (!Number.isFinite(at)) return '';
  let duration = (at - now) / 1000;
  const rtf = new Intl.RelativeTimeFormat(lang, { numeric: 'auto' });
  for (const [amount, unit] of DIVISIONS) {
    if (Math.abs(duration) < amount) return rtf.format(Math.round(duration), unit);
    duration /= amount;
  }
  return rtf.format(Math.round(duration), 'year');
}

export function formatDate(
  iso: string | null | undefined,
  lang: string,
  opts: Intl.DateTimeFormatOptions = { dateStyle: 'medium' },
): string {
  const d = new Date(iso ?? Number.NaN);
  if (!Number.isFinite(d.getTime())) return '';
  return new Intl.DateTimeFormat(lang, opts).format(d);
}

export function formatSpeed(bps: number, lang: string): string {
  const mb = bps / 1024 ** 2;
  return `${new Intl.NumberFormat(lang, { maximumFractionDigits: 1 }).format(mb)} MB/s`;
}

export function formatMemory(mb: number, lang: string): string {
  return `${new Intl.NumberFormat(lang, { maximumFractionDigits: 1 }).format(mb / 1024)} GB`;
}

/** "0.2.14" from "v0.2.14"; nightlies keep their date + commit. */
export function shortVersion(id: string): string {
  if (id.startsWith('nightly-')) {
    const [, y, m, d, commit] = id.split('-');
    return `Nightly ${y}-${m}-${d} · ${commit ?? ''}`.trim();
  }
  return id.replace(/^v/, '');
}
