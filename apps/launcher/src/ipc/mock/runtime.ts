/** Mock runtime shared by every mock handler: event bus, latency, ids, persistence helpers. */
import type { CommandArgs, CommandName, CommandResult, EventName, IpcEvents } from '../commands';
import { IpcFailure } from '../client';
import type { IpcError } from '../types/app';

type Handler = (payload: never) => void;
const listeners = new Map<string, Set<Handler>>();

export function emit<E extends EventName>(event: E, payload: IpcEvents[E]): void {
  const set = listeners.get(event);
  if (!set) return;
  for (const h of [...set]) (h as (p: IpcEvents[E]) => void)(payload);
}

export function on<E extends EventName>(event: E, handler: (p: IpcEvents[E]) => void): () => void {
  let set = listeners.get(event);
  if (!set) {
    set = new Set();
    listeners.set(event, set);
  }
  set.add(handler as Handler);
  return () => set.delete(handler as Handler);
}

function readFlag(name: string): string | null {
  try {
    const url = new URL(window.location.href);
    return url.searchParams.get(name) ?? window.localStorage.getItem(`rustic.mock.${name}`);
  } catch {
    return null;
  }
}

/** Base latency in ms (`?latency=0` or localStorage `rustic.mock.latency`). */
export const LATENCY = Number(readFlag('latency') ?? 220);
/** Speed multiplier for simulated long tasks (downloads, installs). */
export const SPEED = Number(readFlag('speed') ?? 1);

export function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

export function latency(factor = 1): Promise<void> {
  if (LATENCY <= 0) return Promise.resolve();
  return sleep(LATENCY * factor * (0.6 + Math.random() * 0.8));
}

let seq = 1000;
export function uid(prefix: string): string {
  seq += 1;
  return `${prefix}_${seq.toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

export function fail(
  code: IpcError['code'],
  message: string,
  details?: Record<string, unknown>,
): never {
  throw new IpcFailure({ code, message, details });
}

export const nowIso = (): string => new Date().toISOString();
export const minutesAgo = (m: number): string => new Date(Date.now() - m * 60_000).toISOString();
export const daysAgo = (d: number): string => new Date(Date.now() - d * 86_400_000).toISOString();

export function persistGet<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(`rustic.mock.${key}`);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

export function persistSet(key: string, value: unknown): void {
  try {
    window.localStorage.setItem(`rustic.mock.${key}`, JSON.stringify(value));
  } catch {
    /* ignore */
  }
}

/**
 * Runs a simulated progress task: calls `tick(p)` with p in 0..1 until done or cancelled.
 * Returns a cancel function.
 */
export function simulate(
  durationMs: number,
  tick: (p: number) => void,
  done: () => void,
  stepMs = 120,
): () => void {
  const total = Math.max(1, durationMs / SPEED);
  const start = Date.now();
  let cancelled = false;
  const loop = () => {
    if (cancelled) return;
    const p = Math.min(1, (Date.now() - start) / total);
    tick(p);
    if (p >= 1) done();
    else setTimeout(loop, stepMs);
  };
  setTimeout(loop, stepMs);
  return () => {
    cancelled = true;
  };
}

export type MockHandler<K extends CommandName> = (
  args: CommandArgs<K>,
) => CommandResult<K> | Promise<CommandResult<K>>;

export type MockHandlers = { [K in CommandName]?: MockHandler<K> };
