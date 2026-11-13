import { Button } from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  Boxes,
  Compass,
  FileCode2,
  Image,
  Info,
  LayoutList,
  Library,
  ListOrdered,
  Plug,
  Puzzle,
  Sparkles,
} from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CatalogSearch,
  CatalogTabList,
  CatalogTabPanel,
  CatalogTabs,
  CatalogToolbar,
  ScopeMenu,
  type ScopeOption,
} from '@/components/catalog';
import { PageTitle } from '@/components/Page';
import type { ContentKind } from '@/ipc/types/content';
import { useActiveInstance, useInstances } from '@/queries/core';
import { BrowseTab } from './BrowseTab';
import { DiscoverTab } from './DiscoverTab';
import { MineTab } from './MineTab';
import { NoRusticServer, TargetPicker } from './parts';
import { contentRoute, type ContentSearch, type ContentTab } from './routes';
import {
  eligibleServers,
  formatTarget,
  parseTarget,
  resolveTarget,
  type TargetRef,
} from './target';
import { KINDS, useServers } from './useContent';

export const KIND_ICONS: Record<ContentKind, ReactNode> = {
  mod: <Puzzle />,
  modpack: <Boxes />,
  resourcepack: <Image />,
  shader: <Sparkles />,
  datapack: <FileCode2 />,
  plugin: <Plug />,
};

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

/** What every tab knows about the page: kind, target and the text being searched. */
export interface ExploreContext {
  kind: ContentKind;
  target: TargetRef | null;
  targetName: string;
  /** The target's Horizon version id, when known. */
  targetVersion: string | null;
  query: string;
  search: ContentSearch;
  /** Moves to Browse with these filters (a row's "See all"). */
  browse: (filters: Partial<ContentSearch>) => void;
  setSearch: (patch: Partial<ContentSearch>, replace?: boolean) => void;
}

/**
 * Explore (ADR 0156): Discover (featured carousel and themed rows), Browse (dense list or grid
 * with filters and paging) and Mine (installed content, the Rustic library and own creations),
 * under one kind selector, search field and install target.
 */
export function ContentPage() {
  const { t } = useTranslation('content');
  const { t: tc } = useTranslation();
  const search = contentRoute.useSearch();
  const navigate = useNavigate({ from: '/content' });
  const kind: ContentKind = search.kind ?? 'mod';
  const [text, setText] = useState(search.q ?? '');
  const query = useDebounced(text, 250);
  const tab: ContentTab = search.tab ?? (search.q ? 'browse' : 'discover');

  const instances = useInstances();
  const servers = useServers();
  const { instance: active } = useActiveInstance();
  const target = useMemo(
    () =>
      resolveTarget(
        kind,
        parseTarget(search.target),
        instances.data ?? [],
        servers.data ?? [],
        active?.id,
      ),
    [kind, search.target, instances.data, servers.data, active?.id],
  );
  const { targetName, targetVersion } = useMemo(() => {
    if (!target) return { targetName: '', targetVersion: null };
    const found =
      target.kind === 'instance'
        ? instances.data?.find((i) => i.id === target.id)
        : servers.data?.find((s) => s.id === target.id);
    return { targetName: found?.name ?? '', targetVersion: found?.horizon_version ?? null };
  }, [target, instances.data, servers.data]);

  const setSearch = (patch: Partial<ContentSearch>, replace = false) =>
    void navigate({ to: '/content', search: (prev) => ({ ...prev, ...patch }), replace });

  // Keep the URL in sync with the debounced text (shareable, survives back/forward); typing on
  // Discover moves to Browse, where results are listed.
  useEffect(() => {
    if ((search.q ?? '') === query) return;
    const toBrowse = !!query && tab === 'discover';
    void navigate({
      to: '/content',
      search: (prev) => ({
        ...prev,
        q: query || undefined,
        tab: toBrowse ? 'browse' : prev.tab,
      }),
      replace: !toBrowse,
    });
  }, [query, search.q, tab, navigate]);

  const ctx: ExploreContext = {
    kind,
    target,
    targetName,
    targetVersion,
    query,
    search,
    setSearch,
    browse: (filters) =>
      void navigate({
        to: '/content',
        search: (prev) => ({
          kind: prev.kind,
          target: prev.target,
          q: prev.q,
          tab: 'browse',
          ...filters,
        }),
      }),
  };

  const targetsLoading = instances.isLoading || servers.isLoading;
  const noPluginServer =
    kind === 'plugin' &&
    !servers.isLoading &&
    eligibleServers('plugin', servers.data ?? []).length === 0;

  const kindOptions: ScopeOption<ContentKind>[] = KINDS.map((k) => ({
    value: k,
    label: t(`kinds.${k}`),
    icon: KIND_ICONS[k],
    group: k === 'plugin' ? t('kindsServer') : undefined,
  }));

  return (
    <div className="rx-page-enter flex min-h-full flex-col gap-4 p-4">
      <CatalogTabs value={tab} onValueChange={(v) => setSearch({ tab: v })}>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <PageTitle title={t('heading')} />
          <CatalogTabList
            label={t('tabs.label')}
            testIdPrefix="explore-tab"
            tabs={[
              { value: 'discover', label: t('tabs.discover'), icon: <Compass /> },
              { value: 'browse', label: t('tabs.browse'), icon: <LayoutList /> },
              { value: 'mine', label: t('tabs.mine'), icon: <Library /> },
            ]}
          />
        </div>
        <CatalogToolbar
          start={
            <CatalogSearch
              value={text}
              onChange={setText}
              testId="content-search"
              label={t('searchLabel')}
              clearLabel={tc('actions.clear')}
              placeholder={
                tab === 'mine'
                  ? t('mine.searchPlaceholder')
                  : t('searchSources', { kind: t(`kinds.${kind}`).toLowerCase() })
              }
              scope={
                <ScopeMenu
                  value={kind}
                  options={kindOptions}
                  label={t('kindsLabel')}
                  testId="content-kind"
                  itemTestIdPrefix="content-tab"
                  onChange={(k) => setSearch({ kind: k, category: undefined })}
                />
              }
            />
          }
          end={
            kind !== 'modpack' && !noPluginServer ? (
              <TargetPicker
                kind={kind}
                value={target}
                onChange={(tr: TargetRef) => setSearch({ target: formatTarget(tr) }, true)}
                instances={instances.data ?? []}
                servers={servers.data ?? []}
                loading={targetsLoading}
              />
            ) : undefined
          }
        />
        {noPluginServer && tab !== 'mine' ? (
          <NoRusticServer />
        ) : (
          <>
            <CatalogTabPanel value="discover">
              <DiscoverTab ctx={ctx} ready={!targetsLoading} />
            </CatalogTabPanel>
            <CatalogTabPanel value="browse">
              <KindBanner kind={kind} target={target} />
              <BrowseTab ctx={ctx} ready={!targetsLoading} />
            </CatalogTabPanel>
          </>
        )}
        <CatalogTabPanel value="mine">
          <MineTab ctx={ctx} />
        </CatalogTabPanel>
      </CatalogTabs>
    </div>
  );
}

/** At most 6 builds (newest first as the backend lists them), always including the target's. */
export function shownVersions<T extends { id: string }>(
  list: T[],
  target: string | null,
  max = 6,
): T[] {
  const head = list.slice(0, max);
  const t = target ? list.find((v) => v.id === target) : undefined;
  return t && !head.includes(t) ? [...head.slice(0, max - 1), t] : head;
}

function KindBanner({ kind, target }: { kind: ContentKind; target: TargetRef | null }) {
  const { t } = useTranslation('content');
  let action: ReactNode = null;
  if (kind === 'resourcepack' && target?.kind === 'instance') {
    action = (
      <Button asChild size="sm" variant="soft" leftIcon={<ListOrdered />}>
        <Link
          to="/instances/$instanceId"
          params={{ instanceId: target.id }}
          search={{ tab: 'resourcepacks' }}
        >
          {t('resourcepacks.manageOrder')}
        </Link>
      </Button>
    );
  }
  return (
    <div
      className="border-border bg-surface/60 text-fg-muted flex items-center gap-3 rounded-xl border px-4 py-2.5 text-sm"
      role="note"
    >
      <Info className="text-info-text size-4 shrink-0" aria-hidden />
      <p className="flex-1">{t(`banners.${kind}`)}</p>
      {action}
    </div>
  );
}
