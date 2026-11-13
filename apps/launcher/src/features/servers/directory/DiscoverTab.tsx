import { Button, IconButton, toast } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import {
  Clock,
  Compass,
  ExternalLink,
  Flame,
  Hammer,
  History,
  Mountain,
  Play,
  Sparkles,
  Swords,
  Theater,
  Trees,
  Trophy,
  Users,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CardRow,
  CardRowItem,
  CatalogEmpty,
  CatalogError,
  HeroCarousel,
  HeroSkeleton,
  MetaItem,
  RowSkeletonItems,
  VisualCard,
} from '@/components/catalog';
import type { ServerListingQuery } from '@/cloud/types';
import { usePlay } from '@/features/play/usePlay';
import { invoke } from '@/ipc';
import { formatRelative } from '@/lib/format';
import { useActiveInstance } from '@/queries/core';
import { useMultiplayer } from '../list/queries';
import type { ServersContext } from '../ServersPage';
import type { ServersSearch } from '../routes';
import { ListingCard, ListingHeroSlide } from './cards';
import { heroListings, PUBLISH_SERVER_URL, useListings } from './queries';

const ROW_SIZE = 12;

interface RowSpec {
  id: string;
  title: string;
  subtitle?: string;
  icon: ReactNode;
  query: ServerListingQuery;
  seeAll: Partial<ServersSearch>;
}

/** Game styles that get a row of their own, in this order. */
const CATEGORY_ROWS: { tag: string; icon: ReactNode }[] = [
  { tag: 'survival', icon: <Trees /> },
  { tag: 'minigames', icon: <Trophy /> },
  { tag: 'pvp', icon: <Swords /> },
  { tag: 'skyblock', icon: <Mountain /> },
  { tag: 'roleplay', icon: <Theater /> },
  { tag: 'creative', icon: <Hammer /> },
];

/** Discover: a carousel of staff picks and verified servers, then themed rows. */
export function DiscoverTab({ ctx }: { ctx: ServersContext }) {
  const { t } = useTranslation('servers');
  const featured = useListings({ featured: true, sort: 'featured', limit: 6 });
  const verified = useListings({ verified: true, sort: 'trending', limit: ROW_SIZE });
  const picks = heroListings(featured.data?.items ?? [], verified.data?.items ?? []);
  // The same query as the "Trending" row (shared cache): nothing at all means a fresh directory.
  const trending = useListings({ sort: 'trending', limit: ROW_SIZE });

  const rows: RowSpec[] = [
    {
      id: 'trending',
      title: t('directory.rows.trending'),
      subtitle: t('directory.rows.trendingHint'),
      icon: <Flame />,
      query: { sort: 'trending' },
      seeAll: { sort: 'trending' },
    },
    {
      id: 'players',
      title: t('directory.rows.players'),
      subtitle: t('directory.rows.playersHint'),
      icon: <Users />,
      query: { sort: 'players' },
      seeAll: { sort: 'players' },
    },
    {
      id: 'new',
      title: t('directory.rows.new'),
      subtitle: t('directory.rows.newHint'),
      icon: <Sparkles />,
      query: { sort: 'new' },
      seeAll: { sort: 'new' },
    },
    ...CATEGORY_ROWS.map(({ tag, icon }) => ({
      id: `tag-${tag}`,
      title: t(`directory.tags.${tag as 'survival'}`),
      icon,
      query: { tag, sort: 'trending' as const },
      seeAll: { tag },
    })),
  ];

  const loading = featured.isLoading || verified.isLoading;
  if (trending.data && trending.data.total === 0) {
    return (
      <>
        <DirectoryEmpty />
        <RecentRow />
        <PublishCard />
      </>
    );
  }
  return (
    <>
      {featured.isError && verified.isError ? (
        <CatalogError
          onRetry={() => {
            void featured.refetch();
            void verified.refetch();
          }}
          message={(featured.error as Error).message}
        />
      ) : loading ? (
        <HeroSkeleton />
      ) : picks.length > 0 ? (
        <HeroCarousel
          label={t('directory.hero.label')}
          items={picks}
          getKey={(l) => l.id}
          className="h-[clamp(288px,36vw,380px)] w-full"
          testId="servers-hero"
          renderSlide={(l) => <ListingHeroSlide listing={l} />}
        />
      ) : null}
      {rows.slice(0, 3).map((r) => (
        <DiscoverRow key={r.id} spec={r} ctx={ctx} />
      ))}
      <RecentRow />
      {rows.slice(3).map((r) => (
        <DiscoverRow key={r.id} spec={r} ctx={ctx} />
      ))}
      <PublishCard />
    </>
  );
}

function DiscoverRow({ spec, ctx }: { spec: RowSpec; ctx: ServersContext }) {
  const { t } = useTranslation('servers');
  const list = useListings({ ...spec.query, limit: ROW_SIZE });
  const items = list.data?.items ?? [];
  // A row with nothing to show is left out (no empty shelves).
  if (list.data && items.length === 0) return null;
  return (
    <CardRow
      title={spec.title}
      subtitle={spec.subtitle}
      icon={spec.icon}
      onSeeAll={() => ctx.browse(spec.seeAll)}
      testId={`servers-row-${spec.id}`}
    >
      {list.isError ? (
        <li className="w-full">
          <div className="border-border text-fg-muted flex items-center gap-3 rounded-xl border border-dashed px-4 py-6 text-sm">
            {t('directory.rowError')}
            <Button size="sm" onClick={() => void list.refetch()}>
              {t('directory.retry')}
            </Button>
          </div>
        </li>
      ) : !list.data ? (
        <RowSkeletonItems />
      ) : (
        items.map((l) => (
          <CardRowItem key={l.id}>
            <ListingCard listing={l} />
          </CardRowItem>
        ))
      )}
    </CardRow>
  );
}

/** The servers of the multiplayer list joined lately (local: the list's last join dates). */
function RecentRow() {
  const { t, i18n } = useTranslation('servers');
  const { data } = useMultiplayer();
  const { instance } = useActiveInstance();
  const play = usePlay(instance);
  const recent = (data ?? [])
    .filter((e) => e.last_joined_at)
    .sort((a, b) => (b.last_joined_at ?? '').localeCompare(a.last_joined_at ?? ''))
    .slice(0, ROW_SIZE);
  if (recent.length === 0) return null;
  return (
    <CardRow
      title={t('directory.rows.recent')}
      subtitle={t('directory.rows.recentHint')}
      icon={<History />}
      testId="servers-row-recent"
    >
      {recent.map((e) => (
        <CardRowItem key={e.id} width="sm">
          <VisualCard
            seed={e.address}
            title={e.name}
            testId={`recent-${e.id}`}
            renderTitle={(className) => (
              <Link to="/servers" search={{ tab: 'mine' }} className={className}>
                {e.name}
              </Link>
            )}
            byline={<code className="font-mono">{e.address}</code>}
            meta={
              <MetaItem icon={<Clock />} label={t('directory.lastPlayed')}>
                {formatRelative(e.last_joined_at!, i18n.language)}
              </MetaItem>
            }
            action={
              <IconButton
                size="sm"
                variant="secondary"
                label={t('multiplayer.joinAria', { name: e.name })}
                icon={<Play className="fill-current" />}
                disabled={!instance}
                onClick={() => {
                  void play.play({ join: { address: e.address, name: e.name } });
                  toast({ tone: 'info', title: t('multiplayer.joining', { name: e.name }) });
                }}
              />
            }
          />
        </CardRowItem>
      ))}
    </CardRow>
  );
}

/** "Your server here": the website's publishing form. */
function PublishCard() {
  const { t } = useTranslation('servers');
  return (
    <section
      className="border-border bg-surface/60 flex flex-wrap items-center gap-4 rounded-2xl border border-dashed px-6 py-5"
      aria-labelledby="publish-cta"
    >
      <span className="bg-accent-soft text-accent-text grid size-11 shrink-0 place-items-center rounded-xl">
        <Compass className="size-5" aria-hidden />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <h2 id="publish-cta" className="text-base font-semibold">
          {t('directory.publishCta.title')}
        </h2>
        <p className="text-fg-muted text-sm">{t('directory.publishCta.hint')}</p>
      </div>
      <Button
        variant="primary"
        leftIcon={<ExternalLink />}
        onClick={() => void invoke('app_open_url', { url: PUBLISH_SERVER_URL })}
      >
        {t('directory.publish')}
      </Button>
    </section>
  );
}

/** Nothing listed at all yet (a fresh cloud): a hint instead of empty rows. */
function DirectoryEmpty() {
  const { t } = useTranslation('servers');
  return (
    <CatalogEmpty
      icon={<Compass />}
      title={t('directory.empty.title')}
      description={t('directory.empty.hint')}
    />
  );
}
