import { Button } from '@rustic/ui';
import { BadgeCheck, Flame, Sparkles, Target, Users } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CardRow,
  CardRowItem,
  CatalogEmpty,
  CatalogError,
  HeroCarousel,
  HeroSkeleton,
  RowSkeletonItems,
} from '@/components/catalog';
import type { ContentSearchQuery, ContentSummary } from '@/ipc/types/content';
import { shortVersion } from '@/lib/format';
import { usePrefs } from '@/stores/prefs';
import { ContentCard, HeroSlide, type InstallContext } from './cards';
import type { ExploreContext } from './ContentPage';
import type { ContentSearch } from './routes';
import { toQueryTarget } from './target';
import { ALL_SOURCES, useContentList } from './useContent';

const ROW_SIZE = 12;

/** Never promoted: flagged by the fair-play policy, or refused by it. */
export function promotable(h: ContentSummary): boolean {
  return (
    !h.flagged &&
    !(h.compatibility.status === 'incompatible' && h.compatibility.reason === 'not_fair_play')
  );
}

/**
 * The hero's picks: promotable, installable projects, verified and featured ones first, with a
 * picture first among equals; at most `max`.
 */
export function heroPicks(hits: ContentSummary[], max = 5): ContentSummary[] {
  const score = (h: ContentSummary) =>
    (h.verified ? 4 : 0) + (h.featured ? 2 : 0) + (h.banner_url ? 1 : 0);
  const seen = new Set<string>();
  return hits
    .filter((h) => !seen.has(h.id) && !!seen.add(h.id))
    .filter((h) => promotable(h) && h.compatibility.status !== 'incompatible')
    .map((h, i) => ({ h, i }))
    .sort((a, b) => score(b.h) - score(a.h) || a.i - b.i)
    .slice(0, max)
    .map(({ h }) => h);
}

interface RowSpec {
  id: string;
  title: string;
  subtitle?: string;
  icon: ReactNode;
  query: Omit<ContentSearchQuery, 'offset' | 'limit' | 'query' | 'kind' | 'target'>;
  /** The Browse filters "See all" opens. */
  seeAll: Partial<ContentSearch>;
}

/** Discover: a featured carousel, then themed rows of cards. */
export function DiscoverTab({ ctx, ready }: { ctx: ExploreContext; ready: boolean }) {
  const { t } = useTranslation('content');
  const compatibleOnly = usePrefs((s) => s.compatibleOnly);
  const base = { query: '', kind: ctx.kind, target: toQueryTarget(ctx.target, ctx.kind) };
  const install: InstallContext = { target: ctx.target, targetName: ctx.targetName };

  const hero = useContentList(
    {
      ...base,
      sources: ALL_SOURCES,
      sort: 'downloads',
      compatible_only: true,
      limit: ROW_SIZE,
    },
    ready,
  );
  // Rustic's verified projects lead the hero (the same query as their row, shared).
  const verified = useContentList(
    {
      ...base,
      sources: ['rustic'],
      sort: 'downloads',
      compatible_only: false,
      verified: true,
      limit: ROW_SIZE,
    },
    ready,
  );
  const picks = heroPicks([...(verified.data?.hits ?? []), ...(hero.data?.hits ?? [])]);

  const rows: RowSpec[] = [
    {
      id: 'new',
      title: t('discover.rows.new'),
      subtitle: t('discover.rows.newHint'),
      icon: <Sparkles />,
      query: { sources: ALL_SOURCES, sort: 'newest', compatible_only: compatibleOnly },
      seeAll: { sort: 'newest' },
    },
    ...(ctx.kind === 'plugin'
      ? []
      : [
          {
            id: 'horizon',
            title: t('discover.rows.horizon'),
            subtitle: t('discover.rows.horizonHint'),
            icon: <Flame />,
            query: {
              sources: ['horizon'],
              sort: 'downloads',
              compatible_only: compatibleOnly,
            },
            seeAll: { sort: 'downloads', sources: 'horizon' },
          } satisfies RowSpec,
        ]),
    {
      id: 'verified',
      title: t('discover.rows.verified'),
      subtitle: t('discover.rows.verifiedHint'),
      icon: <BadgeCheck />,
      query: { sources: ['rustic'], sort: 'downloads', compatible_only: false, verified: true },
      seeAll: { sources: 'rustic', verified: true, sort: 'downloads' },
    },
    ...(ctx.target && ctx.kind !== 'modpack'
      ? [
          {
            id: 'yours',
            title: t('discover.rows.yours', { target: ctx.targetName }),
            subtitle: ctx.targetVersion
              ? t('discover.rows.yoursHint', { version: shortVersion(ctx.targetVersion) })
              : undefined,
            icon: <Target />,
            query: { sources: ALL_SOURCES, sort: 'relevance', compatible_only: true },
            seeAll: { sort: 'relevance' },
          } satisfies RowSpec,
        ]
      : []),
    {
      id: 'community',
      title: t('discover.rows.community'),
      subtitle: t('discover.rows.communityHint'),
      icon: <Users />,
      query: { sources: ['rustic'], sort: 'updated', compatible_only: false },
      seeAll: { sources: 'rustic', sort: 'updated' },
    },
  ];

  return (
    <>
      {hero.isError ? (
        <CatalogError onRetry={() => void hero.refetch()} message={(hero.error as Error).message} />
      ) : !ready || hero.isLoading ? (
        <HeroSkeleton />
      ) : picks.length > 0 ? (
        <HeroCarousel
          label={t(`discover.heroLabel.${ctx.kind}`)}
          items={picks}
          getKey={(h) => h.id}
          className="h-[clamp(288px,36vw,380px)] w-full"
          testId="discover-hero"
          renderSlide={(h) => <HeroSlide item={h} ctx={install} />}
        />
      ) : null}
      {rows.map((r) => (
        <DiscoverRow
          key={`${ctx.kind}-${r.id}`}
          spec={r}
          ctx={ctx}
          install={install}
          ready={ready}
        />
      ))}
      {!hero.isLoading && hero.data && hero.data.hits.length === 0 && (
        <CatalogEmpty
          icon={<Sparkles />}
          title={t('discover.empty.title')}
          description={t('discover.empty.hint')}
          action={
            <Button size="sm" onClick={() => ctx.browse({})}>
              {t('discover.empty.browse')}
            </Button>
          }
        />
      )}
    </>
  );
}

function DiscoverRow({
  spec,
  ctx,
  install,
  ready,
}: {
  spec: RowSpec;
  ctx: ExploreContext;
  install: InstallContext;
  ready: boolean;
}) {
  const { t } = useTranslation('content');
  const list = useContentList(
    {
      ...spec.query,
      query: '',
      kind: ctx.kind,
      target: toQueryTarget(ctx.target, ctx.kind),
      limit: ROW_SIZE,
    },
    ready,
  );
  const hits = (list.data?.hits ?? []).filter((h) => spec.id !== 'horizon' || promotable(h));
  // A row with nothing to show is left out (no empty shelves).
  if (list.data && hits.length === 0) return null;
  return (
    <CardRow
      title={spec.title}
      subtitle={spec.subtitle}
      icon={spec.icon}
      onSeeAll={() => ctx.browse(spec.seeAll)}
      testId={`discover-row-${spec.id}`}
    >
      {list.isError ? (
        <li className="w-full">
          <div className="border-border text-fg-muted flex items-center gap-3 rounded-xl border border-dashed px-4 py-6 text-sm">
            {t('discover.rowError')}
            <Button size="sm" onClick={() => void list.refetch()}>
              {t('discover.retry')}
            </Button>
          </div>
        </li>
      ) : !list.data ? (
        <RowSkeletonItems />
      ) : (
        hits.map((h) => (
          <CardRowItem key={h.id}>
            <ContentCard item={h} ctx={install} />
          </CardRowItem>
        ))
      )}
    </CardRow>
  );
}
