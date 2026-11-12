import {
  Badge,
  Button,
  Card,
  Carousel,
  cn,
  formatBytes,
  Popover,
  PopoverClose,
  PopoverContent,
  PopoverTrigger,
  Skeleton,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  toast,
} from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  CalendarClock,
  Download,
  ExternalLink,
  Flag,
  Globe,
  HardDrive,
  Heart,
  Link2,
  Network,
  Scale,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CompatibilityBadge } from '@/components/CompatibilityBadge';
import { Page } from '@/components/Page';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { ContentDetail } from '@/ipc/types/content';
import { formatCompact, formatDate, formatRelative, shortVersion } from '@/lib/format';
import { useActiveInstance, useInstances } from '@/queries/core';
import { Markdown } from './Markdown';
import {
  ContentIcon,
  InstallButton,
  NoRusticServer,
  ShaderFormatBadge,
  SideBadge,
  SourceBadge,
  TargetPicker,
} from './parts';
import { contentDetailRoute } from './routes';
import {
  eligibleServers,
  formatTarget,
  isInstalledOn,
  parseTarget,
  resolveTarget,
  toQueryTarget,
  type TargetRef,
} from './target';
import { useContentDetail, useServers } from './useContent';

export function ContentDetailPage() {
  const { t } = useTranslation('content');
  const { projectId } = contentDetailRoute.useParams();
  const search = contentDetailRoute.useSearch();
  const navigate = useNavigate();
  const instances = useInstances();
  const servers = useServers();
  const { instance: active } = useActiveInstance();
  const requested = parseTarget(search.target);
  const [kindKnown, setKindKnown] = useState<ContentDetail['kind'] | null>(null);

  const target: TargetRef | null = useMemo(() => {
    if (kindKnown)
      return resolveTarget(
        kindKnown,
        requested,
        instances.data ?? [],
        servers.data ?? [],
        active?.id,
      );
    return requested ?? (active ? { kind: 'instance', id: active.id } : null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kindKnown, search.target, instances.data, servers.data, active?.id]);

  const detail = useContentDetail(projectId, toQueryTarget(target, kindKnown ?? 'mod'));
  const d = detail.data;
  const loadedKind = d?.kind;
  useEffect(() => {
    if (loadedKind) setKindKnown(loadedKind);
  }, [loadedKind]);

  const targetName = !target
    ? ''
    : target.kind === 'instance'
      ? (instances.data?.find((i) => i.id === target.id)?.name ?? '')
      : (servers.data?.find((s) => s.id === target.id)?.name ?? '');

  if (detail.isError) {
    return (
      <Page>
        <QueryError
          onRetry={() => void detail.refetch()}
          message={(detail.error as Error).message}
        />
      </Page>
    );
  }
  if (!d) return <DetailSkeleton />;

  const setTarget = (tr: TargetRef) =>
    void navigate({
      to: '/content/$projectId',
      params: { projectId },
      search: { target: formatTarget(tr) },
      replace: true,
    });

  return (
    <Page>
      <nav aria-label={t('detail.breadcrumb')} className="text-fg-subtle text-xs">
        <Link
          to="/content"
          search={{ kind: d.kind, target: target ? formatTarget(target) : undefined }}
          className="rx-focus-ring hover:text-fg rounded"
        >
          {t('title')} › {t(`kinds.${d.kind}`)}
        </Link>
        <span aria-hidden> › </span>
        <span className="text-fg-muted">{d.name}</span>
      </nav>
      <Header
        detail={d}
        target={target}
        targetName={targetName}
        onTarget={setTarget}
        instancesLoading={instances.isLoading || servers.isLoading}
      />
      <div className="grid grid-cols-[minmax(0,1fr)_280px] gap-6">
        <Tabs defaultValue="description" variant="underline">
          <TabsList aria-label={t('detail.sections')}>
            <TabsTrigger value="description">{t('detail.tabs.description')}</TabsTrigger>
            <TabsTrigger value="gallery">{t('detail.tabs.gallery')}</TabsTrigger>
            <TabsTrigger value="versions" count={d.versions.length}>
              {t('detail.tabs.versions')}
            </TabsTrigger>
            <TabsTrigger value="changelog">{t('detail.tabs.changelog')}</TabsTrigger>
            <TabsTrigger value="dependencies" count={d.dependencies.length}>
              {t('detail.tabs.dependencies')}
            </TabsTrigger>
            <TabsTrigger value="compatibility">{t('detail.tabs.compatibility')}</TabsTrigger>
          </TabsList>
          <TabsContent value="description">
            <Markdown source={d.description_md} />
          </TabsContent>
          <TabsContent value="gallery">
            <Gallery detail={d} />
          </TabsContent>
          <TabsContent value="versions">
            <Versions detail={d} target={target} targetName={targetName} />
          </TabsContent>
          <TabsContent value="changelog">
            <ol className="flex flex-col gap-4">
              {d.changelog.map((c) => (
                <li key={c.version} className="flex gap-4">
                  <span className="bg-accent mt-1.5 size-2 shrink-0 rounded-full" aria-hidden />
                  <div>
                    <p className="text-sm font-semibold">
                      {c.version}{' '}
                      <span className="text-fg-subtle font-normal">
                        · <FormattedDate iso={c.published_at} />
                      </span>
                    </p>
                    <p className="text-fg-muted text-sm">{c.notes}</p>
                  </div>
                </li>
              ))}
            </ol>
          </TabsContent>
          <TabsContent value="dependencies">
            <Dependencies detail={d} target={target} />
          </TabsContent>
          <TabsContent value="compatibility">
            <CompatibilityExplained detail={d} targetName={targetName} />
          </TabsContent>
        </Tabs>
        <Sidebar detail={d} />
      </div>
    </Page>
  );
}

function FormattedDate({ iso }: { iso: string }) {
  const { i18n } = useTranslation();
  return <>{formatDate(iso, i18n.language)}</>;
}

function Header({
  detail: d,
  target,
  targetName,
  onTarget,
  instancesLoading,
}: {
  detail: ContentDetail;
  target: TargetRef | null;
  targetName: string;
  onTarget: (t: TargetRef) => void;
  instancesLoading: boolean;
}) {
  const { t } = useTranslation('content');
  const instances = useInstances();
  const servers = useServers();
  const noServer =
    d.kind === 'plugin' &&
    !servers.isLoading &&
    eligibleServers('plugin', servers.data ?? []).length === 0;
  return (
    <header className="border-border relative overflow-hidden rounded-2xl border p-6">
      <div
        aria-hidden
        className="absolute inset-0 -z-10 opacity-60"
        style={{
          background: `radial-gradient(600px 220px at 15% 0%, hsl(${d.gallery[0]?.hue ?? 20} 60% 35% / 0.35), transparent 70%)`,
        }}
      />
      <div className="flex flex-wrap items-start gap-5">
        <ContentIcon id={d.id} name={d.name} size="xl" />
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          <h1 className="text-3xl font-bold tracking-tight">{d.name}</h1>
          <p className="text-fg-muted text-sm">
            {t('by', { author: d.author })} · {d.license} · {t(`sourceLong.${d.source}`)}
          </p>
          <p className="text-fg max-w-2xl text-sm">{d.summary}</p>
          <div className="flex flex-wrap items-center gap-1.5 pt-1">
            <CompatibilityBadge compat={d.compatibility} />
            <SourceBadge source={d.source} />
            <SideBadge side={d.side} />
            {d.shader_format && <ShaderFormatBadge format={d.shader_format} />}
            {d.kind === 'plugin' && (
              <Badge variant="accent">{t('plugins.badge', { api: '0.1' })}</Badge>
            )}
          </div>
        </div>
        <div className="flex w-full max-w-sm flex-col items-stretch gap-3 sm:w-auto">
          {d.kind !== 'modpack' && !noServer && (
            <TargetPicker
              kind={d.kind}
              value={target}
              onChange={onTarget}
              instances={instances.data ?? []}
              servers={servers.data ?? []}
              loading={instancesLoading}
            />
          )}
          <div className="flex items-center justify-end gap-2">
            <ReportButton name={d.name} />
            <InstallButton item={d} target={target} targetName={targetName} size="md" />
          </div>
        </div>
      </div>
      {noServer && (
        <div className="mt-5">
          <NoRusticServer />
        </div>
      )}
    </header>
  );
}

function ReportButton({ name }: { name: string }) {
  const { t } = useTranslation('content');
  const reasons = ['fair_play', 'malware', 'copyright', 'other'] as const;
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size="md" variant="ghost" leftIcon={<Flag />}>
          {t('report.button')}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-2">
        <p className="text-fg-muted px-2 pt-1 pb-2 text-xs font-semibold">
          {t('report.title', { name })}
        </p>
        <ul>
          {reasons.map((r) => (
            <li key={r}>
              <PopoverClose asChild>
                <button
                  type="button"
                  onClick={() =>
                    toast({
                      tone: 'success',
                      title: t('report.sent'),
                      description: t('report.sentBody'),
                    })
                  }
                  className="rx-focus-ring hover:bg-surface-3 w-full rounded-lg px-2 py-2 text-left text-sm"
                >
                  {t(`report.reasons.${r}`)}
                </button>
              </PopoverClose>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}

function Gallery({ detail: d }: { detail: ContentDetail }) {
  const { t } = useTranslation('content');
  return (
    <Carousel
      items={d.gallery}
      getKey={(g) => g.title}
      label={t('detail.galleryLabel', { name: d.name })}
      prevLabel={t('detail.prev')}
      nextLabel={t('detail.next')}
      slideLabel={(i, n) => t('detail.slide', { index: i, total: n })}
      autoPlayMs={0}
      className="aspect-[16/7] w-full"
      renderItem={(g) => (
        <div
          className="relative flex size-full items-end p-6"
          style={{
            background: `linear-gradient(135deg, hsl(${g.hue} 55% 38%), hsl(${(g.hue + 50) % 360} 60% 16%))`,
          }}
        >
          <div
            aria-hidden
            className="absolute inset-0 [background-image:linear-gradient(90deg,rgba(255,255,255,.08)_1px,transparent_1px),linear-gradient(rgba(255,255,255,.08)_1px,transparent_1px)] [background-size:32px_32px] opacity-25"
          />
          <p className="relative text-lg font-semibold text-white drop-shadow">{g.title}</p>
        </div>
      )}
    />
  );
}

function Versions({
  detail: d,
  target,
  targetName,
}: {
  detail: ContentDetail;
  target: TargetRef | null;
  targetName: string;
}) {
  const { t, i18n } = useTranslation('content');
  const installed = isInstalledOn(d, target);
  return (
    <div className="border-border overflow-hidden rounded-xl border">
      <table className="w-full text-left text-sm">
        <thead className="bg-surface-2 text-fg-muted text-xs">
          <tr>
            <th scope="col" className="px-4 py-2.5 font-medium">
              {t('versions.version')}
            </th>
            <th scope="col" className="px-4 py-2.5 font-medium whitespace-nowrap">
              {t('versions.horizon')}
            </th>
            <th scope="col" className="px-4 py-2.5 font-medium">
              {t('versions.published')}
            </th>
            <th scope="col" className="px-4 py-2.5 font-medium">
              {t('versions.downloads')}
            </th>
            <th scope="col" className="px-4 py-2.5 font-medium">
              {t('versions.compatibility')}
            </th>
            <th scope="col" className="px-4 py-2.5 text-right font-medium">
              <span className="sr-only">{t('versions.actions')}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {d.versions.map((v, i) => (
            <tr key={v.id} className="border-border border-t">
              <td className="px-4 py-3">
                <span className="font-semibold">{v.version}</span>{' '}
                {v.channel !== 'release' && (
                  <Badge variant="warning">{t(`versions.channel.${v.channel}`)}</Badge>
                )}
                <span className="text-2xs text-fg-subtle block">
                  {formatBytes(v.size_bytes, i18n.language)}
                </span>
              </td>
              <td className="text-fg-muted px-4 py-3 text-xs whitespace-nowrap">
                {v.horizon_versions.length
                  ? v.horizon_versions.map(shortVersion).join(', ')
                  : t('versions.none')}
              </td>
              <td className="text-fg-muted px-4 py-3 text-xs whitespace-nowrap">
                {formatDate(v.published_at, i18n.language)}
              </td>
              <td className="text-fg-muted px-4 py-3 text-xs tabular-nums">
                {formatCompact(v.downloads, i18n.language)}
              </td>
              <td className="px-4 py-3">
                <CompatibilityBadge compat={v.compatibility} />
              </td>
              <td className="px-4 py-3 text-right">
                {installed ? (
                  i === 0 ? (
                    <Badge variant="success">{t('install.installed')}</Badge>
                  ) : (
                    <span className="text-fg-subtle text-xs">—</span>
                  )
                ) : (
                  <InstallButton
                    item={d}
                    target={target}
                    targetName={targetName}
                    versionId={i === 0 ? undefined : v.id}
                    compatibility={v.compatibility}
                  />
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Dependencies({ detail: d, target }: { detail: ContentDetail; target: TargetRef | null }) {
  const { t } = useTranslation('content');
  if (d.dependencies.length === 0) {
    return <p className="text-fg-muted text-sm">{t('deps.none')}</p>;
  }
  return (
    <ul className="flex flex-col gap-2">
      {d.dependencies.map((dep) => (
        <li key={dep.project_id}>
          <Card padding="sm" className="flex items-center gap-3">
            <ContentIcon id={dep.project_id} name={dep.name} size="sm" />
            <Link
              to="/content/$projectId"
              params={{ projectId: dep.project_id }}
              search={{ target: target ? formatTarget(target) : undefined }}
              className="rx-focus-ring hover:text-accent-text flex-1 rounded text-sm font-medium"
            >
              {dep.name}
            </Link>
            <Badge
              variant={
                dep.relation === 'required'
                  ? 'accent'
                  : dep.relation === 'incompatible'
                    ? 'danger'
                    : 'neutral'
              }
            >
              {t(`deps.relation.${dep.relation}`)}
            </Badge>
            {dep.relation !== 'incompatible' && (
              <Badge variant={dep.installed ? 'success' : 'neutral'}>
                {dep.installed
                  ? t('deps.installed')
                  : dep.relation === 'required'
                    ? t('deps.willInstall')
                    : t('deps.notInstalled')}
              </Badge>
            )}
          </Card>
        </li>
      ))}
    </ul>
  );
}

function CompatibilityExplained({
  detail: d,
  targetName,
}: {
  detail: ContentDetail;
  targetName: string;
}) {
  const { t } = useTranslation('content');
  const c = d.compatibility;
  const network = d.kind === 'plugin' ? /Network: ([^\n]+)/.exec(d.description_md)?.[1] : undefined;
  return (
    <div className="flex max-w-2xl flex-col gap-4">
      <Card padding="md" className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <CompatibilityBadge compat={c} />
          {targetName && (
            <span className="text-fg-subtle text-xs">
              {t('compatTab.for', { target: targetName })}
            </span>
          )}
        </div>
        <p className="text-fg-muted text-sm">
          {c.status === 'incompatible'
            ? c.detail
            : c.status === 'compatible'
              ? c.builds_on_horizon
                ? t('compatTab.builds')
                : t('compatTab.format')
              : t('compatTab.unknown')}
        </p>
      </Card>
      {d.kind === 'plugin' && (
        <Card padding="md" className="flex flex-col gap-2">
          <h3 className="flex items-center gap-2 text-sm font-semibold">
            <Network className="text-fg-muted size-4" aria-hidden />
            {t('permissions.title')}
          </h3>
          <p className="text-fg-muted text-sm">
            {t('permissions.network', {
              value: network && network !== 'none' ? network : t('permissions.none'),
            })}
          </p>
          <p className="text-fg-muted text-sm">{t('permissions.files')}</p>
        </Card>
      )}
      <p className="text-fg-subtle text-xs">{t('compatTab.fairPlay')}</p>
    </div>
  );
}

function Sidebar({ detail: d }: { detail: ContentDetail }) {
  const { t, i18n } = useTranslation('content');
  const row = (icon: React.ReactNode, label: string, value: React.ReactNode) => (
    <div className="flex items-center gap-3 py-2 text-sm">
      <span className="text-fg-subtle [&_svg]:size-4">{icon}</span>
      <span className="text-fg-muted flex-1">{label}</span>
      <span className="text-fg font-medium tabular-nums">{value}</span>
    </div>
  );
  const links = Object.entries(d.links).filter(([, v]) => !!v) as [string, string][];
  return (
    <aside className="flex flex-col gap-4" aria-label={t('detail.about')}>
      <Card padding="md" className="divide-border divide-y py-2">
        {row(<Download />, t('stats.downloads'), formatCompact(d.downloads, i18n.language))}
        {row(<Heart />, t('stats.follows'), formatCompact(d.follows, i18n.language))}
        {row(<CalendarClock />, t('stats.updated'), formatRelative(d.updated_at, i18n.language))}
        {row(<Scale />, t('stats.license'), d.license)}
        {row(
          <HardDrive />,
          t('stats.size'),
          formatBytes(d.versions[0]?.size_bytes ?? 0, i18n.language),
        )}
      </Card>
      <Card padding="md" className="flex flex-col gap-2">
        <h3 className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
          {t('detail.categories')}
        </h3>
        <div className="flex flex-wrap gap-1.5">
          {d.categories.map((c) => (
            <Badge key={c}>{t(`categories.${c as 'utility'}`, { defaultValue: c })}</Badge>
          ))}
        </div>
      </Card>
      {links.length > 0 && (
        <Card padding="md" className="flex flex-col gap-1">
          <h3 className="text-fg-subtle mb-1 text-xs font-semibold tracking-wider uppercase">
            {t('detail.links')}
          </h3>
          {links.map(([k, url]) => (
            <button
              key={k}
              type="button"
              onClick={() => void invoke('app_open_url', { url })}
              className={cn(
                'rx-focus-ring text-accent-text flex items-center gap-2 rounded-md px-1 py-1.5 text-left text-sm hover:underline',
              )}
            >
              {k === 'website' ? <Globe className="size-4" /> : <Link2 className="size-4" />}
              <span className="flex-1">{t(`detail.link.${k as 'source'}`)}</span>
              <ExternalLink className="size-3.5 opacity-60" />
            </button>
          ))}
        </Card>
      )}
    </aside>
  );
}

function DetailSkeleton() {
  return (
    <Page>
      <Skeleton className="h-4 w-48" />
      <div className="border-border flex gap-5 rounded-2xl border p-6" aria-busy="true">
        <Skeleton className="size-20 rounded-2xl" />
        <div className="flex flex-1 flex-col gap-3">
          <Skeleton className="h-8 w-64" />
          <Skeleton className="h-4 w-80" />
          <Skeleton className="h-4 w-full max-w-xl" />
          <Skeleton className="h-5 w-72" />
        </div>
        <Skeleton className="h-9 w-40" />
      </div>
      <div className="grid grid-cols-[minmax(0,1fr)_280px] gap-6">
        <Skeleton className="h-72 rounded-xl" />
        <Skeleton className="h-72 rounded-xl" />
      </div>
    </Page>
  );
}
