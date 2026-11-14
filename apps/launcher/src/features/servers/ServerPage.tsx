import { cn, Skeleton, Tabs, TabsContent, TabsList, TabsTrigger } from '@rustic/ui';
import { useNavigate } from '@tanstack/react-router';
import {
  Archive,
  Gauge,
  Globe,
  Package,
  Puzzle,
  ScrollText,
  SlidersHorizontal,
  TriangleAlert,
  Users,
} from 'lucide-react';
import { Fragment } from 'react';
import { useTranslation } from 'react-i18next';
import { Page } from '@/components/Page';
import { QueryError } from '@/components/RouteError';
import { AccessTab } from './manage/access/AccessTab';
import { BackupsTab } from './manage/BackupsTab';
import { ConfigTab } from './manage/ConfigTab';
import { ConsoleTab } from './manage/ConsoleTab';
import { DangerTab } from './manage/DangerTab';
import { DatapacksTab } from './manage/DatapacksTab';
import { OverviewTab } from './manage/OverviewTab';
import { PlayersTab } from './manage/PlayersTab';
import { PluginsTab } from './manage/PluginsTab';
import { useServer } from './manage/queries';
import { ServerHeader } from './manage/ServerHeader';
import { serverRoute } from './routes';

export const SERVER_TABS = [
  'overview',
  'console',
  'players',
  'plugins',
  'config',
  'datapacks',
  'backups',
  'access',
  'danger',
] as const;
export type ServerTab = (typeof SERVER_TABS)[number];

const ICONS: Record<ServerTab, React.ReactNode> = {
  overview: <Gauge />,
  console: <ScrollText />,
  players: <Users />,
  plugins: <Puzzle />,
  config: <SlidersHorizontal />,
  datapacks: <Package />,
  backups: <Archive />,
  access: <Globe />,
  danger: <TriangleAlert />,
};

/** `/servers/$serverId`: management of a local or remote (VPS) server. */
export function ServerPage() {
  const { t } = useTranslation('servers');
  const { serverId } = serverRoute.useParams();
  const search = serverRoute.useSearch();
  const navigate = useNavigate();
  const tab: ServerTab = SERVER_TABS.includes(search.tab as ServerTab)
    ? (search.tab as ServerTab)
    : 'overview';
  const { data: server, isLoading, isError, error, refetch } = useServer(serverId);

  if (isLoading) {
    return (
      <Page wide>
        <div className="flex items-center gap-4" aria-busy="true">
          <Skeleton className="size-16 rounded-2xl" />
          <div className="flex flex-col gap-2">
            <Skeleton className="h-7 w-64" />
            <Skeleton className="h-4 w-96" />
          </div>
        </div>
        <Skeleton className="h-10 w-full" />
        <div className="grid grid-cols-2 gap-4 xl:grid-cols-4">
          {Array.from({ length: 4 }, (_, i) => (
            <Skeleton key={i} className="h-36 rounded-2xl" />
          ))}
        </div>
      </Page>
    );
  }
  if (isError || !server) {
    return (
      <Page>
        <QueryError message={error?.message} onRetry={() => void refetch()} />
      </Page>
    );
  }

  return (
    <Page wide>
      <ServerHeader server={server} />
      <Tabs
        variant="pill"
        className="gap-5"
        value={tab}
        onValueChange={(v) =>
          void navigate({
            to: '/servers/$serverId',
            params: { serverId },
            search: { tab: v },
            replace: true,
          })
        }
      >
        <TabsList
          aria-label={t('manage.tabsLabel')}
          className="bg-surface-2 isolate h-11 max-w-full [scrollbar-width:none] overflow-x-auto rounded-xl p-1"
        >
          {SERVER_TABS.map((k) => (
            <Fragment key={k}>
              {k === 'danger' && (
                <span aria-hidden className="bg-border mx-1.5 h-5 w-px shrink-0" />
              )}
              <TabsTrigger
                value={k}
                icon={ICONS[k]}
                data-testid={`server-tab-${k}`}
                title={t(`manage.tabs.${k}`)}
                className={cn(
                  'h-9 shrink-0 rounded-lg px-3 font-semibold xl:px-4 max-xl:[&>svg]:hidden',
                  k === 'danger' &&
                    'text-danger-text/80 hover:text-danger-text data-[state=active]:text-danger-text max-xl:[&>svg]:inline',
                )}
              >
                {k === 'danger' ? (
                  <span className="max-xl:sr-only">{t(`manage.tabs.${k}`)}</span>
                ) : (
                  t(`manage.tabs.${k}`)
                )}
              </TabsTrigger>
            </Fragment>
          ))}
        </TabsList>
        <TabsContent className="mt-0" value="overview">
          <OverviewTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="console">
          <ConsoleTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="players">
          <PlayersTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="plugins">
          <PluginsTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="config">
          <ConfigTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="datapacks">
          <DatapacksTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="backups">
          <BackupsTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="access">
          <AccessTab server={server} />
        </TabsContent>
        <TabsContent className="mt-0" value="danger">
          <DangerTab server={server} />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
