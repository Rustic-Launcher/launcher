import { describe, expect, it } from 'vitest';
import type { Cosmetic } from '@/cloud/types';
import {
  applyAll,
  emoteAnimation,
  equip,
  equippedCount,
  isEquipped,
  ownedOnly,
  sameLoadout,
  SLOTS,
  toggle,
  unequip,
  visualsFor,
} from './loadout';

const cos = (id: string, slot: Cosmetic['slot'], owned = true, shape = 'x'): Cosmetic => ({
  id,
  name: id,
  description: '',
  slot,
  rarity: 'common',
  visual: { slot, shape, color: '#fff' },
  owned,
  acquired_at: null,
  source: null,
});

const hat = cos('hat', 'head');
const hat2 = cos('hat2', 'head');
const pack = cos('pack', 'back');
const wave = cos('wave', 'emote', true, 'wave');
const tag = cos('tag', 'nametag');

describe('loadout helpers', () => {
  it('has no cape slot', () => {
    expect(SLOTS as string[]).not.toContain('cape');
    expect(SLOTS).toHaveLength(9);
  });

  it('equips one item per slot', () => {
    let l = equip({}, hat);
    expect(isEquipped(l, hat)).toBe(true);
    l = equip(l, hat2);
    expect(isEquipped(l, hat)).toBe(false);
    expect(l.head).toBe('hat2');
  });

  it('toggles and unequips', () => {
    const l = toggle({}, pack);
    expect(l.back).toBe('pack');
    expect(toggle(l, pack).back).toBeNull();
    expect(unequip(l, 'back').back).toBeNull();
  });

  it('does not mutate the input', () => {
    const l = { head: 'hat' };
    equip(l, hat2);
    expect(l.head).toBe('hat');
  });

  it('applies several items (product preview)', () => {
    expect(applyAll({ head: 'hat' }, [hat2, pack])).toMatchObject({ head: 'hat2', back: 'pack' });
  });

  it('keeps only owned items of the right slot', () => {
    const notOwned = cos('crown', 'head', false);
    const all = [hat, pack, notOwned];
    expect(ownedOnly({ head: 'crown', back: 'pack' }, all)).toEqual({ back: 'pack' });
    expect(ownedOnly({ back: 'hat' }, all)).toEqual({});
  });

  it('builds 3D visuals without emotes and nametags', () => {
    const v = visualsFor({ head: 'hat', emote: 'wave', nametag: 'tag' }, [hat, wave, tag]);
    expect(v.map((x) => x.slot)).toEqual(['head']);
  });

  it('maps emotes to animations', () => {
    expect(emoteAnimation(wave)).toBe('wave');
    expect(emoteAnimation(cos('e', 'emote', true, 'unknown'))).toBeNull();
    expect(emoteAnimation(undefined)).toBeNull();
  });

  it('compares and counts loadouts', () => {
    expect(sameLoadout({ head: 'hat', back: null }, { head: 'hat' })).toBe(true);
    expect(sameLoadout({ head: 'hat' }, { head: 'hat2' })).toBe(false);
    expect(equippedCount({ head: 'hat', back: null, emote: 'wave' })).toBe(2);
  });
});
