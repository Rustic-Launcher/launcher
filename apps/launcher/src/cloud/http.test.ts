import { describe, expect, it, vi } from 'vitest';
import { createMockCloud } from './mock';
import {
  ageGroup,
  createHttpCloud,
  currencyFor,
  mapFriend,
  mapLoadout,
  mapPresence,
  rosterEvent,
  toEquippedSlots,
} from './http';

const user = { id: 'u1', username: 'lea', display_name: 'Léa' };

describe('cloud http mapping', () => {
  it('maps presence and joinable activity', () => {
    const p = mapPresence({
      status: 'online',
      updated_at: '2026-11-10T10:00:00Z',
      activity: { kind: 'playing', server_address: 'a.example', server_name: 'A' },
    });
    expect(p.presence).toBe('in-game');
    expect(p.activity).toMatchObject({
      kind: 'server',
      joinable: true,
      server_address: 'a.example',
    });
    expect(mapPresence({ status: 'invisible', updated_at: 'x' }).presence).toBe('offline');
  });
  it('maps friends with favourites', () => {
    const f = mapFriend(
      { user, since: 'x', presence: { status: 'offline', updated_at: '2026-11-09T00:00:00Z' } },
      new Set(['u1']),
    );
    expect(f).toMatchObject({
      id: 'u1',
      favorite: true,
      presence: 'offline',
      last_seen_at: '2026-11-09T00:00:00Z',
    });
  });
  it('maps cosmetic slots both ways', () => {
    const l = mapLoadout({
      items: [
        { slot: 'head', item_id: 'i1' },
        { slot: 'shoulder_companion', item_id: 'i2' },
      ],
    });
    expect(l).toEqual({ head: 'i1', shoulder: 'i2' });
    expect(toEquippedSlots(l)).toEqual([
      { slot: 'head', item_id: 'i1' },
      { slot: 'shoulder_companion', item_id: 'i2' },
    ]);
  });
  it('derives age group and currency', () => {
    const now = new Date('2026-11-10');
    expect(ageGroup('2015-01-01', now)).toBe('child');
    expect(ageGroup('2010-01-01', now)).toBe('teen');
    expect(ageGroup(null, now)).toBe('adult');
    expect(currencyFor('CH')).toBe('CHF');
    expect(currencyFor('fr')).toBe('EUR');
  });
  it('refetches the roster when a friendship changes elsewhere', () => {
    // Regression (QA): an accepted friend request never appeared until a restart.
    expect(rosterEvent({ type: 'friend_added' })).toEqual({ type: 'friends-changed' });
    expect(rosterEvent({ type: 'friend_removed' })).toEqual({ type: 'friends-changed' });
    expect(rosterEvent({ type: 'message_received' })).toBeNull();
  });
  it('reads news and featured servers from the cloud, never the samples', async () => {
    // Regression (QA): a real build showed made-up featured servers, player counts and news.
    const calls: string[] = [];
    vi.stubGlobal('fetch', async (req: Request) => {
      calls.push(new URL(req.url).pathname);
      const body = req.url.includes('/v1/news')
        ? [
            {
              id: '0190',
              slug: 'launcher-0-2',
              source: 'rustic',
              kind: 'patch_notes',
              title: 'Launcher 0.2',
              summary: 'Faster',
              body: '',
              url: 'https://rustic-launcher.com/news/launcher-0-2',
              version: '0.2.0',
              locale: 'en',
              published_at: '2026-11-10T10:00:00Z',
            },
          ]
        : [
            {
              id: '0191',
              name: 'Craft FR',
              address: 'play.example.invalid',
              description: '',
              tags: ['survival'],
              engine: 'horizon',
              language: 'fr',
              verified: true,
              sort_order: 0,
              players_online: null,
              players_max: null,
            },
          ];
      return new Response(JSON.stringify(body), {
        headers: { 'content-type': 'application/json' },
      });
    });
    try {
      const real = createHttpCloud('http://api.test', createMockCloud());
      const news = await real.news();
      expect(news).toHaveLength(1);
      expect(news[0]).toMatchObject({
        title: 'Launcher 0.2',
        kind: 'patch_notes',
        version: '0.2.0',
      });
      const featured = await real.featuredServers();
      expect(featured[0]).toMatchObject({ name: 'Craft FR', players_online: null });
      expect(calls).toEqual(['/v1/news', '/v1/featured-servers']);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
