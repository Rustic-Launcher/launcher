import {
  Avatar,
  Badge,
  Button,
  cn,
  EmptyState,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownTrigger,
  Select,
  Skeleton,
  Switch,
} from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  ArrowDownWideNarrow,
  Clock,
  Eye,
  Boxes,
  Check,
  ChevronDown,
  Search,
  Download,
  FileCode2,
  Image,
  Info,
  ListOrdered,
  Plug,
  Puzzle,
  SearchX,
  ShieldCheck,
  Sparkles,
  Star,
} from 'lucide-react';
import { useEffect, useId, useMemo, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { CompatibilityBadge } from '@/components/CompatibilityBadge';
import { QueryError } from '@/components/RouteError';
import type {
  ContentKind,
  ContentSearchQuery,
  ContentSource,
  ContentSummary,
} from '@/ipc/types/content';
import { formatCompact, formatRelative, shortVersion } from '@/lib/format';
import { useActiveInstance, useHorizonVersions, useInstances } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import {
  ContentIcon,
  InstallButton,
  NoRusticServer,
  ShaderFormatBadge,
  SourceBadge,
  TargetPicker,
} from './parts';
import { contentRoute } from './routes';
import {
  eligibleServers,
  formatTarget,
  parseTarget,
  resolveTarget,
  toQueryTarget,
  type TargetRef,
} from './target';
import { ALL_SOURCES, KINDS, useCategories, useContentSearch, useServers } from './useContent';
import { PageTitle } from '@/components/Page';

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

export function ContentPage() {
  const { t } = useTranslation('content');
  const search = contentRoute.useSearch();
  const navigate = useNavigate();
  const kind: ContentKind = search.kind ?? 'mod';
  const [text, setText] = useState(search.q ?? '');
  const query = useDebounced(text, 250);
  const [sources, setSources] = useState<ContentSource[]>(ALL_SOURCES);
  const [sort, setSort] = useState<ContentSearchQuery['sort']>('relevance');
  const [category, setCategory] = useState<string>('all');
  const compatibleOnly = usePrefs((s) => s.compatibleOnly);
  const setCompatibleOnly = usePrefs((s) => s.setCompatibleOnly);
  const switchId = useId();

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
  const targetName = useMemo(() => {
    if (!target) return '';
    if (target.kind === 'instance')
      return instances.data?.find((i) => i.id === target.id)?.name ?? '';
    return servers.data?.find((s) => s.id === target.id)?.name ?? '';
  }, [target, instances.data, servers.data]);

  // Keep the URL in sync with the debounced query (shareable, survives back/forward).
  useEffect(() => {
    if ((search.q ?? '') === query) return;
    void navigate({
      to: '/content',
      search: (prev) => ({ ...prev, q: query || undefined }),
      replace: true,
    });
  }, [query, search.q, navigate]);

  useEffect(() => setCategory('all'), [kind]);

  const targetVersion = useMemo(() => {
    if (!target) return null;
    if (target.kind === 'instance')
      return instances.data?.find((i) => i.id === target.id)?.horizon_version ?? null;
    return servers.data?.find((s) => s.id === target.id)?.horizon_version ?? null;
  }, [target, instances.data, servers.data]);

  const targetsLoading = instances.isLoading || servers.isLoading;
  const noPluginServer =
    kind === 'plugin' &&
    !servers.isLoading &&
    eligibleServers('plugin', servers.data ?? []).length === 0;

  const results = useContentSearch(
    {
      query,
      kind,
      sources,
      target: toQueryTarget(target, kind),
      compatible_only: compatibleOnly,
      category: category === 'all' ? undefined : category,
      sort,
    },
    !targetsLoading && !noPluginServer,
  );
  const categories = useCategories(kind);
  const hits = results.data?.pages.flatMap((p) => p.hits) ?? [];
  const total = results.data?.pages[0]?.total ?? 0;

  const setKind = (k: string) =>
    void navigate({ to: '/content', search: (prev) => ({ ...prev, kind: k as ContentKind }) });
  const setTarget = (tr: TargetRef) =>
    void navigate({
      to: '/content',
      search: (prev) => ({ ...prev, target: formatTarget(tr) }),
      replace: true,
    });
  const toggleSource = (s: ContentSource) =>
    setSources((cur) =>
      cur.includes(s) ? (cur.length > 1 ? cur.filter((x) => x !== s) : cur) : [...cur, s],
    );

  const compatLabel =
    kind === 'modpack'
      ? t('filters.worksNow')
      : target?.kind === 'server'
        ? t('filters.compatibleServer')
        : t('filters.compatibleInstance');

  return (
    <div className="rx-page-enter grid min-h-full grid-cols-[minmax(0,1fr)_minmax(280px,320px)] gap-4 p-4">
      <div className="flex min-w-0 flex-col gap-4">
        <PageTitle title={t('heading')} />
        <div className="flex items-center gap-2">
          <div className="bg-surface-2 border-border focus-within:border-accent/60 flex h-11 min-w-0 flex-1 items-center rounded-xl border">
            <Dropdown>
              <DropdownTrigger asChild>
                <button
                  type="button"
                  data-testid="content-kind"
                  aria-label={t('kindsLabel')}
                  className="rx-focus-ring flex h-full shrink-0 items-center gap-2 rounded-l-xl px-3.5 text-sm font-semibold [&_svg]:size-4"
                >
                  {KIND_ICONS[kind]}
                  {t(`kinds.${kind}`)}
                  <ChevronDown className="text-fg-subtle size-3.5!" />
                </button>
              </DropdownTrigger>
              <DropdownContent align="start" className="w-56">
                <DropdownLabel>{t('kindsLabel')}</DropdownLabel>
                {KINDS.map((k) => (
                  <DropdownItem
                    key={k}
                    icon={KIND_ICONS[k]}
                    onSelect={() => setKind(k)}
                    data-testid={`content-tab-${k}`}
                  >
                    <span className="flex items-center justify-between">
                      {t(`kinds.${k}`)}
                      {k === kind && <Check className="text-accent-text size-4" />}
                    </span>
                  </DropdownItem>
                ))}
              </DropdownContent>
            </Dropdown>
            <span className="bg-border-default h-5 w-px shrink-0" />
            <Search className="text-fg-subtle ml-3 size-4 shrink-0" aria-hidden />
            <input
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape' && text) {
                  e.stopPropagation();
                  setText('');
                }
              }}
              placeholder={t('searchSources', {
                kind: t(`kinds.${kind}`).toLowerCase(),
                sources: sources.map((s) => t(`sources.${s}`)).join(', '),
              })}
              aria-label={t('searchLabel')}
              data-testid="content-search"
              type="search"
              className="placeholder:text-fg-subtle h-full min-w-0 flex-1 bg-transparent px-2.5 text-sm outline-none"
            />
          </div>
          <Select
            aria-label={t('sort.label')}
            value={sort}
            onValueChange={(v) => setSort(v as ContentSearchQuery['sort'])}
            options={(['relevance', 'downloads', 'updated', 'newest'] as const).map((s) => ({
              value: s,
              label: t(`sort.${s}`),
              icon: <ArrowDownWideNarrow />,
            }))}
            className="bg-surface-2 h-11 w-48 rounded-xl"
          />
        </div>
        {kind !== 'modpack' && (
          <div className="flex items-center gap-3">
            <TargetPicker
              kind={kind}
              value={target}
              onChange={setTarget}
              instances={instances.data ?? []}
              servers={servers.data ?? []}
              loading={targetsLoading}
            />
          </div>
        )}

        <KindBanner kind={kind} target={target} />

        {noPluginServer ? (
          <NoRusticServer />
        ) : results.isError ? (
          <QueryError
            onRetry={() => void results.refetch()}
            message={(results.error as Error).message}
          />
        ) : results.isLoading || targetsLoading ? (
          <ResultsSkeleton />
        ) : hits.length === 0 ? (
          <EmptyState
            icon={<SearchX />}
            title={t('empty.title')}
            description={compatibleOnly ? t('empty.compatibleHint') : t('empty.hint')}
            action={
              compatibleOnly ? (
                <Button size="sm" onClick={() => setCompatibleOnly(false)}>
                  {t('empty.showAll')}
                </Button>
              ) : undefined
            }
          />
        ) : (
          <>
            <p className="text-fg-subtle text-xs" aria-live="polite">
              {t('results', { count: total })}
            </p>
            <ul
              className={cn(
                'flex flex-col transition-opacity',
                results.isFetching && !results.isFetchingNextPage && 'opacity-70',
              )}
            >
              {hits.map((h) => (
                <li key={h.id}>
                  <ContentRow item={h} target={target} targetName={targetName} />
                </li>
              ))}
            </ul>
            {results.hasNextPage && (
              <div className="flex justify-center">
                <Button
                  onClick={() => void results.fetchNextPage()}
                  loading={results.isFetchingNextPage}
                >
                  {t('loadMore', { shown: hits.length, total })}
                </Button>
              </div>
            )}
          </>
        )}
      </div>
      <ExploreFilters
        kind={kind}
        sources={sources}
        toggleSource={toggleSource}
        categories={categories.data?.map((c) => c.id) ?? []}
        categoriesLoading={categories.isLoading}
        category={category}
        setCategory={setCategory}
        compatLabel={compatLabel}
        compatibleOnly={compatibleOnly}
        setCompatibleOnly={setCompatibleOnly}
        switchId={switchId}
        targetVersion={targetVersion}
      />
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

/** Right column of Explore: sources as big tabs, categories, compatibility, Horizon versions. */
function ExploreFilters({
  kind,
  sources,
  toggleSource,
  categories,
  categoriesLoading,
  category,
  setCategory,
  compatLabel,
  compatibleOnly,
  setCompatibleOnly,
  switchId,
  targetVersion,
}: {
  kind: ContentKind;
  sources: ContentSource[];
  toggleSource: (s: ContentSource) => void;
  categories: string[];
  categoriesLoading: boolean;
  category: string;
  setCategory: (c: string) => void;
  compatLabel: string;
  compatibleOnly: boolean;
  setCompatibleOnly: (v: boolean) => void;
  switchId: string;
  targetVersion: string | null;
}) {
  const { t } = useTranslation('content');
  const [all, setAll] = useState(false);
  const { data: versions } = useHorizonVersions();
  const shown = all ? categories : categories.slice(0, 6);
  return (
    <aside className="flex min-w-0 flex-col gap-5" aria-label={t('filtersLabel')}>
      <div
        role="group"
        aria-label={t('sources.label')}
        className="bg-surface-2 border-border grid grid-cols-3 gap-1 rounded-xl border p-1"
      >
        {ALL_SOURCES.map((s) => {
          const on = sources.includes(s);
          return (
            <button
              key={s}
              type="button"
              aria-pressed={on}
              onClick={() => toggleSource(s)}
              data-testid={`source-${s}`}
              className={cn(
                'rx-focus-ring h-9 rounded-lg text-sm font-semibold transition-colors',
                on ? 'bg-surface-3 text-fg' : 'text-fg-subtle hover:text-fg',
              )}
            >
              {t(`sources.${s}`)}
            </button>
          );
        })}
      </div>

      <section className="flex flex-col gap-2.5" aria-labelledby="explore-categories">
        <h2 id="explore-categories" className="text-sm font-semibold">
          {t('filters.categories')}
        </h2>
        {categoriesLoading ? (
          <div className="grid grid-cols-2 gap-2">
            {Array.from({ length: 6 }, (_, i) => (
              <Skeleton key={i} className="h-8 rounded-lg" />
            ))}
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2">
            {shown.map((c) => (
              <button
                key={c}
                type="button"
                aria-pressed={category === c}
                onClick={() => setCategory(category === c ? 'all' : c)}
                className={cn(
                  'rx-focus-ring h-8 truncate rounded-lg border px-2.5 text-xs font-medium transition-colors',
                  category === c
                    ? 'border-accent/50 bg-accent-soft text-accent-text'
                    : 'bg-surface-2 border-border text-fg-muted hover:text-fg',
                )}
              >
                {t(`categories.${c as 'utility'}`, { defaultValue: c })}
              </button>
            ))}
          </div>
        )}
        {categories.length > 6 && (
          <button
            type="button"
            onClick={() => setAll((a) => !a)}
            className="text-fg-muted hover:text-fg flex items-center justify-center gap-1 text-xs font-medium"
          >
            <ChevronDown className={cn('size-3.5 transition-transform', all && 'rotate-180')} />
            {all ? t('filters.viewLess') : t('filters.viewMore', { count: categories.length - 6 })}
          </button>
        )}
      </section>

      <section className="flex flex-col gap-2.5" aria-labelledby="explore-compat">
        <h2 id="explore-compat" className="text-sm font-semibold">
          {t('filters.compatibility')}
        </h2>
        <label
          htmlFor={switchId}
          className="bg-surface-2 border-border flex cursor-pointer items-center justify-between gap-3 rounded-lg border px-3 py-2.5 text-sm"
        >
          <span className="flex items-center gap-2">
            <ShieldCheck className="text-success-text size-4" aria-hidden />
            {compatLabel}
          </span>
          <Switch
            id={switchId}
            checked={compatibleOnly}
            onCheckedChange={setCompatibleOnly}
            data-testid="compatible-only"
          />
        </label>
        <p className="text-fg-subtle text-xs">
          {kind === 'plugin' ? t('filters.pluginHint') : t('filters.fairPlayHint')}
        </p>
      </section>

      <section className="flex flex-col gap-2.5" aria-labelledby="explore-versions">
        <h2 id="explore-versions" className="text-sm font-semibold">
          {t('filters.versions')}
        </h2>
        <div className="flex flex-wrap gap-2">
          {shownVersions(versions ?? [], targetVersion).map((v) => (
            <span
              key={v.id}
              className={cn(
                'rounded-lg border px-2.5 py-1.5 text-xs font-medium',
                v.id === targetVersion
                  ? 'border-accent/50 bg-accent-soft text-accent-text'
                  : 'bg-surface-2 border-border text-fg-muted',
              )}
              title={v.id === targetVersion ? t('filters.targetVersion') : undefined}
            >
              {shortVersion(v.id)}
            </span>
          ))}
        </div>
        {targetVersion && <p className="text-fg-subtle text-xs">{t('filters.checkingAgainst')}</p>}
      </section>
    </aside>
  );
}

function KindBanner({ kind, target }: { kind: ContentKind; target: TargetRef | null }) {
  const { t } = useTranslation('content');
  let body: ReactNode = null;
  let action: ReactNode = null;
  if (kind === 'shader') body = t('banners.shader');
  if (kind === 'plugin') body = t('banners.plugin');
  if (kind === 'datapack') body = t('banners.datapack');
  if (kind === 'modpack') body = t('banners.modpack');
  if (kind === 'mod') body = t('banners.mod');
  if (kind === 'resourcepack') {
    body = t('banners.resourcepack');
    if (target?.kind === 'instance') {
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
  }
  if (!body) return null;
  return (
    <div
      className="border-border bg-surface/60 text-fg-muted flex items-center gap-3 rounded-xl border px-4 py-3 text-sm"
      role="note"
    >
      <Info className="text-info-text size-4 shrink-0" aria-hidden />
      <p className="flex-1">{body}</p>
      {action}
    </div>
  );
}

function ResultsSkeleton() {
  return (
    <div className="flex flex-col gap-2.5" aria-busy="true">
      {Array.from({ length: 6 }, (_, i) => (
        <div key={i} className="rx-surface flex items-center gap-4 rounded-xl p-4">
          <Skeleton className="size-14 rounded-xl" />
          <div className="flex flex-1 flex-col gap-2">
            <Skeleton className="h-4 w-48" />
            <Skeleton className="h-3 w-3/4" />
            <Skeleton className="h-4 w-40" />
          </div>
          <Skeleton className="h-8 w-24" />
        </div>
      ))}
    </div>
  );
}

function ContentRow({
  item,
  target,
  targetName,
}: {
  item: ContentSummary;
  target: TargetRef | null;
  targetName: string;
}) {
  const { t, i18n } = useTranslation('content');
  const search = { target: target ? formatTarget(target) : undefined };
  return (
    <div
      className="group border-border hover:bg-surface-2/50 flex items-center gap-4 border-b px-1 py-4 transition-colors"
      data-testid={`content-row-${item.id}`}
    >
      <ContentIcon id={item.id} name={item.name} size="lg" />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex min-w-0 items-center gap-2">
          <Link
            to="/content/$projectId"
            params={{ projectId: item.id }}
            search={search}
            className="rx-focus-ring text-fg hover:text-accent-text truncate rounded text-lg font-semibold"
          >
            {item.name}
          </Link>
          {item.featured && (
            <Badge variant="accent" icon={<Star />}>
              {t('featured')}
            </Badge>
          )}
        </div>
        <p className="text-fg-muted line-clamp-1 text-sm">{item.summary}</p>
        <div className="text-fg-muted flex flex-wrap items-center gap-1.5 text-xs">
          <Avatar name={item.author} size="xs" shape="circle" />
          <span className="text-fg font-medium">{item.author}</span>
          <span aria-hidden>·</span>
          <CompatibilityBadge compat={item.compatibility} />
          <SourceBadge source={item.source} />
          {item.shader_format && <ShaderFormatBadge format={item.shader_format} />}
          {item.categories.slice(0, 2).map((c) => (
            <span key={c} className="bg-surface-3 rounded-md px-1.5 py-0.5">
              {t(`categories.${c as 'utility'}`, { defaultValue: c })}
            </span>
          ))}
        </div>
      </div>
      <div className="flex shrink-0 flex-col items-end gap-2">
        <span className="text-fg-muted flex items-center gap-2 text-xs tabular-nums">
          <span className="inline-flex items-center gap-1">
            <Download className="size-3" aria-hidden />
            {formatCompact(item.downloads, i18n.language)}
          </span>
          <span aria-hidden>·</span>
          <span className="inline-flex items-center gap-1">
            <Clock className="size-3" aria-hidden />
            {formatRelative(item.updated_at, i18n.language)}
          </span>
        </span>
        <div className="flex items-center gap-2">
          <Button asChild size="sm">
            <Link to="/content/$projectId" params={{ projectId: item.id }} search={search}>
              <Eye />
              {t('view')}
            </Link>
          </Button>
          <InstallButton item={item} target={target} targetName={targetName} />
        </div>
      </div>
    </div>
  );
}
