import { describe, expect, it } from 'vitest';
import { listingsDb, queryListings, trendScore } from '@/cloud/mock/servers';
import { validateServersSearch } from '../routes';
import { heroListings, sameAddress } from './queries';
import { languageName, TAGS } from './vocab';

describe('the Servers page URL', () => {
  it('keeps valid state and drops the rest', () => {
    expect(
      validateServersSearch({
        tab: 'browse',
        q: 'sky',
        tag: 'pvp',
        language: 'fr',
        engine: 'rustic',
        sort: 'players',
        verified: 'true',
      }),
    ).toEqual({
      tab: 'browse',
      q: 'sky',
      tag: 'pvp',
      language: 'fr',
      engine: 'rustic',
      sort: 'players',
      verified: true,
    });
    expect(
      validateServersSearch({
        tab: 'nope',
        tag: 'PvP!',
        language: 'fra',
        engine: 'paper',
        sort: 'downloads',
        verified: 'yes',
      }),
    ).toEqual({
      tab: undefined,
      q: undefined,
      tag: undefined,
      language: undefined,
      engine: undefined,
      sort: undefined,
      verified: undefined,
    });
  });

  it('opens Mine for the older multiplayer links', () => {
    expect(validateServersSearch({ tab: 'joined' }).tab).toBe('mine');
    expect(validateServersSearch({ tab: 'multiplayer' }).tab).toBe('mine');
  });
});

describe('directory helpers', () => {
  it('puts staff picks first in the hero, without duplicates', () => {
    const [a, b, c] = listingsDb;
    const picks = heroListings([a!, b!], [b!, c!], 6);
    expect(picks.map((l) => l.id)).toEqual([a!.id, b!.id, c!.id]);
    expect(heroListings([a!, b!], [c!], 2)).toHaveLength(2);
  });

  it('compares addresses like the multiplayer list', () => {
    expect(sameAddress('Play.Example.org:25565', 'play.example.org')).toBe(true);
    expect(sameAddress('play.example.org.', 'play.example.org')).toBe(true);
    expect(sameAddress('play.example.org:25570', 'play.example.org')).toBe(false);
  });

  it('names languages in the reader’s language', () => {
    expect(languageName('fr', 'en')).toBe('French');
    expect(languageName('en', 'fr')).toBe('anglais');
    expect(TAGS).toContain('skyblock');
  });
});

describe('the mock directory follows the API', () => {
  it('filters by text, tag, language, engine and verified', () => {
    expect(queryListings(listingsDb, { q: 'arena' }).items.map((l) => l.slug)).toEqual([
      'arena-nord',
    ]);
    const pvp = queryListings(listingsDb, { tag: 'pvp' }).items;
    expect(pvp.length).toBeGreaterThan(1);
    expect(pvp.every((l) => l.tags.includes('pvp'))).toBe(true);
    expect(
      queryListings(listingsDb, { language: 'fr', engine: 'horizon' }).items.every(
        (l) => l.languages.includes('fr') && l.engine === 'horizon',
      ),
    ).toBe(true);
    expect(queryListings(listingsDb, { verified: true }).items.every((l) => l.verified)).toBe(true);
  });

  it('sorts like the API and pages with has_more', () => {
    const trending = queryListings(listingsDb, { limit: 50 }).items;
    for (let i = 1; i < trending.length; i++)
      expect(trendScore(trending[i - 1]!)).toBeGreaterThanOrEqual(trendScore(trending[i]!));
    const players = queryListings(listingsDb, { sort: 'players', limit: 3 }).items;
    expect(players[0]!.slug).toBe('blockparty');
    const featured = queryListings(listingsDb, { featured: true }).items.map((l) => l.slug);
    expect(featured).toEqual(['arena-nord', 'skyreach', 'builders-guild']);
    const page = queryListings(listingsDb, { sort: 'name', limit: 5 });
    expect(page.items).toHaveLength(5);
    expect(page.has_more).toBe(true);
    expect(page.total).toBe(listingsDb.length);
    expect(page.items.every((l) => l.description === null)).toBe(true);
  });
});
