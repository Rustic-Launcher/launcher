/**
 * Proximity voice (ADR 0176): how loud and from which side a speaker sounds, from the two
 * players' positions in game. Pure functions; the engine applies them to each speaker's
 * WebAudio gain and stereo panner.
 *
 * Positions come from the companion's `rustic-voice` lines (our own) and from other launchers'
 * data messages (anyone in the room): everything is validated and clamped here.
 */

export const DEFAULT_RANGE = 48;
export const MIN_RANGE = 8;
export const MAX_RANGE = 128;

/** Where a player is: the server's host as the game connected to it, the dimension, the feet. */
export interface GamePosition {
  server: string;
  dim: string;
  x: number;
  y: number;
  z: number;
  /** Degrees, the game's convention: 0 faces +Z (south), 90 faces -X (west). */
  yaw: number;
}

const COORD_LIMIT = 30_000_000;

function finite(v: unknown, limit: number): number | null {
  return typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= limit ? v : null;
}

function label(v: unknown, max: number): string | null {
  if (typeof v !== 'string') return null;
  const s = v.trim().toLowerCase();
  return s && s.length <= max && /^[a-z0-9._:\-[\]/]+$/.test(s) ? s : null;
}

/** A position from untrusted JSON, or null. */
export function sanitizePosition(v: unknown): GamePosition | null {
  if (!v || typeof v !== 'object') return null;
  const o = v as Record<string, unknown>;
  const server = label(o.server, 255);
  const dim = label(o.dim, 128);
  const x = finite(o.x, COORD_LIMIT);
  const y = finite(o.y, 100_000);
  const z = finite(o.z, COORD_LIMIT);
  const yaw = finite(o.yaw, 1e6) ?? 0;
  if (!server || !dim || x === null || y === null || z === null) return null;
  return { server, dim, x, y, z, yaw: ((yaw % 360) + 360) % 360 };
}

export function clampRange(r: number): number {
  return Math.min(MAX_RANGE, Math.max(MIN_RANGE, Math.round(Number.isFinite(r) ? r : 48)));
}

export function sameWorld(a: GamePosition, b: GamePosition): boolean {
  return a.server === b.server && a.dim === b.dim;
}

export function distance(a: GamePosition, b: GamePosition): number {
  return Math.hypot(a.x - b.x, a.y - b.y, a.z - b.z);
}

/**
 * Volume by distance: full within the first 15 % of the range, then a smooth fade (quadratic)
 * to silence at the range, as the game's own sounds fade.
 */
export function proximityGain(d: number, range: number): number {
  const r = clampRange(range);
  const near = r * 0.15;
  if (!(d >= 0)) return 0;
  if (d <= near) return 1;
  if (d >= r) return 0;
  const t = (d - near) / (r - near);
  return (1 - t) * (1 - t);
}

/** -0.8 (left) to 0.8 (right): where `other` stands for `self` looking along its yaw. */
export function stereoPan(self: GamePosition, other: GamePosition): number {
  const dx = other.x - self.x;
  const dz = other.z - self.z;
  const len = Math.hypot(dx, dz);
  if (len < 0.5) return 0;
  const yaw = (self.yaw * Math.PI) / 180;
  // Facing (-sin yaw, cos yaw); the right hand is (-cos yaw, -sin yaw).
  const right = (dx * -Math.cos(yaw) + dz * -Math.sin(yaw)) / len;
  return Math.max(-0.8, Math.min(0.8, right * 0.8));
}

export interface MixInput {
  /** The room uses proximity (always for a server room; the party's setting otherwise). */
  proximity: boolean;
  /** A server room: no shared position means silence. A party room: full volume then. */
  strict: boolean;
  range: number;
  self: GamePosition | null;
  other: GamePosition | null;
}

/** The gain (0-1, before the listener's own volume for that speaker) and pan of a speaker. */
export function mix(i: MixInput): { gain: number; pan: number } {
  if (!i.proximity) return { gain: 1, pan: 0 };
  if (!i.self || !i.other || !sameWorld(i.self, i.other)) {
    return i.strict ? { gain: 0, pan: 0 } : { gain: 1, pan: 0 };
  }
  const gain = proximityGain(distance(i.self, i.other), i.range);
  return { gain, pan: gain > 0 ? stereoPan(i.self, i.other) : 0 };
}

/** The data message a launcher sends the room with its player's position (topic `pos`). */
export const POSITION_TOPIC = 'pos';

export function encodePosition(p: GamePosition | null): Uint8Array<ArrayBuffer> {
  return new TextEncoder().encode(JSON.stringify({ v: 1, p }));
}

/** `undefined`: not a valid message; `null`: the player left the world. */
export function decodePosition(bytes: Uint8Array): GamePosition | null | undefined {
  if (bytes.byteLength > 1024) return undefined;
  try {
    const m = JSON.parse(new TextDecoder().decode(bytes)) as { v?: unknown; p?: unknown };
    if (m.v !== 1) return undefined;
    if (m.p === null) return null;
    return sanitizePosition(m.p) ?? undefined;
  } catch {
    return undefined;
  }
}

/** Whether a position changed enough to be sent again (or a second has passed). */
export function moved(a: GamePosition | null, b: GamePosition | null): boolean {
  if (!a || !b) return a !== b;
  return (
    !sameWorld(a, b) || distance(a, b) > 0.25 || Math.abs(((a.yaw - b.yaw + 540) % 360) - 180) > 5
  );
}
