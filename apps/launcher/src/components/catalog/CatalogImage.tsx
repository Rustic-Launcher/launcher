import { cn } from '@rustic/ui';
import { useState, type ReactNode } from 'react';

/**
 * Image hosts the webview may load (tauri.conf.json `img-src`): Rustic's CDN and Modrinth's.
 * Anything else (a registry's GitHub-hosted icon, a stray http link) falls back to generated art,
 * so no third-party request is ever attempted.
 */
const ALLOWED_HOSTS = ['cdn.rustic-launcher.com', 'cdn.modrinth.com'];

/** The URL when the webview may show it (allowed https host, or inline `data:`/`blob:`), else null. */
export function safeImageUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  if (url.startsWith('data:image/') || url.startsWith('blob:')) return url;
  try {
    const u = new URL(url);
    return u.protocol === 'https:' && ALLOWED_HOSTS.includes(u.hostname) ? url : null;
  } catch {
    return null;
  }
}

/** Deterministic hue for generated art. */
export function hueOf(seed: string): number {
  let h = 7;
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) % 360;
  return h;
}

export function initialsOf(name: string): string {
  return name
    .split(/[\s&_-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join('');
}

/** Original art for a project without a picture: voxel hills under a hue gradient sky. */
export function GeneratedArt({ seed, className }: { seed: string; className?: string }) {
  const hue = hueOf(seed);
  const hills = Array.from({ length: 16 }, (_, i) => 16 + (((hue + 7) * (i + 5)) % 30));
  return (
    <div
      aria-hidden
      className={cn('absolute inset-0 overflow-hidden', className)}
      style={{
        background: `linear-gradient(170deg, hsl(${hue} 62% 56%) 0%, hsl(${(hue + 25) % 360} 52% 34%) 55%, hsl(${(hue + 40) % 360} 45% 16%) 100%)`,
      }}
    >
      <svg
        viewBox="0 0 160 90"
        preserveAspectRatio="xMidYMax slice"
        className="absolute inset-0 size-full"
      >
        <circle cx="128" cy="22" r="10" fill="white" fillOpacity="0.3" />
        {hills.map((h, i) => (
          <rect
            key={i}
            x={i * 10}
            y={90 - h}
            width="10.5"
            height={h}
            fill={`hsl(${(hue + 40) % 360} 40% 13%)`}
            fillOpacity={0.55 + (i % 3) * 0.12}
          />
        ))}
      </svg>
    </div>
  );
}

/**
 * A wide cover (cards, hero slides): the picture when one is allowed, else generated art.
 * `children` are drawn on top (badges, gradients).
 */
export function CatalogCover({
  src,
  seed,
  className,
  imgClassName,
  children,
}: {
  src?: string | null;
  seed: string;
  className?: string;
  imgClassName?: string;
  children?: ReactNode;
}) {
  const safe = safeImageUrl(src);
  const [failed, setFailed] = useState<string | null>(null);
  const show = safe && failed !== safe;
  return (
    <div className={cn('relative overflow-hidden', className)}>
      {show ? (
        <img
          src={safe}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          onError={() => setFailed(safe)}
          className={cn('absolute inset-0 size-full object-cover', imgClassName)}
        />
      ) : (
        <GeneratedArt seed={seed} />
      )}
      {children}
    </div>
  );
}

const thumbSizes = {
  xs: 'size-8 rounded-lg text-[10px]',
  sm: 'size-10 rounded-lg text-xs',
  md: 'size-12 rounded-xl text-sm',
  lg: 'size-16 rounded-xl text-base',
  xl: 'size-20 rounded-2xl text-2xl',
};

/** A square icon: the project's when allowed, else its initials on a hue gradient. */
export function CatalogThumb({
  src,
  seed,
  name,
  size = 'md',
  className,
}: {
  src?: string | null;
  seed: string;
  name: string;
  size?: keyof typeof thumbSizes;
  className?: string;
}) {
  const safe = safeImageUrl(src);
  const [failed, setFailed] = useState<string | null>(null);
  const hue = hueOf(seed);
  return (
    <span
      aria-hidden
      className={cn(
        'relative grid shrink-0 place-items-center overflow-hidden font-bold text-white shadow-sm',
        thumbSizes[size],
        className,
      )}
      style={{
        background: `linear-gradient(140deg, hsl(${hue} 55% 46%), hsl(${(hue + 35) % 360} 60% 26%))`,
      }}
    >
      {safe && failed !== safe ? (
        <img
          src={safe}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          onError={() => setFailed(safe)}
          className="absolute inset-0 size-full object-cover"
        />
      ) : (
        initialsOf(name)
      )}
    </span>
  );
}
