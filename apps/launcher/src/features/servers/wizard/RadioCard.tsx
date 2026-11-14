import { cn } from '@rustic/ui';
import { Check } from 'lucide-react';
import type { ReactNode } from 'react';
import { GeneratedArt } from '@/components/catalog';

export interface RadioCardProps {
  name: string;
  value: string;
  checked: boolean;
  onChange: (v: string) => void;
  title: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  badge?: ReactNode;
  disabled?: boolean;
  children?: ReactNode;
  className?: string;
  testId?: string;
  /**
   * A visual card: an illustrated band (generated art seeded by `art`) with the icon large on it,
   * above the title. Without it, the compact card with the icon on the left.
   */
  art?: string;
}

/** A large selectable card backed by a native radio input (keyboard: arrows within the group). */
export function RadioCard({
  name,
  value,
  checked,
  onChange,
  title,
  description,
  icon,
  badge,
  disabled,
  children,
  className,
  testId,
  art,
}: RadioCardProps) {
  const tick = checked && (
    <span className="bg-accent text-accent-fg grid size-5 shrink-0 place-items-center rounded-full shadow">
      <Check className="size-3" strokeWidth={3} />
    </span>
  );
  return (
    <label
      data-testid={testId}
      data-checked={checked || undefined}
      className={cn(
        'relative flex cursor-pointer flex-col overflow-hidden rounded-xl border transition-[border-color,background-color,box-shadow] duration-150',
        'has-[:focus-visible]:ring-accent/50 has-[:focus-visible]:ring-2',
        !art && 'gap-2 p-4',
        checked
          ? 'border-accent bg-accent-soft shadow-glow'
          : 'border-border bg-surface/70 hover:border-border-strong/60',
        disabled && 'hover:border-border cursor-not-allowed opacity-70',
        className,
      )}
    >
      <input
        type="radio"
        name={name}
        value={value}
        checked={checked}
        disabled={disabled}
        onChange={() => onChange(value)}
        className="sr-only"
      />
      {art ? (
        <>
          <span aria-hidden className="relative block h-16 shrink-0 overflow-hidden">
            <GeneratedArt
              seed={art}
              wide
              className={cn(
                'transition-[filter] duration-200',
                !checked && 'brightness-[0.62] saturate-[0.55]',
              )}
            />
            <span className="absolute inset-0 bg-gradient-to-t from-black/45 to-transparent" />
            {icon && (
              <span className="absolute bottom-2.5 left-3 grid size-9 place-items-center rounded-lg bg-black/45 text-white backdrop-blur-sm [&_svg]:size-5">
                {icon}
              </span>
            )}
            {checked && <span className="absolute top-2.5 right-2.5">{tick}</span>}
          </span>
          <span className="flex min-w-0 flex-1 flex-col gap-1 px-3.5 pt-2.5 pb-3.5">
            <span className="text-fg flex flex-wrap items-center gap-2 text-sm font-semibold">
              {title}
              {badge}
            </span>
            {description && (
              <span className="text-fg-muted text-xs leading-relaxed">{description}</span>
            )}
            {children}
          </span>
        </>
      ) : (
        <>
          <div className="flex items-start gap-3">
            {icon && (
              <span
                className={cn(
                  'grid size-10 shrink-0 place-items-center rounded-lg [&_svg]:size-5',
                  checked ? 'bg-accent text-accent-fg' : 'bg-surface-3 text-fg-muted',
                )}
              >
                {icon}
              </span>
            )}
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <span className="text-fg flex flex-wrap items-center gap-2 text-sm font-semibold">
                {title}
                {badge}
              </span>
              {description && <span className="text-fg-muted text-xs">{description}</span>}
            </div>
            {tick}
          </div>
          {children}
        </>
      )}
    </label>
  );
}
