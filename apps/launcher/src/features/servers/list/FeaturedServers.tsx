import {
  Badge,
  cn,
  EmptyState,
  IconButton,
  Popover,
  PopoverContent,
  PopoverTrigger,
  PremiumCard,
  PremiumGrid,
  Skeleton,
  toast,
} from '@rustic/ui';
import { BadgeCheck, Compass, Eye, Play, Plus } from 'lucide-react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { FeaturedServer } from '@/cloud/types';
import { QueryError } from '@/components/RouteError';
import { usePlay } from '@/features/play/usePlay';
import { formatCompact } from '@/lib/format';
import { useActiveInstance } from '@/queries/core';
import { useFeaturedServers } from '@/queries/social';
import { useMultiplayer, useMultiplayerActions } from './queries';
import { EngineBadge } from './ServerCard';

export type FeaturedSort = 'recommended' | 'players' | 'name';

/** Original banner art for a featured server: layered voxel hills under a hue gradient sky. */
function ServerBanner({ s }: { s: FeaturedServer }) {
  const hills = Array.from({ length: 16 }, (_, i) => 18 + (((s.hue + 7) * (i + 5)) % 30));
  return (
    <div
      aria-hidden
      className="relative aspect-[16/9] overflow-hidden"
      style={{
        background: `linear-gradient(170deg, hsl(${s.hue} 70% 62%) 0%, hsl(${(s.hue + 25) % 360} 55% 38%) 55%, hsl(${(s.hue + 40) % 360} 45% 18%) 100%)`,
      }}
    >
      <svg viewBox="0 0 160 90" preserveAspectRatio="none" className="absolute inset-0 size-full">
        <circle cx="128" cy="22" r="10" fill="white" fillOpacity="0.35" />
        {hills.map((h, i) => (
          <rect
            key={i}
            x={i * 10}
            y={90 - h}
            width="10.5"
            height={h}
            fill={`hsl(${(s.hue + 40) % 360} 40% 14%)`}
            fillOpacity={0.55 + (i % 3) * 0.12}
          />
        ))}
        {hills.map((h, i) => (
          <rect
            key={`t${i}`}
            x={i * 10}
            y={90 - h}
            width="10.5"
            height="2.4"
            fill="white"
            fillOpacity="0.15"
          />
        ))}
      </svg>
      <span className="absolute top-3 left-3 rounded-full bg-black/60 backdrop-blur-sm">
        <EngineBadge engine={s.engine} />
      </span>
      <span className="text-2xs absolute top-3 right-3 rounded-md bg-black/45 px-2 py-0.5 font-semibold text-white uppercase">
        {s.language}
      </span>
      <span className="absolute inset-0 grid place-items-center text-4xl font-black tracking-tight text-white/90 drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
        {s.name}
      </span>
    </div>
  );
}

export function FeaturedServers({
  query = '',
  sort = 'recommended',
  layout = 'grid',
}: {
  query?: string;
  sort?: FeaturedSort;
  layout?: 'grid' | 'list';
}) {
  const { t, i18n } = useTranslation('servers');
  const { data, isLoading, isError, error, refetch } = useFeaturedServers();
  const { data: saved } = useMultiplayer();
  const { add } = useMultiplayerActions();
  const { instance } = useActiveInstance();
  const play = usePlay(instance);

  const list = useMemo(() => {
    const q = query.trim().toLowerCase();
    const l = (data ?? []).filter(
      (s) => !q || s.name.toLowerCase().includes(q) || s.tags.some((x) => x.includes(q)),
    );
    if (sort === 'players')
      return [...l].sort((a, b) => (b.players_online ?? -1) - (a.players_online ?? -1));
    if (sort === 'name') return [...l].sort((a, b) => a.name.localeCompare(b.name));
    return [...l].sort((a, b) => Number(b.verified) - Number(a.verified));
  }, [data, query, sort]);

  if (isError) return <QueryError onRetry={() => void refetch()} message={error.message} />;
  if (data && !list.length)
    return <EmptyState icon={<Compass />} title={t('list.featured.empty')} />;

  return (
    <PremiumGrid
      as="ul"
      className={cn(
        'grid gap-x-4 gap-y-5',
        layout === 'grid' ? 'grid-cols-[repeat(auto-fill,minmax(300px,1fr))]' : 'grid-cols-1',
      )}
      data-testid="featured-list"
    >
      {isLoading &&
        Array.from({ length: 6 }, (_, i) => (
          <li key={i} className="flex flex-col gap-2">
            <Skeleton className="aspect-[16/9] rounded-xl" />
            <Skeleton className="h-5 w-1/2" />
          </li>
        ))}
      {list.map((s) => {
        const inList = saved?.some((e) => e.address === s.address);
        return (
          <li
            key={s.id}
            className={cn('flex gap-3', layout === 'grid' ? 'flex-col' : 'items-center')}
            data-testid={`featured-card-${s.id}`}
          >
            <PremiumCard
              variant={s.verified ? 'featured' : 'default'}
              className={cn(layout === 'list' && 'w-64 shrink-0')}
              bodyClassName="overflow-hidden p-0"
            >
              <ServerBanner s={s} />
            </PremiumCard>
            <div className="flex min-w-0 flex-1 items-center gap-3">
              <span
                aria-hidden
                className="grid size-9 shrink-0 place-items-center rounded-lg text-sm font-black text-white"
                style={{ background: `hsl(${s.hue} 55% 40%)` }}
              >
                {s.name.slice(0, 1)}
              </span>
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="flex items-center gap-1.5">
                  <span className="truncate text-base font-semibold">{s.name}</span>
                  {s.verified && (
                    <BadgeCheck
                      className="text-accent-text size-4 shrink-0"
                      aria-label={t('list.featured.verified')}
                    />
                  )}
                </span>
                {s.players_online != null && (
                  <span className="text-fg-muted text-sm tabular-nums">
                    {t('list.featured.online', {
                      count: s.players_online,
                      value: formatCompact(s.players_online, i18n.language),
                    })}
                  </span>
                )}
              </div>
              <Popover>
                <PopoverTrigger asChild>
                  <IconButton
                    size="sm"
                    label={t('list.featured.view', { name: s.name })}
                    icon={<Eye />}
                  />
                </PopoverTrigger>
                <PopoverContent className="flex w-80 flex-col gap-2 p-4">
                  <span className="font-semibold">{s.name}</span>
                  <code className="text-fg-subtle font-mono text-xs">{s.address}</code>
                  <p className="text-fg-muted text-sm">{s.description}</p>
                  <div className="flex flex-wrap gap-1.5">
                    {s.tags.map((tag) => (
                      <Badge key={tag}>{tag}</Badge>
                    ))}
                  </div>
                  <button
                    type="button"
                    disabled={inList || add.isPending}
                    className="text-accent-text mt-1 flex w-fit items-center gap-1 text-sm font-medium hover:underline disabled:opacity-50"
                    onClick={() =>
                      add.mutate(
                        { name: s.name, address: s.address },
                        {
                          onSuccess: () =>
                            toast({
                              tone: 'success',
                              title: t('multiplayer.added', { name: s.name }),
                            }),
                        },
                      )
                    }
                  >
                    <Plus className="size-4" />
                    {inList ? t('list.featured.inList') : t('list.featured.save')}
                  </button>
                </PopoverContent>
              </Popover>
              <IconButton
                size="sm"
                label={t('multiplayer.joinAria', { name: s.name })}
                icon={<Play className="fill-current" />}
                className="text-accent-text hover:bg-accent-soft hover:text-accent-text"
                disabled={!instance}
                data-testid={`featured-join-${s.id}`}
                onClick={() => {
                  void play.play({ join: { address: s.address, name: s.name } });
                  toast({ tone: 'info', title: t('multiplayer.joining', { name: s.name }) });
                }}
              />
            </div>
          </li>
        );
      })}
    </PremiumGrid>
  );
}
