/**
 * The hero video set (ADR 0179): which rendition of `GET /v1/media/hero` to play. Shared by the
 * launcher's Launchpad and the website's account pages. Pure functions, no DOM access except the
 * optional default codec probe.
 */

export type HeroCodec = 'av1' | 'vp9' | 'h264';

/** The fields of a rendition the pick needs (the API's `HeroRendition`). */
export interface HeroRenditionLike {
  width: number;
  height: number;
  codec: string;
  mime: string;
  url: string;
  bytes: number;
  sha256: string;
}

export interface HeroPosterLike {
  width: number;
  height: number;
  mime: string;
  url: string;
}

/** Best compression first; H.264 plays everywhere. */
export const HERO_CODECS: readonly HeroCodec[] = ['av1', 'vp9', 'h264'];

/** The device pixel ratio is capped: past 2, the eye gains nothing and the file is 2× bigger. */
export const MAX_DPR = 2;

export interface Viewport {
  /** CSS pixels of the box the video covers (`object-fit: cover`). */
  width: number;
  height: number;
  dpr: number;
}

/**
 * The device pixels a 16:9-ish rendition must have to cover the box without upscaling: `cover`
 * scales by the larger of the two ratios, so both sides count.
 */
export function neededHeight(view: Viewport, aspect = 16 / 9): number {
  const dpr = Math.min(Math.max(view.dpr || 1, 1), MAX_DPR);
  const w = Math.max(view.width, 1) * dpr;
  const h = Math.max(view.height, 1) * dpr;
  return Math.ceil(Math.max(h, w / aspect));
}

/** The smallest rendition tall enough for `need` (5 % below counts), else the tallest. */
function bySize<T extends HeroRenditionLike>(list: T[], need: number): T | null {
  if (!list.length) return null;
  const sorted = [...list].sort((a, b) => a.height - b.height || a.bytes - b.bytes);
  // 5 % under the need is invisible behind the overlays and saves a whole step.
  return sorted.find((r) => r.height >= need * 0.95) ?? sorted[sorted.length - 1]!;
}

/**
 * The rendition to play: the first codec (AV1, VP9, H.264) this engine plays, at the smallest
 * size that covers the viewport. `null` when nothing plays.
 */
export function pickHeroRendition<T extends HeroRenditionLike>(
  renditions: T[],
  view: Viewport,
  canPlay: (mime: string) => boolean,
): T | null {
  const need = neededHeight(view);
  for (const codec of HERO_CODECS) {
    const list = renditions.filter((r) => r.codec === codec && canPlay(r.mime));
    const pick = bySize(list, need);
    if (pick) return pick;
  }
  return null;
}

/**
 * One `<source>` per codec, best first, each at the size the viewport needs: for a plain
 * `<video>` whose engine picks the first type it plays (the website).
 */
export function heroSources<T extends HeroRenditionLike>(
  renditions: T[],
  view: Viewport,
): { src: string; type: string }[] {
  const need = neededHeight(view);
  return HERO_CODECS.flatMap((codec) => {
    const r = bySize(
      renditions.filter((x) => x.codec === codec),
      need,
    );
    return r ? [{ src: r.url, type: r.mime }] : [];
  });
}

/** The poster closest above the need (else the largest); WebP/JPEG before AVIF for reach. */
export function pickHeroPoster<T extends HeroPosterLike>(posters: T[], view: Viewport): T | null {
  const need = neededHeight(view);
  const sorted = [...posters].sort((a, b) => a.height - b.height);
  return sorted.find((p) => p.height >= need * 0.95) ?? sorted[sorted.length - 1] ?? null;
}

/**
 * Whether this engine plays `mime` (codecs included): Media Source's answer when it has one
 * (exact), else `canPlayType` ("probably", or "maybe" for a type with codecs).
 */
export function canPlayMime(mime: string): boolean {
  try {
    const ms = (globalThis as { MediaSource?: { isTypeSupported?: (t: string) => boolean } })
      .MediaSource;
    if (ms?.isTypeSupported) return ms.isTypeSupported(mime);
    if (typeof document === 'undefined') return false;
    return document.createElement('video').canPlayType(mime) !== '';
  } catch {
    return false;
  }
}

/** The browser asks to save data, or reports a cellular link: no big download then. */
export function saveDataRequested(): boolean {
  const c = (
    globalThis.navigator as
      (Navigator & { connection?: { saveData?: boolean; type?: string } }) | undefined
  )?.connection;
  return !!c && (c.saveData === true || c.type === 'cellular');
}
