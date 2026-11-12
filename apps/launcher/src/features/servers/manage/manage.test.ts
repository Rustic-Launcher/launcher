import { describe, expect, it } from 'vitest';
import type { DomainAvailability } from '@/ipc/types/dns';
import type { ConfigField, ServerPlayer } from '@/ipc/types/servers';
import { purchasable } from './access/BuyPath';
import { configDiff } from './ConfigTab';
import { historyStep } from './ConsoleTab';
import { tickHealth } from './OverviewTab';
import { filterPlayers } from './PlayersTab';

describe('historyStep', () => {
  const h = ['list', 'say hi', 'tps'];
  it('starts from the most recent entry going up', () => {
    expect(historyStep(h, null, -1)).toBe(2);
    expect(historyStep(h, 2, -1)).toBe(1);
    expect(historyStep(h, 0, -1)).toBe(0);
  });
  it('goes back to an empty line past the newest', () => {
    expect(historyStep(h, 2, 1)).toBeNull();
    expect(historyStep(h, null, 1)).toBeNull();
    expect(historyStep([], null, -1)).toBeNull();
  });
});

describe('filterPlayers', () => {
  const p = (name: string, extra: Partial<ServerPlayer> = {}): ServerPlayer => ({
    uuid: name,
    name,
    online: false,
    op: false,
    whitelisted: false,
    banned: false,
    ping_ms: null,
    joined_at: null,
    playtime_s: 0,
    ...extra,
  });
  const list = [
    p('zed', { online: true }),
    p('amy', { op: true }),
    p('bob', { banned: true }),
    p('cal', { online: true, whitelisted: true }),
  ];
  it('filters by view and sorts online first', () => {
    expect(filterPlayers(list, 'online', '').map((x) => x.name)).toEqual(['cal', 'zed']);
    expect(filterPlayers(list, 'all', '').map((x) => x.name)).toEqual(['cal', 'zed', 'amy', 'bob']);
    expect(filterPlayers(list, 'ops', '').map((x) => x.name)).toEqual(['amy']);
    expect(filterPlayers(list, 'banned', '').map((x) => x.name)).toEqual(['bob']);
    expect(filterPlayers(list, 'whitelist', '').map((x) => x.name)).toEqual(['cal']);
  });
  it('searches by name', () => {
    expect(filterPlayers(list, 'all', 'A').map((x) => x.name)).toEqual(['cal', 'amy']);
  });
});

describe('configDiff', () => {
  const f = (key: string, value: string | number | boolean): ConfigField => ({
    key,
    group: 'general',
    type: typeof value === 'number' ? 'integer' : typeof value === 'boolean' ? 'boolean' : 'string',
    value,
    default: value,
    restart_required: false,
  });
  it('keeps only changed values', () => {
    const fields = [f('motd', 'hi'), f('max-players', 20), f('pvp', true)];
    expect(configDiff(fields, { motd: 'hi', 'max-players': 30, pvp: false })).toEqual({
      'max-players': 30,
      pvp: false,
    });
    expect(configDiff(fields, {})).toEqual({});
  });
});

describe('tickHealth', () => {
  const s = (tps: number, mspt: number) => ({
    server_id: 's',
    ts: '',
    cpu_percent: 0,
    ram_used_mb: 0,
    ram_max_mb: 0,
    tps,
    mspt,
    players: 0,
  });
  it('classifies the tick loop', () => {
    expect(tickHealth(s(20, 12))).toBe('good');
    expect(tickHealth(s(20, 40))).toBe('busy');
    expect(tickHealth(s(15, 66))).toBe('lagging');
    expect(tickHealth(undefined)).toBe('good');
  });
});

describe('purchasable', () => {
  const d = (amount: number | null, available = true): DomainAvailability => ({
    domain: 'x.net',
    available,
    premium: false,
    price: amount == null ? null : { amount_minor: amount, currency: 'USD' },
    renewal_price: null,
  });
  it('refuses taken names and anything over $100', () => {
    expect(purchasable(d(1108))).toBe(true);
    expect(purchasable(d(10_000))).toBe(true);
    expect(purchasable(d(24_900))).toBe(false);
    expect(purchasable(d(1108, false))).toBe(false);
    expect(purchasable(d(null))).toBe(false);
  });
});
