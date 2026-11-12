/** Pure helpers for loadouts (what is equipped per slot). */
import type { CosmeticVisual, PlayerAnimation } from '@rustic/ui/three';
import type { Cosmetic, CosmeticSlot, Loadout } from '@/cloud/types';

/** Order of the Locker categories. No capes: they are Mojang's (see docs/LEGAL.md). */
export const SLOTS: CosmeticSlot[] = [
  'head',
  'back',
  'top',
  'shoulder',
  'companion',
  'trail',
  'aura',
  'emote',
  'nametag',
];

export function isEquipped(
  loadout: Loadout | undefined,
  c: Pick<Cosmetic, 'id' | 'slot'>,
): boolean {
  return !!loadout && loadout[c.slot] === c.id;
}

export function equip(loadout: Loadout, c: Pick<Cosmetic, 'id' | 'slot'>): Loadout {
  return { ...loadout, [c.slot]: c.id };
}

export function unequip(loadout: Loadout, slot: CosmeticSlot): Loadout {
  return { ...loadout, [slot]: null };
}

/** Equip, or unequip when it is already equipped. */
export function toggle(loadout: Loadout, c: Pick<Cosmetic, 'id' | 'slot'>): Loadout {
  return isEquipped(loadout, c) ? unequip(loadout, c.slot) : equip(loadout, c);
}

/** Apply several cosmetics on top of a loadout (e.g. a product preview). */
export function applyAll(loadout: Loadout, items: Pick<Cosmetic, 'id' | 'slot'>[]): Loadout {
  return items.reduce<Loadout>((l, c) => equip(l, c), { ...loadout });
}

/** Keep only entries the user owns (outfits saved before an item was removed, previews...). */
export function ownedOnly(loadout: Loadout, cosmetics: Cosmetic[]): Loadout {
  const out: Loadout = {};
  for (const slot of SLOTS) {
    const id = loadout[slot];
    if (id && cosmetics.some((c) => c.id === id && c.owned && c.slot === slot)) out[slot] = id;
  }
  return out;
}

function byId(cosmetics: Cosmetic[], id: string | null | undefined): Cosmetic | undefined {
  return id ? cosmetics.find((c) => c.id === id) : undefined;
}

/** 3D visuals for the viewer (emotes and nametags are not meshes). */
export function visualsFor(loadout: Loadout, cosmetics: Cosmetic[]): CosmeticVisual[] {
  return SLOTS.filter((s) => s !== 'emote' && s !== 'nametag')
    .map((s) => byId(cosmetics, loadout[s])?.visual)
    .filter((v): v is CosmeticVisual => !!v);
}

const ANIMATIONS: PlayerAnimation[] = ['wave', 'dance', 'cheer', 'bow'];

export function emoteAnimation(c: Pick<Cosmetic, 'visual'> | undefined): PlayerAnimation | null {
  const a = c?.visual.shape as PlayerAnimation | undefined;
  return a && ANIMATIONS.includes(a) ? a : null;
}

export function equippedNametag(loadout: Loadout, cosmetics: Cosmetic[]): Cosmetic | undefined {
  return byId(cosmetics, loadout.nametag);
}

export function sameLoadout(a: Loadout, b: Loadout): boolean {
  return SLOTS.every((s) => (a[s] ?? null) === (b[s] ?? null));
}

export function equippedCount(loadout: Loadout): number {
  return SLOTS.filter((s) => !!loadout[s]).length;
}
