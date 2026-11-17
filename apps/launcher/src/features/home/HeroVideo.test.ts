import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type * as Ui from '@rustic/ui';
import type { HeroMedia } from '@/cloud/types';

type UiModule = typeof Ui;

const invoke = vi.fn();
let native = false;
vi.mock('@/ipc', () => ({
  invoke: (...a: unknown[]) => invoke(...a),
  isNative: () => native,
}));
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (p: string) => `http://asset.localhost/${encodeURIComponent(p)}`,
}));
vi.mock('@rustic/ui', async (orig) => ({
  ...(await orig<UiModule>()),
  // happy-dom plays nothing: say AV1 and H.264 play, VP9 not.
  canPlayMime: (m: string) => m.includes('av01') || m.includes('avc1'),
}));

const { hdSource, readLastSet } = await import('./HeroVideo');

const r = (codec: string, height: number, mime: string) => ({
  width: Math.round((height * 16) / 9),
  height,
  codec,
  mime,
  url: `https://cdn.rustic-launcher.com/media/hero/0123456789ab/hero-${height}p-${codec}.mp4`,
  bytes: height * 10_000,
  sha256: 'a'.repeat(64),
});

const SET: HeroMedia = {
  id: '0123456789ab',
  duration_ms: 21_955,
  renditions: [720, 1080, 1440, 2160].flatMap((h) => [
    r('av1', h, 'video/mp4; codecs="av01.0.08M.08"'),
    r('vp9', h, 'video/webm; codecs="vp09.00.40.08"'),
    r('h264', h, 'video/mp4; codecs="avc1.640028"'),
  ]),
  posters: [],
  credit: { title: 't', author: 'a', source_url: 'https://x.test', licence: 'Pexels License' },
};

function setWindow(width: number, height: number, dpr: number, online = true) {
  Object.defineProperty(window, 'innerWidth', { value: width, configurable: true });
  Object.defineProperty(window, 'innerHeight', { value: height, configurable: true });
  Object.defineProperty(window, 'devicePixelRatio', { value: dpr, configurable: true });
  Object.defineProperty(navigator, 'onLine', { value: online, configurable: true });
}

beforeEach(() => {
  invoke.mockReset();
  native = false;
  setWindow(1280, 800, 1);
});
afterEach(() => localStorage.clear());

describe('Launchpad hero video', () => {
  it('streams the best codec at the window size in a browser', async () => {
    expect(await hdSource(SET)).toEqual({
      src: SET.renditions.find((x) => x.codec === 'av1' && x.height === 1080)!.url,
      type: 'video/mp4; codecs="av01.0.08M.08"',
    });
    setWindow(1600, 900, 2);
    expect((await hdSource(SET))?.src).toContain('-2160p-av1');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('streams nothing offline in a browser', async () => {
    setWindow(1280, 800, 1, false);
    expect(await hdSource(SET)).toBeNull();
  });

  it('plays the cached copy in the desktop app, downloading only when online', async () => {
    native = true;
    invoke.mockResolvedValue('C:\\cache\\hero-media\\aaa.mp4');
    const s = await hdSource(SET);
    expect(invoke).toHaveBeenCalledWith('hero_media_fetch', {
      url: expect.stringContaining('-1080p-av1.mp4'),
      sha256: 'a'.repeat(64),
      bytes: 1080 * 10_000,
      download: true,
    });
    expect(s?.src).toMatch(/^http:\/\/asset\.localhost\//);
    setWindow(1280, 800, 1, false);
    invoke.mockResolvedValue(null);
    expect(await hdSource(SET)).toBeNull();
    expect(invoke).toHaveBeenLastCalledWith(
      'hero_media_fetch',
      expect.objectContaining({ download: false }),
    );
    invoke.mockRejectedValue(new Error('network'));
    expect(await hdSource(SET)).toBeNull();
  });

  it('remembers only a well-formed set', () => {
    expect(readLastSet()).toBeNull();
    localStorage.setItem('rustic.launcher.heroMedia', '{"renditions":3}');
    expect(readLastSet()).toBeNull();
    localStorage.setItem('rustic.launcher.heroMedia', JSON.stringify(SET));
    expect(readLastSet()?.id).toBe('0123456789ab');
  });
});
