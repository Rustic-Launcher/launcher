import { cn } from '@rustic/ui';
import { useId, type ReactNode } from 'react';

/** One group of a wizard step: an icon chip, a title, an optional hint, then its fields. */
export function WizardSection({
  icon,
  title,
  hint,
  action,
  children,
  className,
  testId,
}: {
  icon: ReactNode;
  title: ReactNode;
  hint?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  testId?: string;
}) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className={cn('rx-surface flex flex-col gap-4 rounded-2xl p-5', className)}
      data-testid={testId}
    >
      <header className="flex items-start gap-3">
        <span
          aria-hidden
          className="bg-accent-soft text-accent-text grid size-8 shrink-0 place-items-center rounded-lg [&_svg]:size-4"
        >
          {icon}
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-0.5 pt-0.5">
          <h3 id={id} className="text-fg text-sm font-semibold">
            {title}
          </h3>
          {hint && <p className="text-fg-muted text-xs">{hint}</p>}
        </div>
        {action}
      </header>
      {children}
    </section>
  );
}
