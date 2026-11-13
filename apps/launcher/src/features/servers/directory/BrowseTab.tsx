import { Button, cn, Skeleton } from '@rustic/ui';
import { BadgeCheck, SearchX, X } from 'lucide-react';
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
import type { ServerEngine } from '@/cloud/types';
import { usePrefs } from '@/stores/prefs';
import type { ServersContext } from '../ServersPage';
import { ListingCard, ListingRow } from './cards';
import { useListingPages } from './queries';
import { LANGUAGES, languageName, TAGS } from './vocab';

const SORTS = ['trending', 'players', 'new', 'name'] as const;
const ENGINES: ServerEngine[] = ['horizon', 'rustic'];

/** Browse: the whole directory, as a grid or a dense list, with filters and paging. */
export function BrowseTab({ ctx }: { ctx: ServersContext }) {
  const { t, i18n } = useTranslation('servers');
  const layout = usePrefs((s) => s.serversLayout);
  const setLayout = usePrefs((s) => s.setServersLayout);
  const { search } = ctx;
  const sort = search.sort ?? 'trending';
  const results = useListingPages({
    q: ctx.query || undefined,
    tag: search.tag,
    language: search.language,
    engine: search.engine,
    verified: search.verified,
    sort,
  });
  const items = results.data?.pages.flatMap((p) => p.items) ?? [];
  const total = results.data?.pages[0]?.total ?? 0;
  const activeFilters =
    (search.tag ? 1 : 0) +
    (search.language ? 1 : 0) +
    (search.engine ? 1 : 0) +
    (search.verified ? 1 : 0);
  const resetFilters = () =>
    ctx.setSearch(
      { tag: undefined, language: undefined, engine: undefined, verified: undefined },
      true,
    );
  const languages = [...new Set([...LANGUAGES, ...(search.language ? [search.language] : [])])];

  return (
    <>
      <CatalogToolbar
        start={results.data ? <ResultCount count={total} /> : <Skeleton className="h-4 w-24" />}
        end={
          <>
            <SortSelect
              value={sort}
              onChange={(s) => ctx.setSearch({ sort: s === 'trending' ? undefined : s }, true)}
              options={SORTS.map((s) => ({ value: s, label: t(`directory.sorts.${s}`) }))}
            />
            <FilterPopover active={activeFilters} onReset={resetFilters} testId="servers-filters">
              <FilterSection title={t('directory.filters.tags')}>
                <div className="flex flex-wrap gap-1.5" role="group">
                  {TAGS.map((tag) => (
                    <FilterChip
                      key={tag}
                      on={search.tag === tag}
                      onToggle={() =>
                        ctx.setSearch({ tag: search.tag === tag ? undefined : tag }, true)
                      }
                      testId={`servers-tag-${tag}`}
                    >
                      {t(`directory.tags.${tag}`)}
                    </FilterChip>
                  ))}
                </div>
              </FilterSection>
              <FilterSection title={t('directory.filters.language')}>
                <div className="flex flex-wrap gap-1.5" role="group">
                  {languages.map((l) => (
                    <FilterChip
                      key={l}
                      on={search.language === l}
                      onToggle={() =>
                        ctx.setSearch({ language: search.language === l ? undefined : l }, true)
                      }
                      testId={`servers-lang-${l}`}
                    >
                      {languageName(l, i18n.language)}
                    </FilterChip>
                  ))}
                </div>
              </FilterSection>
              <FilterSection title={t('directory.filters.engine')}>
                <div className="flex flex-wrap gap-1.5" role="group">
                  {ENGINES.map((e) => (
                    <FilterChip
                      key={e}
                      on={search.engine === e}
                      onToggle={() =>
                        ctx.setSearch({ engine: search.engine === e ? undefined : e }, true)
                      }
                      testId={`servers-engine-${e}`}
                    >
                      {t(`list.engine.${e}`)}
                    </FilterChip>
                  ))}
                </div>
              </FilterSection>
              <FilterSection
                title={t('directory.filters.rustic')}
                hint={t('directory.verifiedHint')}
              >
                <FilterSwitch
                  checked={!!search.verified}
                  onCheckedChange={(v) => ctx.setSearch({ verified: v || undefined }, true)}
                  label={t('directory.filters.verifiedOnly')}
                  icon={<BadgeCheck className="text-accent-text" />}
                  testId="servers-verified-only"
                />
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
      ) : results.isLoading ? (
        layout === 'grid' ? (
          <GridSkeletonCards className="grid-cols-[repeat(auto-fill,minmax(260px,1fr))]" />
        ) : (
          <ListSkeletonRows />
        )
      ) : items.length === 0 ? (
        <CatalogEmpty
          icon={<SearchX />}
          testId="servers-browse-empty"
          title={t('directory.emptySearch.title')}
          description={t('directory.emptySearch.hint')}
          action={
            activeFilters > 0 ? (
              <Button size="sm" onClick={resetFilters}>
                {t('directory.filters.reset')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        <>
          {layout === 'grid' ? (
            <ul
              className={cn(
                'grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4 transition-opacity',
                results.isFetching && !results.isFetchingNextPage && 'opacity-70',
              )}
              data-testid="servers-browse-grid"
            >
              {items.map((l) => (
                <li key={l.id}>
                  <ListingCard listing={l} />
                </li>
              ))}
            </ul>
          ) : (
            <ul
              className={cn(
                'flex flex-col transition-opacity',
                results.isFetching && !results.isFetchingNextPage && 'opacity-70',
              )}
              data-testid="servers-browse-list"
            >
              {items.map((l) => (
                <li key={l.id}>
                  <ListingRow listing={l} />
                </li>
              ))}
            </ul>
          )}
          {results.hasNextPage && (
            <LoadMore
              shown={items.length}
              total={total}
              loading={results.isFetchingNextPage}
              onLoad={() => void results.fetchNextPage()}
              testId="servers-load-more"
            />
          )}
        </>
      )}
    </>
  );
}

/** Removable chips for the filters that are set. */
function ActiveFilters({ ctx }: { ctx: ServersContext }) {
  const { t, i18n } = useTranslation('servers');
  const { search } = ctx;
  const chips: { key: string; label: string; clear: () => void }[] = [];
  if (search.tag)
    chips.push({
      key: 'tag',
      label: t(`directory.tags.${search.tag as 'survival'}`, { defaultValue: search.tag }),
      clear: () => ctx.setSearch({ tag: undefined }, true),
    });
  if (search.language)
    chips.push({
      key: 'language',
      label: languageName(search.language, i18n.language),
      clear: () => ctx.setSearch({ language: undefined }, true),
    });
  if (search.engine)
    chips.push({
      key: 'engine',
      label: t(`list.engine.${search.engine}`),
      clear: () => ctx.setSearch({ engine: undefined }, true),
    });
  if (search.verified)
    chips.push({
      key: 'verified',
      label: t('directory.filters.verifiedOnly'),
      clear: () => ctx.setSearch({ verified: undefined }, true),
    });
  if (chips.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5" data-testid="servers-active-filters">
      {chips.map((c) => (
        <button
          key={c.key}
          type="button"
          onClick={c.clear}
          aria-label={t('directory.filters.remove', { filter: c.label })}
          className="rx-focus-ring border-accent/40 bg-accent-soft text-accent-text hover:bg-accent/20 inline-flex h-7 items-center gap-1 rounded-full border px-2.5 text-xs font-medium"
        >
          {c.label}
          <X className="size-3" aria-hidden />
        </button>
      ))}
    </div>
  );
}
