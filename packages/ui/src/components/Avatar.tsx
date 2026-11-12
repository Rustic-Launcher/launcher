import { Avatar as A } from 'radix-ui';
import { cn } from '../lib/cn';

export type Presence = 'online' | 'in-game' | 'away' | 'busy' | 'offline';

export interface AvatarProps {
  name: string;
  src?: string;
  presence?: Presence;
  /** Accessible presence text, e.g. "Online". */
  presenceLabel?: string;
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
  shape?: 'circle' | 'square';
}

const sizes = {
  xs: 'size-6 text-2xs',
  sm: 'size-8 text-xs',
  md: 'size-10 text-sm',
  lg: 'size-14 text-lg',
  xl: 'size-20 text-2xl',
};
const dot = { xs: 'size-2', sm: 'size-2.5', md: 'size-3', lg: 'size-3.5', xl: 'size-4' };
const presenceColor: Record<Presence, string> = {
  online: 'bg-success',
  'in-game': 'bg-accent',
  away: 'bg-warning',
  busy: 'bg-danger',
  offline: 'bg-fg-subtle',
};

/** Deterministic hue from a name, so placeholder avatars are stable. */
export function nameHue(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) % 360;
  return h;
}

export function Avatar({
  name,
  src,
  presence,
  presenceLabel,
  size = 'md',
  className,
  shape = 'square',
}: AvatarProps) {
  const hue = nameHue(name);
  return (
    <span className={cn('relative inline-flex shrink-0', className)}>
      <A.Root
        className={cn(
          'inline-flex items-center justify-center overflow-hidden font-semibold select-none',
          shape === 'circle' ? 'rounded-full' : 'rounded-[30%]',
          sizes[size],
        )}
      >
        {src && (
          <A.Image
            src={src}
            alt={name}
            className="size-full object-cover [image-rendering:pixelated]"
          />
        )}
        <A.Fallback
          className="flex size-full items-center justify-center text-white"
          style={{
            background: `linear-gradient(135deg, hsl(${hue} 45% 42%), hsl(${(hue + 40) % 360} 50% 30%))`,
          }}
          aria-label={name}
        >
          {name.slice(0, 2).toUpperCase()}
        </A.Fallback>
      </A.Root>
      {presence && (
        <span
          role="img"
          aria-label={presenceLabel ?? presence}
          className={cn(
            'ring-bg-elevated absolute -right-0.5 -bottom-0.5 rounded-full ring-2',
            dot[size],
            presenceColor[presence],
          )}
        />
      )}
    </span>
  );
}
