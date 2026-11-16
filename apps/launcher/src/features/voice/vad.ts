/**
 * Voice activity detection (ADR 0176): the microphone opens when its level passes a threshold
 * set by the sensitivity slider, and stays open a little after the voice stops (hangover), so
 * words are not cut. Pure; the engine feeds it the level of an analyser on a copy of the mic.
 */

/** RMS level of a block of samples (-1..1) in dBFS (-100 for silence). */
export function rmsDb(samples: Float32Array): number {
  let sum = 0;
  for (const s of samples) sum += s * s;
  const rms = Math.sqrt(sum / Math.max(1, samples.length));
  return rms > 0 ? Math.max(-100, 20 * Math.log10(rms)) : -100;
}

/** Sensitivity 0-100 to a threshold: 0 needs a loud voice (-20 dBFS), 100 opens on a whisper (-75). */
export function thresholdDb(sensitivity: number): number {
  const s = Math.min(100, Math.max(0, Number.isFinite(sensitivity) ? sensitivity : 50));
  return -20 - (s / 100) * 55;
}

/** A level in dBFS as 0-1 for a meter (-100 → 0, 0 → 1). */
export function meterValue(db: number): number {
  return Math.min(1, Math.max(0, (db + 100) / 100));
}

export class VoiceGate {
  private openUntil = -Infinity;

  constructor(private readonly hangoverMs = 350) {}

  /** Feeds one level; returns whether the mic is open now. */
  update(db: number, threshold: number, now: number): boolean {
    if (db >= threshold) this.openUntil = now + this.hangoverMs;
    return now < this.openUntil;
  }

  reset(): void {
    this.openUntil = -Infinity;
  }
}

export type TalkMode = 'vad' | 'ptt';

/** Whether the microphone sends: muted never, push-to-talk while held, else the gate. */
export function micOpen(o: {
  muted: boolean;
  deafened: boolean;
  mode: TalkMode;
  pttDown: boolean;
  gateOpen: boolean;
}): boolean {
  if (o.muted || o.deafened) return false;
  return o.mode === 'ptt' ? o.pttDown : o.gateOpen;
}
