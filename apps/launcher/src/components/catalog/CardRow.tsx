import { cn, IconButton } from '@rustic/ui';
import { ChevronLeft, ChevronRight, ArrowRight } from 'lucide-react';
import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * A themed row of cards (Discover: "New", "Popular on Horizon"...): a title, an optional
 * "See all", and a horizontal, snap-scrolling list with previous/next buttons that page by the
 * visible width. Children are the `<li>` items (use {@link CardRowItem}).
 */
export function CardRow({
  title,
  subtitle,
  icon,
  onSeeAll,
  children,
  className,
  testId,
}: {
  title: string;
  subtitle?: ReactNode;
  icon?: ReactNode;
  /** "See all" (e.g. the Browse tab with this row's filters). */
  onSeeAll?: () => void;
  children: ReactNode;
  className?: string;
  testId?: string;
}) {
  const { t } = useTranslation();
  const headingId = useId();
  const scroller = useRef<HTMLUListElement>(null);
  const [edges, setEdges] = useState({ start: true, end: true });

  const measure = useCallback(() => {
    const el = scroller.current;
    if (!el) return;
    setEdges({
      start: el.scrollLeft <= 4,
      end: el.scrollLeft + el.clientWidth >= el.scrollWidth - 4,
    });
  }, []);

  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [measure, children]);

  const page = (d: 1 | -1) => {
    const el = scroller.current;
    if (!el) return;
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    el.scrollBy({ left: d * el.clientWidth * 0.9, behavior: reduced ? 'auto' : 'smooth' });
  };

  return (
    <section
      aria-labelledby={headingId}
      className={cn('flex min-w-0 flex-col gap-3', className)}
      data-testid={testId}
    >
      <div className="flex items-end justify-between gap-3 px-1">
        <div className="flex min-w-0 items-center gap-2.5">
          {icon && (
            <span className="bg-accent-soft text-accent-text grid size-8 shrink-0 place-items-center rounded-lg [&_svg]:size-4">
              {icon}
            </span>
          )}
          <div className="flex min-w-0 flex-col">
            <h2 id={headingId} className="truncate text-base font-semibold">
              {title}
            </h2>
            {subtitle && <p className="text-fg-subtle truncate text-xs">{subtitle}</p>}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {onSeeAll && (
            <button
              type="button"
              onClick={onSeeAll}
              aria-label={t('catalog.row.seeAllAria', { title })}
              className="rx-focus-ring text-fg-muted hover:text-fg mr-1 flex items-center gap-1 rounded-md px-1.5 py-1 text-sm font-medium"
              data-testid={testId ? `${testId}-see-all` : undefined}
            >
              {t('catalog.row.seeAll')}
              <ArrowRight className="size-3.5" aria-hidden />
            </button>
          )}
          <IconButton
            size="sm"
            variant="secondary"
            label={t('catalog.row.scrollPrev')}
            icon={<ChevronLeft />}
            disabled={edges.start}
            onClick={() => page(-1)}
            tabIndex={-1}
          />
          <IconButton
            size="sm"
            variant="secondary"
            label={t('catalog.row.scrollNext')}
            icon={<ChevronRight />}
            disabled={edges.end}
            onClick={() => page(1)}
            tabIndex={-1}
          />
        </div>
      </div>
      <ul
        ref={scroller}
        onScroll={measure}
        className="-mx-1 flex snap-x snap-mandatory scroll-px-1 [scrollbar-width:thin] gap-4 overflow-x-auto px-1 pt-1 pb-3"
      >
        {children}
      </ul>
    </section>
  );
}

/** One item of a {@link CardRow}: a fixed card width that snaps. */
export function CardRowItem({
  children,
  className,
  width = 'md',
}: {
  children: ReactNode;
  className?: string;
  width?: 'sm' | 'md' | 'lg';
}) {
  return (
    <li
      className={cn(
        'shrink-0 snap-start',
        width === 'sm' && 'w-52',
        width === 'md' && 'w-64',
        width === 'lg' && 'w-80',
        className,
      )}
    >
      {children}
    </li>
  );
}
