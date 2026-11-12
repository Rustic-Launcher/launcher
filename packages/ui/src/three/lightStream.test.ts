import { describe, expect, it } from 'vitest';
import { bezierPoint, HERO_PATH, maxTurnDegrees, SEGMENTS } from './lightStream';

describe('light stream curves', () => {
  it('the hero spine is one smooth curve: no joint turns more than 0.6° when tessellated', () => {
    for (const aspect of [16 / 9, 21 / 9, 4 / 3, 9 / 16]) {
      expect(maxTurnDegrees(SEGMENTS.high, aspect)).toBeLessThan(0.6);
      expect(maxTurnDegrees(SEGMENTS.low, aspect)).toBeLessThan(0.6);
    }
  });

  it('keeps the vertex budget close to the original light (it runs on software GL too)', () => {
    // Before the smooth spine: 110 / 64 segments with two shape evaluations per vertex.
    expect(SEGMENTS.high).toBeLessThanOrEqual(128);
    expect(SEGMENTS.low).toBeLessThanOrEqual(80);
  });

  it('the old polyline through a node would have shown a corner', () => {
    // Two straight halves meeting at the node: a single joint carries the whole turn.
    const poly = (t: number): [number, number] => {
      const a: [number, number] = [-0.25, -1.35];
      const n: [number, number] = [0.4, 0.1];
      const b: [number, number] = [1.45, 0.92];
      const k = t < 0.62 ? t / 0.62 : (t - 0.62) / 0.38;
      const [p, q] = t < 0.62 ? [a, n] : [n, b];
      return [p[0] + (q[0] - p[0]) * k, p[1] + (q[1] - p[1]) * k];
    };
    let max = 0;
    let prev: [number, number] | null = null;
    let last = poly(0);
    for (let i = 1; i <= 110; i++) {
      const q = poly(i / 110);
      const d: [number, number] = [(q[0] - last[0]) * (16 / 9), q[1] - last[1]];
      if (prev) {
        const c = (prev[0] * d[0] + prev[1] * d[1]) / (Math.hypot(...prev) * Math.hypot(...d));
        max = Math.max(max, (Math.acos(Math.min(1, c)) * 180) / Math.PI);
      }
      prev = d;
      last = q;
    }
    expect(max).toBeGreaterThan(20);
  });

  it('the spine starts below the screen and leaves it at the top right', () => {
    expect(bezierPoint(HERO_PATH, 0)[1]).toBeLessThan(-1);
    const end = bezierPoint(HERO_PATH, 1);
    expect(end[0]).toBeGreaterThan(1);
    expect(end[1]).toBeGreaterThan(0.9);
  });
});
