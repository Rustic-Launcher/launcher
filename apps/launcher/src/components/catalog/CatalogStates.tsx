import { Button, cn, EmptyState, Skeleton } from '@rustic/ui';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { CardRowItem } from './CardRow';

/** Nothing to show: an icon, a title, a hint and an optional action. */
export function CatalogEmpty({
  icon,
  title,
  description,
  action,
  className,
  testId,
}: {
  icon: ReactNode;
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
  testId?: string;
}) {
  return (
    <div data-testid={testId} className={className}>
      <EmptyState icon={icon} title={title} description={description} action={action} />
    </div>
  );
}

/** A failed query, with Retry. */
export function CatalogError({ message, onRetry }: { message?: string; onRetry: () => void }) {
  return <QueryError onRetry={onRetry} message={message} />;
}

export function ListSkeletonRows({ rows = 6 }: { rows?: number }) {
  return (
    <div className="flex flex-col" aria-busy="true">
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="border-border flex items-center gap-4 border-b px-2 py-3.5">
          <Skeleton className="size-16 rounded-xl" />
          <div className="flex flex-1 flex-col gap-2">
            <Skeleton className="h-4 w-56" />
            <Skeleton className="h-3 w-3/4" />
            <Skeleton className="h-3 w-64" />
          </div>
          <Skeleton className="h-8 w-28 rounded-full" />
        </div>
      ))}
    </div>
  );
}

export function GridSkeletonCards({
  count = 8,
  className,
}: {
  count?: number;
  className?: string;
}) {
  return (
    <div className={cn('grid gap-4', className)} aria-busy="true">
      {Array.from({ length: count }, (_, i) => (
        <CardSkeleton key={i} />
      ))}
    </div>
  );
}

function CardSkeleton() {
  return (
    <div className="rx-surface flex flex-col overflow-hidden rounded-xl">
      <Skeleton className="aspect-[16/9] rounded-none" />
      <div className="flex flex-col gap-2 p-3.5">
        <Skeleton className="h-4 w-2/3" />
        <Skeleton className="h-3 w-1/2" />
      </div>
    </div>
  );
}

/** Skeleton items for a {@link CardRow}. */
export function RowSkeletonItems({ count = 5 }: { count?: number }) {
  return (
    <>
      {Array.from({ length: count }, (_, i) => (
        <CardRowItem key={i}>
          <CardSkeleton />
        </CardRowItem>
      ))}
    </>
  );
}

export function HeroSkeleton({ className }: { className?: string }) {
  return <Skeleton className={cn('h-[clamp(288px,36vw,380px)] w-full rounded-2xl', className)} />;
}

/** "Load more" with the count shown so far (announced politely). */
export function LoadMore({
  shown,
  total,
  loading,
  onLoad,
  testId,
}: {
  shown: number;
  total: number;
  loading: boolean;
  onLoad: () => void;
  testId?: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center gap-2 py-2">
      <Button onClick={onLoad} loading={loading} data-testid={testId}>
        {t('catalog.loadMore')}
      </Button>
      <p className="text-fg-subtle text-xs tabular-nums" aria-live="polite">
        {t('catalog.loadMoreCount', { shown, total: Math.max(total, shown) })}
      </p>
    </div>
  );
}
