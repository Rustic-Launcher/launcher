import { Button, IconButton } from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { Link, useNavigate } from '@tanstack/react-router';
import { Compass, ExternalLink, LayoutList, Plus, RefreshCw, Star } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CatalogSearch,
  CatalogTabList,
  CatalogTabPanel,
  CatalogTabs,
  CatalogToolbar,
} from '@/components/catalog';
import { PageTitle } from '@/components/Page';
import { invoke } from '@/ipc';
import { sk } from '@/queries/social';
import { BrowseTab } from './directory/BrowseTab';
import { DiscoverTab } from './directory/DiscoverTab';
import { MineTab } from './directory/MineTab';
import { PUBLISH_SERVER_URL } from './directory/queries';
import { useMultiplayer, useServers, useServersLive } from './list/queries';
import { serversRoute, type ServersSearch, type ServersTab } from './routes';

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

/** What every tab knows: the URL state and the text being searched. */
export interface ServersContext {
  search: ServersSearch;
  /** Debounced search text. */
  query: string;
  setSearch: (patch: Partial<ServersSearch>, replace?: boolean) => void;
  /** Moves to Browse with these filters (a row's "See all"). */
  browse: (filters: Partial<ServersSearch>) => void;
}

/**
 * Servers (ADR 0158), in the Explore design: Mine (the multiplayer list and the servers hosted
 * here, as cards), Discover (staff picks and verified servers in a carousel, themed rows) and
 * Browse (the whole directory with filters and paging). Hosting, management and the creation
 * wizard stay where they were (`/servers/new`, `/servers/<id>`).
 */
export function ServersPage() {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const qc = useQueryClient();
  const search = serversRoute.useSearch();
  const navigate = useNavigate({ from: '/servers' });
  const [text, setText] = useState(search.q ?? '');
  const query = useDebounced(text.trim(), 250);
  const hosted = useServers();
  const saved = useMultiplayer();
  useServersLive();
  const mineCount = (hosted.data?.length ?? 0) + (saved.data?.length ?? 0);
  // Without a tab in the URL: the player's own servers when there is at least one (or while
  // they load), else Discover; a search lands on Browse.
  const mineLoading = hosted.isPending || saved.isPending;
  const tab: ServersTab =
    search.tab ?? (search.q ? 'browse' : mineCount > 0 || mineLoading ? 'mine' : 'discover');

  const setSearch = (patch: Partial<ServersSearch>, replace = false) =>
    void navigate({ to: '/servers', search: (prev) => ({ ...prev, ...patch }), replace });

  // The URL follows the debounced text; typing on Discover moves to Browse, where results are.
  useEffect(() => {
    if ((search.q ?? '') === query) return;
    const toBrowse = !!query && tab === 'discover';
    void navigate({
      to: '/servers',
      search: (prev) => ({ ...prev, q: query || undefined, tab: toBrowse ? 'browse' : prev.tab }),
      replace: !toBrowse,
    });
  }, [query, search.q, tab, navigate]);

  const ctx: ServersContext = {
    search,
    query,
    setSearch,
    browse: (filters) =>
      void navigate({
        to: '/servers',
        search: (prev) => ({ q: prev.q, tab: 'browse', ...filters }),
      }),
  };

  return (
    <div className="rx-page-enter flex min-h-full flex-col gap-4 p-4">
      <CatalogTabs value={tab} onValueChange={(v) => setSearch({ tab: v })}>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <PageTitle title={t('list.heading')} />
          <CatalogTabList
            label={t('directory.tabsLabel')}
            testIdPrefix="servers-tab"
            tabs={[
              {
                value: 'mine',
                label: t('list.views.mine'),
                icon: <Star />,
                count: mineCount || undefined,
              },
              { value: 'discover', label: t('directory.tabs.discover'), icon: <Compass /> },
              { value: 'browse', label: t('directory.tabs.browse'), icon: <LayoutList /> },
            ]}
          />
        </div>
        <CatalogToolbar
          start={
            <CatalogSearch
              value={text}
              onChange={setText}
              testId="servers-search"
              label={t('directory.searchLabel')}
              clearLabel={tc('actions.clear')}
              placeholder={
                tab === 'mine' ? t('list.searchPlaceholder.mine') : t('directory.searchPlaceholder')
              }
            />
          }
          end={
            <>
              <IconButton
                variant="secondary"
                size="lg"
                className="rounded-xl"
                label={t('list.refreshPing')}
                icon={<RefreshCw />}
                onClick={() => {
                  void qc.invalidateQueries({ queryKey: ['multiplayer'] });
                  void qc.invalidateQueries({ queryKey: ['server-listings'] });
                  void qc.invalidateQueries({ queryKey: sk.featured });
                  void qc.invalidateQueries({ queryKey: ['servers'] });
                }}
              />
              <Button
                variant="secondary"
                className="h-11 rounded-xl"
                leftIcon={<ExternalLink />}
                onClick={() => void invoke('app_open_url', { url: PUBLISH_SERVER_URL })}
                data-testid="publish-server"
              >
                {t('directory.publish')}
              </Button>
              <Button asChild variant="primary" className="h-11" leftIcon={<Plus />}>
                <Link to="/servers/new" data-testid="create-server" title={t('list.createHint')}>
                  {t('list.create')}
                </Link>
              </Button>
            </>
          }
        />
        <CatalogTabPanel value="mine">
          <MineTab ctx={ctx} />
        </CatalogTabPanel>
        <CatalogTabPanel value="discover">
          <DiscoverTab ctx={ctx} />
        </CatalogTabPanel>
        <CatalogTabPanel value="browse">
          <BrowseTab ctx={ctx} />
        </CatalogTabPanel>
      </CatalogTabs>
    </div>
  );
}
