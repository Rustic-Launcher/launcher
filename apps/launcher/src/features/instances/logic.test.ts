import { describe, expect, it } from 'vitest';
import { initialInstances } from '@/ipc/mock/data/core';
import { filterInstances, initials, moveItem, sameOrder, sortInstances } from './logic';

describe('instances logic', () => {
  it('filters by name, version and modpack', () => {
    expect(filterInstances(initialInstances, 'pvp').map((i) => i.id)).toEqual(['inst_pvp']);
    expect(filterInstances(initialInstances, 'v0.2.13').map((i) => i.id)).toEqual([
      'inst_creative',
    ]);
    expect(filterInstances(initialInstances, '  ')).toHaveLength(initialInstances.length);
  });

  it('sorts pinned first, then by the chosen order', () => {
    expect(sortInstances(initialInstances, 'name').map((i) => i.id)).toEqual([
      'inst_survival',
      'inst_creative',
      'inst_essentials',
      'inst_pvp',
    ]);
    expect(sortInstances(initialInstances, 'playtime')[1]?.id).toBe('inst_creative');
    expect(sortInstances(initialInstances, 'recent')[1]?.id).toBe('inst_pvp');
  });

  it('moves items and clamps', () => {
    expect(moveItem(['a', 'b', 'c'], 0, 2)).toEqual(['b', 'c', 'a']);
    expect(moveItem(['a', 'b', 'c'], 2, -5)).toEqual(['c', 'a', 'b']);
    expect(moveItem(['a', 'b'], 5, 0)).toEqual(['a', 'b']);
  });

  it('compares orders', () => {
    expect(sameOrder([{ id: 'a' }], [{ id: 'a' }])).toBe(true);
    expect(sameOrder([{ id: 'a' }, { id: 'b' }], [{ id: 'b' }, { id: 'a' }])).toBe(false);
  });

  it('makes initials', () => {
    expect(initials('Atlas Minimap')).toBe('AM');
    expect(initials('Lumen')).toBe('LU');
  });
});
