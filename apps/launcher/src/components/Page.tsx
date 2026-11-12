import { AccentText, cn } from '@rustic/ui';
import type { ReactNode } from 'react';

/**
 * Scrollable page body with consistent padding. Its blocks enter blur → sharp, 40 ms apart
 * (language.css `.rx-page-enter`; none with reduced motion or while a game runs).
 */
export function Page({
  children,
  className,
  wide,
}: {
  children: ReactNode;
  className?: string;
  wide?: boolean;
}) {
  return (
    <div
      className={cn(
        'rx-page-enter mx-auto flex w-full flex-col gap-6 px-8 pt-6 pb-10',
        wide ? 'max-w-[1600px]' : 'max-w-[1320px]',
        className,
      )}
    >
      {children}
    </div>
  );
}

export interface PageHeaderProps {
  /** A string may carry one `*accent*` word (serif italic, the website's titles). */
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  icon?: ReactNode;
}

/** A page's title row. No label above the title: the title bar's breadcrumb already says where. */
export function PageHeader({ title, description, actions, icon }: PageHeaderProps) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-4">
      <div className="flex min-w-0 items-center gap-4">
        {icon}
        <div className="flex min-w-0 flex-col gap-1">
          <h1 className="text-fg truncate text-3xl font-semibold tracking-tight">
            {typeof title === 'string' ? <AccentText text={title} /> : title}
          </h1>
          {description && <p className="text-fg-muted text-sm">{description}</p>}
        </div>
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </header>
  );
}

export function Section({
  title,
  action,
  children,
  className,
  id,
}: {
  title: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  id?: string;
}) {
  return (
    <section className={cn('flex flex-col gap-3', className)} aria-labelledby={id}>
      <div className="flex items-center justify-between gap-3">
        <h2 id={id} className="rx-eyebrow text-fg-muted">
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}

/**
 * Compact page title for the toolbar pages (Versions, Explore, Servers): a mono eyebrow and a title
 * with one serif-italic `*accent*` word, as on the website.
 */
export function PageTitle({ title }: { title: string }) {
  return (
    <header className="flex flex-col gap-1.5 px-1 pt-1">
      <h1 className="text-fg text-2xl font-semibold tracking-tight">
        <AccentText text={title} />
      </h1>
    </header>
  );
}
