import { Button, cn, Skeleton } from '@rustic/ui';
import { BadgeCheck, SearchX, ShieldCheck, X } from 'lucide-react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CatalogEmpty,
  CatalogError,
  CatalogToolbar,
  FilterChip,
  FilterPopover,
  FilterSection,
  FilterSwitch,
  GridSkeletonCards,
  LayoutToggle,
  ListSkeletonRows,
  LoadMore,
  ResultCount,
  SortSelect,
} from '@/components/catalog';
import type { ContentSort, ContentSource, PriceFilter } from '@/ipc/types/content';
import type { InstalledContent } from '@/ipc/types/instances';
import { useQuery } from '@tanstack/react-query';
import { useContentMutations } from '@/features/instances/queries';
import { invoke } from '@/ipc';
import { qk } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { ContentCard, ContentRow, type InstallContext } from './cards';
import type { ExploreContext } from './ContentPage';
import { formatSources, parseSources } from './routes';
import { toQueryTarget } from './target';
import { ALL_SOURCES, useCategories, useContentSearch } from './useContent';

const SORTS: ContentSort[] = ['relevance', 'downloads', 'updated', 'newest'];

/** Installed entries of the target instance by project id, and the update action. */
export function useInstalledMap(instanceId: string | undefined) {
  const content = useQuery({
    queryKey: qk.instanceContent(instanceId ?? '', undefined),
    queryFn: () => invoke('instance_content_list', { instance_id: instanceId! }),
    enabled: !!instanceId,
  });
  const m = useContentMutations(instanceId ?? '');
  const map = useMemo(() => {
    const out = new Map<string, InstalledContent>();
    if (!instanceId) return out;
    for (const c of content.data ?? []) if (c.project_id) out.set(c.project_id, c);
    return out;
  }, [content.data, instanceId]);
  return { map, update: m.update };
}

/** Browse: every matching project, as a dense list or a grid, with filters and paging. */
export function BrowseTab({ ctx, ready }: { ctx: ExploreContext; ready: boolean }) {
  const { t } = useTranslation('content');
  const compatibleOnly = usePrefs((s) => s.compatibleOnly);
  const setCompatibleOnly = usePrefs((s) => s.setCompatibleOnly);
  const layout = usePrefs((s) => s.contentLayout);
  const setLayout = usePrefs((s) => s.setContentLayout);
  const { kind, search } = ctx;
  const sort: ContentSort = search.sort ?? 'relevance';
  const sources = parseSources(search.sources);
  const category = search.category;
  const verified = !!search.verified;
  const price = search.price;

  const instanceId = ctx.target?.kind === 'instance' ? ctx.target.id : undefined;
  const installed = useInstalledMap(instanceId);
  const install: InstallContext = {
    target: ctx.target,
    targetName: ctx.targetName,
    installed: installed.map,
    onUpdate: (e) => installed.update.mutate(e.id),
    updating: installed.update.isPending,
  };

  const results = useContentSearch(
    {
      query: ctx.query,
      kind,
      sources,
      target: toQueryTarget(ctx.target, kind),
      compatible_only: compatibleOnly,
      category,
      sort,
      price,
      verified: verified || undefined,
    },
    ready,
  );
  const categories = useCategories(kind);
  const hits = results.data?.pages.flatMap((p) => p.hits) ?? [];
  const total = results.data?.pages[0]?.total ?? 0;

  const toggleSource = (s: ContentSource) => {
    const next = sources.includes(s)
      ? sources.length > 1
        ? sources.filter((x) => x !== s)
        : sources
      : [...sources, s];
    ctx.setSearch({ sources: formatSources(next) }, true);
  };
  const activeFilters =
    (sources.length < ALL_SOURCES.length ? 1 : 0) +
    (category ? 1 : 0) +
    (verified ? 1 : 0) +
    (price ? 1 : 0) +
    (compatibleOnly ? 0 : 1);
  const resetFilters = () => {
    setCompatibleOnly(true);
    ctx.setSearch(
      { sources: undefined, category: undefined, verified: undefined, price: undefined },
      true,
    );
  };
  const compatLabel =
    kind === 'modpack'
      ? t('filters.worksNow')
      : ctx.target?.kind === 'server'
        ? t('filters.compatibleServer')
        : t('filters.compatibleInstance');

  return (
    <>
      <CatalogToolbar
        start={results.data ? <ResultCount count={total} /> : <Skeleton className="h-4 w-24" />}
        end={
          <>
            <SortSelect
              value={sort}
              onChange={(s) => ctx.setSearch({ sort: s === 'relevance' ? undefined : s }, true)}
              options={SORTS.map((s) => ({ value: s, label: t(`sort.${s}`) }))}
            />
            <FilterPopover active={activeFilters} onReset={resetFilters} testId="content-filters">
              <FilterSection title={t('sources.label')}>
                <div
                  className="flex flex-wrap gap-1.5"
                  role="group"
                  aria-label={t('sources.label')}
                >
                  {ALL_SOURCES.map((s) => (
                    <FilterChip
                      key={s}
                      on={sources.includes(s)}
                      onToggle={() => toggleSource(s)}
                      testId={`source-${s}`}
                    >
                      {t(`sources.${s}`)}
                    </FilterChip>
                  ))}
                </div>
              </FilterSection>
              <FilterSection
                title={t('filters.compatibility')}
                hint={kind === 'plugin' ? t('filters.pluginHint') : t('filters.fairPlayHint')}
              >
                <FilterSwitch
                  checked={compatibleOnly}
                  onCheckedChange={setCompatibleOnly}
                  label={compatLabel}
                  icon={<ShieldCheck className="text-success-text" />}
                  testId="compatible-only"
                />
              </FilterSection>
              <FilterSection title={t('filters.rustic')} hint={t('filters.rusticHint')}>
                <FilterSwitch
                  checked={verified}
                  onCheckedChange={(v) => ctx.setSearch({ verified: v || undefined }, true)}
                  label={t('filters.verifiedOnly')}
                  icon={<BadgeCheck className="text-accent-text" />}
                  testId="verified-only"
                />
                <div
                  className="flex flex-wrap gap-1.5"
                  role="group"
                  aria-label={t('filters.price')}
                >
                  {([undefined, 'free', 'paid'] as (PriceFilter | undefined)[]).map((p) => (
                    <FilterChip
                      key={p ?? 'all'}
                      on={price === p}
                      onToggle={() => ctx.setSearch({ price: p }, true)}
                      testId={`price-${p ?? 'all'}`}
                    >
                      {t(`filters.priceOptions.${p ?? 'all'}`)}
                    </FilterChip>
                  ))}
                </div>
              </FilterSection>
              <FilterSection title={t('filters.categories')}>
                {categories.isLoading ? (
                  <Skeleton className="h-16 rounded-lg" />
                ) : (
                  <div className="flex flex-wrap gap-1.5">
                    {(categories.data ?? []).map((c) => (
                      <FilterChip
                        key={c.id}
                        on={category === c.id}
                        onToggle={() =>
                          ctx.setSearch({ category: category === c.id ? undefined : c.id }, true)
                        }
                        testId={`category-${c.id}`}
                      >
                        {t(`categories.${c.id as 'utility'}`, { defaultValue: c.id })}
                      </FilterChip>
                    ))}
                  </div>
                )}
              </FilterSection>
            </FilterPopover>
            <LayoutToggle value={layout} onChange={setLayout} />
          </>
        }
      />
      <ActiveFilters ctx={ctx} />

      {results.isError ? (
        <CatalogError
          onRetry={() => void results.refetch()}
          message={(results.error as Error).message}
        />
      ) : !ready || results.isLoading ? (
        layout === 'grid' ? (
          <GridSkeletonCards className="grid-cols-[repeat(auto-fill,minmax(240px,1fr))]" />
        ) : (
          <ListSkeletonRows />
        )
      ) : hits.length === 0 ? (
        <CatalogEmpty
          icon={<SearchX />}
          testId="browse-empty"
          title={t('empty.title')}
          description={compatibleOnly ? t('empty.compatibleHint') : t('empty.hint')}
          action={
            compatibleOnly ? (
              <Button size="sm" onClick={() => setCompatibleOnly(false)}>
                {t('empty.showAll')}
              </Button>
            ) : activeFilters > 0 ? (
              <Button size="sm" onClick={resetFilters}>
                {t('filters.reset')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        <>
          {layout === 'grid' ? (
            <ul
              className={cn(
                'grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4 transition-opacity',
                results.isFetching && !results.isFetchingNextPage && 'opacity-70',
              )}
              data-testid="browse-grid"
            >
              {hits.map((h) => (
                <li key={h.id}>
                  <ContentCard item={h} ctx={install} withAction />
                </li>
              ))}
            </ul>
          ) : (
            <ul
              className={cn(
                'flex flex-col transition-opacity',
                results.isFetching && !results.isFetchingNextPage && 'opacity-70',
              )}
              data-testid="browse-list"
            >
              {hits.map((h) => (
                <li key={h.id}>
                  <ContentRow item={h} ctx={install} />
                </li>
              ))}
            </ul>
          )}
          {results.hasNextPage && (
            <LoadMore
              shown={hits.length}
              total={total}
              loading={results.isFetchingNextPage}
              onLoad={() => void results.fetchNextPage()}
              testId="load-more"
            />
          )}
        </>
      )}
    </>
  );
}

/** Removable chips for the filters that differ from the defaults. */
function ActiveFilters({ ctx }: { ctx: ExploreContext }) {
  const { t } = useTranslation('content');
  const { search } = ctx;
  const chips: { key: string; label: string; clear: () => void }[] = [];
  if (search.sources) {
    chips.push({
      key: 'sources',
      label: parseSources(search.sources)
        .map((s) => t(`sources.${s}`))
        .join(' + '),
      clear: () => ctx.setSearch({ sources: undefined }, true),
    });
  }
  if (search.category) {
    chips.push({
      key: 'category',
      label: t(`categories.${search.category as 'utility'}`, { defaultValue: search.category }),
      clear: () => ctx.setSearch({ category: undefined }, true),
    });
  }
  if (search.verified) {
    chips.push({
      key: 'verified',
      label: t('filters.verifiedOnly'),
      clear: () => ctx.setSearch({ verified: undefined }, true),
    });
  }
  if (search.price) {
    chips.push({
      key: 'price',
      label: t(`filters.priceOptions.${search.price}`),
      clear: () => ctx.setSearch({ price: undefined }, true),
    });
  }
  if (chips.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5" data-testid="active-filters">
      {chips.map((c) => (
        <button
          key={c.key}
          type="button"
          onClick={c.clear}
          aria-label={t('filters.remove', { filter: c.label })}
          className="rx-focus-ring border-accent/40 bg-accent-soft text-accent-text hover:bg-accent/20 inline-flex h-7 items-center gap-1 rounded-full border px-2.5 text-xs font-medium"
        >
          {c.label}
          <X className="size-3" aria-hidden />
        </button>
      ))}
    </div>
  );
}
