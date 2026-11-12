import { describe, expect, it } from 'vitest';
import { faceRect, faceST, skinBoxGeometry, skinParts } from './skin';

/** The pixel bounds a face's UVs cover, for a box built by skinBoxGeometry. */
function faceBounds(
  g: ReturnType<typeof skinBoxGeometry>,
  normal: [number, number, number],
  texH = 64,
) {
  const nor = g.getAttribute('normal');
  const uv = g.getAttribute('uv');
  const xs: number[] = [];
  const ys: number[] = [];
  for (let i = 0; i < nor.count; i++) {
    if (
      Math.round(nor.getX(i)) === normal[0] &&
      Math.round(nor.getY(i)) === normal[1] &&
      Math.round(nor.getZ(i)) === normal[2]
    ) {
      xs.push(Math.round(uv.getX(i) * 64));
      ys.push(Math.round((1 - uv.getY(i)) * texH));
    }
  }
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

describe('skin layout', () => {
  const parts = skinParts(false);

  it('puts each face of the head on its square of the unfolded texture', () => {
    const g = skinBoxGeometry(parts.head.base, 1 / 16);
    expect(faceBounds(g, [0, 0, 1])).toEqual([8, 8, 16, 16]); // front
    expect(faceBounds(g, [0, 0, -1])).toEqual([24, 8, 32, 16]); // back
    expect(faceBounds(g, [-1, 0, 0])).toEqual([0, 8, 8, 16]); // right side
    expect(faceBounds(g, [1, 0, 0])).toEqual([16, 8, 24, 16]); // left side
    expect(faceBounds(g, [0, 1, 0])).toEqual([8, 0, 16, 8]); // top
    expect(faceBounds(g, [0, -1, 0])).toEqual([16, 0, 24, 8]); // bottom
  });

  it('the outer layer is a grown box on the same squares', () => {
    const g = skinBoxGeometry(parts.head.outer, 1 / 16, 0.5);
    g.computeBoundingBox();
    expect(g.boundingBox!.max.x * 16).toBeCloseTo(4.5);
    expect(faceBounds(g, [0, 0, 1])).toEqual([40, 8, 48, 16]);
  });

  it('slim arms are three pixels wide', () => {
    const slim = skinParts(true);
    expect(slim.rightArm.base.w).toBe(3);
    expect(faceRect(slim.rightArm.base, [0, 0, 1])).toEqual([44, 20, 3, 12]);
    expect(faceRect(slim.rightArm.base, [0, 0, -1])).toEqual([51, 20, 3, 12]);
  });

  it('a legacy 64×32 skin maps onto the upper half', () => {
    const g = skinBoxGeometry(parts.body.base, 1 / 16, 0, 32);
    expect(faceBounds(g, [0, 0, 1], 32)).toEqual([20, 20, 28, 32]);
  });

  it('faces read as seen from outside: the front edge of each side meets the front', () => {
    const size: [number, number, number] = [8, 8, 8];
    // A point on the front edge of the right side (−X) is at the rectangle's right (s = 1).
    expect(faceST([-1, 0, 0], [-4, 0, 4], size)[0]).toBeCloseTo(1);
    // ...and on the left side (+X) at its left (s = 0).
    expect(faceST([1, 0, 0], [4, 0, 4], size)[0]).toBeCloseTo(0);
    // The top's lower edge is the front.
    expect(faceST([0, 1, 0], [0, 4, 4], size)[1]).toBeCloseTo(0);
  });
});
