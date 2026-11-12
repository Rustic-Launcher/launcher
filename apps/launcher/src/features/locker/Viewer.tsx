/** Lazy 3D viewer (three + r3f live in their own chunk) and flat cosmetic art for tiles. */
import { cn, Skeleton } from '@rustic/ui';
import type { PlayerViewerProps } from '@rustic/ui/three';
import {
  Backpack,
  Boxes,
  Bird,
  Crown,
  Footprints,
  Hand,
  PawPrint,
  Shirt,
  Sparkles,
  Tag,
  type LucideIcon,
} from 'lucide-react';
import { lazy, Suspense } from 'react';
import type { CosmeticSlot, Rarity } from '@/cloud/types';

const LazyPlayerViewer = lazy(() =>
  import('@rustic/ui/three').then((m) => ({ default: m.PlayerViewer })),
);

export function Viewer(props: PlayerViewerProps) {
  return (
    <Suspense fallback={<Skeleton className="size-full rounded-2xl" />}>
      <LazyPlayerViewer {...props} />
    </Suspense>
  );
}

export const SLOT_ICONS: Record<CosmeticSlot, LucideIcon> = {
  head: Crown,
  back: Backpack,
  top: Shirt,
  shoulder: Bird,
  companion: PawPrint,
  trail: Footprints,
  aura: Sparkles,
  emote: Hand,
  nametag: Tag,
};

export const RARITY_CLASS: Record<Rarity, string> = {
  common: 'text-fg-muted',
  rare: 'text-info-text',
  epic: 'text-accent-text',
  legendary: 'text-warning-text',
};

/** Generated tile art: slot glyph on a gradient of the item's colours (no third-party images). */
export function CosmeticArt({
  slot,
  color,
  accent,
  hue,
  className,
  size = 'md',
}: {
  slot: CosmeticSlot | null;
  color?: string;
  accent?: string;
  hue?: number;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}) {
  const Icon = slot ? SLOT_ICONS[slot] : Boxes;
  const a = color ?? `hsl(${hue ?? 25} 55% 50%)`;
  const b = accent ?? `hsl(${((hue ?? 25) + 35) % 360} 45% 22%)`;
  return (
    <div
      aria-hidden
      className={cn('relative grid place-items-center overflow-hidden', className)}
      style={{
        background: `radial-gradient(120% 90% at 30% 15%, color-mix(in oklab, ${a} 70%, white 10%), transparent 60%), linear-gradient(150deg, color-mix(in oklab, ${a} 55%, black), color-mix(in oklab, ${b} 60%, black))`,
      }}
    >
      <span className="absolute inset-0 [background-image:linear-gradient(rgba(255,255,255,0.12)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.12)_1px,transparent_1px)] [background-size:16px_16px] opacity-25" />
      <Icon
        className={cn(
          'relative text-white drop-shadow-[0_4px_10px_rgba(0,0,0,0.45)]',
          size === 'sm' && 'size-6',
          size === 'md' && 'size-10',
          size === 'lg' && 'size-16',
        )}
        strokeWidth={1.75}
      />
    </div>
  );
}

/** Name tag as drawn above the head in game (flat preview over the 3D view). */
export function NametagPreview({
  name,
  tag,
  className,
}: {
  name: string;
  tag: { visual: { shape: string; color: string; accent?: string } };
  className?: string;
}) {
  const gradient = tag.visual.shape === 'gradient';
  return (
    <div
      data-testid="nametag-preview"
      className={cn('rounded-md px-3 py-1 text-sm font-bold shadow-md', className)}
      style={{
        background: gradient
          ? `linear-gradient(90deg, ${tag.visual.color}, ${tag.visual.accent ?? tag.visual.color})`
          : 'rgba(0,0,0,0.55)',
        color: gradient ? '#14100E' : tag.visual.color,
      }}
    >
      {name}
    </div>
  );
}
