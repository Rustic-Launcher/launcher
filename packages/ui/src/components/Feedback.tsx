import { motion } from 'motion/react';
import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '../lib/cn';
import { clamp } from '../lib/format';

/** Placeholder block shown while content loads (skeletons, never spinners, for pages). */
export function Skeleton({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      aria-hidden
      className={cn('rx-skeleton animate-shimmer rounded-md', className)}
      {...props}
    />
  );
}

export interface ProgressProps {
  /** 0..1, or null for indeterminate. */
  value: number | null;
  label: string;
  size?: 'xs' | 'sm' | 'md';
  tone?: 'accent' | 'success' | 'warning' | 'danger';
  className?: string;
  /** Human value text, e.g. "42 % · 3.1 MB/s". */
  valueText?: string;
}

export function Progress({
  value,
  label,
  size = 'sm',
  tone = 'accent',
  className,
  valueText,
}: ProgressProps) {
  const pct = value == null ? null : clamp(value, 0, 1) * 100;
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct == null ? undefined : Math.round(pct)}
      aria-valuetext={valueText}
      className={cn(
        'bg-surface-3 relative w-full overflow-hidden rounded-full',
        size === 'xs' && 'h-1',
        size === 'sm' && 'h-1.5',
        size === 'md' && 'h-2.5',
        className,
      )}
    >
      {pct == null ? (
        <span
          className={cn(
            'animate-indeterminate absolute inset-y-0 w-2/5 rounded-full',
            tone === 'accent' && 'bg-accent',
            tone === 'success' && 'bg-success',
            tone === 'warning' && 'bg-warning',
            tone === 'danger' && 'bg-danger',
          )}
        />
      ) : (
        <motion.span
          className={cn(
            'absolute inset-y-0 left-0 rounded-full',
            tone === 'accent' && 'bg-accent',
            tone === 'success' && 'bg-success',
            tone === 'warning' && 'bg-warning',
            tone === 'danger' && 'bg-danger',
          )}
          initial={false}
          animate={{ width: `${pct}%` }}
          transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
        />
      )}
    </div>
  );
}

export interface EmptyStateProps {
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
  tone?: 'default' | 'error';
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
  tone = 'default',
}: EmptyStateProps) {
  return (
    <div
      role={tone === 'error' ? 'alert' : undefined}
      className={cn(
        'border-border flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed px-6 py-12 text-center',
        className,
      )}
    >
      {icon && (
        <span
          className={cn(
            'grid size-12 place-items-center rounded-2xl [&_svg]:size-6',
            tone === 'error' ? 'bg-danger/12 text-danger-text' : 'bg-accent-soft text-accent-text',
          )}
        >
          {icon}
        </span>
      )}
      <div className="flex max-w-sm flex-col gap-1">
        <p className="text-fg text-base font-semibold">{title}</p>
        {description && <p className="text-fg-muted text-sm">{description}</p>}
      </div>
      {action && <div className="mt-1">{action}</div>}
    </div>
  );
}

export function Separator({ className, vertical }: { className?: string; vertical?: boolean }) {
  return (
    <div
      role="separator"
      aria-orientation={vertical ? 'vertical' : 'horizontal'}
      className={cn(vertical ? 'bg-border w-px self-stretch' : 'bg-border h-px w-full', className)}
    />
  );
}
