import type { LightStreamHandle } from '@rustic/ui/light-stream';
import { useEffect, useRef, useState } from 'react';

/** Hero beam preset at a calm intensity (the website's "hero" preset). */
const WEIGHTS: [number, number, number, number] = [1, 0, 0, 0];
const INTENSITY = 0.75;

type BatteryLike = EventTarget & { charging: boolean; level: number };

const reducedMotion = () =>
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Windows' battery saver turns on at 20 % by default: treat "discharging and ≤ 20 %" the same. */
export function batterySaver(b: { charging: boolean; level: number } | null): boolean {
  return !!b && !b.charging && b.level <= 0.2;
}

/** Whether the light should render right now. */
export function shouldRun(s: {
  reduced: boolean;
  running: boolean;
  hidden: boolean;
  saver: boolean;
}): boolean {
  return !s.reduced && !s.running && !s.hidden && !s.saver;
}

/**
 * The website's light stream behind the Launchpad hero (shared code: `@rustic/ui/light-stream`).
 * Loaded lazily after first paint; paused while an instance is running, while the window is hidden
 * or minimised, in battery saver; never started with reduced motion (the hero art stays).
 */
export function HeroLight({ running }: { running: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const handle = useRef<LightStreamHandle | null>(null);
  const [ready, setReady] = useState(false);
  const [hidden, setHidden] = useState(() => document.hidden);
  const [saver, setSaver] = useState(false);
  const reduced = reducedMotion();

  useEffect(() => {
    if (reduced) return;
    let cancelled = false;
    const start = async () => {
      try {
        const { createLightStream } = await import('@rustic/ui/light-stream');
        if (cancelled || !canvas.current) return;
        const h = createLightStream(canvas.current, true);
        h.setTarget(WEIGHTS, INTENSITY, 0);
        handle.current = h;
        setReady(true);
      } catch {
        // No WebGL: the hero art alone.
      }
    };
    const w = window as Window & { requestIdleCallback?: (cb: () => void) => number };
    if (w.requestIdleCallback) w.requestIdleCallback(() => void start());
    else window.setTimeout(() => void start(), 300);
    return () => {
      cancelled = true;
      handle.current?.dispose();
      handle.current = null;
    };
  }, [reduced]);

  useEffect(() => {
    const onVisibility = () => setHidden(document.hidden);
    document.addEventListener('visibilitychange', onVisibility);
    let battery: BatteryLike | null = null;
    const onBattery = () => setSaver(batterySaver(battery));
    const nav = navigator as Navigator & { getBattery?: () => Promise<BatteryLike> };
    void nav
      .getBattery?.()
      .then((b) => {
        battery = b;
        onBattery();
        b.addEventListener('chargingchange', onBattery);
        b.addEventListener('levelchange', onBattery);
      })
      .catch(() => undefined);
    return () => {
      document.removeEventListener('visibilitychange', onVisibility);
      battery?.removeEventListener('chargingchange', onBattery);
      battery?.removeEventListener('levelchange', onBattery);
    };
  }, []);

  const run = shouldRun({ reduced, running, hidden, saver });
  useEffect(() => {
    handle.current?.pause(!run);
  }, [run, ready]);

  if (reduced) return null;
  return (
    <canvas
      ref={canvas}
      aria-hidden
      data-testid="hero-light"
      data-state={ready ? (run ? 'running' : 'paused') : 'loading'}
      className="pointer-events-none absolute inset-0 size-full mix-blend-screen transition-opacity duration-700"
      style={{ opacity: ready && run ? 0.9 : ready ? 0.35 : 0 }}
    />
  );
}
