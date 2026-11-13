/**
 * The Profiles page's presentations over the catalog components (ADR 0159): the visual card, the
 * dense list row, the "Jump back in" card, and the create/import tile.
 */
import { Badge, Button, cn, Tooltip } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import {
  Clock,
  Cpu,
  Download,
  FolderInput,
  Layers,
  LogIn,
  Package,
  Pin,
  Play,
  Plus,
  Sparkles,
  Square,
  X,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  CardRowItem,
  CatalogListRow,
  Chip,
  MetaItem,
  OverlayBadge,
  VisualCard,
} from '@/components/catalog';
import { HorizonVersion, isRusticEdition } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import { taskFraction, useNativeTask } from '@/features/native/useNative';
import { usePlay, type PlayModel } from '@/features/play/usePlay';
import type { Instance } from '@/ipc/types/instances';
import type { NativeOverview } from '@/ipc/types/native';
import {
  formatDate,
  formatDuration,
  formatPercent,
  formatRelative,
  shortVersion,
} from '@/lib/format';
import { useHorizonVersions } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { InstanceActions } from './InstanceActions';
import { InstanceCover } from './InstanceCover';
import { nativeLineKey, nativeLineTone } from './logic';
import { InstancePlayControl } from './InstancePlayControl';

type Tone = 'neutral' | 'accent' | 'success' | 'info' | 'warning';

/** The native-mods line of a profile: its tone, short label and explanation, or null. */
export function useNativeLine(
  instance: Instance,
  overview: NativeOverview | undefined,
): { tone: Tone; label: string; hint: string } | null {
  const { t, i18n } = useTranslation('instances');
  const task = useNativeTask(instance.id);
  const running = !!task && (task.state === 'running' || task.state === 'queued');
  const key = nativeLineKey(overview, running);
  if (!key) return null;
  if (key === 'building') {
    const f = running ? taskFraction(task) : null;
    return {
      tone: 'accent',
      label:
        f != null
          ? t('profiles.native.building', { pct: formatPercent(f, i18n.language) })
          : t('profiles.native.buildingNoPct'),
      hint: t('profiles.native.hint.building'),
    };
  }
  const hint = t(`profiles.native.hint.${key}`);
  return {
    tone: nativeLineTone(key),
    label: t(`profiles.native.${key}`),
    hint: overview ? `${t('profiles.native.count', { count: overview.mods })} · ${hint}` : hint,
  };
}

/** The state badges of a profile, in reading order (active, running, update, native, pinned). */
function useBadges(
  instance: Instance,
  play: PlayModel,
  active: boolean,
  overview?: NativeOverview,
) {
  const { t } = useTranslation('instances');
  const native = useNativeLine(instance, overview);
  const out: { key: string; tone: Tone; label: string; icon?: ReactNode; hint?: string }[] = [];
  if (active) out.push({ key: 'active', tone: 'accent', label: t('profiles.card.active') });
  if (play.state === 'running')
    out.push({ key: 'running', tone: 'success', label: t('profiles.card.running') });
  if (instance.state === 'broken')
    out.push({ key: 'broken', tone: 'warning', label: t('profiles.card.broken') });
  else if (instance.update_available && instance.state === 'needs_update')
    out.push({
      key: 'update',
      tone: 'warning',
      icon: <Download />,
      label: t('profiles.card.update', { version: shortVersion(instance.update_available) }),
    });
  if (native)
    out.push({
      key: 'native',
      tone: native.tone,
      icon: <Cpu />,
      label: native.label,
      hint: native.hint,
    });
  if (instance.pinned)
    out.push({ key: 'pinned', tone: 'neutral', icon: <Pin />, label: t('profiles.card.pinned') });
  return out;
}

function TitleLink({ instance, className }: { instance: Instance; className: string }) {
  return (
    <Link
      to="/instances/$instanceId"
      params={{ instanceId: instance.id }}
      search={{}}
      className={className}
    >
      {instance.name}
    </Link>
  );
}

function PlayedText({ instance }: { instance: Instance }) {
  const { t, i18n } = useTranslation('instances');
  return (
    <>
      {instance.last_played_at
        ? t('profiles.card.played', {
            when: formatRelative(instance.last_played_at, i18n.language),
          })
        : t('profiles.card.neverPlayed')}
    </>
  );
}

/** Mods, packs and shaders, then play time: the meta line of cards and rows. */
function ContentMeta({ instance, playtime = true }: { instance: Instance; playtime?: boolean }) {
  const { t, i18n } = useTranslation('instances');
  return (
    <>
      <MetaItem icon={<Package />} label={t('profiles.card.mods')}>
        {instance.mod_count}
      </MetaItem>
      <MetaItem icon={<Layers />} label={t('profiles.card.packs')}>
        {instance.resource_pack_count}
      </MetaItem>
      <MetaItem icon={<Sparkles />} label={t('profiles.card.shaders')}>
        {instance.shader_count}
      </MetaItem>
      {playtime && instance.playtime_s > 0 && (
        <MetaItem icon={<Clock />} label={t('profiles.card.playtime')}>
          {formatDuration(instance.playtime_s, i18n.language)}
        </MetaItem>
      )}
    </>
  );
}

/** The version stamp along a cover's bottom edge, or the launch's progress while it runs. */
function CoverFooter({ instance, play }: { instance: Instance; play: PlayModel }) {
  const { t } = useTranslation();
  const { data: versions } = useHorizonVersions();
  if (play.state === 'progress') {
    return (
      <div className="flex flex-col gap-1 bg-gradient-to-t from-black/85 via-black/60 to-transparent px-3 pt-8 pb-9 text-white">
        <span className="truncate text-xs font-semibold tabular-nums drop-shadow">
          {play.progressLabel}
        </span>
        <span className="block h-1 overflow-hidden rounded-full bg-white/20">
          <span
            className={cn(
              'bg-accent block h-full rounded-full transition-[width] duration-300',
              play.progress == null && 'w-1/3 animate-pulse',
            )}
            style={play.progress != null ? { width: `${play.progress * 100}%` } : undefined}
          />
        </span>
      </div>
    );
  }
  const rustic = isRusticEdition(instance.horizon_version, versions);
  return (
    <div
      className="flex items-center justify-end gap-1.5 px-2.5 pb-2.5"
      data-testid={`version-${instance.id}`}
    >
      <OverlayBadge>
        {t('horizon.version', { version: shortVersion(instance.horizon_version) })}
      </OverlayBadge>
      {rustic && <OverlayBadge tone="accent">{t('horizon.rusticEdition')}</OverlayBadge>}
    </div>
  );
}

/** The big round Play over a cover: shown on hover or focus, always while a game runs or loads. */
function CoverPlay({ instance, play }: { instance: Instance; play: PlayModel }) {
  const { t } = useTranslation();
  const setActive = usePrefs((s) => s.setActiveInstance);
  const map = {
    play: { label: t('play.play'), icon: <Play className="translate-x-px fill-current" /> },
    update: { label: t('play.update'), icon: <Download /> },
    running: { label: t('play.stop'), icon: <Square className="fill-current" /> },
    progress: { label: t('play.cancel'), icon: <X /> },
    disabled: { label: t('play.play'), icon: <Play /> },
  } as const;
  const m = map[play.state];
  const busy = play.state === 'running' || play.state === 'progress';
  return (
    <button
      type="button"
      aria-label={`${m.label} — ${instance.name}`}
      title={m.label}
      data-testid={`play-${instance.id}`}
      data-state={play.state}
      disabled={play.state === 'disabled'}
      onClick={() => {
        setActive(instance.id);
        play.primary();
      }}
      className={cn(
        'rx-focus-ring grid size-14 place-items-center rounded-full shadow-[0_10px_30px_rgba(0,0,0,0.45)] transition-[opacity,transform,background-color] duration-200 ease-out [&_svg]:size-6',
        'disabled:opacity-40',
        busy
          ? 'border border-white/20 bg-black/60 text-white backdrop-blur-sm hover:bg-black/75'
          : 'bg-accent text-accent-fg hover:bg-accent-hover',
        !busy &&
          'scale-90 opacity-0 group-focus-within/card:scale-100 group-focus-within/card:opacity-100 group-hover/card:scale-100 group-hover/card:opacity-100 focus-visible:scale-100 focus-visible:opacity-100 motion-reduce:scale-100',
      )}
    >
      {m.icon}
    </button>
  );
}

/** The grid's visual card. */
export function ProfileCard({
  instance,
  active,
  overview,
}: {
  instance: Instance;
  active: boolean;
  overview?: NativeOverview;
}) {
  const play = usePlay(instance);
  const badges = useBadges(instance, play, active, overview);
  return (
    <VisualCard
      testId={`instance-card-${instance.id}`}
      seed={instance.id}
      title={instance.name}
      highlighted={active}
      coverArt={<InstanceCover instance={instance} />}
      thumb={
        <InstanceIcon
          icon={instance.icon}
          size="md"
          className="ring-surface relative -mt-7 mb-0.5 ring-4"
        />
      }
      renderTitle={(className) => <TitleLink instance={instance} className={className} />}
      byline={
        instance.source ? (
          <span className="flex items-center gap-1">
            <Package className="size-3 shrink-0" aria-hidden />
            <span className="truncate">
              {instance.source.name} {instance.source.version}
            </span>
          </span>
        ) : (
          <PlayedText instance={instance} />
        )
      }
      overlay={
        badges.length > 0 ? (
          <>
            {badges.map((b) => (
              <span key={b.key} title={b.hint} data-testid={`badge-${b.key}-${instance.id}`}>
                <OverlayBadge tone={b.tone} icon={b.icon}>
                  {b.label}
                </OverlayBadge>
              </span>
            ))}
          </>
        ) : undefined
      }
      coverFooter={<CoverFooter instance={instance} play={play} />}
      coverAction={<CoverPlay instance={instance} play={play} />}
      corner={
        <div className="opacity-0 transition-opacity group-focus-within/card:opacity-100 group-hover/card:opacity-100 has-[[data-state=open]]:opacity-100">
          <InstanceActions
            instance={instance}
            afterDelete={() => undefined}
            triggerClassName="size-8 bg-black/50 text-white backdrop-blur-sm hover:bg-black/70 hover:text-white data-[state=open]:bg-black/70"
          />
        </div>
      }
      meta={<ContentMeta instance={instance} />}
    />
  );
}

/** The list view's dense row. */
export function ProfileRow({
  instance,
  active,
  overview,
}: {
  instance: Instance;
  active: boolean;
  overview?: NativeOverview;
}) {
  const { t, i18n } = useTranslation('instances');
  const play = usePlay(instance);
  const badges = useBadges(instance, play, active, overview);
  const variant = (tone: Tone) => (tone === 'neutral' ? 'neutral' : tone);
  return (
    <CatalogListRow
      testId={`instance-row-${instance.id}`}
      className={cn(active && 'bg-accent-soft/40 hover:bg-accent-soft/60')}
      thumb={
        <span className="relative size-16 shrink-0 overflow-hidden rounded-xl">
          <InstanceCover instance={instance} />
          <InstanceIcon
            icon={instance.icon}
            size="sm"
            className="ring-surface absolute right-1 bottom-1 ring-2"
          />
        </span>
      }
      title={
        <TitleLink
          instance={instance}
          className="rx-focus-ring text-fg hover:text-accent-text truncate rounded text-base font-semibold"
        />
      }
      byline={<HorizonVersion id={instance.horizon_version} />}
      badges={
        <>
          {badges.map((b) => (
            <Badge
              key={b.key}
              variant={variant(b.tone)}
              icon={b.icon}
              title={b.hint}
              data-testid={`badge-${b.key}-${instance.id}`}
            >
              {b.label}
            </Badge>
          ))}
        </>
      }
      summary={
        instance.source
          ? t('card.fromModpack', { name: instance.source.name, version: instance.source.version })
          : instance.last_server
            ? t('profiles.card.lastServer', { name: instance.last_server.name })
            : undefined
      }
      chips={
        <>
          <Chip>
            {t('profiles.card.contentCount', {
              mods: instance.mod_count,
              packs: instance.resource_pack_count,
              shaders: instance.shader_count,
            })}
          </Chip>
        </>
      }
      meta={
        <>
          <span>
            <PlayedText instance={instance} />
          </span>
          {instance.playtime_s > 0 && (
            <MetaItem icon={<Clock />} label={t('profiles.card.playtime')}>
              {formatDuration(instance.playtime_s, i18n.language)}
            </MetaItem>
          )}
          <span>
            {t('profiles.card.created', { date: formatDate(instance.created_at, i18n.language) })}
          </span>
        </>
      }
      actions={
        <>
          <InstancePlayControl instance={instance} />
          <InstanceActions instance={instance} afterDelete={() => undefined} />
        </>
      }
    />
  );
}

/** "Jump back in": a wide card with Play and, when there is one, Rejoin the last server. */
export function RecentCard({ instance, active }: { instance: Instance; active: boolean }) {
  const { t } = useTranslation('instances');
  const play = usePlay(instance);
  const setActive = usePrefs((s) => s.setActiveInstance);
  const server = instance.last_server;
  const busy = play.state === 'progress' || play.state === 'running';
  return (
    <CardRowItem width="lg">
      <article
        className={cn(
          'group/card relative isolate flex h-36 flex-col justify-between overflow-hidden rounded-xl p-3.5 text-white ring-1 ring-white/5 transition-transform duration-200 ease-out hover:-translate-y-0.5 motion-reduce:hover:translate-y-0',
          active && 'ring-accent ring-2',
        )}
        data-testid={`instance-recent-${instance.id}`}
      >
        <InstanceCover instance={instance} className="-z-10" />
        <span
          aria-hidden
          className="absolute inset-0 -z-10 bg-gradient-to-r from-black/85 via-black/55 to-black/10"
        />
        <div className="flex min-w-0 items-center gap-2.5">
          <InstanceIcon icon={instance.icon} size="md" className="ring-2 ring-white/15" />
          <div className="flex min-w-0 flex-col gap-0.5">
            <TitleLink
              instance={instance}
              className="rx-focus-ring truncate rounded text-[15px] leading-tight font-semibold text-white after:absolute after:inset-0 after:content-[''] hover:underline"
            />
            <p className="truncate text-xs text-white/75">
              <PlayedText instance={instance} />
              {server && (
                <>
                  <span aria-hidden> · </span>
                  {t('profiles.card.lastServer', { name: server.name })}
                </>
              )}
            </p>
          </div>
        </div>
        <div className="relative z-10 flex min-w-0 items-center gap-2 [&_.text-fg-muted]:text-white/80">
          <InstancePlayControl instance={instance} testId={`recent-play-${instance.id}`} />
          {server && !busy && (
            <Tooltip content={server.address}>
              <Button
                size="sm"
                variant="secondary"
                leftIcon={<LogIn />}
                disabled={play.state === 'disabled'}
                className="border-white/15 bg-white/10 text-white hover:bg-white/20"
                aria-label={t('profiles.card.rejoinAria', {
                  server: server.name,
                  name: instance.name,
                })}
                data-testid={`recent-rejoin-${instance.id}`}
                onClick={() => {
                  setActive(instance.id);
                  void play.play({ join: server });
                }}
              >
                {t('profiles.card.rejoin')}
              </Button>
            </Tooltip>
          )}
        </div>
      </article>
    </CardRowItem>
  );
}

/** The grid's last tile: create a profile, or import one. */
export function CreateTile({ onCreate, onImport }: { onCreate: () => void; onImport: () => void }) {
  const { t } = useTranslation('instances');
  return (
    <div
      className="border-border hover:border-accent/50 group/tile relative flex h-full min-h-64 flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed p-5 text-center transition-colors"
      data-testid="create-profile-tile"
    >
      <span className="bg-accent-soft text-accent-text grid size-14 place-items-center rounded-2xl transition-transform group-hover/tile:scale-105 [&_svg]:size-6">
        <Plus />
      </span>
      <div className="flex flex-col gap-0.5">
        <p className="text-fg text-[15px] font-semibold">{t('profiles.create')}</p>
        <p className="text-fg-muted text-xs">{t('profiles.createHint')}</p>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-2 pt-1">
        <Button size="sm" variant="primary" leftIcon={<Plus />} onClick={onCreate}>
          {t('profiles.create')}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          leftIcon={<FolderInput />}
          onClick={onImport}
          data-testid="create-profile-tile-import"
        >
          {t('profiles.import')}
        </Button>
      </div>
    </div>
  );
}
