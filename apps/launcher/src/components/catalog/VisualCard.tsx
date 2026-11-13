import { cn } from '@rustic/ui';
import type { ReactNode } from 'react';
import { CatalogCover, CatalogThumb } from './CatalogImage';

/**
 * A visual card (Discover rows, the Browse grid): a 16:9 cover (picture or generated art) with
 * overlay badges, the icon overlapping it, the title, a byline and a meta line. The title is the
 * card's one link (`renderTitle` gets the class that stretches it over the whole card); `action`
 * sits above that link and stays clickable.
 */
export function VisualCard({
  seed,
  title,
  renderTitle,
  byline,
  cover,
  icon,
  overlay,
  meta,
  action,
  className,
  testId,
  coverArt,
  thumb,
  corner,
  coverAction,
  coverFooter,
  highlighted,
}: {
  /** Seeds the generated art (a project id). */
  seed: string;
  title: string;
  /** Renders the title as a link; spread `className` on it. */
  renderTitle: (className: string) => ReactNode;
  byline?: ReactNode;
  cover?: string | null;
  icon?: string | null;
  /** Badges over the cover (top left). */
  overlay?: ReactNode;
  /** Small facts under the title (downloads, size...). */
  meta?: ReactNode;
  action?: ReactNode;
  className?: string;
  testId?: string;
  /** The page's own art instead of the picture / generated art (fills the cover). */
  coverArt?: ReactNode;
  /** The page's own icon instead of the square thumbnail. */
  thumb?: ReactNode;
  /** A control in the cover's top-right corner (a menu), above the title link. */
  corner?: ReactNode;
  /** A control centred on the cover (a Play button), above the title link. */
  coverAction?: ReactNode;
  /** Text along the cover's bottom edge (a version stamp, a progress bar); not clickable. */
  coverFooter?: ReactNode;
  /** Marks the card (the active profile). */
  highlighted?: boolean;
}) {
  const layers = (
    <>
      <span
        aria-hidden
        className="absolute inset-0 bg-gradient-to-t from-black/45 via-transparent to-transparent"
      />
      {overlay && (
        <div
          className={cn(
            'absolute top-2.5 left-2.5 flex flex-wrap gap-1.5',
            corner ? 'right-12' : 'right-2.5',
          )}
        >
          {overlay}
        </div>
      )}
      {coverFooter && (
        <div className="pointer-events-none absolute inset-x-0 bottom-0">{coverFooter}</div>
      )}
      {coverAction && (
        <div className="pointer-events-none absolute inset-0 z-10 grid place-items-center [&>*]:pointer-events-auto">
          {coverAction}
        </div>
      )}
      {corner && <div className="absolute top-2 right-2 z-10">{corner}</div>}
    </>
  );
  return (
    <article
      className={cn(
        'group/card rx-surface relative flex h-full flex-col overflow-hidden rounded-xl transition-transform duration-200 ease-out hover:-translate-y-0.5 motion-reduce:hover:translate-y-0',
        highlighted && 'ring-accent ring-2',
        className,
      )}
      data-testid={testId}
    >
      {coverArt ? (
        <div className="relative aspect-[16/9] shrink-0 overflow-hidden">
          {coverArt}
          {layers}
        </div>
      ) : (
        <CatalogCover src={cover} seed={seed} className="aspect-[16/9] shrink-0">
          {layers}
        </CatalogCover>
      )}
      <div className="flex flex-1 flex-col gap-1.5 px-3.5 pt-1 pb-3.5">
        {thumb ?? (
          <CatalogThumb
            src={icon}
            seed={seed}
            name={title}
            size="md"
            className="ring-surface -mt-7 mb-0.5 ring-4"
          />
        )}
        {renderTitle(
          "rx-focus-ring text-fg hover:text-accent-text truncate rounded text-[15px] font-semibold leading-tight after:absolute after:inset-0 after:content-['']",
        )}
        {byline && <p className="text-fg-muted truncate text-xs">{byline}</p>}
        {meta && (
          <div className="text-fg-subtle mt-auto flex flex-wrap items-center gap-x-2.5 gap-y-1 pt-1 text-xs tabular-nums">
            {meta}
          </div>
        )}
        {action && <div className="relative z-10 mt-2 flex items-center gap-2">{action}</div>}
      </div>
    </article>
  );
}

/** One fact of a card or row meta line. */
export function MetaItem({
  icon,
  children,
  label,
}: {
  icon?: ReactNode;
  children: ReactNode;
  label?: string;
}) {
  return (
    <span className="inline-flex items-center gap-1 [&_svg]:size-3.5" title={label}>
      {icon && (
        <span aria-hidden className="contents">
          {icon}
        </span>
      )}
      {label && <span className="sr-only">{label}: </span>}
      {children}
    </span>
  );
}
