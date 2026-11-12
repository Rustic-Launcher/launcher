import { cn, IconButton, SegmentedControl, Select } from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { Link, useNavigate, useSearch } from '@tanstack/react-router';
import {
  ArrowDownWideNarrow,
  Compass,
  LayoutGrid,
  List,
  Plus,
  RefreshCw,
  Search,
  Star,
  X,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { sk } from '@/queries/social';
import { FeaturedServers, type FeaturedSort } from './list/FeaturedServers';
import { MultiplayerList } from './list/MultiplayerList';
import { MyServers } from './list/MyServers';
import { useServers, useServersLive } from './list/queries';
import { PageTitle } from '@/components/Page';

/** "My servers" (the ones hosted here and the multiplayer list) or "Discover" (featured). */
type View = 'mine' | 'discover';

function normaliseTab(tab: string | undefined): View {
  // Older links: the multiplayer list (`joined`, `multiplayer`) is part of "My servers" now.
  if (tab === 'mine' || tab === 'joined' || tab === 'multiplayer') return 'mine';
  return 'discover';
}

/** Servers: a "My servers / Discover" switch, then search, sort, ping + layout, Create server. */
export function ServersPage() {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const search = useSearch({ from: '/shell/servers' });
  const navigate = useNavigate({ from: '/servers' });
  const view = normaliseTab(search.tab);
  const { data } = useServers();
  useServersLive();
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState<FeaturedSort>('recommended');
  const [layout, setLayout] = useState<'grid' | 'list'>('grid');
  const setView = (v: View) => void navigate({ search: { tab: v }, replace: true });

  return (
    <div className="rx-page-enter flex min-h-full flex-col gap-4 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <PageTitle title={t('list.heading')} />
        <SegmentedControl
          label={t('list.viewLabel')}
          value={view}
          onValueChange={(v) => setView(v as View)}
          className="bg-surface-2 rounded-xl"
          options={[
            {
              value: 'mine',
              label: data?.length
                ? `${t('list.views.mine')} · ${data.length}`
                : t('list.views.mine'),
              icon: <Star />,
            },
            { value: 'discover', label: t('list.views.discover'), icon: <Compass /> },
          ]}
        />
      </div>
      <div className="flex items-center gap-2">
        <div className="bg-surface-2 border-border focus-within:border-accent/60 flex h-11 min-w-0 flex-1 items-center rounded-xl border">
          <Search className="text-fg-subtle ml-3.5 size-4 shrink-0" aria-hidden />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t(`list.searchPlaceholder.${view}`)}
            aria-label={t(`list.searchPlaceholder.${view}`)}
            type="search"
            className="placeholder:text-fg-subtle h-full min-w-0 flex-1 bg-transparent px-2.5 text-sm outline-none"
          />
          {query && (
            <IconButton
              size="sm"
              label={t('list.clearSearch')}
              icon={<X />}
              onClick={() => setQuery('')}
              className="mr-1.5"
            />
          )}
        </div>
        {view === 'discover' && (
          <Select
            aria-label={t('list.sortLabel')}
            value={sort}
            onValueChange={(v) => setSort(v as FeaturedSort)}
            className="bg-surface-2 h-11 w-48 rounded-xl"
            options={(['recommended', 'players', 'name'] as const).map((s) => ({
              value: s,
              label: t(`list.sorts.${s}`),
              icon: <ArrowDownWideNarrow />,
            }))}
          />
        )}
        <IconButton
          variant="secondary"
          size="lg"
          className="rounded-xl"
          label={t('list.refreshPing')}
          icon={<RefreshCw />}
          onClick={() => {
            void qc.invalidateQueries({ queryKey: ['multiplayer'] });
            void qc.invalidateQueries({ queryKey: sk.featured });
            void qc.invalidateQueries({ queryKey: ['servers'] });
          }}
        />
        {view === 'discover' && (
          <IconButton
            variant="secondary"
            size="lg"
            className="rounded-xl"
            label={layout === 'grid' ? t('list.layoutList') : t('list.layoutGrid')}
            icon={layout === 'grid' ? <List /> : <LayoutGrid />}
            onClick={() => setLayout((l) => (l === 'grid' ? 'list' : 'grid'))}
          />
        )}
        <Link
          to="/servers/new"
          data-testid="create-server"
          className={cn(
            'rx-focus-ring border-accent/50 bg-accent-soft text-accent-text hover:bg-accent/20 flex h-11 items-center gap-2.5 rounded-xl border px-4 text-sm font-semibold transition-colors',
          )}
        >
          <Plus className="size-4" />
          <span className="flex flex-col leading-tight">
            {t('list.create')}
            <span className="text-fg-muted text-2xs font-medium">{t('list.createHint')}</span>
          </span>
        </Link>
      </div>
      {view === 'discover' && <FeaturedServers query={query} sort={sort} layout={layout} />}
      {view === 'mine' && (
        <div className="flex flex-col gap-6">
          <section aria-labelledby="servers-hosted" className="flex flex-col gap-3">
            <h2 id="servers-hosted" className="text-fg-muted text-sm font-semibold">
              {t('list.sections.hosted')}
            </h2>
            <MyServers query={query} />
          </section>
          <section aria-labelledby="servers-joined" className="flex flex-col gap-3">
            <h2 id="servers-joined" className="text-fg-muted text-sm font-semibold">
              {t('list.sections.joined')}
            </h2>
            <MultiplayerList query={query} />
          </section>
        </div>
      )}
    </div>
  );
}
