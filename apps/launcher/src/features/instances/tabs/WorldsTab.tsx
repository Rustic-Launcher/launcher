import { Badge, Button, Card, EmptyState, formatBytes } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { FolderOpen, Globe2, Play, Puzzle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { GridSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { usePlay } from '@/features/play/usePlay';
import type { Instance } from '@/ipc/types/instances';
import { formatRelative } from '@/lib/format';
import { useInstanceMutations, useInstanceWorlds } from '../queries';

export function WorldsTab({ instance }: { instance: Instance }) {
  const { t, i18n } = useTranslation('instances');
  const { data, isLoading, isError, refetch } = useInstanceWorlds(instance.id);
  const play = usePlay(instance);
  const { openFolder } = useInstanceMutations();
  const busy = play.state === 'progress' || play.state === 'running';

  if (isLoading) return <GridSkeleton count={2} className="h-36" />;
  if (isError) return <QueryError onRetry={() => void refetch()} />;
  if (!data || data.length === 0) {
    return (
      <EmptyState
        icon={<Globe2 />}
        title={t('worlds.empty.title')}
        description={t('worlds.empty.description')}
      />
    );
  }
  return (
    <div className="flex flex-col gap-3">
      <div className="flex justify-end">
        <Button
          size="sm"
          leftIcon={<FolderOpen />}
          onClick={() => openFolder.mutate({ id: instance.id, folder: 'saves' })}
        >
          {t('worlds.openSaves')}
        </Button>
      </div>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
        {data.map((w) => (
          <li key={w.id}>
            <Card padding="md" className="flex h-full flex-col gap-3">
              <div className="flex items-start justify-between gap-2">
                <div className="flex min-w-0 flex-col">
                  <span className="truncate text-base font-semibold">{w.name}</span>
                  <span className="text-fg-subtle text-xs">
                    {w.last_played_at
                      ? t('worlds.lastPlayed', {
                          when: formatRelative(w.last_played_at, i18n.language),
                        })
                      : t('card.never')}
                    {' · '}
                    {formatBytes(w.size_bytes, i18n.language)}
                  </span>
                </div>
                <Badge variant={w.mode === 'hardcore' ? 'danger' : 'neutral'}>
                  {t(`worlds.modes.${w.mode}`)}
                </Badge>
              </div>
              <p className="text-fg-muted text-xs">
                {t('worlds.datapacks', { count: w.datapack_count })}
              </p>
              <div className="mt-auto flex gap-2">
                <Button
                  size="sm"
                  variant="primary"
                  leftIcon={<Play />}
                  disabled={busy}
                  onClick={() => void play.play({ world_id: w.id })}
                >
                  {t('worlds.play')}
                </Button>
                <Button asChild size="sm">
                  <Link
                    to="/content"
                    search={{ kind: 'datapack', target: `instance:${instance.id}` }}
                  >
                    <Puzzle /> {t('worlds.addDatapacks')}
                  </Link>
                </Button>
              </div>
            </Card>
          </li>
        ))}
      </ul>
    </div>
  );
}
