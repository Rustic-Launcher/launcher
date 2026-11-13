import { Badge, Button, Dialog, IconButton, Skeleton, Switch, Tooltip } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import {
  ArrowUpCircle,
  BookmarkMinus,
  Download,
  ExternalLink,
  Eye,
  Library,
  LogIn,
  Package,
  PenSquare,
  Server as ServerIcon,
  Trash2,
  UploadCloud,
  XCircle,
} from 'lucide-react';
import { useMemo, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CatalogEmpty,
  CatalogError,
  CatalogListRow,
  ListSkeletonRows,
  MetaItem,
} from '@/components/catalog';
import type { MarketplaceKind, MarketplaceProject } from '@/cloud/types';
import { useContentMutations } from '@/features/instances/queries';
import { WEB_BASE } from '@/features/settings/legal';
import { invoke } from '@/ipc';
import type { ContentKind } from '@/ipc/types/content';
import type { InstalledContent } from '@/ipc/types/instances';
import { formatCompact, formatRelative } from '@/lib/format';
import { qk, useRusticSession } from '@/queries/core';
import type { ExploreContext } from './ContentPage';
import { ContentIcon, PriceBadge, VerifiedBadge } from './parts';
import { formatTarget } from './target';
import { useContentLibrary, useLibraryMutations, useMyContent } from './useContent';

/** The website's pages for creators (opened in the browser). */
export const PUBLISH_URL = `${WEB_BASE}/marketplace/publish`;
export const editUrl = (id: string) => `${WEB_BASE}/marketplace/edit/${encodeURIComponent(id)}`;

/** The launcher's kind for an API kind (native mods are mods, both shader formats shaders). */
export function uiKind(k: MarketplaceKind): ContentKind {
  if (k === 'native_mod') return 'mod';
  if (k === 'shaderpack') return 'shader';
  return k;
}

const matches = (q: string, ...texts: string[]) => {
  const needle = q.trim().toLowerCase();
  return !needle || texts.some((x) => x.toLowerCase().includes(needle));
};

/** Mine: what is installed on the target, the Rustic library, and the account's own creations. */
export function MineTab({ ctx }: { ctx: ExploreContext }) {
  const session = useRusticSession();
  const signedIn = !!session.data?.signed_in;
  return (
    <div className="flex flex-col gap-8">
      <InstalledSection ctx={ctx} />
      {session.isLoading ? (
        <Skeleton className="h-40 rounded-xl" />
      ) : signedIn ? (
        <>
          <LibrarySection ctx={ctx} />
          <CreationsSection ctx={ctx} />
        </>
      ) : (
        <SignInCard />
      )}
    </div>
  );
}

function Section({
  title,
  hint,
  action,
  children,
  testId,
}: {
  title: string;
  hint?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  testId?: string;
}) {
  return (
    <section className="flex flex-col gap-3" data-testid={testId} aria-label={title}>
      <div className="flex flex-wrap items-end justify-between gap-3 px-1">
        <div className="flex flex-col gap-0.5">
          <h2 className="text-base font-semibold">{title}</h2>
          {hint && <p className="text-fg-subtle text-xs">{hint}</p>}
        </div>
        {action}
      </div>
      {children}
    </section>
  );
}

function InstalledSection({ ctx }: { ctx: ExploreContext }) {
  const { t } = useTranslation('content');
  const { kind, target } = ctx;
  if (kind === 'modpack') {
    return (
      <Section title={t('mine.installed.title')} testId="mine-installed">
        <CatalogEmpty
          icon={<Package />}
          title={t('mine.installed.modpacksTitle')}
          description={t('mine.installed.modpacksHint')}
          action={
            <Button asChild size="sm">
              <Link to="/instances">{t('mine.installed.openProfiles')}</Link>
            </Button>
          }
        />
      </Section>
    );
  }
  if (!target) {
    return (
      <Section title={t('mine.installed.title')} testId="mine-installed">
        <CatalogEmpty icon={<Package />} title={t('mine.installed.noTarget')} />
      </Section>
    );
  }
  if (target.kind === 'server') {
    return (
      <Section
        title={t('mine.installed.titleOn', { target: ctx.targetName })}
        testId="mine-installed"
      >
        <div className="rx-surface flex flex-wrap items-center gap-3 rounded-xl p-4 text-sm">
          <ServerIcon className="text-fg-muted size-5" aria-hidden />
          <p className="text-fg-muted flex-1">{t('mine.installed.serverHint')}</p>
          <Button asChild size="sm" variant="soft">
            <Link
              to="/servers/$serverId"
              params={{ serverId: target.id }}
              search={{ tab: kind === 'plugin' ? 'plugins' : 'datapacks' }}
            >
              {t('mine.installed.manageServer')}
            </Link>
          </Button>
        </div>
      </Section>
    );
  }
  return <InstanceContent ctx={ctx} instanceId={target.id} />;
}

function InstanceContent({ ctx, instanceId }: { ctx: ExploreContext; instanceId: string }) {
  const { t } = useTranslation('content');
  const { t: tc } = useTranslation();
  const content = useQuery({
    queryKey: qk.instanceContent(instanceId, ctx.kind),
    queryFn: () => invoke('instance_content_list', { instance_id: instanceId, kind: ctx.kind }),
  });
  const m = useContentMutations(instanceId);
  const [removing, setRemoving] = useState<InstalledContent | null>(null);
  const list = useMemo(
    () =>
      (content.data ?? [])
        .filter((c) => c.kind === ctx.kind)
        .filter((c) => matches(ctx.query, c.name, c.filename))
        .sort((a, b) => a.name.localeCompare(b.name)),
    [content.data, ctx.kind, ctx.query],
  );
  const updates = list.filter((c) => c.update_available);
  return (
    <Section
      title={t('mine.installed.titleOn', { target: ctx.targetName })}
      hint={t('mine.installed.hint', { kind: t(`kinds.${ctx.kind}`).toLowerCase() })}
      testId="mine-installed"
      action={
        <div className="flex items-center gap-2">
          {updates.length > 0 && (
            <Button
              size="sm"
              leftIcon={<ArrowUpCircle />}
              loading={m.update.isPending}
              onClick={() => m.update.mutate(undefined)}
            >
              {t('mine.installed.updateAll', { count: updates.length })}
            </Button>
          )}
          <Button asChild size="sm" variant="ghost">
            <Link to="/instances/$instanceId" params={{ instanceId }} search={{ tab: 'content' }}>
              {t('mine.installed.openProfile')}
            </Link>
          </Button>
        </div>
      }
    >
      {content.isError ? (
        <CatalogError onRetry={() => void content.refetch()} />
      ) : content.isLoading ? (
        <ListSkeletonRows rows={3} />
      ) : list.length === 0 ? (
        <CatalogEmpty
          icon={<Package />}
          title={ctx.query ? t('mine.noMatch') : t('mine.installed.empty')}
          description={ctx.query ? undefined : t('mine.installed.emptyHint')}
          action={
            ctx.query ? undefined : (
              <Button
                size="sm"
                variant="primary"
                onClick={() => ctx.setSearch({ tab: 'discover' })}
              >
                {t('mine.installed.discover')}
              </Button>
            )
          }
        />
      ) : (
        <ul className="rx-surface divide-border divide-y overflow-hidden rounded-xl">
          {list.map((c) => (
            <li key={c.id}>
              <CatalogListRow
                testId={`mine-row-${c.id}`}
                className="border-b-0"
                thumb={
                  <ContentIcon id={c.project_id ?? c.id} name={c.name} src={c.icon_url} size="md" />
                }
                title={
                  c.project_id && c.source !== 'local' ? (
                    <Link
                      to="/content/$projectId"
                      params={{ projectId: c.project_id }}
                      search={{ target: formatTarget({ kind: 'instance', id: instanceId }) }}
                      className={`rx-focus-ring hover:text-accent-text truncate rounded text-sm font-semibold ${c.enabled ? '' : 'text-fg-subtle line-through'}`}
                    >
                      {c.name}
                    </Link>
                  ) : (
                    <span
                      className={`truncate text-sm font-semibold ${c.enabled ? '' : 'text-fg-subtle line-through'}`}
                    >
                      {c.name}
                    </span>
                  )
                }
                byline={<span className="tabular-nums">{c.version}</span>}
                badges={
                  <>
                    <Badge variant="neutral">{t(`mine.source.${c.source}`)}</Badge>
                    {c.update_available && (
                      <Badge variant="info" icon={<ArrowUpCircle />}>
                        {t('mine.installed.updateTo', { version: c.update_available })}
                      </Badge>
                    )}
                    {!c.compatible && (
                      <Tooltip content={c.incompatibility ?? ''}>
                        <Badge variant="danger" icon={<XCircle />} tabIndex={0}>
                          {t('mine.installed.incompatible')}
                        </Badge>
                      </Tooltip>
                    )}
                  </>
                }
                summary={<span className="font-mono text-xs">{c.filename}</span>}
                actions={
                  <>
                    {c.update_available && (
                      <IconButton
                        size="sm"
                        label={t('mine.installed.update', { name: c.name })}
                        icon={<ArrowUpCircle />}
                        onClick={() => m.update.mutate(c.id)}
                      />
                    )}
                    <Switch
                      checked={c.enabled}
                      disabled={!c.compatible && !c.enabled}
                      onCheckedChange={(v) => m.setEnabled.mutate({ content_id: c.id, enabled: v })}
                      aria-label={t('mine.installed.toggle', { name: c.name })}
                    />
                    <IconButton
                      size="sm"
                      variant="danger"
                      label={t('mine.installed.remove', { name: c.name })}
                      icon={<Trash2 />}
                      onClick={() => setRemoving(c)}
                    />
                  </>
                }
              />
            </li>
          ))}
        </ul>
      )}
      <Dialog
        open={!!removing}
        onOpenChange={(o) => !o && setRemoving(null)}
        tone="destructive"
        icon={<Trash2 />}
        title={t('mine.installed.removeTitle', { name: removing?.name ?? '' })}
        description={t('mine.installed.removeBody')}
        closeLabel={tc('actions.close')}
        footer={
          <>
            <Button onClick={() => setRemoving(null)}>{tc('actions.cancel')}</Button>
            <Button
              variant="danger"
              loading={m.remove.isPending}
              onClick={() =>
                removing && m.remove.mutate(removing.id, { onSuccess: () => setRemoving(null) })
              }
            >
              {tc('actions.remove')}
            </Button>
          </>
        }
      />
    </Section>
  );
}

function SignInCard() {
  const { t } = useTranslation('content');
  return (
    <section
      className="rx-surface flex flex-wrap items-center gap-4 rounded-xl p-5"
      data-testid="mine-signin"
    >
      <span className="bg-accent-soft text-accent-text grid size-11 place-items-center rounded-xl">
        <Library className="size-5" aria-hidden />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <h2 className="text-base font-semibold">{t('mine.signIn.title')}</h2>
        <p className="text-fg-muted text-sm">{t('mine.signIn.body')}</p>
      </div>
      <Button asChild variant="primary" leftIcon={<LogIn />}>
        <Link to="/settings" search={{ section: 'accounts' }}>
          {t('mine.signIn.action')}
        </Link>
      </Button>
    </section>
  );
}

function ProjectRow({
  p,
  testId,
  ctx,
  status,
  actions,
}: {
  p: MarketplaceProject;
  testId: string;
  ctx: ExploreContext;
  status?: ReactNode;
  actions: ReactNode;
}) {
  const { t, i18n } = useTranslation('content');
  const listed = p.visibility !== 'private' && p.review !== 'pending' && !p.taken_down;
  const title = listed ? (
    <Link
      to="/content/$projectId"
      params={{ projectId: `rustic:${p.id}` }}
      search={{ target: ctx.target ? formatTarget(ctx.target) : undefined }}
      className="rx-focus-ring text-fg hover:text-accent-text truncate rounded text-sm font-semibold"
    >
      {p.name}
    </Link>
  ) : (
    <span className="truncate text-sm font-semibold">{p.name}</span>
  );
  return (
    <CatalogListRow
      testId={testId}
      className="border-b-0"
      thumb={<ContentIcon id={`rustic:${p.id}`} name={p.name} src={p.icon_url} size="md" />}
      title={title}
      byline={p.author ? t('by', { author: p.author }) : undefined}
      badges={
        <>
          {p.verified && <VerifiedBadge compact />}
          <Badge variant="neutral">
            {p.kind === 'native_mod' ? t('native.kind') : t(`kinds.${uiKind(p.kind)}`)}
          </Badge>
          {p.price && <PriceBadge price={p.price} />}
          {status}
        </>
      }
      summary={p.summary}
      meta={
        <>
          <MetaItem icon={<Download />} label={t('stats.downloads')}>
            {formatCompact(p.downloads, i18n.language)}
          </MetaItem>
          {p.updated_at && (
            <MetaItem label={t('stats.updated')}>
              {formatRelative(p.updated_at, i18n.language)}
            </MetaItem>
          )}
          {p.latest_version && <MetaItem label={t('stats.version')}>v{p.latest_version}</MetaItem>}
        </>
      }
      actions={actions}
    />
  );
}

function LibrarySection({ ctx }: { ctx: ExploreContext }) {
  const { t } = useTranslation('content');
  const library = useContentLibrary(true);
  const m = useLibraryMutations();
  const list = (library.data ?? []).filter((e) =>
    matches(ctx.query, e.project.name, e.project.summary, e.project.author),
  );
  return (
    <Section title={t('mine.library.title')} hint={t('mine.library.hint')} testId="mine-library">
      {library.isError ? (
        <CatalogError
          onRetry={() => void library.refetch()}
          message={(library.error as Error).message}
        />
      ) : library.isLoading ? (
        <ListSkeletonRows rows={2} />
      ) : list.length === 0 ? (
        <CatalogEmpty
          icon={<Library />}
          title={ctx.query ? t('mine.noMatch') : t('mine.library.empty')}
          description={ctx.query ? undefined : t('mine.library.emptyHint')}
        />
      ) : (
        <ul className="rx-surface divide-border divide-y overflow-hidden rounded-xl">
          {list.map(({ project: p }) => (
            <li key={p.id}>
              <ProjectRow
                p={p}
                ctx={ctx}
                testId={`library-row-${p.id}`}
                actions={
                  <>
                    <IconButton
                      size="sm"
                      label={t('library.remove', { name: p.name })}
                      icon={<BookmarkMinus />}
                      onClick={() => m.remove.mutate({ id: p.id, name: p.name })}
                    />
                    <Button asChild size="sm" variant="ghost">
                      <Link
                        to="/content/$projectId"
                        params={{ projectId: `rustic:${p.id}` }}
                        search={{ target: ctx.target ? formatTarget(ctx.target) : undefined }}
                      >
                        <Eye />
                        {t('view')}
                      </Link>
                    </Button>
                    <Button asChild size="sm" variant="primary">
                      <Link
                        to="/content/$projectId"
                        params={{ projectId: `rustic:${p.id}` }}
                        search={{
                          target: ctx.target ? formatTarget(ctx.target) : undefined,
                          install: true,
                        }}
                        aria-label={t('mine.library.installAria', { name: p.name })}
                      >
                        <Download />
                        {t('install.install')}
                      </Link>
                    </Button>
                  </>
                }
              />
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}

function CreationsSection({ ctx }: { ctx: ExploreContext }) {
  const { t } = useTranslation('content');
  const mine = useMyContent(true);
  const list = (mine.data ?? []).filter((p) => matches(ctx.query, p.name, p.summary));
  const publish = (
    <Button
      size="sm"
      variant="soft"
      leftIcon={<UploadCloud />}
      rightIcon={<ExternalLink />}
      onClick={() => void invoke('app_open_url', { url: PUBLISH_URL })}
      data-testid="publish-on-site"
    >
      {t('mine.creations.publish')}
    </Button>
  );
  return (
    <Section
      title={t('mine.creations.title')}
      hint={t('mine.creations.hint')}
      action={publish}
      testId="mine-creations"
    >
      {mine.isError ? (
        <CatalogError onRetry={() => void mine.refetch()} message={(mine.error as Error).message} />
      ) : mine.isLoading ? (
        <ListSkeletonRows rows={2} />
      ) : list.length === 0 ? (
        <CatalogEmpty
          icon={<UploadCloud />}
          title={ctx.query ? t('mine.noMatch') : t('mine.creations.empty')}
          description={ctx.query ? undefined : t('mine.creations.emptyHint')}
        />
      ) : (
        <ul className="rx-surface divide-border divide-y overflow-hidden rounded-xl">
          {list.map((p) => (
            <li key={p.id}>
              <ProjectRow
                p={p}
                ctx={ctx}
                testId={`creation-row-${p.id}`}
                status={<CreationStatus p={p} />}
                actions={
                  <Button
                    size="sm"
                    variant="secondary"
                    leftIcon={<PenSquare />}
                    rightIcon={<ExternalLink />}
                    onClick={() => void invoke('app_open_url', { url: editUrl(p.id) })}
                    aria-label={t('mine.creations.manageAria', { name: p.name })}
                  >
                    {t('mine.creations.manage')}
                  </Button>
                }
              />
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}

function CreationStatus({ p }: { p: MarketplaceProject }) {
  const { t } = useTranslation('content');
  if (p.taken_down) return <Badge variant="danger">{t('mine.creations.status.takenDown')}</Badge>;
  if (p.review === 'pending')
    return <Badge variant="warning">{t('mine.creations.status.pending')}</Badge>;
  if (p.review === 'rejected')
    return <Badge variant="danger">{t('mine.creations.status.rejected')}</Badge>;
  return (
    <Badge variant={p.visibility === 'public' ? 'success' : 'neutral'}>
      {t(`mine.creations.status.${p.visibility}`)}
    </Badge>
  );
}
