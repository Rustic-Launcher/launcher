/** Pure helpers for Home (unit-tested). */
import type { World } from '@/ipc/types/instances';
import type { MultiplayerEntry } from '@/ipc/types/multiplayer';

export type Place =
  | { kind: 'server'; entry: MultiplayerEntry; at: string }
  | { kind: 'world'; world: World; at: string };

/**
 * Recent servers and worlds, most recent first: up to `limit` items, always at least one world
 * when the instance has any (so solo players see their save).
 */
export function recentPlaces(servers: MultiplayerEntry[], worlds: World[], limit: number): Place[] {
  const s: Place[] = servers
    .filter((e) => e.last_joined_at)
    .map((entry) => ({ kind: 'server', entry, at: entry.last_joined_at! }));
  const w: Place[] = worlds
    .filter((x) => x.last_played_at)
    .map((world) => ({ kind: 'world', world, at: world.last_played_at! }));
  const all = [...s, ...w].sort((a, b) => b.at.localeCompare(a.at));
  const top = all.slice(0, limit);
  if (w.length > 0 && !top.some((p) => p.kind === 'world')) {
    top[top.length - 1] = w.sort((a, b) => b.at.localeCompare(a.at))[0]!;
  }
  return top;
}

/** Cover gradient for news/servers/store tiles from a hue (original art, no images). */
export function coverGradient(hue: number): string {
  const h2 = (hue + 38) % 360;
  return `linear-gradient(135deg, hsl(${hue} 62% 38%) 0%, hsl(${h2} 55% 22%) 55%, hsl(${h2} 40% 12%) 100%)`;
}
