import { Button, EmptyState } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { Plus, Server as ServerIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { GridSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { useServers } from './queries';
import { ServerCard } from './ServerCard';

export function MyServers({ query = '' }: { query?: string }) {
  const { t } = useTranslation('servers');
  const { data: all, isLoading, isError, error, refetch } = useServers();
  const q = query.trim().toLowerCase();
  const data = all?.filter((s) => !q || s.name.toLowerCase().includes(q));

  if (isLoading) return <GridSkeleton count={3} className="h-52" />;
  if (isError) return <QueryError onRetry={() => void refetch()} message={error.message} />;
  if (!data || data.length === 0) {
    return (
      <EmptyState
        icon={<ServerIcon />}
        title={t('list.empty.title')}
        description={t('list.empty.description')}
        action={
          <Button asChild variant="primary">
            <Link to="/servers/new">
              <Plus />
              {t('list.create')}
            </Link>
          </Button>
        }
      />
    );
  }
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
      {data.map((s) => (
        <ServerCard key={s.id} server={s} />
      ))}
      <Link
        to="/servers/new"
        className="rx-focus-ring border-border text-fg-muted hover:border-accent/50 hover:bg-accent-soft hover:text-accent-text flex min-h-52 flex-col items-center justify-center gap-2 rounded-xl border border-dashed text-sm font-medium transition-colors"
      >
        <Plus className="size-6" aria-hidden />
        {t('list.create')}
      </Link>
    </div>
  );
}
