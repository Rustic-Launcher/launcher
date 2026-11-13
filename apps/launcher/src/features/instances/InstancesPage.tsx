import { Button, cn } from '@rustic/ui';
import { useNavigate } from '@tanstack/react-router';
import {
  Cpu,
  FolderInput,
  Gamepad2,
  History,
  Layers3,
  PackageOpen,
  Plus,
  SearchX,
  UserRound,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CardRow,
  CatalogEmpty,
  CatalogError,
  CatalogSearch,
  CatalogToolbar,
  FilterChip,
  FilterPopover,
  FilterSection,
  FilterSwitch,
  GridSkeletonCards,
  LayoutToggle,
  ListSkeletonRows,
  ResultCount,
  ScopeMenu,
  SortSelect,
  type ScopeOption,
} from '@/components/catalog';
import { isRusticEdition } from '@/components/HorizonVersion';
import { PageTitle } from '@/components/Page';
import { formatDuration, shortVersion } from '@/lib/format';
import { useActiveInstance, useHorizonVersions, useInstances } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { CreateTile, ProfileCard, ProfileRow, RecentCard } from './cards';
import {
  activeFilterCount,
  buildsInUse,
  filterProfiles,
  PROFILE_SORTS,
  recentlyPlayed,
  sortInstances,
  totalPlaytime,
  type ProfileSource,
} from './logic';
import { NewInstanceSheet } from './NewInstanceSheet';
import { useNativeOverview } from './queries';
import { instancesRoute, type InstancesSearch } from './routes';

const GRID = 'grid-cols-[repeat(auto-fill,minmax(248px,1fr))]';

/**
 * Profiles (ADR 0159), in the Explore / Servers design: a search with the source fused on its
 * left, Create and Import, a "Jump back in" row, then every profile as a visual card (or a dense
 * row) with its build, content, native mods and a Play button on hover. The URL holds the search,
 * sort and filters; the grid/list choice is a preference.
 */
export function InstancesPage() {
  const { t, i18n } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const navigate = useNavigate({ from: '/instances' });
  const search = instancesRoute.useSearch();
  const instances = useInstances();
  const { data: versions } = useHorizonVersions();
  const native = useNativeOverview();
  const activeId = useActiveInstance().instance?.id;
  const layout = usePrefs((s) => s.profilesLayout);
  const setLayout = usePrefs((s) => s.setProfilesLayout);
  const [text, setText] = useState(search.q ?? '');
  const sort = search.sort ?? 'recent';

  const setSearch = (patch: Partial<InstancesSearch>) =>
    void navigate({ to: '/instances', search: (prev) => ({ ...prev, ...patch }), replace: true });

  // The URL follows the text a moment after typing (filtering itself is instant).
  useEffect(() => {
    const q = text.trim() || undefined;
    if (q === search.q) return;
    const id = setTimeout(
      () => void navigate({ to: '/instances', search: (prev) => ({ ...prev, q }), replace: true }),
      250,
    );
    return () => clearTimeout(id);
  }, [text, search.q, navigate]);

  const sheet = search.new === 'import' ? 'import' : search.new ? 'create' : null;
  const openSheet = (tab: 'create' | 'import' | null) =>
    void navigate({
      to: '/instances',
      search: (prev) => ({ ...prev, new: tab === 'import' ? 'import' : tab ? '1' : undefined }),
      replace: true,
    });

  const all = useMemo(() => instances.data ?? [], [instances.data]);
  const nativeIds = useMemo(() => new Set(native.byId.keys()), [native.byId]);
  const filters = useMemo(
    () => ({
      q: text,
      source: search.source,
      version: search.version,
      edition: search.edition,
      native: search.native,
    }),
    [text, search.source, search.version, search.edition, search.native],
  );
  const list = useMemo(
    () =>
      sortInstances(
        filterProfiles(all, filters, {
          isRustic: (v) => isRusticEdition(v, versions),
          native: nativeIds,
        }),
        sort,
      ),
    [all, filters, versions, nativeIds, sort],
  );
  const activeFilters = activeFilterCount(filters);
  const narrowed = activeFilters > 0 || !!text.trim() || !!search.source;
  const recent = narrowed ? [] : recentlyPlayed(all);
  const resetFilters = () =>
    setSearch({ version: undefined, edition: undefined, native: undefined });
  const resetAll = () => {
    setText('');
    setSearch({
      q: undefined,
      source: undefined,
      version: undefined,
      edition: undefined,
      native: undefined,
    });
  };

  // Older links (`?selected=<id>`) land on that profile.
  useEffect(() => {
    if (!search.selected || !instances.data) return;
    document
      .querySelector(`[data-testid="instance-card-${CSS.escape(search.selected)}"]`)
      ?.scrollIntoView({ block: 'center' });
  }, [search.selected, instances.data]);

  const sources: ScopeOption<ProfileSource | 'all'>[] = [
    { value: 'all', label: t('profiles.source.all'), icon: <Layers3 /> },
    { value: 'custom', label: t('profiles.source.custom'), icon: <UserRound /> },
    { value: 'modpack', label: t('profiles.source.modpack'), icon: <PackageOpen /> },
  ];

  return (
    <div className="rx-page-enter flex min-h-full flex-col gap-4 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <PageTitle title={t('heading')} />
        {all.length > 0 && (
          <p
            className="bg-surface-2 border-border text-fg-muted rounded-full border px-3 py-1.5 text-xs tabular-nums"
            data-testid="profiles-summary"
          >
            {t('profiles.summary', {
              count: all.length,
              playtime: formatDuration(totalPlaytime(all), i18n.language),
            })}
          </p>
        )}
      </div>
      <CatalogToolbar
        start={
          <CatalogSearch
            value={text}
            onChange={setText}
            testId="profiles-search"
            label={t('profiles.searchLabel')}
            clearLabel={tc('actions.clear')}
            placeholder={t('profiles.searchPlaceholder')}
            scope={
              <ScopeMenu
                value={search.source ?? 'all'}
                options={sources}
                label={t('profiles.source.label')}
                onChange={(v) => setSearch({ source: v === 'all' ? undefined : v })}
                testId="profiles-source"
                itemTestIdPrefix="profiles-source"
              />
            }
          />
        }
        end={
          <>
            <Button
              variant="secondary"
              className="h-11 rounded-xl"
              leftIcon={<FolderInput />}
              onClick={() => openSheet('import')}
              data-testid="import-profile"
            >
              {t('profiles.import')}
            </Button>
            <button
              type="button"
              onClick={() => openSheet('create')}
              data-testid="new-instance"
              className="rx-focus-ring border-accent/50 bg-accent-soft text-accent-text hover:bg-accent/20 flex h-11 items-center gap-2.5 rounded-xl border px-4 text-left text-sm font-semibold transition-colors"
            >
              <Plus className="size-4" aria-hidden />
              <span className="flex flex-col leading-tight">
                {t('profiles.create')}
                <span className="text-fg-muted text-2xs font-medium max-[1100px]:hidden">
                  {t('profiles.createHint')}
                </span>
              </span>
            </button>
          </>
        }
      />

      {recent.length > 0 && (
        <CardRow
          title={t('profiles.recent')}
          subtitle={t('profiles.recentHint')}
          icon={<History />}
          testId="profiles-recent"
        >
          {recent.map((i) => (
            <RecentCard key={i.id} instance={i} active={i.id === activeId} />
          ))}
        </CardRow>
      )}

      {instances.data && all.length === 0 ? (
        <CatalogEmpty
          icon={<Gamepad2 />}
          testId="profiles-empty"
          title={t('profiles.empty.title')}
          description={t('profiles.empty.description')}
          action={
            <div className="flex flex-wrap justify-center gap-2">
              <Button variant="primary" leftIcon={<Plus />} onClick={() => openSheet('create')}>
                {t('profiles.create')}
              </Button>
              <Button leftIcon={<FolderInput />} onClick={() => openSheet('import')}>
                {t('profiles.import')}
              </Button>
            </div>
          }
        />
      ) : (
        <section aria-labelledby="profiles-all" className="flex flex-col gap-4">
          <CatalogToolbar
            start={
              <div className="flex items-baseline gap-3">
                <h2 id="profiles-all" className="text-base font-semibold">
                  {t('profiles.all')}
                </h2>
                {instances.data && <ResultCount count={list.length} />}
              </div>
            }
            end={
              <>
                <SortSelect
                  value={sort}
                  onChange={(s) => setSearch({ sort: s === 'recent' ? undefined : s })}
                  options={PROFILE_SORTS.map((s) => ({
                    value: s,
                    label: t(`profiles.sorts.${s}`),
                  }))}
                />
                <FilterPopover
                  active={activeFilters}
                  onReset={resetFilters}
                  testId="profiles-filters"
                >
                  <FilterSection title={t('profiles.filters.version')}>
                    <div className="flex flex-wrap gap-1.5" role="group">
                      {buildsInUse(all).map((v) => (
                        <FilterChip
                          key={v}
                          on={search.version === v}
                          onToggle={() =>
                            setSearch({ version: search.version === v ? undefined : v })
                          }
                          testId={`profiles-version-${v}`}
                        >
                          {shortVersion(v)}
                        </FilterChip>
                      ))}
                    </div>
                  </FilterSection>
                  <FilterSection title={t('profiles.filters.edition')}>
                    <div className="flex flex-wrap gap-1.5" role="group">
                      {(['rustic', 'horizon'] as const).map((e) => (
                        <FilterChip
                          key={e}
                          on={search.edition === e}
                          onToggle={() =>
                            setSearch({ edition: search.edition === e ? undefined : e })
                          }
                          testId={`profiles-edition-${e}`}
                        >
                          {t(`profiles.filters.editions.${e}`)}
                        </FilterChip>
                      ))}
                    </div>
                  </FilterSection>
                  <FilterSection
                    title={t('profiles.filters.native')}
                    hint={t('profiles.filters.nativeHint')}
                  >
                    <FilterSwitch
                      checked={!!search.native}
                      onCheckedChange={(v) => setSearch({ native: v || undefined })}
                      label={t('profiles.filters.nativeOnly')}
                      icon={<Cpu className="text-info-text" />}
                      testId="profiles-native-only"
                    />
                  </FilterSection>
                </FilterPopover>
                <LayoutToggle value={layout} onChange={setLayout} />
              </>
            }
          />
          <ActiveFilters search={search} onChange={setSearch} />

          {instances.isError ? (
            <CatalogError
              onRetry={() => void instances.refetch()}
              message={(instances.error as Error | null)?.message}
            />
          ) : instances.isLoading ? (
            layout === 'grid' ? (
              <GridSkeletonCards count={4} className={GRID} />
            ) : (
              <ListSkeletonRows rows={4} />
            )
          ) : list.length === 0 ? (
            <CatalogEmpty
              icon={<SearchX />}
              testId="profiles-no-match"
              title={t('profiles.noMatch.title')}
              description={t('profiles.noMatch.description')}
              action={
                <Button size="sm" onClick={resetAll}>
                  {t('profiles.filters.reset')}
                </Button>
              }
            />
          ) : layout === 'grid' ? (
            <ul
              className={cn('grid gap-4', GRID)}
              aria-label={t('profiles.listLabel')}
              data-testid="profiles-grid"
            >
              {list.map((i) => (
                <li key={i.id}>
                  <ProfileCard
                    instance={i}
                    active={i.id === activeId}
                    overview={native.byId.get(i.id)}
                  />
                </li>
              ))}
              {!narrowed && (
                <li>
                  <CreateTile
                    onCreate={() => openSheet('create')}
                    onImport={() => openSheet('import')}
                  />
                </li>
              )}
            </ul>
          ) : (
            <ul
              className="flex flex-col"
              aria-label={t('profiles.listLabel')}
              data-testid="profiles-list"
            >
              {list.map((i) => (
                <li key={i.id}>
                  <ProfileRow
                    instance={i}
                    active={i.id === activeId}
                    overview={native.byId.get(i.id)}
                  />
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
      <NewInstanceSheet
        open={sheet !== null}
        initialTab={sheet ?? 'create'}
        onOpenChange={(open) => openSheet(open ? (sheet ?? 'create') : null)}
      />
    </div>
  );
}

/** Removable chips for the filters that are set. */
function ActiveFilters({
  search,
  onChange,
}: {
  search: InstancesSearch;
  onChange: (patch: Partial<InstancesSearch>) => void;
}) {
  const { t } = useTranslation('instances');
  const chips: { key: string; label: string; clear: () => void }[] = [];
  if (search.version)
    chips.push({
      key: 'version',
      label: `${t('profiles.filters.version')} ${shortVersion(search.version)}`,
      clear: () => onChange({ version: undefined }),
    });
  if (search.edition)
    chips.push({
      key: 'edition',
      label: t(`profiles.filters.editions.${search.edition}`),
      clear: () => onChange({ edition: undefined }),
    });
  if (search.native)
    chips.push({
      key: 'native',
      label: t('profiles.filters.nativeOnly'),
      clear: () => onChange({ native: undefined }),
    });
  if (chips.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5" data-testid="profiles-active-filters">
      {chips.map((c) => (
        <button
          key={c.key}
          type="button"
          onClick={c.clear}
          aria-label={t('profiles.filters.remove', { filter: c.label })}
          className="rx-focus-ring border-accent/40 bg-accent-soft text-accent-text hover:bg-accent/20 inline-flex h-7 items-center gap-1 rounded-full border px-2.5 text-xs font-medium"
        >
          {c.label}
          <X className="size-3" aria-hidden />
        </button>
      ))}
    </div>
  );
}
