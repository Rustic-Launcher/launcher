/**
 * A player's face from their skin, for avatars: the head's front (8×8 at 8,8 of the public skin
 * layout) with the outer "hat" layer (8×8 at 40,8) drawn over it.
 */
import { useEffect, useState } from 'react';

/** RGBA pixels of the 8×8 face, from a skin's RGBA pixels (`width` 64, `height` 64 or 32). */
export function composeHead(
  src: Uint8ClampedArray,
  width: number,
  height: number,
): Uint8ClampedArray<ArrayBuffer> {
  if (width !== 64 || (height !== 64 && height !== 32) || src.length < width * height * 4) {
    throw new Error('not a 64×64 or 64×32 skin');
  }
  const at = (x: number, y: number) => (y * width + x) * 4;
  // Legacy 64×32 skins often fill the hat area with an opaque colour that was never meant to show:
  // the game ignores a hat layer without any transparent pixel there, and so do we.
  let hatHasTransparency = height === 64;
  for (let y = 8; y < 16 && !hatHasTransparency; y++) {
    for (let x = 40; x < 48; x++) {
      if (src[at(x, y) + 3]! < 255) {
        hatHasTransparency = true;
        break;
      }
    }
  }
  const out = new Uint8ClampedArray(new ArrayBuffer(8 * 8 * 4));
  for (let y = 0; y < 8; y++) {
    for (let x = 0; x < 8; x++) {
      const o = (y * 8 + x) * 4;
      const b = at(8 + x, 8 + y);
      // The base layer is opaque.
      out[o] = src[b]!;
      out[o + 1] = src[b + 1]!;
      out[o + 2] = src[b + 2]!;
      out[o + 3] = 255;
      if (!hatHasTransparency) continue;
      const h = at(40 + x, 8 + y);
      const a = src[h + 3]! / 255;
      if (a === 0) continue;
      for (let c = 0; c < 3; c++) out[o + c] = Math.round(src[h + c]! * a + out[o + c]! * (1 - a));
    }
  }
  return out;
}

const cache = new Map<string, Promise<string>>();

/** A PNG data URL of the face (8×8, scale it with `image-rendering: pixelated`). Cached per URL. */
export function skinHeadUrl(url: string): Promise<string> {
  let p = cache.get(url);
  if (!p) {
    p = new Promise<string>((resolve, reject) => {
      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.onload = () => {
        try {
          const canvas = document.createElement('canvas');
          canvas.width = img.naturalWidth;
          canvas.height = img.naturalHeight;
          const ctx = canvas.getContext('2d');
          if (!ctx) throw new Error('no 2D canvas');
          ctx.drawImage(img, 0, 0);
          const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);
          const face = composeHead(data, width, height);
          const out = document.createElement('canvas');
          out.width = 8;
          out.height = 8;
          const octx = out.getContext('2d');
          if (!octx) throw new Error('no 2D canvas');
          octx.putImageData(new ImageData(face, 8, 8), 0, 0);
          resolve(out.toDataURL('image/png'));
        } catch (e) {
          reject(e instanceof Error ? e : new Error(String(e)));
        }
      };
      img.onerror = () => reject(new Error(`skin not loaded: ${url}`));
      img.src = url;
    });
    // A failure is not cached: the next mount tries again (e.g. back online).
    p.catch(() => cache.delete(url));
    cache.set(url, p);
  }
  return p;
}

/** The face's data URL for a skin URL, or `undefined` while loading, without a skin or on failure. */
export function useSkinHead(url: string | null | undefined): string | undefined {
  const [head, setHead] = useState<{ url: string; data: string }>();
  useEffect(() => {
    if (!url) return;
    let live = true;
    skinHeadUrl(url).then(
      (data) => live && setHead({ url, data }),
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [url]);
  return url && head?.url === url ? head.data : undefined;
}
