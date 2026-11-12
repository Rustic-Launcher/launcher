import { beforeEach, describe, expect, it, vi } from 'vitest';

async function load() {
  vi.resetModules();
  window.localStorage.clear();
  window.localStorage.setItem('rustic.mock.latency', '0');
  const { storeMock } = await import('./store');
  const { accountMock } = await import('./account');
  return { store: storeMock, account: accountMock };
}

const req = (product_id: string) => ({
  product_id,
  currency: 'EUR' as const,
  consent_immediate_execution: true as const,
  waiver_withdrawal_right: true as const,
});

describe('store mock', () => {
  beforeEach(() => window.localStorage.clear());

  it('lists original cosmetics without capes', async () => {
    const { store } = await load();
    const all = await store.cosmetics!();
    expect(all.length).toBeGreaterThanOrEqual(28);
    expect(all.some((c) => (c.slot as string) === 'cape' || /cape/i.test(c.visual.shape))).toBe(
      false,
    );
  });

  it('prices the catalogue per currency, without timers', async () => {
    const { store } = await load();
    const eur = await store.catalog!('EUR');
    const usd = await store.catalog!('USD');
    expect(eur.products.find((x) => x.id === 'p_beanie_ember')!.price).toEqual({
      amount_minor: 299,
      currency: 'EUR',
    });
    expect(usd.products.find((x) => x.id === 'p_beanie_ember')!.price).toEqual({
      amount_minor: 349,
      currency: 'USD',
    });
    expect(eur.bundles.every((b) => b.compare_at!.amount_minor > b.price.amount_minor)).toBe(true);
    expect(eur.products.every((x) => x.available_until === null)).toBe(true);
  });

  it('refuses a purchase without both consents', async () => {
    const { store } = await load();
    await expect(
      store.purchase!({ ...req('p_halo_dawn'), waiver_withdrawal_right: false as unknown as true }),
    ).rejects.toMatchObject({ message: 'consent_required' });
  });

  it('grants the items and allows equipping them', async () => {
    const { store } = await load();
    const res = await store.purchase!(req('p_halo_dawn'));
    expect(res.status).toBe('completed');
    expect(res.granted_cosmetic_ids).toEqual(['halo_dawn']);
    expect(res.receipt_url).toContain('rustic.example.invalid');
    const l = await store.equip!({ head: 'halo_dawn' });
    expect(l.head).toBe('halo_dawn');
    await expect(store.purchase!(req('p_halo_dawn'))).rejects.toMatchObject({
      message: 'already_owned',
    });
  });

  it('refuses to equip something not owned', async () => {
    const { store } = await load();
    await expect(store.equip!({ head: 'crown_copper' })).rejects.toBeTruthy();
  });

  it('respects parental controls', async () => {
    const { store, account } = await load();
    await account.setParental!({ enabled: true, purchases: 'blocked' });
    await expect(store.purchase!(req('p_crown_copper'))).rejects.toMatchObject({
      code: 'parental_block',
      message: 'purchases_blocked',
    });
    await account.setParental!({ purchases: 'pin', pin: '2468' });
    await expect(store.purchase!(req('p_crown_copper'))).rejects.toMatchObject({
      message: 'pin_required',
    });
    await expect(
      store.purchase!({ ...req('p_crown_copper'), parental_pin: '1111' }),
    ).rejects.toMatchObject({
      message: 'pin_invalid',
    });
    expect((await store.purchase!({ ...req('p_crown_copper'), parental_pin: '2468' })).status).toBe(
      'completed',
    );
  });

  it('sends gifts without granting them to the buyer', async () => {
    const { store } = await load();
    const res = await store.purchase!({ ...req('p_crown_copper'), gift_to: 'u_lea' });
    expect(res.granted_cosmetic_ids).toEqual([]);
    const all = await store.cosmetics!();
    expect(all.find((c) => c.id === 'crown_copper')!.owned).toBe(false);
  });
});
