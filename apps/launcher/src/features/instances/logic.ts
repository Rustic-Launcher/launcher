/** Pure helpers for the instances area (unit-tested). */
import type { Instance } from '@/ipc/types/instances';

export type InstanceSort = 'recent' | 'name' | 'playtime';

export function filterInstances(list: Instance[], query: string): Instance[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  return list.filter(
    (i) =>
      i.name.toLowerCase().includes(q) ||
      i.horizon_version.toLowerCase().includes(q) ||
      (i.source?.name.toLowerCase().includes(q) ?? false),
  );
}

/** Pinned first, then by the chosen order. */
export function sortInstances(list: Instance[], sort: InstanceSort): Instance[] {
  const by = (a: Instance, b: Instance): number => {
    switch (sort) {
      case 'name':
        return a.name.localeCompare(b.name);
      case 'playtime':
        return b.playtime_s - a.playtime_s;
      default:
        return (b.last_played_at ?? '').localeCompare(a.last_played_at ?? '');
    }
  };
  return [...list].sort((a, b) => Number(b.pinned) - Number(a.pinned) || by(a, b));
}

/** Moves one item; returns a new array (clamped indices). */
export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
  const next = [...list];
  if (from < 0 || from >= next.length) return next;
  const target = Math.max(0, Math.min(next.length - 1, to));
  const [item] = next.splice(from, 1);
  next.splice(target, 0, item as T);
  return next;
}

export function sameOrder(a: readonly { id: string }[], b: readonly { id: string }[]): boolean {
  return a.length === b.length && a.every((x, i) => x.id === b[i]?.id);
}

/** Initials for a content tile without an icon. */
export function initials(name: string): string {
  const words = name.split(/\s+/).filter(Boolean);
  return ((words[0]?.[0] ?? '') + (words[1]?.[0] ?? words[0]?.[1] ?? '')).toUpperCase();
}
