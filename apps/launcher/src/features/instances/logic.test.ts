import { describe, expect, it } from 'vitest';
import { initialInstances } from '@/ipc/mock/data/core';
import type { Instance } from '@/ipc/types/instances';
import {
  importKindOf,
  activeFilterCount,
  buildsInUse,
  compareBuilds,
  filterInstances,
  filterProfiles,
  initials,
  moveItem,
  nativeLineKey,
  nativeLineTone,
  recentlyPlayed,
  sameOrder,
  sortInstances,
  totalPlaytime,
  type ProfileFacts,
} from './logic';

const ids = (l: Instance[]) => l.map((i) => i.id);

const facts: ProfileFacts = {
  isRustic: (v) => v !== 'v0.2.13',
  native: new Set(['inst_essentials']),
};

describe('instances logic', () => {
  it('filters by name, version and modpack', () => {
    expect(ids(filterInstances(initialInstances, 'pvp'))).toEqual(['inst_pvp']);
    expect(ids(filterInstances(initialInstances, 'v0.2.13'))).toEqual(['inst_creative']);
    expect(filterInstances(initialInstances, '  ')).toHaveLength(initialInstances.length);
  });

  it('sorts pinned first, then by the chosen order', () => {
    expect(ids(sortInstances(initialInstances, 'name'))).toEqual([
      'inst_survival',
      'inst_creative',
      'inst_essentials',
      'inst_pvp',
    ]);
    expect(sortInstances(initialInstances, 'playtime')[1]?.id).toBe('inst_creative');
    expect(sortInstances(initialInstances, 'recent')[1]?.id).toBe('inst_pvp');
    // Newest created first: Essentials (12 days), PvP (40 days), Creative (200 days).
    expect(ids(sortInstances(initialInstances, 'created'))).toEqual([
      'inst_survival',
      'inst_essentials',
      'inst_pvp',
      'inst_creative',
    ]);
    // Newest build first, the name breaks ties.
    expect(ids(sortInstances(initialInstances, 'version'))).toEqual([
      'inst_survival',
      'inst_essentials',
      'inst_pvp',
      'inst_creative',
    ]);
  });

  it('orders builds numerically, releases before nightlies', () => {
    expect(
      ['v0.2.9', 'v0.2.15', 'nightly-2026-11-08-a84f637', 'v0.2.14'].sort(compareBuilds),
    ).toEqual(['v0.2.15', 'v0.2.14', 'v0.2.9', 'nightly-2026-11-08-a84f637']);
    expect(buildsInUse(initialInstances)).toEqual(['v0.2.15', 'v0.2.14', 'v0.2.13']);
  });

  it('filters profiles by source, build, edition and native mods', () => {
    expect(ids(filterProfiles(initialInstances, { source: 'modpack' }, facts))).toEqual([
      'inst_essentials',
    ]);
    expect(filterProfiles(initialInstances, { source: 'custom' }, facts)).toHaveLength(3);
    expect(ids(filterProfiles(initialInstances, { version: 'v0.2.14' }, facts))).toEqual([
      'inst_survival',
    ]);
    expect(ids(filterProfiles(initialInstances, { edition: 'horizon' }, facts))).toEqual([
      'inst_creative',
    ]);
    expect(filterProfiles(initialInstances, { edition: 'rustic' }, facts)).toHaveLength(3);
    expect(ids(filterProfiles(initialInstances, { native: true }, facts))).toEqual([
      'inst_essentials',
    ]);
    expect(filterProfiles(initialInstances, { q: 'pvp', edition: 'horizon' }, facts)).toHaveLength(
      0,
    );
    expect(filterProfiles(initialInstances, {}, facts)).toHaveLength(initialInstances.length);
    expect(activeFilterCount({ q: 'x', source: 'custom' })).toBe(0);
    expect(activeFilterCount({ version: 'v0.2.15', edition: 'rustic', native: true })).toBe(3);
  });

  it('offers a recently played row only with enough played profiles', () => {
    expect(ids(recentlyPlayed(initialInstances))).toEqual([
      'inst_survival',
      'inst_pvp',
      'inst_essentials',
    ]);
    expect(recentlyPlayed(initialInstances.slice(0, 3))).toEqual([]);
    const unplayed = initialInstances.map((i) => ({ ...i, last_played_at: null }));
    expect(recentlyPlayed(unplayed)).toEqual([]);
  });

  it('words native mods from the overview and the build task', () => {
    const o = { state: 'not_built' as const, enabled: 1, supported: true, building: false };
    expect(nativeLineKey(undefined, false)).toBeNull();
    expect(nativeLineKey(undefined, true)).toBe('building');
    expect(nativeLineKey({ ...o, building: true }, false)).toBe('building');
    expect(nativeLineKey(o, false)).toBe('not_built');
    expect(nativeLineKey({ ...o, state: 'up_to_date' }, false)).toBe('up_to_date');
    expect(nativeLineKey({ ...o, state: 'needs_rebuild' }, false)).toBe('needs_rebuild');
    expect(nativeLineKey({ ...o, enabled: 0 }, false)).toBe('off');
    expect(nativeLineKey({ ...o, state: 'not_needed' }, false)).toBe('off');
    expect(nativeLineKey({ ...o, supported: false }, false)).toBe('unsupported');
    expect(nativeLineTone('building')).toBe('accent');
    expect(nativeLineTone('up_to_date')).toBe('success');
    expect(nativeLineTone('off')).toBe('neutral');
    expect(nativeLineTone('not_built')).toBe('warning');
    expect(nativeLineTone('unsupported')).toBe('warning');
  });

  it('adds play time', () => {
    expect(totalPlaytime(initialInstances)).toBe(
      initialInstances.reduce((s, i) => s + i.playtime_s, 0),
    );
    expect(totalPlaytime([])).toBe(0);
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

describe('importKindOf', () => {
  it('reads packs by extension, whatever the case or the separators', () => {
    expect(importKindOf('C:\\Users\\me\\Downloads\\Pack.MRPACK')).toBe('mrpack');
    expect(importKindOf('C:\\Users\\me\\Packs\\Old\\')).toBe('folder');
    expect(importKindOf('/home/me/Fabulous 5.4.1.zip')).toBe('curseforge_zip');
  });
  it('reads a path without a file extension as an instance folder', () => {
    expect(importKindOf('C:/Prism/instances/Survival')).toBe('folder');
    expect(importKindOf('/home/me/instances/Optimized 5.4.1/')).toBe('folder');
  });
  it('refuses other files', () => {
    expect(importKindOf('C:/Downloads/mod.jar')).toBeNull();
    expect(importKindOf('notes.txt')).toBeNull();
    expect(importKindOf('')).toBeNull();
  });
});
