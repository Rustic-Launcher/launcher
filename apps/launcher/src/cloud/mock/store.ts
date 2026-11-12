/** Mock cosmetics, loadout, outfits and store (Rustic Cloud). */
import type { CloudClient } from '../client';
import type {
  Cosmetic,
  Currency,
  Loadout,
  Outfit,
  PurchaseResult,
  StoreCatalog,
  StoreProduct,
} from '../types';
import { fail, latency, nowIso, persistGet, persistSet, uid } from '../../ipc/mock/runtime';
import { accountMock } from './account';
import {
  BUNDLES,
  COSMETICS,
  FEATURED_IDS,
  hexHue,
  INITIAL_LOADOUT,
  INITIAL_OWNED,
  price,
  RARITY_TIER,
  type CosmeticDef,
} from './data/cosmetics';

const lang = (): 'en' | 'fr' =>
  typeof document !== 'undefined' && document.documentElement.lang === 'fr' ? 'fr' : 'en';

const state = {
  owned: new Map<string, string>(
    persistGet<[string, string][]>(
      'cosmetics_owned',
      INITIAL_OWNED.map((id) => [id, '2026-08-01T10:00:00Z']),
    ),
  ),
  loadout: persistGet<Loadout>('loadout', { ...INITIAL_LOADOUT }),
  outfits: persistGet<Outfit[]>('outfits', [
    {
      id: 'of_cosy',
      name: 'Cosy',
      loadout: {
        head: 'beanie_ember',
        top: 'top_hoodie_amethyst',
        shoulder: 'shoulder_fox',
        nametag: 'tag_mono',
      },
      created_at: '2026-10-02T18:00:00Z',
    },
  ]),
};

function save() {
  persistSet('cosmetics_owned', [...state.owned.entries()]);
  persistSet('loadout', state.loadout);
  persistSet('outfits', state.outfits);
}

function toCosmetic(d: CosmeticDef): Cosmetic {
  const acquired = state.owned.get(d.id) ?? null;
  return {
    id: d.id,
    name: d.name[lang()],
    description: d.description[lang()],
    slot: d.slot,
    rarity: d.rarity,
    visual: { slot: d.slot, shape: d.shape, color: d.color, accent: d.accent },
    owned: acquired != null,
    acquired_at: acquired,
    source: acquired ? (INITIAL_OWNED.includes(d.id) ? 'default' : 'purchase') : null,
  };
}

function itemProduct(d: CosmeticDef, currency: Currency): StoreProduct {
  return {
    id: `p_${d.id}`,
    kind: 'item',
    name: d.name[lang()],
    description: d.description[lang()],
    cosmetic_ids: [d.id],
    slot: d.slot,
    rarity: d.rarity,
    price: price(RARITY_TIER[d.rarity], currency),
    compare_at: null,
    owned: state.owned.has(d.id),
    is_new: !!d.is_new,
    available_until: null,
    hue: hexHue(d.color),
  };
}

const RARITY_RANK = { common: 0, rare: 1, epic: 2, legendary: 3 } as const;

function bundleProducts(currency: Currency): StoreProduct[] {
  return BUNDLES.map((b) => {
    const items = b.cosmetic_ids.map((id) => COSMETICS.find((x) => x.id === id)!);
    const compare = items.reduce(
      (sum, d) => sum + price(RARITY_TIER[d.rarity], currency).amount_minor,
      0,
    );
    const best = [...items].sort((x, y) => RARITY_RANK[y.rarity] - RARITY_RANK[x.rarity])[0]!;
    return {
      id: b.id,
      kind: 'bundle',
      name: b.name[lang()],
      description: b.description[lang()],
      cosmetic_ids: b.cosmetic_ids,
      slot: null,
      rarity: best.rarity,
      price: price(b.tier, currency),
      compare_at: { amount_minor: compare, currency },
      owned: b.cosmetic_ids.every((id) => state.owned.has(id)),
      is_new: !!b.is_new,
      available_until: null,
      hue: b.hue,
    };
  });
}

function allProducts(currency: Currency): StoreProduct[] {
  return [...bundleProducts(currency), ...COSMETICS.map((d) => itemProduct(d, currency))];
}

function findProduct(id: string, currency: Currency): StoreProduct {
  return allProducts(currency).find((p) => p.id === id) ?? fail('not_found', `product ${id}`);
}

export const storeMock: Partial<CloudClient> = {
  async cosmetics() {
    await latency(0.8);
    return COSMETICS.map(toCosmetic);
  },
  async loadout() {
    await latency(0.5);
    return { ...state.loadout };
  },
  async equip(loadout) {
    await latency(0.3);
    for (const [slot, id] of Object.entries(loadout)) {
      if (!id) continue;
      if (!state.owned.has(id)) fail('invalid_argument', `cosmetic ${id} not owned`, { slot });
      if (COSMETICS.find((d) => d.id === id)?.slot !== slot)
        fail('invalid_argument', `cosmetic ${id} wrong slot`);
    }
    state.loadout = { ...loadout };
    save();
    return { ...state.loadout };
  },
  async outfits() {
    await latency(0.5);
    return structuredClone(state.outfits);
  },
  async saveOutfit(name, loadout) {
    await latency(0.5);
    const outfit: Outfit = { id: uid('of'), name, loadout: { ...loadout }, created_at: nowIso() };
    state.outfits = [...state.outfits, outfit];
    save();
    return structuredClone(outfit);
  },
  async deleteOutfit(id) {
    await latency(0.4);
    state.outfits = state.outfits.filter((o) => o.id !== id);
    save();
  },
  async catalog(currency) {
    await latency(1);
    const products = allProducts(currency);
    const catalog: StoreCatalog = {
      currency,
      featured: FEATURED_IDS.map((id) => products.find((p) => p.id === id)).filter(
        (p): p is StoreProduct => !!p,
      ),
      bundles: products.filter((p) => p.kind === 'bundle'),
      new_items: products.filter((p) => p.is_new),
      products,
    };
    return catalog;
  },
  async product(id, currency) {
    await latency(0.6);
    return findProduct(id, currency);
  },
  async purchase(req) {
    await latency(2);
    if (req.consent_immediate_execution !== true || req.waiver_withdrawal_right !== true) {
      fail('invalid_argument', 'consent_required');
    }
    const product = findProduct(req.product_id, req.currency);
    if (product.owned && !req.gift_to) fail('conflict', 'already_owned');
    const parental = await accountMock.parental!();
    const me = await accountMock.me!();
    const order_id = uid('ord');
    // Under-18 accounts: purchases need a parent's approval by default (PIN on this device, or a
    // request emailed to the linked parent).
    if (me.age_group !== 'adult' && !(parental.enabled && parental.purchases === 'allowed')) {
      const pinOk = req.parental_pin
        ? await accountMock.verifyParentalPin!(req.parental_pin)
        : false;
      if (!pinOk) {
        return {
          order_id,
          status: 'parental_approval_required',
          charged: { amount_minor: 0, currency: req.currency },
          receipt_url: `https://rustic.example.invalid/account/orders/${order_id}`,
          granted_cosmetic_ids: [],
        };
      }
    }
    if (parental.enabled) {
      if (parental.purchases === 'blocked') fail('parental_block', 'purchases_blocked');
      if (parental.monthly_limit && parental.monthly_limit.currency === product.price.currency) {
        const after = parental.spent_this_month.amount_minor + product.price.amount_minor;
        if (after > parental.monthly_limit.amount_minor) fail('parental_block', 'monthly_limit');
      }
      if (parental.purchases === 'pin') {
        if (!req.parental_pin) fail('parental_block', 'pin_required');
        const ok = await accountMock.verifyParentalPin!(req.parental_pin);
        if (!ok) fail('parental_block', 'pin_invalid');
      }
    }
    const granted = req.gift_to ? [] : product.cosmetic_ids.filter((id) => !state.owned.has(id));
    const when = nowIso();
    for (const id of granted) state.owned.set(id, when);
    save();
    const result: PurchaseResult = {
      order_id,
      status: 'completed',
      charged: product.price,
      receipt_url: `https://rustic.example.invalid/account/orders/${order_id}`,
      granted_cosmetic_ids: granted,
    };
    return result;
  },
};
