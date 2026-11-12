import { describe, expect, it } from 'vitest';
import { composeHead } from './skinHead';

/** A blank skin with one pixel set: [x, y, r, g, b, a]. */
function skin(height: 64 | 32, pixels: [number, number, number, number, number, number][]) {
  const data = new Uint8ClampedArray(64 * height * 4);
  for (const [x, y, r, g, b, a] of pixels) data.set([r, g, b, a], (y * 64 + x) * 4);
  return data;
}
const px = (face: Uint8ClampedArray, x: number, y: number) =>
  Array.from(face.slice((y * 8 + x) * 4, (y * 8 + x) * 4 + 4));

describe('composeHead', () => {
  it('takes the head front and makes it opaque', () => {
    const face = composeHead(
      skin(64, [
        [8, 8, 10, 20, 30, 0],
        [15, 15, 200, 100, 50, 255],
      ]),
      64,
      64,
    );
    expect(px(face, 0, 0)).toEqual([10, 20, 30, 255]);
    expect(px(face, 7, 7)).toEqual([200, 100, 50, 255]);
  });

  it('draws the hat layer over it, with its transparency', () => {
    const face = composeHead(
      skin(64, [
        [9, 8, 0, 0, 0, 255],
        [41, 8, 255, 255, 255, 255],
        [10, 8, 0, 0, 0, 255],
        [42, 8, 200, 0, 0, 128],
      ]),
      64,
      64,
    );
    expect(px(face, 1, 0)).toEqual([255, 255, 255, 255]);
    expect(px(face, 2, 0)).toEqual([100, 0, 0, 255]);
  });

  it('ignores a legacy 64×32 hat area without any transparent pixel', () => {
    const opaqueHat: [number, number, number, number, number, number][] = [];
    for (let y = 8; y < 16; y++) for (let x = 40; x < 48; x++) opaqueHat.push([x, y, 0, 0, 0, 255]);
    const face = composeHead(skin(32, [[8, 8, 50, 60, 70, 255], ...opaqueHat]), 64, 32);
    expect(px(face, 0, 0)).toEqual([50, 60, 70, 255]);
  });

  it('refuses an image that is not a skin', () => {
    expect(() => composeHead(new Uint8ClampedArray(16 * 16 * 4), 16, 16)).toThrow();
  });
});
