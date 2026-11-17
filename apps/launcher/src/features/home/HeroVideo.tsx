import {
  canPlayMime,
  HeroLoop,
  pickHeroPoster,
  pickHeroRendition,
  saveDataRequested,
  type HeroSource,
} from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { cloud } from '@/cloud/client';
import type { HeroMedia } from '@/cloud/types';
import { invoke, isNative } from '@/ipc';
import { HeroArt } from './HeroArt';
import { batterySaver, shouldRun } from './HeroLight';

/**
 * The bundled loop (`public/media/hero-480.*`, ADR 0179 and its addendum, see docs/THIRD_PARTY.md):
 * the 480p of the built-in set, which plays at once and offline. The HD rendition comes from the
 * CDN and takes over at the same point of the loop.
 */
export const HERO_VIDEO = {
  webm: '/media/hero-480.webm',
  mp4: '/media/hero-480.mp4',
  poster: '/media/hero-poster.webp',
} as const;

const FALLBACK: HeroSource[] = [
  { src: HERO_VIDEO.webm, type: 'video/webm' },
  { src: HERO_VIDEO.mp4, type: 'video/mp4' },
];

/** The last set seen, so a cached HD file still plays offline (a convenience: guarded). */
const LAST_SET = 'rustic.launcher.heroMedia';

type BatteryLike = EventTarget & { charging: boolean; level: number };

const reducedMotion = () =>
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export function readLastSet(): HeroMedia | null {
  try {
    const v = JSON.parse(localStorage.getItem(LAST_SET) ?? 'null') as HeroMedia | null;
    return v && Array.isArray(v.renditions) && Array.isArray(v.posters) ? v : null;
  } catch {
    return null;
  }
}

function rememberSet(m: HeroMedia) {
  try {
    localStorage.setItem(LAST_SET, JSON.stringify(m));
  } catch {
    /* a convenience only */
  }
}

/** The window (CSS pixels and ratio): the hero never covers more than it. */
const viewport = () => ({
  width: window.innerWidth,
  height: window.innerHeight,
  dpr: window.devicePixelRatio || 1,
});

/**
 * The HD rendition as a playable URL. In the desktop app the Rust side caches the file
 * (`hero_media_fetch`) and it plays from disk; a file already cached plays offline too, a new one
 * is only downloaded online and without data saver. In a browser the CDN URL streams. `null` when
 * there is nothing to play.
 */
export async function hdSource(set: HeroMedia): Promise<HeroSource | null> {
  const pick = pickHeroRendition(set.renditions, viewport(), canPlayMime);
  if (!pick) return null;
  const allowDownload = navigator.onLine && !saveDataRequested();
  if (!isNative()) return allowDownload ? { src: pick.url, type: pick.mime } : null;
  try {
    const path = await invoke('hero_media_fetch', {
      url: pick.url,
      sha256: pick.sha256,
      bytes: pick.bytes,
      download: allowDownload,
    });
    if (!path) return null;
    const { convertFileSrc } = await import('@tauri-apps/api/core');
    return { src: convertFileSrc(path), type: pick.mime };
  } catch {
    return null;
  }
}

/**
 * The hero's backdrop: a muted, looping video behind the launch button. The bundled 480p clip
 * plays first; the HD rendition of `GET /v1/media/hero` (picked by codec and window size) fades
 * in once it plays. Same rest policy as the light stream it replaced (`shouldRun`): paused while
 * an instance runs, while the window is hidden or minimised and in battery saver; a still poster
 * with reduced motion. If no video can play at all, the original SVG illustration takes its place.
 */
export function HeroVideo({ running }: { running: boolean }) {
  const [failed, setFailed] = useState(false);
  const [hidden, setHidden] = useState(() => document.hidden);
  const [saver, setSaver] = useState(false);
  const [hd, setHd] = useState<HeroSource | null>(null);
  const reduced = reducedMotion();
  const play = shouldRun({ reduced, running, hidden, saver });

  const q = useQuery({
    queryKey: ['hero-media'],
    queryFn: async () => (await cloud()).heroMedia(),
    staleTime: 30 * 60_000,
    retry: false,
  });
  // Offline (or the API down): the last set seen, whose file may be cached.
  const set = useMemo(() => q.data ?? (q.isError ? readLastSet() : null), [q.data, q.isError]);

  useEffect(() => {
    if (q.data) rememberSet(q.data);
  }, [q.data]);

  useEffect(() => {
    if (!set || reduced) return;
    let live = true;
    void hdSource(set).then((s) => {
      if (live) setHd(s);
    });
    return () => {
      live = false;
    };
  }, [set, reduced]);

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

  if (failed) return <HeroArt />;
  // Reduced motion: the set's still at the window's size (the CDN), else the bundled one.
  const still = (set && pickHeroPoster(set.posters, viewport())?.url) || HERO_VIDEO.poster;
  return (
    <div
      aria-hidden
      className="absolute inset-0 overflow-hidden"
      data-testid="hero-video"
      data-state={reduced ? 'still' : play ? 'playing' : 'paused'}
      data-hd={hd ? 'true' : 'false'}
    >
      {reduced ? (
        <img
          src={still}
          alt=""
          className="size-full object-cover"
          onError={(e) => {
            const img = e.currentTarget;
            if (img.src.endsWith(HERO_VIDEO.poster)) setFailed(true);
            else img.src = HERO_VIDEO.poster;
          }}
        />
      ) : (
        <HeroLoop
          fallback={FALLBACK}
          hd={hd ? [hd] : null}
          poster={HERO_VIDEO.poster}
          playing={play}
          onFail={() => setFailed(true)}
        />
      )}
      {/* Grade: keep the launch button and its text readable, sink the edges into the card. */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_50%_45%,transparent_0%,rgba(8,9,12,0.25)_55%,rgba(8,9,12,0.7)_100%)]" />
      <div className="absolute inset-x-0 bottom-0 h-2/5 bg-gradient-to-t from-black/60 to-transparent" />
      <div className="absolute inset-y-0 start-0 w-1/3 bg-gradient-to-r from-black/30 to-transparent" />
    </div>
  );
}
