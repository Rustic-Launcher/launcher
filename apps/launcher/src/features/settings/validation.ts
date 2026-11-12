/** Pure validation helpers for Settings and Onboarding forms. */

export type PinError = 'format' | 'mismatch' | 'weak';

/** Parental PIN: 4-6 digits, not all identical, not a plain sequence, confirmed. */
export function validatePin(pin: string, confirm: string): PinError | null {
  if (!/^\d{4,6}$/.test(pin)) return 'format';
  if (/^(\d)\1+$/.test(pin) || '0123456789'.includes(pin) || '9876543210'.includes(pin))
    return 'weak';
  if (pin !== confirm) return 'mismatch';
  return null;
}

/**
 * Parses a money amount typed by the user ("15", "15,50", "15.5") into minor units.
 * Returns null for empty input (no limit) and NaN for invalid input.
 */
export function parseAmountMinor(input: string): number | null {
  const v = input.trim().replace(/\s/g, '').replace(',', '.');
  if (!v) return null;
  if (!/^\d+(\.\d{1,2})?$/.test(v)) return Number.NaN;
  return Math.round(Number.parseFloat(v) * 100);
}

/** Rounds a RAM value to the slider step and clamps it to the allowed range. */
export function clampMemory(mb: number, totalMb: number, step = 512): number {
  const max = Math.max(1024, Math.floor((totalMb * 0.9) / step) * step);
  return Math.min(max, Math.max(1024, Math.round(mb / step) * step));
}

/** Upper bound offered by the RAM slider: 90 % of physical memory, rounded to the step. */
export function memoryMax(totalMb: number, step = 512): number {
  return Math.max(2048, Math.floor((totalMb * 0.9) / step) * step);
}

/** Over this amount we warn: the OS and launcher need headroom. */
export function memoryWarnAbove(totalMb: number): number {
  return Math.floor(totalMb * 0.6);
}

export type FailureReason =
  | 'expired'
  | 'denied'
  | 'no_ownership'
  | 'not_configured'
  | 'not_approved'
  | 'rate_limited'
  | 'network'
  | 'xbox'
  | 'error';

/** The serialised `IpcError` of a failed sign-in: `details.reason` is the auth error's stable code. */
type LoginError = { code?: string; message?: string; details?: { reason?: unknown } } | null;

const BY_REASON: Record<string, FailureReason> = {
  'auth.not_configured': 'not_configured',
  'auth.not_approved': 'not_approved',
  'auth.rate_limited': 'rate_limited',
  'auth.network': 'network',
  'auth.xbox': 'xbox',
  'auth.declined': 'denied',
  'auth.cancelled': 'denied',
  'auth.expired': 'expired',
  'auth.timeout': 'expired',
  'auth.not_owned': 'no_ownership',
  'auth.no_profile': 'no_ownership',
};

/**
 * Maps a start/poll failure to what the user is told. The auth error's own code (`details.reason`)
 * wins; the coarse IPC code is the fallback (the build may lack an Azure client id).
 */
export function loginFailureReason(e: unknown): FailureReason {
  const err = e as LoginError;
  const reason = err?.details?.reason;
  if (typeof reason === 'string') return BY_REASON[reason] ?? 'error';
  if (err?.code === 'unavailable') return 'not_configured';
  if (err?.code === 'network') return 'network';
  return 'error';
}

/** The technical message of a failed sign-in, shown under the explanation (never a token). */
export function loginFailureDetail(e: unknown): string | undefined {
  const message = (e as LoginError)?.message;
  return typeof message === 'string' && message.trim() !== '' ? message : undefined;
}
