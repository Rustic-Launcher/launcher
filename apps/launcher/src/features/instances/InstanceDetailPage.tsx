import {
  Badge,
  Button,
  EmptyState,
  Skeleton,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@rustic/ui';
import { Link, useNavigate, useParams, useSearch } from '@tanstack/react-router';
import {
  ChevronLeft,
  Gamepad2,
  Globe2,
  Layers,
  LayoutDashboard,
  Package,
  Settings2,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import { Page } from '@/components/Page';
import { QueryError } from '@/components/RouteError';
import { shortVersion } from '@/lib/format';
import { InstanceActions } from './InstanceActions';
import { InstancePlayControl } from './InstanceCard';
import { useInstance } from './queries';
import { ContentTab } from './tabs/ContentTab';
import { OverviewTab } from './tabs/OverviewTab';
import { ResourcePackOrder } from './tabs/ResourcePackOrder';
import { SettingsTab } from './tabs/SettingsTab';
import { WorldsTab } from './tabs/WorldsTab';

const TABS = ['overview', 'content', 'resourcepacks', 'worlds', 'settings'] as const;
type Tab = (typeof TABS)[number];

export function InstanceDetailPage() {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const { instanceId } = useParams({ from: '/shell/instances/$instanceId' });
  const { tab } = useSearch({ from: '/shell/instances/$instanceId' });
  const navigate = useNavigate({ from: '/instances/$instanceId' });
  const { data: inst, isLoading, isError, refetch } = useInstance(instanceId);
  const current: Tab = TABS.includes(tab as Tab) ? (tab as Tab) : 'overview';

  if (isLoading) {
    return (
      <Page>
        <div className="flex items-center gap-4">
          <Skeleton className="size-24 rounded-3xl" />
          <div className="flex flex-col gap-2">
            <Skeleton className="h-7 w-64" />
            <Skeleton className="h-4 w-48" />
          </div>
        </div>
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-64 w-full rounded-2xl" />
      </Page>
    );
  }
  if (isError) {
    return (
      <Page>
        <QueryError onRetry={refetch} />
      </Page>
    );
  }
  if (!inst) {
    return (
      <Page>
        <EmptyState
          icon={<Gamepad2 />}
          title={t('detail.notFound')}
          action={
            <Button asChild>
              <Link to="/instances">{t('detail.backToList')}</Link>
            </Button>
          }
        />
      </Page>
    );
  }

  const tabIcon: Record<Tab, React.ReactNode> = {
    overview: <LayoutDashboard />,
    content: <Package />,
    resourcepacks: <Layers />,
    worlds: <Globe2 />,
    settings: <Settings2 />,
  };

  return (
    <Page>
      <Link
        to="/instances"
        className="rx-focus-ring text-fg-subtle hover:text-fg flex w-fit items-center gap-1 rounded text-xs font-medium"
      >
        <ChevronLeft className="size-3.5" /> {t('title')}
      </Link>
      <header className="rx-surface relative flex flex-wrap items-center gap-5 overflow-hidden rounded-2xl p-5">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-60"
          style={{
            background:
              inst.icon.kind === 'preset'
                ? `radial-gradient(80% 140% at 0% 0%, color-mix(in oklab, ${inst.icon.color} 35%, transparent), transparent 70%)`
                : undefined,
          }}
        />
        <InstanceIcon icon={inst.icon} size="xl" className="relative" />
        <div className="relative flex min-w-0 flex-1 flex-col gap-1.5">
          <h1 className="truncate text-2xl font-bold tracking-tight">{inst.name}</h1>
          <HorizonVersion id={inst.horizon_version} badge className="text-fg-muted text-sm" />
          <div className="flex flex-wrap gap-1.5">
            {inst.update_available && (
              <Badge variant="warning">
                {tc('play.newVersion', { version: shortVersion(inst.update_available) })}
              </Badge>
            )}
            {inst.source && (
              <Badge variant="info">
                {t('card.fromModpack', { name: inst.source.name, version: inst.source.version })}
              </Badge>
            )}
            {inst.state === 'running' && <Badge variant="accent">{t('state.running')}</Badge>}
            {inst.state === 'installing' && <Badge variant="info">{t('state.installing')}</Badge>}
          </div>
        </div>
        <div className="relative flex items-center gap-2">
          <InstancePlayControl instance={inst} className="h-10 px-5 text-sm" />
          <InstanceActions instance={inst} />
        </div>
      </header>

      <Tabs
        value={current}
        onValueChange={(v) => void navigate({ search: { tab: v }, replace: true })}
      >
        <TabsList aria-label={t('detail.tabsLabel')}>
          {TABS.map((id) => (
            <TabsTrigger key={id} value={id} icon={tabIcon[id]} data-testid={`tab-${id}`}>
              {t(`detail.tabs.${id}`)}
            </TabsTrigger>
          ))}
        </TabsList>
        <TabsContent value="overview">
          <OverviewTab instance={inst} />
        </TabsContent>
        <TabsContent value="content">
          <ContentTab instance={inst} />
        </TabsContent>
        <TabsContent value="resourcepacks">
          <ResourcePackOrder instanceId={inst.id} />
        </TabsContent>
        <TabsContent value="worlds">
          <WorldsTab instance={inst} />
        </TabsContent>
        <TabsContent value="settings">
          <SettingsTab instance={inst} />
        </TabsContent>
      </Tabs>
    </Page>
  );
}
