import { describe, expect, it } from 'vitest';
import {
  heroSources,
  neededHeight,
  pickHeroPoster,
  pickHeroRendition,
  type HeroRenditionLike,
} from './heroMedia';

const MIME = {
  av1: 'video/mp4; codecs="av01.0.08M.08"',
  vp9: 'video/webm; codecs="vp09.00.40.08"',
  h264: 'video/mp4; codecs="avc1.640028"',
} as const;

const SIZES = [
  [854, 480],
  [1280, 720],
  [1920, 1080],
  [2560, 1440],
  [3840, 2160],
] as const;

const SET: HeroRenditionLike[] = (['av1', 'vp9', 'h264'] as const).flatMap((codec) =>
  SIZES.map(([width, height]) => ({
    width,
    height,
    codec,
    mime: MIME[codec],
    url: `https://cdn.test/media/hero/abc/hero-${height}p-${codec}.mp4`,
    bytes: height * 1000,
    sha256: 'a'.repeat(64),
  })),
);

const all = () => true;

describe('neededHeight', () => {
  it('covers both sides of the box, with the pixel ratio capped at 2', () => {
    expect(neededHeight({ width: 1280, height: 720, dpr: 1 })).toBe(720);
    // a tall box: its height decides
    expect(neededHeight({ width: 800, height: 900, dpr: 1 })).toBe(900);
    // a wide box: its width decides (16:9)
    expect(neededHeight({ width: 1920, height: 600, dpr: 1 })).toBe(1080);
    expect(neededHeight({ width: 1280, height: 720, dpr: 3 })).toBe(1440);
    expect(neededHeight({ width: 1280, height: 720, dpr: 0 })).toBe(720);
  });
});

describe('pickHeroRendition', () => {
  it('prefers AV1, then VP9, then H.264, by what the engine plays', () => {
    const view = { width: 1280, height: 800, dpr: 1 };
    expect(pickHeroRendition(SET, view, all)?.codec).toBe('av1');
    expect(pickHeroRendition(SET, view, (m) => !m.includes('av01'))?.codec).toBe('vp9');
    expect(pickHeroRendition(SET, view, (m) => m.includes('avc1'))?.codec).toBe('h264');
    expect(pickHeroRendition(SET, view, () => false)).toBeNull();
  });

  it('takes the smallest size that covers the window', () => {
    const h = (width: number, height: number, dpr: number) =>
      pickHeroRendition(SET, { width, height, dpr }, all)?.height;
    expect(h(1280, 800, 1)).toBe(1080); // needs 800
    expect(h(1024, 640, 1)).toBe(720); // needs 640
    expect(h(1280, 720, 1.5)).toBe(1080);
    expect(h(1600, 900, 1.5)).toBe(1440); // needs 1350
    expect(h(1920, 1080, 2)).toBe(2160);
    expect(h(5120, 2880, 2)).toBe(2160); // nothing bigger: the largest
    expect(h(400, 300, 1)).toBe(480);
    // 5 % under the need is close enough, more is not
    expect(h(1920, 1130, 1)).toBe(1080);
    expect(h(1920, 1180, 1)).toBe(1440);
  });

  it('falls back to another codec only when none of the preferred one plays', () => {
    const onlyBigAv1 = SET.filter((r) => r.codec !== 'av1' || r.height === 2160);
    expect(pickHeroRendition(onlyBigAv1, { width: 1280, height: 720, dpr: 1 }, all)).toMatchObject({
      codec: 'av1',
      height: 2160,
    });
  });
});

describe('heroSources', () => {
  it('lists one source per codec, best first, at the needed size', () => {
    const s = heroSources(SET, { width: 1440, height: 900, dpr: 1 });
    expect(s.map((x) => x.type)).toEqual([MIME.av1, MIME.vp9, MIME.h264]);
    expect(s.every((x) => x.src.includes('-1080p-'))).toBe(true);
    expect(heroSources([], { width: 1, height: 1, dpr: 1 })).toEqual([]);
  });
});

describe('pickHeroPoster', () => {
  it('takes the poster that covers the box', () => {
    const posters = [720, 1080, 2160].map((height) => ({
      width: (height * 16) / 9,
      height,
      mime: 'image/webp',
      url: `p-${height}.webp`,
    }));
    expect(pickHeroPoster(posters, { width: 1280, height: 720, dpr: 1 })?.height).toBe(720);
    expect(pickHeroPoster(posters, { width: 1280, height: 720, dpr: 2 })?.height).toBe(2160);
    expect(pickHeroPoster([], { width: 1, height: 1, dpr: 1 })).toBeNull();
  });
});
