import { cn } from '@rustic/ui';
import { Check } from 'lucide-react';
import { RadioGroup } from 'radix-ui';
import { PRESET_COLORS, PRESET_ICONS } from '@/components/InstanceIcon';
import type { InstanceIconPreset } from '@/ipc/types/instances';

const PRESETS = Object.keys(PRESET_ICONS) as InstanceIconPreset[];

/**
 * The glyph + colour picker of the create forms (new profile, new server): one look everywhere.
 * Glyphs sit on neutral tiles; only the chosen one takes the chosen colour, as the icon will.
 * Two radio groups (arrow keys move and select, Tab leaves the group).
 */
export function IconPicker({
  preset,
  color,
  onPreset,
  onColor,
  iconsLabel,
  colorsLabel,
  iconLabel,
  colorLabel,
  className,
}: {
  preset: InstanceIconPreset;
  color: string;
  onPreset: (p: InstanceIconPreset) => void;
  onColor: (c: string) => void;
  /** Accessible names of the two groups. */
  iconsLabel: string;
  colorsLabel: string;
  iconLabel: (p: InstanceIconPreset) => string;
  colorLabel: (index: number) => string;
  className?: string;
}) {
  return (
    <div className={cn('flex flex-col gap-3', className)}>
      <RadioGroup.Root
        value={preset}
        onValueChange={(v) => onPreset(v as InstanceIconPreset)}
        aria-label={iconsLabel}
        orientation="horizontal"
        loop
        className="grid max-w-[36rem] grid-cols-5 gap-2 sm:grid-cols-10"
      >
        {PRESETS.map((p) => {
          const Icon = PRESET_ICONS[p];
          const on = preset === p;
          return (
            <RadioGroup.Item
              key={p}
              value={p}
              aria-label={iconLabel(p)}
              title={iconLabel(p)}
              className={cn(
                'rx-focus-ring grid h-11 place-items-center rounded-xl border transition-[background-color,border-color,color,transform] duration-150 [&_svg]:size-5',
                on
                  ? 'scale-[1.04] border-transparent text-white shadow-md'
                  : 'border-border bg-surface-2/60 text-fg-muted hover:border-border-strong/70 hover:bg-surface-2 hover:text-fg motion-reduce:transform-none',
              )}
              style={
                on
                  ? {
                      background: `linear-gradient(140deg, ${color}, color-mix(in oklab, ${color} 55%, black))`,
                    }
                  : undefined
              }
            >
              <Icon strokeWidth={on ? 2.2 : 1.8} aria-hidden />
            </RadioGroup.Item>
          );
        })}
      </RadioGroup.Root>
      <RadioGroup.Root
        value={color}
        onValueChange={onColor}
        aria-label={colorsLabel}
        orientation="horizontal"
        loop
        className="flex flex-wrap items-center gap-2"
      >
        {PRESET_COLORS.map((c, i) => {
          const on = color === c;
          return (
            <RadioGroup.Item
              key={c}
              value={c}
              aria-label={colorLabel(i + 1)}
              title={colorLabel(i + 1)}
              className={cn(
                'rx-focus-ring ring-offset-surface grid size-7 place-items-center rounded-full text-white ring-offset-2 transition-transform duration-150 hover:scale-110 motion-reduce:hover:scale-100',
                on && 'ring-fg/80 ring-2',
              )}
              style={{ background: c }}
            >
              {on && <Check className="size-3.5 drop-shadow" strokeWidth={3} aria-hidden />}
            </RadioGroup.Item>
          );
        })}
      </RadioGroup.Root>
    </div>
  );
}
