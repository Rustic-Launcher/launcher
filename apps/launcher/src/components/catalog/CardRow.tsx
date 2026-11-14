import { cn } from '@rustic/ui';
import { ChevronLeft, ChevronRight, ArrowRight } from 'lucide-react';
import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * A themed row of cards (Discover: "New", "Popular on Horizon"...): a title, an optional
 * "See all", and a horizontal, snap-scrolling list without a scrollbar: round previous/next
 * arrows on its edges page by the visible width (shown while there is more that way). Children
 * are the `<li>` items (use {@link CardRowItem}).
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
        </div>
      </div>
      <div className="group/row relative">
        <ul
          ref={scroller}
          onScroll={measure}
          className="-mx-1 flex snap-x snap-mandatory scroll-px-1 [scrollbar-width:none] gap-4 overflow-x-auto px-1 pt-1 pb-3 [&::-webkit-scrollbar]:hidden"
        >
          {children}
        </ul>
        <RowArrow
          side="prev"
          hidden={edges.start}
          label={t('catalog.row.scrollPrev')}
          onClick={() => page(-1)}
          testId={testId ? `${testId}-prev` : undefined}
        />
        <RowArrow
          side="next"
          hidden={edges.end}
          label={t('catalog.row.scrollNext')}
          onClick={() => page(1)}
          testId={testId ? `${testId}-next` : undefined}
        />
      </div>
    </section>
  );
}

/** A round arrow over one edge of a {@link CardRow}, on a fade into the page. */
function RowArrow({
  side,
  hidden,
  label,
  onClick,
  testId,
}: {
  side: 'prev' | 'next';
  hidden: boolean;
  label: string;
  onClick: () => void;
  testId?: string;
}) {
  const prev = side === 'prev';
  return (
    <div
      className={cn(
        'pointer-events-none absolute inset-y-0 flex w-16 items-center pb-2 transition-opacity duration-200',
        prev
          ? 'from-surface -left-1 justify-start bg-gradient-to-r to-transparent pl-1'
          : 'from-surface -right-1 justify-end bg-gradient-to-l to-transparent pr-1',
        hidden ? 'opacity-0' : 'opacity-100',
      )}
    >
      <button
        type="button"
        aria-label={label}
        onClick={onClick}
        disabled={hidden}
        tabIndex={-1}
        data-testid={testId}
        className={cn(
          'rx-focus-ring bg-surface-3/95 border-border text-fg hover:bg-accent hover:border-accent grid size-10 place-items-center rounded-full border shadow-lg backdrop-blur transition-colors hover:text-white',
          hidden ? 'pointer-events-none' : 'pointer-events-auto',
        )}
      >
        {prev ? (
          <ChevronLeft className="size-5" aria-hidden />
        ) : (
          <ChevronRight className="size-5" aria-hidden />
        )}
      </button>
    </div>
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
