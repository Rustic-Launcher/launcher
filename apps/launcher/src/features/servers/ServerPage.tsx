import { Skeleton, Tabs, TabsContent, TabsList, TabsTrigger } from '@rustic/ui';
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
        <TabsList aria-label={t('manage.tabsLabel')} className="overflow-x-auto">
          {SERVER_TABS.map((k) => (
            <TabsTrigger
              key={k}
              value={k}
              icon={ICONS[k]}
              data-testid={`server-tab-${k}`}
              className={k === 'danger' ? 'data-[state=active]:text-danger-text' : undefined}
            >
              {t(`manage.tabs.${k}`)}
            </TabsTrigger>
          ))}
        </TabsList>
        <TabsContent value="overview">
          <OverviewTab server={server} />
        </TabsContent>
        <TabsContent value="console">
          <ConsoleTab server={server} />
        </TabsContent>
        <TabsContent value="players">
          <PlayersTab server={server} />
        </TabsContent>
        <TabsContent value="plugins">
          <PluginsTab server={server} />
        </TabsContent>
        <TabsContent value="config">
          <ConfigTab server={server} />
        </TabsContent>
        <TabsContent value="datapacks">
          <DatapacksTab server={server} />
        </TabsContent>
        <TabsContent value="backups">
          <BackupsTab server={server} />
        </TabsContent>
        <TabsContent value="access">
          <AccessTab server={server} />
        </TabsContent>
        <TabsContent value="danger">
          <DangerTab server={server} />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
