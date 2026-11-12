import { Skeleton } from '@rustic/ui';

/** Generic page placeholder while a lazy route chunk or its data loads. */
export function PageSkeleton() {
  return (
    <div className="flex flex-col gap-6 p-8" aria-busy="true">
      <Skeleton className="h-8 w-56" />
      <Skeleton className="h-52 w-full rounded-2xl" />
      <div className="grid grid-cols-3 gap-4">
        <Skeleton className="h-36 rounded-xl" />
        <Skeleton className="h-36 rounded-xl" />
        <Skeleton className="h-36 rounded-xl" />
      </div>
    </div>
  );
}

export function GridSkeleton({
  count = 6,
  className = 'h-40',
}: {
  count?: number;
  className?: string;
}) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4" aria-busy="true">
      {Array.from({ length: count }, (_, i) => (
        <Skeleton key={i} className={`${className} rounded-xl`} />
      ))}
    </div>
  );
}

export function ListSkeleton({ rows = 5 }: { rows?: number }) {
  return (
    <div className="flex flex-col gap-2" aria-busy="true">
      {Array.from({ length: rows }, (_, i) => (
        <Skeleton key={i} className="h-14 rounded-lg" />
      ))}
    </div>
  );
}
