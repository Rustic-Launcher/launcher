/**
 * Mock cosmetics catalogue. Every item is original and purely visual: procedural shapes drawn by
 * the PlayerViewer, no capes or cape-like items, no random draws (rarity is a label only).
 */
import type { CosmeticSlot, Currency, Loadout, Money, Rarity } from '../../types';

type L10n = { en: string; fr: string };

export interface CosmeticDef {
  id: string;
  slot: CosmeticSlot;
  rarity: Rarity;
  shape: string;
  color: string;
  accent?: string;
  name: L10n;
  description: L10n;
  is_new?: boolean;
}

const c = (
  id: string,
  slot: CosmeticSlot,
  rarity: Rarity,
  shape: string,
  color: string,
  accent: string | undefined,
  en: string,
  fr: string,
  den: string,
  dfr: string,
  is_new = false,
): CosmeticDef => ({
  id,
  slot,
  rarity,
  shape,
  color,
  accent,
  name: { en, fr },
  description: { en: den, fr: dfr },
  is_new,
});

export const COSMETICS: CosmeticDef[] = [
  // head
  c(
    'crown_copper',
    'head',
    'epic',
    'crown',
    '#B87333',
    '#F2C14E',
    'Copper Crown',
    'Couronne de cuivre',
    'A hammered copper crown with three glowing studs.',
    'Une couronne de cuivre martelé aux trois éclats lumineux.',
  ),
  c(
    'beanie_ember',
    'head',
    'common',
    'beanie',
    '#C9543E',
    '#F6F0EC',
    'Ember Beanie',
    'Bonnet braise',
    'A warm knit beanie with a soft pompom.',
    'Un bonnet tricoté bien chaud, avec son pompon.',
  ),
  c(
    'horns_obsidian',
    'head',
    'rare',
    'horns',
    '#2B2433',
    undefined,
    'Obsidian Horns',
    'Cornes d’obsidienne',
    'Two polished horns, darker than the night.',
    'Deux cornes polies, plus sombres que la nuit.',
  ),
  c(
    'halo_dawn',
    'head',
    'legendary',
    'halo',
    '#FFD27A',
    undefined,
    'Dawn Halo',
    'Auréole de l’aube',
    'A floating ring of morning light.',
    'Un anneau flottant de lumière matinale.',
  ),
  c(
    'cap_explorer',
    'head',
    'common',
    'cap',
    '#556B2F',
    '#3B2A20',
    'Explorer Cap',
    'Casquette d’explorateur',
    'Sun-proof, map-friendly, always ready.',
    'Anti-soleil, pratique pour lire une carte, toujours prête.',
  ),
  c(
    'antlers_moss',
    'head',
    'rare',
    'antlers',
    '#8C6A4A',
    '#6FAF6B',
    'Mossy Antlers',
    'Bois moussus',
    'Antlers with tiny tufts of moss.',
    'Des bois couverts de petites touffes de mousse.',
    true,
  ),
  c(
    'flower_bloom',
    'head',
    'common',
    'flower',
    '#E86FA0',
    '#F2D14E',
    'Bloom Pin',
    'Fleur épinglée',
    'A single flower tucked behind the ear.',
    'Une fleur glissée derrière l’oreille.',
    true,
  ),
  // back (compact objects only)
  c(
    'backpack_trail',
    'back',
    'common',
    'backpack',
    '#7A5236',
    '#C9A36B',
    'Trail Backpack',
    'Sac de randonnée',
    'A sturdy pack for long walks.',
    'Un sac solide pour les longues marches.',
  ),
  c(
    'lantern_autumn',
    'back',
    'epic',
    'lantern',
    '#3A3330',
    '#FFB347',
    'Autumn Lantern',
    'Lanterne d’automne',
    'A small lantern hanging from your belt, glowing amber.',
    'Une petite lanterne à la ceinture, d’une lueur ambrée.',
    true,
  ),
  c(
    'wings_ember',
    'back',
    'rare',
    'wings',
    '#E8834A',
    undefined,
    'Ember Wing Stubs',
    'Ailerons de braise',
    'Two tiny decorative wing stubs. They do not fly.',
    'Deux petits ailerons décoratifs. Ils ne volent pas.',
  ),
  c(
    'quiver_ranger',
    'back',
    'rare',
    'quiver',
    '#6B4A2B',
    '#D9D2C5',
    'Ranger Quiver',
    'Carquois de rôdeur',
    'A leather quiver worn across the back. Decorative only.',
    'Un carquois de cuir porté en bandoulière. Purement décoratif.',
  ),
  // top
  c(
    'top_hoodie_amethyst',
    'top',
    'common',
    'hoodie',
    '#5B43C9',
    '#9B87F5',
    'Amethyst Hoodie',
    'Sweat améthyste',
    'A cosy violet hoodie.',
    'Un sweat violet tout doux.',
  ),
  c(
    'top_jacket_verdigris',
    'top',
    'rare',
    'jacket',
    '#1F6B58',
    '#3DBE8F',
    'Verdigris Jacket',
    'Veste vert-de-gris',
    'A field jacket with patina-green trims.',
    'Une veste de terrain aux finitions vert-de-gris.',
  ),
  c(
    'top_tunic_copper',
    'top',
    'epic',
    'tunic',
    '#8A4B22',
    '#E8834A',
    'Smith’s Tunic',
    'Tunique de forgeron',
    'Heat-proof tunic with a copper belt.',
    'Une tunique résistante à la chaleur, ceinture de cuivre.',
  ),
  // shoulder companions
  c(
    'shoulder_fox',
    'shoulder',
    'rare',
    'fox',
    '#E07B39',
    '#1A1A1A',
    'Shoulder Fox',
    'Renard d’épaule',
    'A tiny fox napping on your shoulder.',
    'Un minuscule renard qui fait la sieste sur votre épaule.',
  ),
  c(
    'shoulder_owl',
    'shoulder',
    'rare',
    'bird',
    '#8C7A5B',
    '#E8B84A',
    'Little Owl',
    'Petite chouette',
    'Watches everything, judges nothing.',
    'Elle observe tout, ne juge rien.',
  ),
  c(
    'shoulder_cat',
    'shoulder',
    'epic',
    'cat',
    '#3B3B3B',
    '#9FE870',
    'Night Cat',
    'Chat de nuit',
    'A black cat with bright green eyes.',
    'Un chat noir aux yeux vert vif.',
  ),
  // companions
  c(
    'comp_slime',
    'companion',
    'common',
    'slime',
    '#6FD08C',
    '#134B2B',
    'Bouncy Blob',
    'Boule rebondissante',
    'A friendly jelly cube that follows you around.',
    'Un cube gélatineux sympathique qui vous suit partout.',
  ),
  c(
    'comp_fox',
    'companion',
    'epic',
    'fox',
    '#F2F0EA',
    '#2A2A2A',
    'Snow Fox',
    'Renard des neiges',
    'A white fox floating at your side.',
    'Un renard blanc qui flotte à vos côtés.',
    true,
  ),
  c(
    'comp_bird',
    'companion',
    'rare',
    'bird',
    '#3A8FB7',
    '#F2C14E',
    'Sky Sparrow',
    'Moineau du ciel',
    'A bright blue sparrow circling nearby.',
    'Un moineau bleu vif qui tourne autour de vous.',
  ),
  // trails
  c(
    'trail_embers',
    'trail',
    'rare',
    'embers',
    '#FF8A3D',
    undefined,
    'Ember Steps',
    'Pas de braise',
    'Glowing embers fade behind your steps.',
    'Des braises qui s’éteignent derrière vos pas.',
  ),
  c(
    'trail_petals',
    'trail',
    'common',
    'petals',
    '#F59AC2',
    undefined,
    'Petal Path',
    'Chemin de pétales',
    'A few petals mark where you walked.',
    'Quelques pétales marquent votre passage.',
  ),
  c(
    'trail_frost',
    'trail',
    'epic',
    'frost',
    '#9EDBFF',
    undefined,
    'Frost Steps',
    'Pas de givre',
    'Little frost tiles under your feet.',
    'De petites plaques de givre sous vos pieds.',
    true,
  ),
  // auras
  c(
    'aura_fireflies',
    'aura',
    'epic',
    'fireflies',
    '#FFE07A',
    undefined,
    'Fireflies',
    'Lucioles',
    'A swirl of gentle fireflies.',
    'Une ronde de lucioles paisibles.',
  ),
  c(
    'aura_amethyst',
    'aura',
    'legendary',
    'sparkles',
    '#A58BFF',
    undefined,
    'Amethyst Sparkles',
    'Étincelles d’améthyste',
    'Violet sparkles orbiting you.',
    'Des étincelles violettes en orbite autour de vous.',
  ),
  // emotes (shape = animation)
  c(
    'emote_wave',
    'emote',
    'common',
    'wave',
    '#E8834A',
    undefined,
    'Friendly Wave',
    'Salut amical',
    'Say hello.',
    'Dites bonjour.',
  ),
  c(
    'emote_dance',
    'emote',
    'rare',
    'dance',
    '#9B87F5',
    undefined,
    'Block Party',
    'Fête des blocs',
    'A cheerful little dance.',
    'Une petite danse joyeuse.',
  ),
  c(
    'emote_cheer',
    'emote',
    'rare',
    'cheer',
    '#3DBE8F',
    undefined,
    'Victory Cheer',
    'Cri de victoire',
    'Both arms up, you did it!',
    'Les deux bras en l’air, vous l’avez fait !',
  ),
  c(
    'emote_bow',
    'emote',
    'common',
    'bow',
    '#B0AAC6',
    undefined,
    'Polite Bow',
    'Révérence',
    'A courteous bow.',
    'Une révérence courtoise.',
  ),
  // nametags
  c(
    'tag_copper',
    'nametag',
    'common',
    'gradient',
    '#E8834A',
    '#F2C14E',
    'Copper Tag',
    'Étiquette cuivre',
    'A warm copper gradient for your name.',
    'Un dégradé cuivré pour votre pseudo.',
  ),
  c(
    'tag_aurora',
    'nametag',
    'epic',
    'gradient',
    '#3DBE8F',
    '#9B87F5',
    'Aurora Tag',
    'Étiquette aurore',
    'Green to violet, like northern lights.',
    'Du vert au violet, comme une aurore boréale.',
  ),
  c(
    'tag_mono',
    'nametag',
    'common',
    'solid',
    '#F6F0EC',
    '#94877F',
    'Classic Tag',
    'Étiquette classique',
    'Clean and simple.',
    'Sobre et lisible.',
  ),
];

export const INITIAL_OWNED = [
  'beanie_ember',
  'cap_explorer',
  'backpack_trail',
  'top_hoodie_amethyst',
  'shoulder_fox',
  'comp_slime',
  'trail_petals',
  'emote_wave',
  'tag_mono',
];

export const INITIAL_LOADOUT: Loadout = {
  head: 'beanie_ember',
  back: 'backpack_trail',
  shoulder: 'shoulder_fox',
  emote: 'emote_wave',
  nametag: 'tag_mono',
};

/** Prices in minor units per currency, by tier. Prices include VAT. */
export const PRICE_TIERS: Record<
  't1' | 't2' | 't3' | 't4' | 'b1' | 'b2' | 'b3',
  Record<Currency, number>
> = {
  t1: { EUR: 299, USD: 349, GBP: 279, CHF: 290 },
  t2: { EUR: 499, USD: 549, GBP: 449, CHF: 490 },
  t3: { EUR: 799, USD: 899, GBP: 699, CHF: 790 },
  t4: { EUR: 1199, USD: 1299, GBP: 1049, CHF: 1190 },
  b1: { EUR: 1499, USD: 1649, GBP: 1299, CHF: 1490 },
  b2: { EUR: 1799, USD: 1999, GBP: 1549, CHF: 1790 },
  b3: { EUR: 1599, USD: 1749, GBP: 1399, CHF: 1590 },
};

export const RARITY_TIER: Record<Rarity, keyof typeof PRICE_TIERS> = {
  common: 't1',
  rare: 't2',
  epic: 't3',
  legendary: 't4',
};

export interface BundleDef {
  id: string;
  tier: keyof typeof PRICE_TIERS;
  cosmetic_ids: string[];
  name: L10n;
  description: L10n;
  hue: number;
  is_new?: boolean;
}

export const BUNDLES: BundleDef[] = [
  {
    id: 'b_autumn',
    tier: 'b1',
    cosmetic_ids: ['lantern_autumn', 'trail_embers', 'antlers_moss', 'tag_copper'],
    name: { en: 'Autumn Harvest', fr: 'Récolte d’automne' },
    description: {
      en: 'A lantern, ember steps, mossy antlers and a copper name tag for the season.',
      fr: 'Une lanterne, des pas de braise, des bois moussus et une étiquette cuivre pour la saison.',
    },
    hue: 26,
    is_new: true,
  },
  {
    id: 'b_amethyst',
    tier: 'b2',
    cosmetic_ids: ['aura_amethyst', 'horns_obsidian', 'tag_aurora'],
    name: { en: 'Amethyst Night', fr: 'Nuit d’améthyste' },
    description: {
      en: 'Violet sparkles, obsidian horns and the aurora name tag.',
      fr: 'Étincelles violettes, cornes d’obsidienne et étiquette aurore.',
    },
    hue: 258,
  },
  {
    id: 'b_explorer',
    tier: 'b3',
    cosmetic_ids: ['quiver_ranger', 'comp_bird', 'top_jacket_verdigris', 'emote_cheer'],
    name: { en: 'Explorer Kit', fr: 'Kit d’explorateur' },
    description: {
      en: 'Everything for the trail: quiver, sparrow, field jacket and a victory cheer.',
      fr: 'Tout pour l’aventure : carquois, moineau, veste de terrain et cri de victoire.',
    },
    hue: 160,
  },
];

export const FEATURED_IDS = ['b_autumn', 'p_halo_dawn', 'p_comp_fox'];

export function price(tier: keyof typeof PRICE_TIERS, currency: Currency): Money {
  return { amount_minor: PRICE_TIERS[tier][currency], currency };
}

/** Hue (0-360) of a hex colour, for generated cover art. */
export function hexHue(hex: string): number {
  const n = Number.parseInt(hex.replace('#', ''), 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  if (d === 0) return 30;
  let h: number;
  if (max === r) h = ((g - b) / d) % 6;
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  return Math.round((h * 60 + 360) % 360);
}
