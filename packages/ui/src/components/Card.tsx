import { cva, type VariantProps } from 'class-variance-authority';
import { forwardRef, type HTMLAttributes } from 'react';
import { cn } from '../lib/cn';

const cardVariants = cva('relative rounded-xl text-fg', {
  variants: {
    variant: {
      // The website's premium surface (language.css): gradient edge, grain, inner highlight.
      solid: 'rx-surface',
      glass: 'rx-surface rx-surface-glass',
      outline: 'border border-border',
      ghost: '',
    },
    interactive: {
      true: 'cursor-pointer transition-transform duration-200 ease-out hover:-translate-y-0.5',
      false: '',
    },
    padding: { none: '', sm: 'p-3', md: 'p-4', lg: 'p-6' },
  },
  defaultVariants: { variant: 'solid', interactive: false, padding: 'md' },
});

export interface CardProps
  extends HTMLAttributes<HTMLDivElement>, VariantProps<typeof cardVariants> {}

export const Card = forwardRef<HTMLDivElement, CardProps>(function Card(
  { className, variant, interactive, padding, ...props },
  ref,
) {
  return (
    <div
      ref={ref}
      className={cn(cardVariants({ variant, interactive, padding }), className)}
      {...props}
    />
  );
});

export function CardHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('mb-3 flex items-start justify-between gap-3', className)} {...props} />
  );
}

export function CardTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
  // Content comes from children in props.
  // eslint-disable-next-line jsx-a11y/heading-has-content
  return <h3 className={cn('text-fg text-base font-semibold', className)} {...props} />;
}

export function CardDescription({ className, ...props }: HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn('text-fg-muted text-sm', className)} {...props} />;
}
