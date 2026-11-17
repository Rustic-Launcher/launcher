import { useEffect, useRef, useState } from 'react';
import { cn } from '../lib/cn';

export interface HeroSource {
  src: string;
  type: string;
}

export interface HeroLoopProps {
  /** Small bundled clip that plays at once (and offline). */
  fallback: HeroSource[];
  /** The high-definition clip; when it starts playing it fades in over the fallback. */
  hd?: HeroSource[] | null;
  poster?: string;
  /** Play or pause (whichever clip is showing). */
  playing: boolean;
  /** Neither the fallback nor the HD clip can play here. */
  onFail?: () => void;
  className?: string;
  /** Length of the crossfade, in milliseconds. */
  fadeMs?: number;
}

/**
 * A muted looping backdrop video (ADR 0179): the bundled low-resolution clip plays first, then the
 * HD rendition — once it actually plays — crossfades in over it, and the fallback is dropped. When
 * both are the same loop (same length), the HD one starts at the fallback's position. A failing HD
 * clip leaves the fallback in place.
 */
export function HeroLoop({
  fallback,
  hd,
  poster,
  playing,
  onFail,
  className,
  fadeMs = 1200,
}: HeroLoopProps) {
  const low = useRef<HTMLVideoElement>(null);
  const high = useRef<HTMLVideoElement>(null);
  const [hdShown, setHdShown] = useState(false);
  const [lowGone, setLowGone] = useState(false);
  const [hdFailed, setHdFailed] = useState(false);
  const hdKey = hd?.map((s) => s.src).join('|') ?? '';
  const failed = useRef(false);

  // A new HD clip starts hidden again.
  useEffect(() => {
    setHdShown(false);
    setHdFailed(false);
  }, [hdKey]);

  useEffect(() => {
    for (const v of [low.current, high.current]) {
      if (!v) continue;
      if (playing) void v.play().catch(() => undefined);
      else v.pause();
    }
  }, [playing, hdKey, lowGone]);

  useEffect(() => {
    if (!hdShown || lowGone) return;
    const t = window.setTimeout(() => setLowGone(true), fadeMs + 100);
    return () => window.clearTimeout(t);
  }, [hdShown, lowGone, fadeMs]);

  const cls = 'absolute inset-0 size-full object-cover';
  const showHd = !!hd?.length && !hdFailed;
  const lowFailed = () => {
    if (showHd || failed.current) return;
    failed.current = true;
    onFail?.();
  };
  return (
    <div className={cn('absolute inset-0', className)}>
      {!lowGone && (
        <video
          ref={low}
          className={cls}
          poster={poster}
          autoPlay={playing}
          muted
          loop
          playsInline
          disablePictureInPicture
          preload="auto"
          data-hero="fallback"
          onError={lowFailed}
        >
          {fallback.map((s, i) => (
            <source
              key={s.src}
              src={s.src}
              type={s.type}
              onError={i === fallback.length - 1 ? lowFailed : undefined}
            />
          ))}
        </video>
      )}
      {showHd && (
        <video
          key={hdKey}
          ref={high}
          className={cls}
          style={{
            opacity: hdShown ? 1 : 0,
            transition: `opacity ${fadeMs}ms ease`,
          }}
          autoPlay={playing}
          muted
          loop
          playsInline
          disablePictureInPicture
          preload="auto"
          data-hero="hd"
          data-shown={hdShown ? 'true' : 'false'}
          // The same loop at another size (the bundled clip is the set's 480p): start the HD one
          // where the small one is, so the crossfade only sharpens the picture.
          onLoadedMetadata={(e) => {
            const lo = low.current;
            const hi = e.currentTarget;
            if (lo && Number.isFinite(lo.duration) && Math.abs(lo.duration - hi.duration) < 0.25)
              hi.currentTime = lo.currentTime % hi.duration;
          }}
          onPlaying={() => setHdShown(true)}
          // Paused (window hidden, game running): show it once it can play anyway.
          onCanPlay={() => !playing && setHdShown(true)}
          onError={() => setHdFailed(true)}
        >
          {hd.map((s, i) => (
            <source
              key={s.src}
              src={s.src}
              type={s.type}
              onError={i === hd.length - 1 ? () => setHdFailed(true) : undefined}
            />
          ))}
        </video>
      )}
    </div>
  );
}
