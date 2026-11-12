import { cn } from '@rustic/ui';
import {
  Castle,
  Compass,
  Flame,
  Gem,
  Leaf,
  Pickaxe,
  Rocket,
  Sprout,
  Star,
  Swords,
  type LucideIcon,
} from 'lucide-react';
import type { InstanceIcon as Icon, InstanceIconPreset } from '@/ipc/types/instances';

export const PRESET_ICONS: Record<InstanceIconPreset, LucideIcon> = {
  grass: Sprout,
  sword: Swords,
  pickaxe: Pickaxe,
  castle: Castle,
  rocket: Rocket,
  gem: Gem,
  flame: Flame,
  leaf: Leaf,
  star: Star,
  compass: Compass,
};

export const PRESET_COLORS = [
  '#4E9F5B',
  '#C9543E',
  '#7A6AD8',
  '#D4A24C',
  '#3A8FB7',
  '#C2549A',
  '#8C7A5B',
  '#E8834A',
];

const sizes = {
  sm: 'size-8 rounded-lg [&_svg]:size-4',
  md: 'size-11 rounded-xl [&_svg]:size-5',
  lg: 'size-16 rounded-2xl [&_svg]:size-7',
  xl: 'size-24 rounded-3xl [&_svg]:size-10',
};

export function InstanceIcon({
  icon,
  size = 'md',
  className,
}: {
  icon: Icon;
  size?: keyof typeof sizes;
  className?: string;
}) {
  if (icon.kind === 'image') {
    return <img src={icon.url} alt="" className={cn('object-cover', sizes[size], className)} />;
  }
  const C = PRESET_ICONS[icon.preset];
  return (
    <span
      aria-hidden
      className={cn(
        'grid shrink-0 place-items-center text-white shadow-sm',
        sizes[size],
        className,
      )}
      style={{
        background: `linear-gradient(140deg, ${icon.color}, color-mix(in oklab, ${icon.color} 55%, black))`,
      }}
    >
      <C strokeWidth={2.2} />
    </span>
  );
}
