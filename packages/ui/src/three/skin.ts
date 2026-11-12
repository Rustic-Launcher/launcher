/**
 * A player skin on box geometry: the public 64×64 skin layout (and the legacy 64×32 one) mapped
 * onto each cuboid's six faces. Written from the format's documentation (each part's texture is
 * an unfolded box: top and bottom on the first row, then right, front, left and back sides).
 */
import { BoxGeometry, NearestFilter, SRGBColorSpace, TextureLoader, type Texture } from 'three';

/** A box part: its size in skin pixels (w × h × d) and its unfolded texture's corner (u, v). */
export interface SkinPart {
  w: number;
  h: number;
  d: number;
  u: number;
  v: number;
}

/** Texture corners of each part, base and outer layer (64×64 skins). */
export function skinParts(slim: boolean) {
  const arm = slim ? 3 : 4;
  return {
    head: { base: part(8, 8, 8, 0, 0), outer: part(8, 8, 8, 32, 0) },
    body: { base: part(8, 12, 4, 16, 16), outer: part(8, 12, 4, 16, 32) },
    /** The player's right arm and leg are on the model's −X side (it faces +Z). */
    rightArm: { base: part(arm, 12, 4, 40, 16), outer: part(arm, 12, 4, 40, 32) },
    leftArm: { base: part(arm, 12, 4, 32, 48), outer: part(arm, 12, 4, 48, 48) },
    rightLeg: { base: part(4, 12, 4, 0, 16), outer: part(4, 12, 4, 0, 32) },
    leftLeg: { base: part(4, 12, 4, 16, 48), outer: part(4, 12, 4, 0, 48) },
  };
}

function part(w: number, h: number, d: number, u: number, v: number): SkinPart {
  return { w, h, d, u, v };
}

/** The pixel rectangle (x, y, width, height) of a part's face, by the box's outward normal. */
export function faceRect(
  p: SkinPart,
  normal: [number, number, number],
): [number, number, number, number] {
  const { w, h, d, u, v } = p;
  const [nx, ny, nz] = normal;
  if (ny > 0) return [u + d, v, w, d]; // top
  if (ny < 0) return [u + d + w, v, w, d]; // bottom
  if (nx < 0) return [u, v + d, d, h]; // the model's right side
  if (nz > 0) return [u + d, v + d, w, h]; // front
  if (nx > 0) return [u + d + w, v + d, d, h]; // the model's left side
  return [u + 2 * d + w, v + d, w, h]; // back
}

/**
 * Where a point of a face falls inside its rectangle, 0..1 from the rectangle's left (s) and
 * bottom (t), for a box of size (w, h, d) centred on the origin. Each face is unfolded as seen
 * from outside; the top's upper edge is the back, the bottom is unfolded the same way.
 */
export function faceST(
  normal: [number, number, number],
  [x, y, z]: [number, number, number],
  [w, h, d]: [number, number, number],
): [number, number] {
  const [nx, ny, nz] = normal;
  const sx = x / w + 0.5;
  const ty = y / h + 0.5;
  const fromBack = 0.5 - z / d; // 0 at the front edge, 1 at the back
  if (ny !== 0) return [sx, fromBack];
  if (nz > 0) return [sx, ty];
  if (nz < 0) return [1 - sx, ty];
  if (nx > 0) return [fromBack, ty];
  return [1 - fromBack, ty];
}

/**
 * A box of `p`'s size in world units (`unit` per skin pixel), grown by `inflate` pixels on each
 * side (the outer layer), with its UVs on the skin texture (`texH` 64 or 32 pixels high).
 */
export function skinBoxGeometry(p: SkinPart, unit: number, inflate = 0, texH = 64): BoxGeometry {
  const size: [number, number, number] = [p.w + inflate * 2, p.h + inflate * 2, p.d + inflate * 2];
  const g = new BoxGeometry(size[0] * unit, size[1] * unit, size[2] * unit);
  const pos = g.getAttribute('position');
  const nor = g.getAttribute('normal');
  const uv = g.getAttribute('uv');
  for (let i = 0; i < pos.count; i++) {
    const n: [number, number, number] = [
      Math.round(nor.getX(i)),
      Math.round(nor.getY(i)),
      Math.round(nor.getZ(i)),
    ];
    const pt: [number, number, number] = [
      pos.getX(i) / unit,
      pos.getY(i) / unit,
      pos.getZ(i) / unit,
    ];
    const [s, t] = faceST(n, pt, size);
    const [rx, ry, rw, rh] = faceRect(p, n);
    // Image rows grow downwards; texture v grows upwards (flipY).
    uv.setXY(i, (rx + s * rw) / 64, 1 - (ry + (1 - t) * rh) / texH);
  }
  uv.needsUpdate = true;
  return g;
}

/** Loads a skin PNG as a crisp (nearest-filtered) texture; rejects on a failed or odd image. */
export function loadSkinTexture(url: string): Promise<Texture> {
  return new Promise((resolve, reject) => {
    const loader = new TextureLoader();
    loader.setCrossOrigin('anonymous');
    loader.load(
      url,
      (tex) => {
        const img = tex.image as { width?: number; height?: number } | undefined;
        if (img?.width !== 64 || (img.height !== 64 && img.height !== 32)) {
          tex.dispose();
          reject(new Error('not a 64×64 or 64×32 skin'));
          return;
        }
        tex.magFilter = NearestFilter;
        tex.minFilter = NearestFilter;
        tex.generateMipmaps = false;
        tex.colorSpace = SRGBColorSpace;
        resolve(tex);
      },
      undefined,
      () => reject(new Error(`skin not loaded: ${url}`)),
    );
  });
}
