import { cva, type VariantProps } from 'class-variance-authority';
import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '../lib/cn';

const badgeVariants = cva(
  'inline-flex max-w-full items-center gap-1 whitespace-nowrap rounded-full border font-medium [&_svg]:size-3 [&_svg]:shrink-0',
  {
    variants: {
      variant: {
        neutral: 'border-border bg-surface-2 text-fg-muted',
        accent: 'border-accent/30 bg-accent-soft text-accent-text',
        success: 'border-success/30 bg-success/12 text-success-text',
        warning: 'border-warning/30 bg-warning/12 text-warning-text',
        danger: 'border-danger/30 bg-danger/12 text-danger-text',
        info: 'border-info/30 bg-info/12 text-info-text',
        solid: 'border-transparent bg-accent text-accent-fg',
      },
      size: {
        sm: 'h-5 px-2 text-2xs',
        md: 'h-6 px-2.5 text-xs',
      },
    },
    defaultVariants: { variant: 'neutral', size: 'sm' },
  },
);

export interface BadgeProps
  extends HTMLAttributes<HTMLSpanElement>, VariantProps<typeof badgeVariants> {
  icon?: ReactNode;
}

export function Badge({ className, variant, size, icon, children, ...props }: BadgeProps) {
  return (
    <span className={cn(badgeVariants({ variant, size }), className)} {...props}>
      {icon}
      <span className="truncate">{children}</span>
    </span>
  );
}

export type BadgeVariant = NonNullable<VariantProps<typeof badgeVariants>['variant']>;
