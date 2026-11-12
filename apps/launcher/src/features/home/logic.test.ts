import { describe, expect, it } from 'vitest';
import type { World } from '@/ipc/types/instances';
import type { MultiplayerEntry } from '@/ipc/types/multiplayer';
import { coverGradient, recentPlaces } from './logic';

const mp = (id: string, at: string | null): MultiplayerEntry => ({
  id,
  name: id,
  address: `${id}.example`,
  favorite: false,
  last_joined_at: at,
});
const world = (id: string, at: string | null): World => ({
  id,
  name: id,
  mode: 'survival',
  last_played_at: at,
  size_bytes: 1,
  datapack_count: 0,
});

describe('recentPlaces', () => {
  it('orders servers and worlds by recency', () => {
    const r = recentPlaces(
      [mp('a', '2026-11-09'), mp('b', '2026-11-01')],
      [world('w', '2026-11-05')],
      4,
    );
    expect(r.map((p) => (p.kind === 'server' ? p.entry.id : p.world.id))).toEqual(['a', 'w', 'b']);
  });
  it('keeps at least one world when there are many servers', () => {
    const servers = ['a', 'b', 'c', 'd', 'e'].map((id, i) => mp(id, `2026-11-0${9 - i}`));
    const r = recentPlaces(servers, [world('w', '2026-10-01')], 4);
    expect(r).toHaveLength(4);
    expect(r[3]?.kind).toBe('world');
  });
  it('skips never-joined entries', () => {
    expect(recentPlaces([mp('a', null)], [world('w', null)], 4)).toEqual([]);
  });
});

describe('coverGradient', () => {
  it('builds a deterministic gradient', () => {
    expect(coverGradient(22)).toContain('hsl(22 62% 38%)');
    expect(coverGradient(340)).toContain('hsl(18 55% 22%)');
  });
});
