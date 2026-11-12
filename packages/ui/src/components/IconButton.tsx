import { cva, type VariantProps } from 'class-variance-authority';
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';
import { cn } from '../lib/cn';
import { Tooltip } from './Tooltip';

const iconButtonVariants = cva(
  'rx-focus-ring relative inline-flex shrink-0 items-center justify-center rounded-md transition-[background-color,color,transform] duration-150 ease-out active:scale-95 disabled:pointer-events-none disabled:opacity-45',
  {
    variants: {
      variant: {
        ghost: 'text-fg-muted hover:bg-surface-2 hover:text-fg',
        secondary: 'bg-surface-2 text-fg hover:bg-surface-3 border border-border',
        primary: 'bg-accent text-accent-fg hover:bg-accent-hover',
        danger: 'text-danger-text hover:bg-danger/15',
      },
      size: {
        sm: 'size-7 [&_svg]:size-3.5',
        md: 'size-9 [&_svg]:size-4.5',
        lg: 'size-11 [&_svg]:size-5',
      },
    },
    defaultVariants: { variant: 'ghost', size: 'md' },
  },
);

export interface IconButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof iconButtonVariants> {
  /** Accessible name, also shown as tooltip. */
  label: string;
  icon: ReactNode;
  tooltip?: boolean;
  tooltipSide?: 'top' | 'right' | 'bottom' | 'left';
  shortcut?: string;
  /** Small dot/counter badge in the corner. */
  badge?: number | boolean;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  {
    label,
    icon,
    tooltip = true,
    tooltipSide,
    shortcut,
    badge,
    variant,
    size,
    className,
    type = 'button',
    ...props
  },
  ref,
) {
  const btn = (
    <button
      ref={ref}
      type={type}
      aria-label={label}
      className={cn(iconButtonVariants({ variant, size }), className)}
      {...props}
    >
      {icon}
      {badge ? (
        <span
          aria-hidden
          className="bg-accent text-2xs text-accent-fg ring-bg-elevated absolute -top-0.5 -right-0.5 flex min-w-4 items-center justify-center rounded-full px-1 leading-4 font-bold ring-2"
        >
          {typeof badge === 'number' ? (badge > 9 ? '9+' : badge) : null}
        </span>
      ) : null}
    </button>
  );
  if (!tooltip) return btn;
  return (
    <Tooltip content={label} side={tooltipSide} shortcut={shortcut}>
      {btn}
    </Tooltip>
  );
});
