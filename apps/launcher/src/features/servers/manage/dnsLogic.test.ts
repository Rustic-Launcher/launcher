import { describe, expect, it } from 'vitest';
import {
  buildRequiredRecords,
  hostForRegistrar,
  normalizeDomain,
  normalizeSubdomain,
  relativeHost,
  relayRouteTarget,
  requiredOk,
  summarizeVerification,
} from './dnsLogic';
import { RELAY_DOMAIN } from '@/lib/relay';

describe('normalizeDomain', () => {
  it('cleans user input', () => {
    expect(normalizeDomain(' https://Example.FR/path ')).toBe('example.fr');
    expect(normalizeDomain('play.samcraft.fr.')).toBe('play.samcraft.fr');
  });
  it('rejects garbage', () => {
    expect(normalizeDomain('not a domain')).toBeNull();
    expect(normalizeDomain('localhost')).toBeNull();
    expect(normalizeDomain('-bad.com')).toBeNull();
  });
});

describe('normalizeSubdomain', () => {
  it('accepts labels and empty', () => {
    expect(normalizeSubdomain('Play')).toBe('play');
    expect(normalizeSubdomain('mc.eu')).toBe('mc.eu');
    expect(normalizeSubdomain('  ')).toBeUndefined();
    expect(normalizeSubdomain('bad_label')).toBeNull();
  });
});

describe('relativeHost', () => {
  it('maps apex to @', () => {
    expect(relativeHost('example.com', 'example.com')).toBe('@');
    expect(relativeHost('_minecraft._tcp.play.example.com', 'example.com')).toBe(
      '_minecraft._tcp.play',
    );
  });
});

describe('buildRequiredRecords', () => {
  it('local hosting: CNAME to relay, SRV and TXT', () => {
    const target = relayRouteTarget('srv1');
    expect(target.endsWith(`.${RELAY_DOMAIN}`)).toBe(true);
    const r = buildRequiredRecords('srv1', 'example.com', 'play', {
      mode: 'local',
      relayHostname: target,
    });
    expect(r.fqdn).toBe('play.example.com');
    expect(r.records.map((x) => x.type)).toEqual(['CNAME', 'SRV', 'TXT']);
    expect(r.records[0]).toMatchObject({ host: 'play', value: `${target}.` });
    expect(r.records[1]).toMatchObject({
      host: '_minecraft._tcp.play',
      port: 25565,
      optional: true,
    });
    expect(r.records[2]?.host).toBe('_rustic-challenge.play');
    expect(r.records[2]?.value).toMatch(/^rustic-verify=/);
  });
  it('local hosting on the apex has no CNAME', () => {
    const r = buildRequiredRecords('srv1', 'example.com', null, {
      mode: 'local',
      relayHostname: relayRouteTarget('srv1'),
    });
    expect(r.records.map((x) => x.type)).toEqual(['SRV', 'TXT']);
    expect(r.records[0]?.optional).toBe(false);
  });
  it('vps hosting: A + AAAA, SRV only for non-default port', () => {
    const def = buildRequiredRecords('s', 'example.com', 'mc', {
      mode: 'vps',
      ipv4: '1.2.3.4',
      ipv6: '2001:db8::1',
      port: 25565,
    });
    expect(def.records.map((x) => x.type)).toEqual(['A', 'AAAA', 'TXT']);
    const custom = buildRequiredRecords('s', 'example.com', 'mc', {
      mode: 'vps',
      ipv4: '1.2.3.4',
      port: 25570,
    });
    expect(custom.records.map((x) => x.type)).toEqual(['A', 'SRV', 'TXT']);
    expect(custom.records[1]?.value).toBe('0 5 25570 mc.example.com.');
  });
  it('token is stable per server and name', () => {
    const a = buildRequiredRecords('s', 'example.com', 'mc', {
      mode: 'vps',
      ipv4: '1.2.3.4',
      port: 25565,
    });
    const b = buildRequiredRecords('s', 'example.com', 'mc', {
      mode: 'vps',
      ipv4: '1.2.3.4',
      port: 25565,
    });
    expect(a.records.at(-1)?.value).toBe(b.records.at(-1)?.value);
  });
});

describe('summarizeVerification', () => {
  const v = (statuses: ('ok' | 'missing' | 'propagating' | 'wrong_value')[]) => ({
    fqdn: 'x',
    checked_at: '',
    all_ok: statuses.every((s) => s === 'ok'),
    records: statuses.map((status, i) => ({ record_id: String(i), status, found: [] })),
  });
  it('reports the worst status', () => {
    expect(summarizeVerification(v(['ok', 'propagating', 'missing']), 3)).toMatchObject({
      ok: 1,
      worst: 'missing',
      connected: false,
    });
    expect(summarizeVerification(v(['ok', 'wrong_value']), 2).worst).toBe('wrong_value');
  });
  it('requiredOk ignores optional records', () => {
    const recs = buildRequiredRecords('s', 'example.com', 'play', {
      mode: 'local',
      relayHostname: relayRouteTarget('s'),
    }).records;
    const ver = {
      fqdn: 'play.example.com',
      checked_at: '',
      all_ok: true,
      records: recs.map((r) => ({
        record_id: r.id,
        status: r.optional ? ('missing' as const) : ('ok' as const),
        found: [],
      })),
    };
    expect(requiredOk(recs, ver)).toBe(true);
    expect(requiredOk(recs, undefined)).toBe(false);
  });
  it('is connected only when everything is ok', () => {
    expect(summarizeVerification(v(['ok', 'ok', 'ok']), 3).connected).toBe(true);
    expect(summarizeVerification(undefined, 3)).toMatchObject({
      connected: false,
      worst: 'missing',
    });
  });
});

describe('hostForRegistrar', () => {
  it('OVH wants an empty host for the apex', () => {
    const r = buildRequiredRecords('s', 'example.com', null, {
      mode: 'vps',
      ipv4: '1.2.3.4',
      port: 25565,
    }).records[0]!;
    expect(hostForRegistrar(r, 'ovh')).toBe('');
    expect(hostForRegistrar(r, 'cloudflare')).toBe('@');
  });
});
