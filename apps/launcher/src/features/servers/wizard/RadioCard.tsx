import { cn } from '@rustic/ui';
import { Check } from 'lucide-react';
import type { ReactNode } from 'react';

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
}: RadioCardProps) {
  return (
    <label
      data-testid={testId}
      className={cn(
        'relative flex cursor-pointer flex-col gap-2 rounded-xl border p-4 transition-[border-color,background-color,box-shadow] duration-150',
        'has-[:focus-visible]:ring-accent/50 has-[:focus-visible]:ring-2',
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
        {checked && (
          <span className="bg-accent text-accent-fg grid size-5 shrink-0 place-items-center rounded-full">
            <Check className="size-3" strokeWidth={3} />
          </span>
        )}
      </div>
      {children}
    </label>
  );
}
