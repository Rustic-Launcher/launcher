import {
  Badge,
  Button,
  Card,
  Carousel,
  cn,
  EmptyState,
  formatBytes,
  IconButton,
  Popover,
  PopoverClose,
  PopoverContent,
  PopoverTrigger,
  Select,
  Skeleton,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  toast,
} from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  BookmarkCheck,
  BookmarkPlus,
  CalendarClock,
  Clock,
  Download,
  ExternalLink,
  Flag,
  Globe,
  HardDrive,
  Heart,
  Link2,
  Network,
  PackageX,
  Scale,
  User,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CatalogCover, safeImageUrl } from '@/components/catalog';
import { CompatibilityBadge } from '@/components/CompatibilityBadge';
import { Page } from '@/components/Page';
import { QueryError } from '@/components/RouteError';
import type { ContentReportReason } from '@/cloud/types';
import { WEB_BASE } from '@/features/settings/legal';
import { invoke, type IpcFailure } from '@/ipc';
import type { ContentDetail, ContentVersion } from '@/ipc/types/content';
import { formatCompact, formatDate, formatMoney, formatRelative, shortVersion } from '@/lib/format';
import { useActiveInstance, useInstances, useRusticSession } from '@/queries/core';
import { Markdown } from './Markdown';
import {
  ContentIcon,
  InstallButton,
  ItemCompatBadge,
  NoRusticServer,
  PriceBadge,
  ShaderFormatBadge,
  SideBadge,
  SourceBadge,
  TargetPicker,
  VerifiedBadge,
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
import {
  rusticId,
  useContentDetail,
  useContentLibrary,
  useLibraryMutations,
  useReportContent,
  useServers,
} from './useContent';

/** The website's page of a Rustic marketplace project. */
export function siteProjectUrl(d: Pick<ContentDetail, 'id' | 'kind' | 'native'>): string | null {
  const id = rusticId(d.id);
  if (!id) return null;
  const slug = d.native
    ? 'native-mods'
    : (
        {
          mod: 'mods',
          modpack: 'modpacks',
          resourcepack: 'resource-packs',
          shader: 'shaders',
          datapack: 'data-packs',
          plugin: 'mods',
        } as const
      )[d.kind];
  return `${WEB_BASE}/marketplace/${slug}/${encodeURIComponent(id)}`;
}

/** The version an install link asks for (`<project>@<version>`, with or without `rustic:`). */
export function pickVersion(
  versions: ContentVersion[],
  wanted: string | undefined,
): { version: ContentVersion | undefined; missing: boolean } {
  if (wanted) {
    const found = versions.find((v) => v.id === wanted || v.id === `rustic:${wanted}`);
    if (found) return { version: found, missing: false };
  }
  const best = versions.find((v) => v.compatibility.status === 'compatible') ?? versions[0];
  return { version: best, missing: !!wanted };
}

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

  if (detail.isError && !d) {
    const e = detail.error as unknown as IpcFailure;
    if (e.code === 'not_found') return <NotAvailable />;
    return (
      <Page>
        <QueryError onRetry={() => void detail.refetch()} message={e.message} />
      </Page>
    );
  }
  if (!d) return <DetailSkeleton />;

  const setTarget = (tr: TargetRef) =>
    void navigate({
      to: '/content/$projectId',
      params: { projectId },
      search: (prev) => ({ ...prev, target: formatTarget(tr) }),
      replace: true,
    });
  const dismissPrompt = () =>
    void navigate({
      to: '/content/$projectId',
      params: { projectId },
      search: (prev) => ({ ...prev, install: undefined, version: undefined }),
      replace: true,
    });

  return (
    <Page wide>
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
      <Hero
        detail={d}
        target={target}
        targetName={targetName}
        onTarget={setTarget}
        instancesLoading={instances.isLoading || servers.isLoading}
      />
      {search.install && (
        <InstallPrompt
          detail={d}
          wanted={search.version}
          target={target}
          targetName={targetName}
          onTarget={setTarget}
          onDismiss={dismissPrompt}
          instancesLoading={instances.isLoading || servers.isLoading}
        />
      )}
      <div className="grid grid-cols-[minmax(0,1fr)_300px] gap-6 max-[1100px]:grid-cols-1">
        <Tabs defaultValue="description" variant="underline">
          <TabsList aria-label={t('detail.sections')} className="flex-wrap">
            <TabsTrigger value="description">{t('detail.tabs.description')}</TabsTrigger>
            <TabsTrigger value="gallery" count={d.gallery.length}>
              {t('detail.tabs.gallery')}
            </TabsTrigger>
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
            {d.description_md ? (
              <Markdown source={d.description_md} />
            ) : (
              <p className="text-fg-muted text-sm">{d.summary}</p>
            )}
          </TabsContent>
          <TabsContent value="gallery">
            <Gallery detail={d} />
          </TabsContent>
          <TabsContent value="versions">
            <Versions detail={d} target={target} targetName={targetName} />
          </TabsContent>
          <TabsContent value="changelog">
            {d.changelog.length === 0 ? (
              <p className="text-fg-muted text-sm">{t('detail.noChangelog')}</p>
            ) : (
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
                      <p className="text-fg-muted text-sm whitespace-pre-line">{c.notes}</p>
                    </div>
                  </li>
                ))}
              </ol>
            )}
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

function NotAvailable() {
  const { t } = useTranslation('content');
  return (
    <Page>
      <EmptyState
        icon={<PackageX />}
        title={t('detail.notAvailable.title')}
        description={t('detail.notAvailable.body')}
        action={
          <Button asChild size="sm" variant="primary">
            <Link to="/content">{t('detail.notAvailable.back')}</Link>
          </Button>
        }
      />
    </Page>
  );
}

function Hero({
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
  const { t, i18n } = useTranslation('content');
  const instances = useInstances();
  const servers = useServers();
  const noServer =
    d.kind === 'plugin' &&
    !servers.isLoading &&
    eligibleServers('plugin', servers.data ?? []).length === 0;
  const banner = d.banner_url ?? d.gallery.find((g) => safeImageUrl(g.url))?.url ?? null;
  const paid = !!d.price && d.price.amount_minor > 0;
  return (
    <header className="rx-surface relative overflow-hidden rounded-2xl" data-testid="detail-hero">
      <CatalogCover src={banner} seed={d.id} className="h-44 lg:h-52">
        <span
          aria-hidden
          className="from-surface absolute inset-0 bg-gradient-to-t via-black/10 to-transparent"
        />
      </CatalogCover>
      <div className="relative -mt-16 flex flex-wrap items-end gap-5 px-6 pb-6">
        <ContentIcon
          id={d.id}
          name={d.name}
          src={d.icon_url}
          size="xl"
          className="ring-surface size-24 ring-4"
        />
        <div className="flex min-w-[min(100%,320px)] flex-1 flex-col gap-2">
          <h1 className="flex items-center gap-2 text-3xl font-bold tracking-tight">
            <span className="truncate">{d.name}</span>
            {d.verified && <VerifiedBadge compact />}
          </h1>
          <p className="text-fg-muted text-sm">
            {d.author ? t('by', { author: d.author }) : null}
            {d.license ? ` · ${d.license}` : null} · {t(`sourceLong.${d.source}`)}
          </p>
          <p className="text-fg max-w-2xl text-sm">{d.summary}</p>
          <div className="flex flex-wrap items-center gap-1.5 pt-1">
            <ItemCompatBadge item={d} />
            <SourceBadge source={d.source} />
            {d.verified && <VerifiedBadge />}
            <SideBadge side={d.side} />
            {d.shader_format && <ShaderFormatBadge format={d.shader_format} />}
            {d.kind === 'plugin' && (
              <Badge variant="accent">{t('plugins.badge', { api: '0.1' })}</Badge>
            )}
            {d.price && <PriceBadge price={d.price} />}
          </div>
        </div>
        <div className="flex w-full max-w-sm flex-col items-stretch gap-3 sm:w-auto">
          {paid && d.price ? (
            <div
              className="border-warning/30 bg-warning/10 flex flex-col gap-1 rounded-xl border px-4 py-3"
              data-testid="detail-price"
            >
              <span className="text-2xl font-bold tabular-nums">
                {formatMoney(d.price, i18n.language)}
              </span>
              <span className="text-fg-muted text-xs">{t('price.notOnSale')}</span>
            </div>
          ) : (
            d.kind !== 'modpack' &&
            !noServer && (
              <TargetPicker
                kind={d.kind}
                value={target}
                onChange={onTarget}
                instances={instances.data ?? []}
                servers={servers.data ?? []}
                loading={instancesLoading}
              />
            )
          )}
          <div className="flex flex-wrap items-center justify-end gap-2">
            <ReportButton detail={d} />
            <LibraryButton detail={d} />
            <InstallButton item={d} target={target} targetName={targetName} size="md" />
          </div>
        </div>
      </div>
      {noServer && (
        <div className="px-6 pb-6">
          <NoRusticServer />
        </div>
      )}
    </header>
  );
}

/** Opened from an install link: pick where and which version, then confirm (never automatic). */
function InstallPrompt({
  detail: d,
  wanted,
  target,
  targetName,
  onTarget,
  onDismiss,
  instancesLoading,
}: {
  detail: ContentDetail;
  wanted: string | undefined;
  target: TargetRef | null;
  targetName: string;
  onTarget: (t: TargetRef) => void;
  onDismiss: () => void;
  instancesLoading: boolean;
}) {
  const { t, i18n } = useTranslation('content');
  const instances = useInstances();
  const servers = useServers();
  const initial = pickVersion(d.versions, wanted);
  const [versionId, setVersionId] = useState<string | undefined>(initial.version?.id);
  const selected = d.versions.find((v) => v.id === versionId) ?? initial.version;
  const newest = d.versions[0];
  return (
    <section
      aria-labelledby="install-prompt-title"
      className="border-accent/50 bg-accent-soft/40 flex flex-col gap-4 rounded-2xl border p-5"
      data-testid="install-prompt"
    >
      <div className="flex items-start gap-3">
        <span className="bg-accent text-accent-fg grid size-10 shrink-0 place-items-center rounded-xl">
          <Download className="size-5" aria-hidden />
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <h2 id="install-prompt-title" className="text-base font-semibold">
            {t('prompt.title', { name: d.name })}
          </h2>
          <p className="text-fg-muted text-sm">{t('prompt.body')}</p>
        </div>
        <IconButton size="sm" label={t('prompt.dismiss')} icon={<X />} onClick={onDismiss} />
      </div>
      {initial.missing && (
        <p className="text-warning-text text-sm" role="status">
          {t('prompt.versionMissing', { version: wanted?.split('@').pop() ?? '' })}
        </p>
      )}
      <div className="flex flex-wrap items-center gap-3">
        {d.kind !== 'modpack' && (
          <TargetPicker
            kind={d.kind}
            value={target}
            onChange={onTarget}
            instances={instances.data ?? []}
            servers={servers.data ?? []}
            loading={instancesLoading}
          />
        )}
        {d.versions.length > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-fg-muted shrink-0 text-xs font-medium">
              {t('prompt.version')}
            </span>
            <Select
              aria-label={t('prompt.version')}
              value={selected?.id}
              onValueChange={setVersionId}
              className="w-60"
              options={d.versions.map((v) => ({
                value: v.id,
                label: v.version,
                description: [
                  t(`versions.channel.${v.channel}`),
                  formatDate(v.published_at, i18n.language),
                  v.compatibility.status === 'compatible'
                    ? t('card.compatible')
                    : v.compatibility.status === 'incompatible'
                      ? t('card.incompatible')
                      : null,
                ]
                  .filter(Boolean)
                  .join(' · '),
              }))}
            />
          </div>
        )}
        <div className="ml-auto">
          <InstallButton
            item={d}
            target={target}
            targetName={targetName}
            size="md"
            versionId={selected && selected.id !== newest?.id ? selected.id : undefined}
            compatibility={selected?.compatibility}
          />
        </div>
      </div>
    </section>
  );
}

function LibraryButton({ detail: d }: { detail: ContentDetail }) {
  const { t } = useTranslation('content');
  const session = useRusticSession();
  const id = rusticId(d.id);
  const signedIn = !!session.data?.signed_in;
  const eligible = signedIn && !!id && !(d.price && d.price.amount_minor > 0);
  const library = useContentLibrary(eligible);
  const m = useLibraryMutations();
  if (!eligible || !id) return null;
  const inLibrary = !!library.data?.some((e) => e.project.id === id);
  return (
    <Button
      size="md"
      variant={inLibrary ? 'soft' : 'secondary'}
      aria-pressed={inLibrary}
      leftIcon={inLibrary ? <BookmarkCheck /> : <BookmarkPlus />}
      loading={library.isLoading || m.add.isPending || m.remove.isPending}
      onClick={() =>
        inLibrary ? m.remove.mutate({ id, name: d.name }) : m.add.mutate({ id, name: d.name })
      }
      data-testid="library-toggle"
    >
      {inLibrary ? t('library.inLibrary') : t('library.add')}
    </Button>
  );
}

const REPORT_REASONS: Record<'fair_play' | 'malware' | 'copyright' | 'other', ContentReportReason> =
  {
    fair_play: 'cheating',
    malware: 'malware',
    copyright: 'copyright',
    other: 'other',
  };

function ReportButton({ detail: d }: { detail: ContentDetail }) {
  const { t } = useTranslation('content');
  const session = useRusticSession();
  const report = useReportContent();
  const id = rusticId(d.id);
  const reasons = ['fair_play', 'malware', 'copyright', 'other'] as const;
  const send = (r: (typeof reasons)[number]) => {
    // Rustic projects go to Rustic's moderators; the other sources keep their own report flows.
    if (id && session.data?.signed_in) {
      report.mutate({ id, reason: REPORT_REASONS[r] });
    } else {
      toast({ tone: 'success', title: t('report.sent'), description: t('report.sentBody') });
    }
  };
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size="md" variant="ghost" leftIcon={<Flag />} loading={report.isPending}>
          {t('report.button')}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-2">
        <p className="text-fg-muted px-2 pt-1 pb-2 text-xs font-semibold">
          {t('report.title', { name: d.name })}
        </p>
        <ul>
          {reasons.map((r) => (
            <li key={r}>
              <PopoverClose asChild>
                <button
                  type="button"
                  onClick={() => send(r)}
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
  if (d.gallery.length === 0) {
    return <p className="text-fg-muted text-sm">{t('detail.noGallery')}</p>;
  }
  return (
    <Carousel
      items={d.gallery}
      getKey={(g) => `${g.title}-${g.url ?? g.hue}`}
      label={t('detail.galleryLabel', { name: d.name })}
      prevLabel={t('detail.prev')}
      nextLabel={t('detail.next')}
      slideLabel={(i, n) => t('detail.slide', { index: i, total: n })}
      autoPlayMs={0}
      className="aspect-[16/8] w-full"
      renderItem={(g) => (
        <CatalogCover src={g.url} seed={`${d.id}-${g.hue}`} className="size-full">
          <span
            aria-hidden
            className="absolute inset-0 bg-gradient-to-t from-black/55 to-transparent"
          />
          <p className="absolute bottom-5 left-6 text-lg font-semibold text-white drop-shadow">
            {g.title}
          </p>
        </CatalogCover>
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
  if (d.versions.length === 0)
    return <p className="text-fg-muted text-sm">{t('versions.empty')}</p>;
  return (
    <div className="border-border overflow-x-auto rounded-xl border">
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
                <span className="flex items-center gap-1.5">
                  <span className="font-semibold">{v.version}</span>
                  <Badge
                    variant={
                      v.channel === 'release'
                        ? 'success'
                        : v.channel === 'beta'
                          ? 'warning'
                          : 'danger'
                    }
                  >
                    {t(`versions.channel.${v.channel}`)}
                  </Badge>
                </span>
                {v.size_bytes > 0 && (
                  <span className="text-2xs text-fg-subtle block">
                    {formatBytes(v.size_bytes, i18n.language)}
                  </span>
                )}
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
                <ItemCompatBadge item={{ native: d.native, compatibility: v.compatibility }} />
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
              ? c.needs_build
                ? t('compatTab.native')
                : c.builds_on_horizon
                  ? t('compatTab.builds')
                  : t('compatTab.format')
              : t('compatTab.unknown')}
        </p>
      </Card>
      {d.native && (
        <Card padding="md" className="flex flex-col gap-2" data-testid="native-explained">
          <h3 className="text-sm font-semibold">{t('native.title')}</h3>
          <p className="text-fg-muted text-sm">{t('native.body')}</p>
          <p className="text-fg-muted text-sm">{t('native.how')}</p>
        </Card>
      )}
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
      <span className="text-fg truncate font-medium tabular-nums">{value}</span>
    </div>
  );
  const links = Object.entries(d.links).filter(([, v]) => !!v) as [string, string][];
  const site = siteProjectUrl(d);
  const size = d.size_bytes ?? d.versions[0]?.size_bytes ?? 0;
  return (
    <aside className="flex flex-col gap-4" aria-label={t('detail.about')}>
      <Card padding="md" className="divide-border divide-y py-2">
        {d.author && row(<User />, t('stats.author'), d.author)}
        {row(<Download />, t('stats.downloads'), formatCompact(d.downloads, i18n.language))}
        {d.follows > 0 &&
          row(<Heart />, t('stats.follows'), formatCompact(d.follows, i18n.language))}
        {d.updated_at &&
          row(<CalendarClock />, t('stats.updated'), formatRelative(d.updated_at, i18n.language))}
        {d.created_at &&
          row(<Clock />, t('stats.created'), formatDate(d.created_at, i18n.language))}
        {d.license && row(<Scale />, t('stats.license'), d.license)}
        {size > 0 && row(<HardDrive />, t('stats.size'), formatBytes(size, i18n.language))}
      </Card>
      {d.categories.length > 0 && (
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
      )}
      {(links.length > 0 || site) && (
        <Card padding="md" className="flex flex-col gap-1">
          <h3 className="text-fg-subtle mb-1 text-xs font-semibold tracking-wider uppercase">
            {t('detail.links')}
          </h3>
          {site && (
            <LinkRow url={site} icon={<Globe className="size-4" />} label={t('detail.link.site')} />
          )}
          {links.map(([k, url]) => (
            <LinkRow
              key={k}
              url={url}
              icon={k === 'website' ? <Globe className="size-4" /> : <Link2 className="size-4" />}
              label={t(`detail.link.${k as 'source'}`, { defaultValue: k })}
            />
          ))}
        </Card>
      )}
    </aside>
  );
}

function LinkRow({ url, icon, label }: { url: string; icon: React.ReactNode; label: string }) {
  return (
    <button
      type="button"
      onClick={() => void invoke('app_open_url', { url })}
      className={cn(
        'rx-focus-ring text-accent-text flex items-center gap-2 rounded-md px-1 py-1.5 text-left text-sm hover:underline',
      )}
    >
      {icon}
      <span className="flex-1">{label}</span>
      <ExternalLink className="size-3.5 opacity-60" />
    </button>
  );
}

function DetailSkeleton() {
  return (
    <Page wide>
      <Skeleton className="h-4 w-48" />
      <div className="rx-surface overflow-hidden rounded-2xl" aria-busy="true">
        <Skeleton className="h-44 rounded-none" />
        <div className="flex gap-5 p-6">
          <Skeleton className="size-24 rounded-2xl" />
          <div className="flex flex-1 flex-col gap-3">
            <Skeleton className="h-8 w-64" />
            <Skeleton className="h-4 w-80" />
            <Skeleton className="h-4 w-full max-w-xl" />
          </div>
          <Skeleton className="h-9 w-40" />
        </div>
      </div>
      <div className="grid grid-cols-[minmax(0,1fr)_300px] gap-6">
        <Skeleton className="h-72 rounded-xl" />
        <Skeleton className="h-72 rounded-xl" />
      </div>
    </Page>
  );
}
