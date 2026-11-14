import {
  Badge,
  Button,
  Card,
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
  toast,
} from '@rustic/ui';
import { Link, useNavigate } from '@tanstack/react-router';
import {
  BookmarkCheck,
  BookmarkPlus,
  CalendarClock,
  Clock,
  Cpu,
  Download,
  ExternalLink,
  Flag,
  Globe,
  Hand,
  HardDrive,
  Heart,
  Link2,
  Network,
  PackageX,
  Scale,
  ShieldCheck,
  Timer,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CatalogCover,
  CatalogTabList,
  CatalogTabPanel,
  CatalogTabs,
  safeImageUrl,
} from '@/components/catalog';
import { CompatibilityBadge } from '@/components/CompatibilityBadge';
import { Page } from '@/components/Page';
import { QueryError } from '@/components/RouteError';
import type { ContentReportReason } from '@/cloud/types';
import { WEB_BASE } from '@/features/settings/legal';
import { invoke, type IpcFailure } from '@/ipc';
import type { ContentDetail, ContentVersion } from '@/ipc/types/content';
import { formatCompact, formatDate, formatMoney, formatRelative, shortVersion } from '@/lib/format';
import { useActiveInstance, useInstances, useRusticSession } from '@/queries/core';
import { GalleryGrid, GalleryStrip, Lightbox } from './ContentGallery';
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

type DetailTab =
  'description' | 'gallery' | 'versions' | 'changelog' | 'dependencies' | 'compatibility';

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
  const [tab, setTab] = useState<DetailTab>('description');
  const [picture, setPicture] = useState<number | null>(null);

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
  // Another project (a dependency's link): start again on its description.
  useEffect(() => {
    setTab('description');
    setPicture(null);
  }, [projectId]);

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
  const showTab = (v: DetailTab) => {
    setTab(v);
    document.getElementById('detail-sections')?.scrollIntoView({ block: 'start' });
  };

  return (
    <Page wide>
      <nav aria-label={t('detail.breadcrumb')} className="text-fg-subtle -mb-2 text-xs">
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
        onExplainNative={() => showTab('compatibility')}
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
      <div id="detail-sections" className="scroll-mt-4">
        <CatalogTabs value={tab} onValueChange={setTab}>
          <CatalogTabList
            label={t('detail.sections')}
            testIdPrefix="detail-tab"
            className="max-w-full overflow-x-auto"
            tabs={[
              { value: 'description', label: t('detail.tabs.description') },
              { value: 'gallery', label: t('detail.tabs.gallery'), count: d.gallery.length },
              { value: 'versions', label: t('detail.tabs.versions'), count: d.versions.length },
              { value: 'changelog', label: t('detail.tabs.changelog') },
              {
                value: 'dependencies',
                label: t('detail.tabs.dependencies'),
                count: d.dependencies.length,
              },
              { value: 'compatibility', label: t('detail.tabs.compatibility') },
            ]}
          />
          <div className="grid grid-cols-[minmax(0,1fr)_20rem] items-start gap-8 max-[1180px]:grid-cols-1">
            <div className="min-w-0">
              <CatalogTabPanel value="description">
                <GalleryStrip detail={d} onOpen={setPicture} onSeeAll={() => setTab('gallery')} />
                <article
                  className="max-w-[72ch] [&_h3]:mt-3 [&_h3]:text-base [&_li]:leading-7 [&_p]:text-[0.94rem] [&_p]:leading-7 [&_ul]:text-[0.94rem]"
                  data-testid="detail-description"
                >
                  {d.description_md ? (
                    <Markdown source={d.description_md} />
                  ) : (
                    <p className="text-fg-muted text-sm">{d.summary}</p>
                  )}
                </article>
              </CatalogTabPanel>
              <CatalogTabPanel value="gallery">
                <GalleryGrid detail={d} onOpen={setPicture} />
              </CatalogTabPanel>
              <CatalogTabPanel value="versions">
                <Versions detail={d} target={target} targetName={targetName} />
              </CatalogTabPanel>
              <CatalogTabPanel value="changelog">
                {d.changelog.length === 0 ? (
                  <p className="text-fg-muted text-sm">{t('detail.noChangelog')}</p>
                ) : (
                  <ol className="border-border ml-1 flex max-w-[72ch] flex-col gap-6 border-l pl-6">
                    {d.changelog.map((c) => (
                      <li key={c.version} className="relative">
                        <span
                          aria-hidden
                          className="bg-accent ring-bg absolute top-1.5 -left-[1.86rem] size-2.5 rounded-full ring-4"
                        />
                        <p className="text-sm font-semibold">
                          {c.version}{' '}
                          <span className="text-fg-subtle font-normal">
                            · <FormattedDate iso={c.published_at} />
                          </span>
                        </p>
                        <p className="text-fg-muted mt-1 text-sm leading-relaxed whitespace-pre-line">
                          {c.notes}
                        </p>
                      </li>
                    ))}
                  </ol>
                )}
              </CatalogTabPanel>
              <CatalogTabPanel value="dependencies">
                <Dependencies detail={d} target={target} />
              </CatalogTabPanel>
              <CatalogTabPanel value="compatibility">
                <CompatibilityExplained detail={d} targetName={targetName} />
              </CatalogTabPanel>
            </div>
            <Sidebar detail={d} />
          </div>
        </CatalogTabs>
      </div>
      <Lightbox detail={d} index={picture} onIndex={setPicture} onClose={() => setPicture(null)} />
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

/**
 * The project's header: a banner (its own banner picture cropped, else its first screenshot
 * blurred behind a gradient, else generated art; never stretched), the icon, name, author,
 * summary and badges, and on the right the install panel (where, the main action, library and
 * report; for a native mod, what building on the PC means).
 */
function Hero({
  detail: d,
  target,
  targetName,
  onTarget,
  instancesLoading,
  onExplainNative,
}: {
  detail: ContentDetail;
  target: TargetRef | null;
  targetName: string;
  onTarget: (t: TargetRef) => void;
  instancesLoading: boolean;
  onExplainNative: () => void;
}) {
  const { t, i18n } = useTranslation('content');
  const instances = useInstances();
  const servers = useServers();
  const noServer =
    d.kind === 'plugin' &&
    !servers.isLoading &&
    eligibleServers('plugin', servers.data ?? []).length === 0;
  const ownBanner = safeImageUrl(d.banner_url);
  const shot = d.gallery.find((g) => safeImageUrl(g.url))?.url ?? null;
  const paid = !!d.price && d.price.amount_minor > 0;
  return (
    <header className="rx-surface relative overflow-hidden rounded-2xl" data-testid="detail-hero">
      <CatalogCover
        src={ownBanner ?? shot}
        seed={d.id}
        wide
        className="h-36 lg:h-44"
        imgClassName="scale-110 blur-xl brightness-[0.72] saturate-[1.15]"
      >
        <span
          aria-hidden
          className="from-surface via-surface/55 absolute inset-0 bg-gradient-to-t to-transparent"
        />
        <span
          aria-hidden
          className="from-surface/60 absolute inset-0 bg-gradient-to-r via-transparent to-transparent"
        />
      </CatalogCover>
      <div className="relative -mt-20 grid items-start gap-6 px-6 pb-6 lg:grid-cols-[minmax(0,1fr)_21rem]">
        <div className="flex min-w-0 flex-col gap-4">
          <div className="flex min-w-0 flex-col gap-4 sm:flex-row sm:items-end sm:gap-5">
            <ContentIcon
              id={d.id}
              name={d.name}
              src={d.icon_url}
              size="xl"
              className="ring-surface size-24 shrink-0 shadow-lg ring-4"
            />
            <div className="flex min-w-0 flex-1 flex-col gap-2">
              <h1 className="flex items-center gap-2 text-3xl font-bold tracking-tight">
                <span className="truncate">{d.name}</span>
                {d.verified && <VerifiedBadge compact />}
              </h1>
              <p className="text-fg-muted flex flex-wrap items-center gap-x-1.5 text-sm">
                {d.author && (
                  <span className="text-fg font-medium">{t('by', { author: d.author })}</span>
                )}
                {d.license && (
                  <>
                    <span aria-hidden>·</span>
                    <span>{d.license}</span>
                  </>
                )}
                <span aria-hidden>·</span>
                <span>{t(`sourceLong.${d.source}`)}</span>
                <span aria-hidden>·</span>
                <span className="inline-flex items-center gap-1 tabular-nums">
                  <Download className="size-3.5" aria-hidden />
                  {formatCompact(d.downloads, i18n.language)}
                </span>
              </p>
              <p className="text-fg line-clamp-2 max-w-2xl text-sm leading-relaxed">{d.summary}</p>
              <div className="flex flex-wrap items-center gap-1.5 pt-0.5">
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
          </div>
          {d.native && <NativeNote onMore={onExplainNative} />}
        </div>

        <aside
          className="border-border bg-surface/90 flex flex-col gap-3 rounded-xl border p-4 shadow-lg backdrop-blur-md"
          aria-label={t('install.install')}
          data-testid="install-panel"
        >
          {paid && d.price ? (
            <div className="flex flex-col gap-0.5" data-testid="detail-price">
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
                stacked
              />
            )
          )}
          <InstallButton
            item={d}
            target={target}
            targetName={targetName}
            size="lg"
            className="w-full"
          />
          <div className="flex items-center justify-end gap-2 [&>button:not(:last-child)]:flex-1">
            <LibraryButton detail={d} />
            <ReportButton detail={d} compact />
          </div>
        </aside>
      </div>
      {noServer && (
        <div className="px-6 pb-6">
          <NoRusticServer />
        </div>
      )}
    </header>
  );
}

/** A native mod's note under the header: what building on the PC means, calmly (the trust notice
 * itself comes when installing). */
function NativeNote({ onMore }: { onMore: () => void }) {
  const { t } = useTranslation('content');
  return (
    <section
      aria-labelledby="native-note-title"
      className="border-info/25 bg-info/8 @container flex flex-col gap-2.5 rounded-xl border px-4 py-3"
      data-testid="native-note"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2
          id="native-note-title"
          className="text-info-text flex items-center gap-2 text-sm font-semibold"
        >
          <Cpu className="size-4" aria-hidden />
          {t('detail.native.title')}
        </h2>
        <button
          type="button"
          onClick={onMore}
          className="rx-focus-ring text-accent-text rounded text-xs font-medium hover:underline"
        >
          {t('detail.native.more')}
        </button>
      </div>
      <ul className="text-fg-muted grid gap-x-5 gap-y-1.5 text-xs leading-relaxed @2xl:grid-cols-3">
        {(
          [
            ['build', <Timer key="b" />],
            ['review', <ShieldCheck key="r" />],
            ['consent', <Hand key="c" />],
          ] as const
        ).map(([k, icon]) => (
          <li key={k} className="flex gap-2 [&_svg]:mt-0.5 [&_svg]:size-3.5 [&_svg]:shrink-0">
            <span aria-hidden className="text-info-text contents">
              {icon}
            </span>
            <span>{t(`detail.native.${k}`)}</span>
          </li>
        ))}
      </ul>
    </section>
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

function ReportButton({ detail: d, compact }: { detail: ContentDetail; compact?: boolean }) {
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
        {compact ? (
          <IconButton
            label={t('report.button')}
            icon={<Flag />}
            disabled={report.isPending}
            className="shrink-0"
          />
        ) : (
          <Button size="md" variant="ghost" leftIcon={<Flag />} loading={report.isPending}>
            {t('report.button')}
          </Button>
        )}
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
        <thead className="text-fg-subtle border-border border-b text-xs">
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
            <th scope="col" className="px-4 py-2.5 text-right font-medium">
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
        <tbody className="divide-border divide-y">
          {d.versions.map((v, i) => (
            <tr
              key={v.id}
              className={cn(
                'hover:bg-surface-2/50 transition-colors',
                i === 0 && 'bg-accent-soft/20',
              )}
            >
              <td className="px-4 py-3">
                <span className="flex flex-wrap items-center gap-1.5">
                  <span className="font-mono font-semibold">{v.version}</span>
                  {i === 0 && <Badge variant="accent">{t('versions.latest')}</Badge>}
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
                  <span className="text-2xs text-fg-subtle mt-0.5 block">
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
              <td className="text-fg-muted px-4 py-3 text-right text-xs tabular-nums">
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
  const lang = i18n.language;
  const links = Object.entries(d.links).filter(([, v]) => !!v) as [string, string][];
  const site = siteProjectUrl(d);
  const size = d.size_bytes ?? d.versions[0]?.size_bytes ?? 0;
  const stats: [React.ReactNode, string, React.ReactNode][] = [
    [<Download key="d" />, t('stats.downloads'), formatCompact(d.downloads, lang)],
  ];
  if (d.follows > 0)
    stats.push([<Heart key="f" />, t('stats.follows'), formatCompact(d.follows, lang)]);
  if (d.updated_at)
    stats.push([<CalendarClock key="u" />, t('stats.updated'), formatRelative(d.updated_at, lang)]);
  if (d.created_at)
    stats.push([<Clock key="c" />, t('stats.created'), formatDate(d.created_at, lang)]);
  if (d.license) stats.push([<Scale key="l" />, t('stats.license'), d.license]);
  if (size > 0) stats.push([<HardDrive key="s" />, t('stats.size'), formatBytes(size, lang)]);
  return (
    <aside
      className="flex flex-col gap-4 min-[1180px]:sticky min-[1180px]:top-4"
      aria-label={t('detail.about')}
    >
      <section className="rx-surface flex flex-col gap-3 rounded-2xl p-4">
        <h2 className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
          {t('detail.about')}
        </h2>
        {d.author && (
          <p className="flex items-center gap-2.5 text-sm">
            <ContentIcon id={`author-${d.author}`} name={d.author} size="sm" />
            <span className="flex min-w-0 flex-col">
              <span className="text-fg-subtle text-xs">{t('stats.author')}</span>
              <span className="text-fg truncate font-semibold">{d.author}</span>
            </span>
          </p>
        )}
        <dl className="grid grid-cols-2 gap-2">
          {stats.map(([icon, label, value]) => (
            <div
              key={label}
              className="bg-surface-2/60 flex min-w-0 flex-col gap-0.5 rounded-lg px-3 py-2"
            >
              <dt className="text-fg-subtle flex items-center gap-1.5 text-xs [&_svg]:size-3.5">
                <span aria-hidden className="contents">
                  {icon}
                </span>
                {label}
              </dt>
              <dd
                className="text-fg truncate text-sm font-semibold tabular-nums"
                title={String(value)}
              >
                {value}
              </dd>
            </div>
          ))}
        </dl>
      </section>
      {d.categories.length > 0 && (
        <section className="rx-surface flex flex-col gap-2 rounded-2xl p-4">
          <h2 className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
            {t('detail.categories')}
          </h2>
          <div className="flex flex-wrap gap-1.5">
            {d.categories.map((c) => (
              <Badge key={c} size="md">
                {t(`categories.${c as 'utility'}`, { defaultValue: c })}
              </Badge>
            ))}
          </div>
        </section>
      )}
      {(links.length > 0 || site) && (
        <section className="rx-surface flex flex-col gap-1 rounded-2xl p-4">
          <h2 className="text-fg-subtle mb-1 text-xs font-semibold tracking-wider uppercase">
            {t('detail.links')}
          </h2>
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
        </section>
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
        <Skeleton className="h-36 rounded-none lg:h-44" />
        <div className="grid gap-6 p-6 lg:grid-cols-[minmax(0,1fr)_21rem]">
          <div className="flex gap-5">
            <Skeleton className="size-24 rounded-2xl" />
            <div className="flex flex-1 flex-col gap-3">
              <Skeleton className="h-8 w-64" />
              <Skeleton className="h-4 w-80" />
              <Skeleton className="h-4 w-full max-w-xl" />
            </div>
          </div>
          <Skeleton className="h-32 rounded-xl" />
        </div>
      </div>
      <Skeleton className="h-11 w-[32rem] max-w-full rounded-xl" />
      <div className="grid grid-cols-[minmax(0,1fr)_20rem] gap-8 max-[1180px]:grid-cols-1">
        <Skeleton className="h-72 rounded-xl" />
        <Skeleton className="h-72 rounded-2xl" />
      </div>
    </Page>
  );
}
