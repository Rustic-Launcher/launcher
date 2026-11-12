import {
  Badge,
  Button,
  Carousel,
  cn,
  Dropdown,
  DropdownCheckboxItem,
  DropdownContent,
  DropdownLabel,
  DropdownTrigger,
  EmptyState,
  IconButton,
  Select,
  Skeleton,
} from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  ArrowDownWideNarrow,
  ChevronDown,
  Download,
  Gamepad2,
  Play,
  Plus,
  Search,
  Square,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useHorizonVersionLabel } from '@/components/HorizonVersion';
import { QueryError } from '@/components/RouteError';
import { usePlay } from '@/features/play/usePlay';
import { invoke } from '@/ipc';
import type { Instance } from '@/ipc/types/instances';
import { formatDate, formatDuration, formatRelative } from '@/lib/format';
import { useHorizonVersions, useInstances } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { InstanceActions } from './InstanceActions';
import { InstanceCover } from './InstanceCover';
import { filterInstances, sortInstances, type InstanceSort } from './logic';
import { NewInstanceSheet } from './NewInstanceSheet';
import { instancesRoute } from './routes';
import { PageTitle } from '@/components/Page';

type Source = 'all' | 'custom' | 'modpack';

/** Versions (step 1b): portrait cover grid + right detail panel for the selected profile. */
export function InstancesPage() {
  const { t } = useTranslation('instances');
  const navigate = useNavigate();
  const search = instancesRoute.useSearch();
  const { data, isLoading, isError, refetch } = useInstances();
  const activeId = usePrefs((s) => s.activeInstanceId);
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState<InstanceSort>('recent');
  const [source, setSource] = useState<Source>('all');
  const [selected, setSelected] = useState<string | null>(search.selected ?? null);
  const creating = search.new === '1';
  const setCreating = (open: boolean) =>
    void navigate({
      to: '/instances',
      search: (s) => ({ ...s, new: open ? '1' : undefined }),
      replace: true,
    });

  const list = useMemo(() => {
    const bySource = (data ?? []).filter((i) =>
      source === 'all' ? true : source === 'modpack' ? !!i.source : !i.source,
    );
    return sortInstances(filterInstances(bySource, query), sort);
  }, [data, query, sort, source]);
  const current =
    data?.find((i) => i.id === selected) ?? data?.find((i) => i.id === activeId) ?? list[0];

  useEffect(() => {
    if (search.selected) setSelected(search.selected);
  }, [search.selected]);

  return (
    <div className="rx-page-enter grid min-h-full grid-cols-[minmax(0,1fr)_minmax(300px,340px)] gap-4 p-4">
      <div className="flex min-w-0 flex-col gap-4">
        <PageTitle title={t('heading')} />
        <div className="flex items-center gap-2">
          <div className="bg-surface-2 border-border flex h-11 flex-1 items-center rounded-xl border">
            <Dropdown>
              <DropdownTrigger asChild>
                <button
                  type="button"
                  className="rx-focus-ring flex h-full items-center gap-1.5 rounded-l-xl px-3.5 text-sm font-semibold"
                  aria-label={t('versions.source')}
                >
                  {t(`versions.sources.${source}`)}
                  <ChevronDown className="text-fg-subtle size-3.5" />
                </button>
              </DropdownTrigger>
              <DropdownContent align="start">
                <DropdownLabel>{t('versions.source')}</DropdownLabel>
                {(['all', 'custom', 'modpack'] as const).map((s) => (
                  <DropdownCheckboxItem
                    key={s}
                    checked={source === s}
                    onCheckedChange={() => setSource(s)}
                  >
                    {t(`versions.sources.${s}`)}
                  </DropdownCheckboxItem>
                ))}
              </DropdownContent>
            </Dropdown>
            <span className="bg-border-default h-5 w-px" />
            <Search className="text-fg-subtle ml-3 size-4" aria-hidden />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t('versions.search')}
              aria-label={t('versions.search')}
              className="placeholder:text-fg-subtle h-full flex-1 bg-transparent px-2.5 text-sm outline-none"
            />
            {query && (
              <IconButton
                size="sm"
                label={t('versions.clear')}
                icon={<X />}
                onClick={() => setQuery('')}
                className="mr-1.5"
              />
            )}
          </div>
          <Select
            value={sort}
            onValueChange={(v) => setSort(v as InstanceSort)}
            aria-label={t('sort.label')}
            className="bg-surface-2 h-11 w-44 rounded-xl"
            options={[
              { value: 'recent', label: t('sort.recent'), icon: <ArrowDownWideNarrow /> },
              { value: 'name', label: t('sort.name'), icon: <ArrowDownWideNarrow /> },
              { value: 'playtime', label: t('sort.playtime'), icon: <ArrowDownWideNarrow /> },
            ]}
          />
          <Button
            variant="soft"
            size="lg"
            className="h-11 rounded-xl"
            leftIcon={<Plus />}
            onClick={() => setCreating(true)}
            data-testid="new-instance"
          >
            {t('versions.newProfile')}
          </Button>
        </div>

        {isError && <QueryError onRetry={() => void refetch()} />}
        {data && data.length === 0 && (
          <EmptyState
            icon={<Gamepad2 />}
            title={t('empty.title')}
            description={t('empty.description')}
            action={
              <Button variant="primary" leftIcon={<Plus />} onClick={() => setCreating(true)}>
                {t('versions.newProfile')}
              </Button>
            }
          />
        )}
        {data && data.length > 0 && list.length === 0 && (
          <EmptyState
            icon={<Gamepad2 />}
            title={t('noMatch.title')}
            description={t('noMatch.description', { query })}
          />
        )}
        <ul
          className="grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-x-4 gap-y-5"
          aria-label={t('title')}
        >
          {isLoading &&
            Array.from({ length: 6 }, (_, i) => (
              <li key={i} className="flex flex-col gap-2">
                <Skeleton className="aspect-[3/4] rounded-xl" />
                <Skeleton className="h-5 w-2/3" />
              </li>
            ))}
          {list.map((i) => (
            <li key={i.id}>
              <CoverCard
                instance={i}
                selected={current?.id === i.id}
                onSelect={() => setSelected(i.id)}
              />
            </li>
          ))}
        </ul>
      </div>
      {current ? (
        <DetailPanel instance={current} />
      ) : isLoading ? (
        <Skeleton className="rounded-xl" />
      ) : (
        <aside
          className="border-border text-fg-muted flex items-center justify-center rounded-xl border border-dashed p-6 text-center text-sm"
          data-testid="instance-detail-empty"
        >
          {t('versions.noSelection')}
        </aside>
      )}
      <NewInstanceSheet open={creating} onOpenChange={setCreating} />
    </div>
  );
}

function CardPlay({ instance }: { instance: Instance }) {
  const { t } = useTranslation();
  const play = usePlay(instance);
  const setActive = usePrefs((s) => s.setActiveInstance);
  const map = {
    play: { label: t('play.play'), icon: <Play className="fill-current" /> },
    update: { label: t('play.update'), icon: <Download /> },
    running: { label: t('play.stop'), icon: <Square /> },
    progress: { label: t('play.cancel'), icon: <X /> },
    disabled: { label: t('play.play'), icon: <Play /> },
  } as const;
  const m = map[play.state];
  return (
    <IconButton
      size="sm"
      label={`${m.label} — ${instance.name}`}
      icon={m.icon}
      data-testid={`play-${instance.id}`}
      data-state={play.state}
      disabled={play.state === 'disabled'}
      className={cn(
        play.state === 'running' || play.state === 'progress'
          ? 'text-accent-text'
          : 'text-accent-text hover:bg-accent-soft hover:text-accent-text',
      )}
      onClick={(e) => {
        e.stopPropagation();
        setActive(instance.id);
        play.primary();
      }}
    />
  );
}

function CoverCard({
  instance,
  selected,
  onSelect,
}: {
  instance: Instance;
  selected: boolean;
  onSelect: () => void;
}) {
  const { t } = useTranslation('instances');
  const play = usePlay(instance);
  return (
    <div className="flex flex-col gap-2" data-testid={`instance-card-${instance.id}`}>
      <button
        type="button"
        onClick={onSelect}
        aria-pressed={selected}
        aria-label={instance.name}
        className={cn(
          'rx-focus-ring relative aspect-[3/4] overflow-hidden rounded-xl transition-[box-shadow,transform] duration-200 hover:-translate-y-0.5',
          selected ? 'ring-accent ring-offset-surface ring-2 ring-offset-2' : 'ring-border ring-1',
        )}
      >
        <InstanceCover instance={instance} className="size-full" />
        <span className="absolute inset-x-2 bottom-2 flex flex-wrap gap-1">
          {instance.pinned && (
            <Badge variant="neutral" className="bg-black/50 text-white">
              {t('card.pinned')}
            </Badge>
          )}
          {instance.update_available && (
            <Badge variant="warning" className="bg-black/50">
              {t('versions.update')}
            </Badge>
          )}
          {play.state === 'running' && <Badge variant="solid">{t('versions.running')}</Badge>}
        </span>
        {play.state === 'progress' && play.progress != null && (
          <span className="absolute inset-x-0 bottom-0 h-1 bg-black/40">
            <span className="bg-accent block h-full" style={{ width: `${play.progress * 100}%` }} />
          </span>
        )}
      </button>
      <div className="flex items-center justify-between gap-1">
        <span className="truncate text-base font-semibold">{instance.name}</span>
        <CardPlay instance={instance} />
      </div>
    </div>
  );
}

function DetailPanel({ instance }: { instance: Instance }) {
  const { t, i18n } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const versionLabel = useHorizonVersionLabel();
  const { data: versions } = useHorizonVersions();
  const play = usePlay(instance);
  const setActive = usePrefs((s) => s.setActiveInstance);
  const [more, setMore] = useState(false);
  const color = instance.icon.kind === 'preset' ? instance.icon.color : '#5b6070';
  const slides = [0, 1, 2].map((i) => ({ id: `${instance.id}-${i}`, i }));
  const version = versions?.find((v) => v.id === instance.horizon_version);

  return (
    <aside
      className="flex min-w-0 flex-col gap-4"
      aria-label={t('versions.details')}
      data-testid="instance-detail-panel"
    >
      <Carousel
        items={slides}
        getKey={(s) => s.id}
        label={t('versions.media')}
        prevLabel={tc('actions.previous')}
        nextLabel={tc('actions.next')}
        slideLabel={(i, n) => t('versions.slide', { index: i, total: n })}
        className="border-border aspect-video border"
        renderItem={(s) => (
          <div
            className="relative h-full overflow-hidden"
            style={{
              background: `linear-gradient(${120 + s.i * 50}deg, ${color}, color-mix(in oklab, ${color} 30%, #07080b))`,
            }}
          >
            <InstanceCover
              instance={instance}
              className="absolute -right-6 -bottom-10 h-[150%] w-1/2 rotate-6 rounded-xl opacity-90"
            />
            <span className="absolute bottom-3 left-3 text-sm font-semibold text-white drop-shadow">
              {t(`versions.slides.${s.i}` as 'versions.slides.0', { name: instance.name })}
            </span>
          </div>
        )}
      />
      <div className="flex flex-col gap-1.5">
        <h2 className="text-xl font-bold">{instance.name}</h2>
        <p className={cn('text-fg-muted text-sm', !more && 'line-clamp-3')}>
          {t('versions.blurb', {
            version: versionLabel(instance.horizon_version),
            mods: instance.mod_count,
            playtime: formatDuration(instance.playtime_s, i18n.language),
          })}{' '}
          {more && (
            <>
              {instance.source &&
                t('versions.fromModpack', {
                  name: instance.source.name,
                  version: instance.source.version,
                })}{' '}
              {t('versions.created', { date: formatDate(instance.created_at, i18n.language) })}{' '}
              {instance.last_played_at &&
                t('versions.lastPlayed', {
                  when: formatRelative(instance.last_played_at, i18n.language),
                })}
            </>
          )}
        </p>
        <button
          type="button"
          className="text-fg w-fit text-sm font-semibold hover:underline"
          onClick={() => setMore((m) => !m)}
        >
          {more ? t('versions.readLess') : t('versions.readMore')}
        </button>
      </div>
      <dl className="bg-surface-2 border-border flex flex-col divide-y divide-[var(--rl-border)] rounded-xl border text-sm">
        <div className="flex items-center justify-between gap-3 px-3 py-2.5">
          <dt className="text-fg-muted">{t('versions.version')}</dt>
          <dd>
            <Select
              size="sm"
              className="w-48"
              aria-label={t('versions.version')}
              value={instance.horizon_version}
              onValueChange={(v) =>
                void invoke('instance_set_version', { id: instance.id, horizon_version: v })
              }
              options={(versions ?? []).map((v) => ({
                value: v.id,
                label: versionLabel(v.id, false),
              }))}
            />
          </dd>
        </div>
        <div className="flex items-center justify-between gap-3 px-3 py-2.5">
          <dt className="text-fg-muted">{t('versions.build')}</dt>
          <dd>{version ? t(`new.${version.channel}`) : '—'}</dd>
        </div>
        <div className="flex items-center justify-between gap-3 px-3 py-2.5">
          <dt className="text-fg-muted">{t('versions.edition')}</dt>
          <dd>
            {version?.rustic_edition ? (
              <Badge variant="accent">{tc('horizon.rusticEdition')}</Badge>
            ) : (
              <Badge variant="neutral">{t('versions.plainHorizon')}</Badge>
            )}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-3 px-3 py-2.5">
          <dt className="text-fg-muted">{t('versions.content')}</dt>
          <dd className="tabular-nums">
            {t('versions.contentCount', {
              mods: instance.mod_count,
              packs: instance.resource_pack_count,
              shaders: instance.shader_count,
            })}
          </dd>
        </div>
      </dl>
      <div className="flex items-center gap-2">
        <Button
          variant={play.state === 'running' ? 'secondary' : 'primary'}
          size="lg"
          className="flex-1 font-bold uppercase"
          disabled={play.state === 'disabled'}
          data-testid="detail-play"
          onClick={() => {
            setActive(instance.id);
            play.primary();
          }}
        >
          {play.state === 'update'
            ? tc('launch.update')
            : play.state === 'running'
              ? tc('launch.stop')
              : play.state === 'progress'
                ? play.progressLabel || tc('play.cancel')
                : tc('launch.launch')}
        </Button>
        <InstanceActions instance={instance} />
      </div>
      <Button asChild variant="ghost" size="sm" className="w-fit">
        <Link to="/instances/$instanceId" params={{ instanceId: instance.id }}>
          {t('versions.manage')}
        </Link>
      </Button>
    </aside>
  );
}
