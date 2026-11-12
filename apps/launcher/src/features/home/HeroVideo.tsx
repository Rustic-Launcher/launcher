import { useEffect, useRef, useState } from 'react';
import { HeroArt } from './HeroArt';
import { batterySaver, shouldRun } from './HeroLight';

/** The Launchpad's looping forest video (`public/media/hero-forest.*`, see docs/THIRD_PARTY.md). */
export const HERO_VIDEO = {
  webm: '/media/hero-forest.webm',
  mp4: '/media/hero-forest.mp4',
  poster: '/media/hero-forest.webp',
} as const;

type BatteryLike = EventTarget & { charging: boolean; level: number };

const reducedMotion = () =>
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * The hero's backdrop: a muted, looping video behind the launch button. Same rest policy as the
 * light stream it replaces (`shouldRun`): paused while an instance runs, while the window is
 * hidden or minimised and in battery saver; the still poster with reduced motion. If the video
 * cannot play at all, the original SVG illustration takes its place.
 */
export function HeroVideo({ running }: { running: boolean }) {
  const video = useRef<HTMLVideoElement>(null);
  const [failed, setFailed] = useState(false);
  const [hidden, setHidden] = useState(() => document.hidden);
  const [saver, setSaver] = useState(false);
  const reduced = reducedMotion();
  const play = shouldRun({ reduced, running, hidden, saver });

  useEffect(() => {
    const onVisibility = () => setHidden(document.hidden);
    document.addEventListener('visibilitychange', onVisibility);
    return () => document.removeEventListener('visibilitychange', onVisibility);
  }, []);

  useEffect(() => {
    const nav = navigator as Navigator & { getBattery?: () => Promise<BatteryLike> };
    let battery: BatteryLike | null = null;
    const update = () => setSaver(batterySaver(battery));
    void nav
      .getBattery?.()
      .then((b) => {
        battery = b;
        update();
        b.addEventListener('chargingchange', update);
        b.addEventListener('levelchange', update);
      })
      .catch(() => undefined);
    return () => {
      battery?.removeEventListener('chargingchange', update);
      battery?.removeEventListener('levelchange', update);
    };
  }, []);

  useEffect(() => {
    const v = video.current;
    if (!v) return;
    if (play) void v.play().catch(() => undefined);
    else v.pause();
  }, [play]);

  if (failed) return <HeroArt />;
  return (
    <div
      aria-hidden
      className="absolute inset-0 overflow-hidden"
      data-testid="hero-video"
      data-state={reduced ? 'still' : play ? 'playing' : 'paused'}
    >
      {reduced ? (
        <img
          src={HERO_VIDEO.poster}
          alt=""
          className="size-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <video
          ref={video}
          className="size-full object-cover"
          poster={HERO_VIDEO.poster}
          autoPlay={play}
          muted
          loop
          playsInline
          disablePictureInPicture
          preload="auto"
          // The last <source> failing means neither format plays here.
          onError={() => setFailed(true)}
        >
          <source src={HERO_VIDEO.webm} type="video/webm" />
          <source src={HERO_VIDEO.mp4} type="video/mp4" onError={() => setFailed(true)} />
        </video>
      )}
      {/* Grade: keep the launch button and its text readable, sink the edges into the card. */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_50%_45%,transparent_0%,rgba(8,9,12,0.25)_55%,rgba(8,9,12,0.7)_100%)]" />
      <div className="absolute inset-x-0 bottom-0 h-2/5 bg-gradient-to-t from-black/60 to-transparent" />
      <div className="absolute inset-y-0 left-0 w-1/3 bg-gradient-to-r from-black/30 to-transparent" />
    </div>
  );
}
