import { describe, expect, it } from 'vitest';
import { parseContentLink, parsePageLink, parseServerLink } from './deepLink';

describe('parseContentLink (mirror of the desktop parser)', () => {
  it('reads project links with an optional version', () => {
    expect(parseContentLink('rustic://content/rustic/sky_islands')).toMatchObject({
      action: 'content',
      source: 'rustic',
      project: 'sky_islands',
      version: null,
    });
    expect(parseContentLink('rustic://content/rustic/sky-islands/')?.project).toBe('sky-islands');
    expect(parseContentLink('rustic://content/rustic/sky?version=sky%401.2.0%2Bb3')?.version).toBe(
      'sky@1.2.0+b3',
    );
  });

  it('refuses anything else', () => {
    for (const bad of [
      'rustic://join/play.example.com',
      'rustic://content/rustic',
      'rustic://content/modrinth/AANobbMI',
      'rustic://content/rustic/Sky',
      'rustic://content/rustic/sky/extra',
      'rustic://content/rustic/sky%2F..',
      'rustic://content/rustic/sky#x',
      'rustic://content/rustic/sky?install=1',
      'rustic://content/rustic/sky?version=other@1.0',
      'rustic://content/rustic/sky?version=sky@1.0&version=sky@1.1',
      'rustic://content/rustic/sky?version=sky@1.0%20x',
      `rustic://content/rustic/${'a'.repeat(65)}`,
      'rustic://content/rustic/%E0%A4%A',
    ]) {
      expect(parseContentLink(bad), bad).toBeNull();
    }
  });
});

describe('parseServerLink (mirror of the desktop parser)', () => {
  it('reads server links by slug or id', () => {
    expect(parseServerLink('rustic://server/arena-nord')).toEqual({
      action: 'server',
      listing: 'arena-nord',
      url: 'rustic://server/arena-nord',
    });
    expect(parseServerLink('rustic://server/arena-nord/')?.listing).toBe('arena-nord');
    expect(parseServerLink('rustic://server/0190f0aa-0000-7000-8000-000000000001')?.listing).toBe(
      '0190f0aa-0000-7000-8000-000000000001',
    );
    expect(parsePageLink('rustic://server/skyreach')?.action).toBe('server');
    expect(parsePageLink('rustic://content/rustic/sky')?.action).toBe('content');
  });

  it('refuses anything else', () => {
    for (const bad of [
      'rustic://server/',
      'rustic://server/ab',
      'rustic://server/Arena',
      'rustic://server/arena_nord',
      'rustic://server/-arena',
      'rustic://server/arena/extra',
      'rustic://server/arena?join=1',
      'rustic://server/arena#x',
      'rustic://server/%2E%2E',
      'rustic://server/arena%2Fx',
      'rustic://join/arena-nord.fr',
      `rustic://server/${'a'.repeat(49)}`,
    ])
      expect(parseServerLink(bad), bad).toBeNull();
  });
});
