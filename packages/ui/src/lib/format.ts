/** Formats a byte count with binary units, e.g. 1.5 GB. */
export function formatBytes(bytes: number, locale = 'en', digits = 1): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: i === 0 ? 0 : digits }).format(v)} ${units[i]}`;
}

/** Clamps a number between min and max. */
export function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}
