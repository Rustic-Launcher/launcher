import { describe, expect, it } from 'vitest';
import {
  clampRange,
  decodePosition,
  encodePosition,
  mix,
  moved,
  proximityGain,
  sanitizePosition,
  stereoPan,
  type GamePosition,
} from './proximity';
import { meterValue, micOpen, rmsDb, thresholdDb, VoiceGate } from './vad';
import { gameKeyOf, keyLabel } from './keys';

const at = (x: number, z: number, yaw = 0, dim = 'minecraft:overworld'): GamePosition => ({
  server: 'play.example.org',
  dim,
  x,
  y: 64,
  z,
  yaw,
});

describe('proximity', () => {
  it('fades to silence at the range', () => {
    expect(proximityGain(0, 48)).toBe(1);
    expect(proximityGain(7, 48)).toBe(1);
    expect(proximityGain(24, 48)).toBeGreaterThan(0.2);
    expect(proximityGain(24, 48)).toBeLessThan(0.6);
    expect(proximityGain(47.9, 48)).toBeLessThan(0.001);
    expect(proximityGain(48, 48)).toBe(0);
    expect(proximityGain(500, 48)).toBe(0);
    expect(proximityGain(Number.NaN, 48)).toBe(0);
    // Monotonic.
    let last = 1;
    for (let d = 0; d <= 48; d += 1) {
      const g = proximityGain(d, 48);
      expect(g).toBeLessThanOrEqual(last);
      last = g;
    }
  });

  it('clamps the range', () => {
    expect(clampRange(2)).toBe(8);
    expect(clampRange(1000)).toBe(128);
    expect(clampRange(Number.NaN)).toBe(48);
  });

  it('pans to the side the speaker stands on', () => {
    // Facing south (+Z): east (+X) is on the left, west (-X) on the right.
    expect(stereoPan(at(0, 0, 0), at(-10, 0))).toBeCloseTo(0.8);
    expect(stereoPan(at(0, 0, 0), at(10, 0))).toBeCloseTo(-0.8);
    expect(stereoPan(at(0, 0, 0), at(0, 10))).toBeCloseTo(0);
    // Facing west (yaw 90): north (-Z) is on the right.
    expect(stereoPan(at(0, 0, 90), at(0, -10))).toBeCloseTo(0.8);
    expect(stereoPan(at(0, 0, 0), at(0.1, 0.1))).toBe(0);
  });

  it('mixes by room kind', () => {
    const far = at(1000, 0);
    expect(mix({ proximity: false, strict: false, range: 48, self: null, other: null })).toEqual({
      gain: 1,
      pan: 0,
    });
    // A party member elsewhere is heard at full volume; a stranger on a server is not.
    expect(
      mix({ proximity: true, strict: false, range: 48, self: at(0, 0), other: null }).gain,
    ).toBe(1);
    expect(
      mix({ proximity: true, strict: true, range: 48, self: at(0, 0), other: null }).gain,
    ).toBe(0);
    expect(
      mix({
        proximity: true,
        strict: true,
        range: 48,
        self: at(0, 0),
        other: at(0, 0, 0, 'minecraft:the_nether'),
      }).gain,
    ).toBe(0);
    expect(
      mix({ proximity: true, strict: false, range: 48, self: at(0, 0), other: far }).gain,
    ).toBe(0);
    const near = mix({
      proximity: true,
      strict: true,
      range: 48,
      self: at(0, 0),
      other: at(-20, 0),
    });
    expect(near.gain).toBeGreaterThan(0);
    expect(near.pan).toBeGreaterThan(0);
  });

  it('validates positions from the room', () => {
    expect(
      sanitizePosition({
        server: 'A.Example.org',
        dim: 'minecraft:overworld',
        x: 1,
        y: 2,
        z: 3,
        yaw: -90,
      }),
    ).toEqual({
      server: 'a.example.org',
      dim: 'minecraft:overworld',
      x: 1,
      y: 2,
      z: 3,
      yaw: 270,
    });
    expect(sanitizePosition({ server: 'x', dim: 'd', x: Infinity, y: 0, z: 0 })).toBeNull();
    expect(sanitizePosition({ server: '<script>', dim: 'd', x: 0, y: 0, z: 0 })).toBeNull();
    expect(sanitizePosition({ server: 'x', dim: 'd', x: 4e7, y: 0, z: 0 })).toBeNull();
    expect(sanitizePosition('nope')).toBeNull();
    const p = at(5, 6, 30);
    expect(decodePosition(encodePosition(p))).toEqual(p);
    expect(decodePosition(encodePosition(null))).toBeNull();
    expect(decodePosition(new TextEncoder().encode('{"v":2,"p":null}'))).toBeUndefined();
    expect(decodePosition(new TextEncoder().encode('garbage'))).toBeUndefined();
    expect(decodePosition(new Uint8Array(2000))).toBeUndefined();
  });

  it('sends again only after a real move', () => {
    expect(moved(at(0, 0), at(0.1, 0))).toBe(false);
    expect(moved(at(0, 0), at(1, 0))).toBe(true);
    expect(moved(at(0, 0, 0), at(0, 0, 10))).toBe(true);
    expect(moved(at(0, 0, 359), at(0, 0, 1))).toBe(false);
    expect(moved(null, at(0, 0))).toBe(true);
    expect(moved(null, null)).toBe(false);
  });
});

describe('voice activity', () => {
  it('measures levels', () => {
    expect(rmsDb(new Float32Array(128))).toBe(-100);
    expect(rmsDb(new Float32Array(128).fill(1))).toBeCloseTo(0);
    expect(rmsDb(new Float32Array(128).fill(0.1))).toBeCloseTo(-20);
    expect(meterValue(-100)).toBe(0);
    expect(meterValue(0)).toBe(1);
  });

  it('maps the sensitivity to a threshold', () => {
    expect(thresholdDb(0)).toBe(-20);
    expect(thresholdDb(100)).toBe(-75);
    expect(thresholdDb(50)).toBeCloseTo(-47.5);
    expect(thresholdDb(500)).toBe(-75);
  });

  it('holds the gate open a little after the voice', () => {
    const g = new VoiceGate(300);
    expect(g.update(-60, -40, 0)).toBe(false);
    expect(g.update(-30, -40, 10)).toBe(true);
    expect(g.update(-60, -40, 200)).toBe(true);
    expect(g.update(-60, -40, 311)).toBe(false);
  });

  it('decides when the mic sends', () => {
    const base = {
      muted: false,
      deafened: false,
      mode: 'vad' as const,
      pttDown: false,
      gateOpen: true,
    };
    expect(micOpen(base)).toBe(true);
    expect(micOpen({ ...base, muted: true })).toBe(false);
    expect(micOpen({ ...base, deafened: true })).toBe(false);
    expect(micOpen({ ...base, mode: 'ptt' })).toBe(false);
    expect(micOpen({ ...base, mode: 'ptt', pttDown: true, gateOpen: false })).toBe(true);
  });
});

describe('push-to-talk keys', () => {
  it('maps browser key codes to the game key names', () => {
    expect(gameKeyOf('KeyV')).toBe('key.keyboard.v');
    expect(gameKeyOf('Digit4')).toBe('key.keyboard.4');
    expect(gameKeyOf('CapsLock')).toBe('key.keyboard.caps.lock');
    expect(gameKeyOf('AltLeft')).toBe('key.keyboard.left.alt');
    expect(gameKeyOf('F13')).toBe('key.keyboard.f13');
    expect(gameKeyOf('Backquote')).toBe('key.keyboard.grave.accent');
    expect(gameKeyOf('Unidentified')).toBeNull();
    expect(keyLabel('KeyV')).toBe('V');
    expect(keyLabel('AltLeft')).toBe('Left Alt');
  });
});
