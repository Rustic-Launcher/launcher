import { describe, expect, it } from 'vitest';
import { IpcFailure } from '../../client';
import { serversDb } from '../data/servers';
import { dnsHandlers } from './dns';
import { serverAdminHandlers } from './serverAdmin';

const h = dnsHandlers as Required<typeof dnsHandlers>;
const a = serverAdminHandlers as Required<typeof serverAdminHandlers>;

describe('mock dns', () => {
  it('manual verification progresses to connected and binds the domain', async () => {
    const target = { server_id: 'srv_copains', domain: 'example.net', subdomain: 'play' };
    const req = await h.dns_required_records(target);
    expect(req.records.map((r) => r.type)).toEqual(['CNAME', 'SRV', 'TXT']);
    const seen = new Set<string>();
    let ok = false;
    for (let i = 0; i < 8 && !ok; i++) {
      const v = await h.dns_verify(target);
      v.records.forEach((r) => seen.add(r.status));
      ok = v.all_ok;
    }
    expect(ok).toBe(true);
    expect(seen).toEqual(new Set(['missing', 'wrong_value', 'propagating', 'ok']));
    expect(serversDb.domains.srv_copains?.status).toBe('verified');
    expect(serversDb.servers.find((s) => s.id === 'srv_copains')?.share_address).toBe(
      'play.example.net',
    );
    await h.dns_remove_binding({ server_id: 'srv_copains' });
    expect(serversDb.domains.srv_copains).toBeUndefined();
  });

  it('vps records use A/AAAA to the host', async () => {
    const req = await h.dns_required_records({
      server_id: 'srv_vps',
      domain: 'samcraft.fr',
      subdomain: 'mc',
    });
    expect(req.records.map((r) => r.type)).toEqual(['A', 'AAAA', 'TXT']);
    expect(req.records[0]?.value).toBe('203.0.113.84');
  });

  it('buying needs Porkbun, both ticks and the quoted price', async () => {
    const quote = (await h.dns_check_domain({ query: 'loucraft' })).find(
      (d) => d.domain === 'loucraft.net',
    )!;
    expect(quote.available).toBe(true);
    const buy = {
      domain: 'loucraft.net',
      years: 1,
      expected_price: quote.price!,
      agree_registrar_terms: true as const,
      authorize_order: true as const,
    };
    await expect(h.dns_buy_domain(buy)).rejects.toMatchObject({ code: 'unauthorized' });
    await expect(
      h.dns_connect_porkbun({ api_key: 'nope', secret_api_key: 'sk1_aaaaaaaaaa' }),
    ).rejects.toBeInstanceOf(IpcFailure);
    const conn = await h.dns_connect_porkbun({
      api_key: 'pk1_abcdefgh1234',
      secret_api_key: 'sk1_abcdefgh1234',
    });
    expect(conn.account_label).toBe('pk1_…1234');
    await expect(
      h.dns_buy_domain({ ...buy, expected_price: { amount_minor: 1, currency: 'USD' } }),
    ).rejects.toMatchObject({ details: { reason: 'price_changed' } });
    const res = await h.dns_buy_domain(buy);
    expect(res.charged).toEqual(quote.price);
    expect(
      (await h.dns_list_domains({ provider: 'porkbun' })).some((d) => d.domain === 'loucraft.net'),
    ).toBe(true);
    await h.dns_disconnect({ provider: 'porkbun' });
  });
});

describe('mock server admin', () => {
  it('completes commands and player names', () => {
    expect(a.server_complete({ server_id: 'srv_copains', partial: 'ki' })).toEqual(['kick']);
    expect(a.server_complete({ server_id: 'srv_copains', partial: 'kick le' })).toEqual([
      'kick lea_builds',
    ]);
    expect(a.server_complete({ server_id: 'srv_copains', partial: 'time s' })).toEqual([
      'time set day',
      'time set night',
      'time set noon',
    ]);
  });

  it('relay labels map to the relay domain and refuse reserved names', async () => {
    const acc = await a.server_relay_set({
      server_id: 'srv_creative',
      enabled: true,
      label: 'my-test',
    });
    expect(acc.relay.hostname).toBe('my-test.relay.example.invalid');
    await expect(
      a.server_relay_set({ server_id: 'srv_creative', enabled: true, label: 'admin' }),
    ).rejects.toMatchObject({ code: 'conflict' });
    const off = await a.server_relay_set({ server_id: 'srv_creative', enabled: false });
    expect(off.relay.enabled).toBe(false);
  });
});
