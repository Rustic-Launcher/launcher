import { cn } from '@rustic/ui';
import type { ReactNode } from 'react';

/**
 * A dense list row (Browse, like a store's list view): thumbnail, title + byline + badges, a
 * one-line summary, chips, a meta line, and actions on the right. Every slot is optional except
 * the title; the row wraps its actions under the text on narrow windows.
 */
export function CatalogListRow({
  thumb,
  title,
  byline,
  badges,
  summary,
  chips,
  meta,
  actions,
  className,
  testId,
}: {
  thumb?: ReactNode;
  /** Usually a link. */
  title: ReactNode;
  byline?: ReactNode;
  badges?: ReactNode;
  summary?: ReactNode;
  chips?: ReactNode;
  meta?: ReactNode;
  actions?: ReactNode;
  className?: string;
  testId?: string;
}) {
  return (
    <div
      className={cn(
        'border-border hover:bg-surface-2/40 flex flex-wrap items-center gap-x-4 gap-y-3 border-b px-2 py-3.5 transition-colors',
        className,
      )}
      data-testid={testId}
    >
      {thumb}
      <div className="flex min-w-[min(100%,280px)] flex-1 flex-col gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          {title}
          {byline && <span className="text-fg-muted truncate text-sm">{byline}</span>}
          {badges}
        </div>
        {summary && <p className="text-fg-muted line-clamp-1 text-sm">{summary}</p>}
        {(chips || meta) && (
          <div className="text-fg-subtle flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs tabular-nums">
            {chips && <span className="flex flex-wrap items-center gap-1.5">{chips}</span>}
            {meta}
          </div>
        )}
      </div>
      {actions && (
        <div className="ml-auto flex shrink-0 items-center gap-2 max-[980px]:w-full max-[980px]:justify-end">
          {actions}
        </div>
      )}
    </div>
  );
}

/** A small category chip. */
export function Chip({ children }: { children: ReactNode }) {
  return <span className="bg-surface-3 text-fg-muted rounded-md px-1.5 py-0.5">{children}</span>;
}

/** A badge readable over a picture (cards, hero slides). */
export function OverlayBadge({
  children,
  icon,
  tone = 'neutral',
}: {
  children: ReactNode;
  icon?: ReactNode;
  tone?: 'neutral' | 'accent' | 'success' | 'info' | 'warning';
}) {
  return (
    <span
      className={cn(
        'text-2xs inline-flex h-5 items-center gap-1 rounded-full border px-2 font-semibold whitespace-nowrap backdrop-blur-sm [&_svg]:size-3',
        tone === 'neutral' && 'border-white/10 bg-black/55 text-white',
        tone === 'accent' && 'bg-accent text-accent-fg border-transparent',
        tone === 'success' && 'border-success/40 bg-black/60 text-[#7EE2A8]',
        tone === 'info' && 'border-info/40 bg-black/60 text-[#9CCBFF]',
        tone === 'warning' && 'border-warning/40 bg-black/60 text-[#EDBC5C]',
      )}
    >
      {icon}
      {children}
    </span>
  );
}
