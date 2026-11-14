import { Button, cn, LineChart, Skeleton, toast } from '@rustic/ui';
import { useNavigate } from '@tanstack/react-router';
import {
  Activity,
  Clock,
  Cpu,
  Crown,
  FolderOpen,
  Gauge,
  MemoryStick,
  Play,
  ScrollText,
  Timer,
  TriangleAlert,
  Users,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { ConsoleLine, Server, ServerStats } from '@/ipc/types/servers';
import { PlayerAvatar } from '@/components/PlayerAvatar';
import { invoke } from '@/ipc';
import {
  formatDate,
  formatDuration,
  formatMemory,
  formatNumber,
  formatRelative,
  shortVersion,
} from '@/lib/format';
import { useServerPower } from '../list/queries';
import { useConsole, useLiveStats, useNow, usePlayers } from './queries';
import { Panel } from './shared';

function Metric({
  icon,
  label,
  value,
  sub,
  children,
  testId,
  tone,
}: {
  icon: ReactNode;
  label: string;
  value: ReactNode;
  sub?: ReactNode;
  children?: ReactNode;
  testId?: string;
  tone?: 'default' | 'warning' | 'danger';
}) {
  return (
    <div
      className="rx-surface flex min-h-32 flex-col justify-between gap-3 rounded-2xl p-4"
      data-testid={testId}
    >
      <div className="flex items-start justify-between gap-2">
        <span className="text-fg-muted flex items-center gap-2 text-xs font-medium [&_svg]:size-4">
          <span
            aria-hidden
            className="bg-surface-2 text-fg grid size-7 place-items-center rounded-lg [&_svg]:size-3.5"
          >
            {icon}
          </span>
          {label}
        </span>
        <span className="text-right">
          <span
            className={cn(
              'block font-mono text-xl font-semibold tabular-nums',
              tone === 'danger'
                ? 'text-danger-text'
                : tone === 'warning'
                  ? 'text-warning-text'
                  : 'text-fg',
            )}
          >
            {value}
          </span>
          {sub && <span className="text-2xs text-fg-subtle block tabular-nums">{sub}</span>}
        </span>
      </div>
      {children}
    </div>
  );
}

/** "19:26" today, else "12 Nov, 19:26". */
function clockTime(iso: string, lang: string, now: number): string {
  const d = new Date(iso);
  return d.toDateString() === new Date(now).toDateString()
    ? d.toLocaleTimeString(lang, { hour: '2-digit', minute: '2-digit' })
    : d.toLocaleString(lang, {
        day: 'numeric',
        month: 'short',
        hour: '2-digit',
        minute: '2-digit',
      });
}

/** Health status of the tick loop from TPS/MSPT. */
export function tickHealth(s: ServerStats | undefined): 'good' | 'busy' | 'lagging' {
  if (!s) return 'good';
  if (s.mspt > 50 || s.tps < 18) return 'lagging';
  if (s.mspt > 35) return 'busy';
  return 'good';
}

const LINE_TONE: Record<ConsoleLine['level'], string> = {
  info: 'text-fg-muted',
  warn: 'text-warning-text',
  error: 'text-danger-text',
  command: 'text-accent-text',
  chat: 'text-info-text',
};

/** The last lines the server logged (its console's history, live). */
function LastLines({ server, count = 8 }: { server: Server; count?: number }) {
  const { t, i18n } = useTranslation('servers');
  const { data, isLoading } = useConsole(server.id);
  const lines = (data ?? []).slice(-count);
  if (isLoading) return <Skeleton className="h-40 rounded-xl" />;
  if (lines.length === 0) {
    return (
      <p className="bg-surface-2/60 text-fg-subtle grid h-full min-h-28 place-items-center rounded-xl text-sm">
        {t('manage.overview.noLines')}
      </p>
    );
  }
  return (
    <ol
      className="bg-bg/70 border-border flex flex-col gap-0.5 overflow-hidden rounded-xl border px-3 py-2.5 font-mono text-xs leading-5"
      data-testid="overview-last-lines"
      aria-label={t('manage.overview.lastLines')}
    >
      {lines.map((l, i) => (
        <li key={`${l.ts}-${i}`} className="flex min-w-0 gap-2">
          <span className="text-fg-subtle shrink-0 tabular-nums">
            {new Date(l.ts).toLocaleTimeString(i18n.language, {
              hour: '2-digit',
              minute: '2-digit',
              second: '2-digit',
            })}
          </span>
          <span className={cn('truncate', LINE_TONE[l.level])} title={l.text}>
            {l.text}
          </span>
        </li>
      ))}
    </ol>
  );
}

/**
 * Not running: what happened last and the way to start it, instead of an empty box. Crashed and
 * unreachable servers say so; starting/stopping/installing show their state.
 */
function NotRunning({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const navigate = useNavigate();
  const power = useServerPower();
  const failed = server.state === 'crashed' || server.state === 'unreachable';
  const transitional =
    server.state === 'starting' || server.state === 'stopping' || server.state === 'installing';
  const title =
    server.state === 'crashed'
      ? t('manage.overview.crashedTitle')
      : server.state === 'stopped'
        ? t('manage.overview.stoppedTitle')
        : t(`manage.state.${server.state}`);
  const body = failed
    ? t('manage.overview.crashedBody')
    : transitional
      ? undefined
      : t('manage.overview.stoppedBody');
  return (
    <section
      className={cn(
        'grid gap-5 rounded-2xl border p-5 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]',
        failed ? 'border-danger/35 bg-danger/5' : 'border-border bg-surface/70',
      )}
      data-testid="overview-stopped"
    >
      <div className="flex flex-col gap-4">
        <span
          aria-hidden
          className={cn(
            'grid size-11 place-items-center rounded-xl [&_svg]:size-5',
            failed ? 'bg-danger/15 text-danger-text' : 'bg-accent-soft text-accent-text',
          )}
        >
          {failed ? <TriangleAlert /> : <Activity />}
        </span>
        <div className="flex flex-col gap-1">
          <h2 className="text-fg text-lg font-semibold">{title}</h2>
          {body && <p className="text-fg-muted text-sm">{body}</p>}
        </div>
        <dl className="text-sm">
          <dt className="text-fg-subtle text-xs">{t('manage.overview.lastSession')}</dt>
          <dd className="text-fg font-medium">
            {server.last_started_at
              ? t('manage.overview.lastStarted', {
                  when: formatRelative(server.last_started_at, i18n.language),
                })
              : t('manage.overview.neverStarted')}
          </dd>
        </dl>
        {!transitional && (
          <Button
            variant="primary"
            size="lg"
            className="w-fit"
            leftIcon={<Play />}
            loading={power.isPending}
            onClick={() =>
              power.mutate(
                { id: server.id, action: 'start' },
                {
                  onError: (e) =>
                    toast({
                      tone: 'error',
                      title: t('manage.header.powerFailed'),
                      description: e.message,
                    }),
                },
              )
            }
            data-testid="overview-start"
          >
            {t('manage.header.start')}
          </Button>
        )}
      </div>
      <div className="flex min-w-0 flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
            {t('manage.overview.lastLines')}
          </h3>
          <Button
            variant="ghost"
            size="sm"
            leftIcon={<ScrollText />}
            onClick={() =>
              void navigate({
                to: '/servers/$serverId',
                params: { serverId: server.id },
                search: { tab: 'console' },
                replace: true,
              })
            }
          >
            {t('manage.overview.openConsole')}
          </Button>
        </div>
        <LastLines server={server} />
      </div>
    </section>
  );
}

export function OverviewTab({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const lang = i18n.language;
  const { data: stats, isLoading } = useLiveStats(server.id);
  const { data: players } = usePlayers(server.id);
  const now = useNow(30_000);
  const running = server.state === 'running';
  const last = stats?.at(-1);
  const online = players?.filter((p) => p.online) ?? [];
  const health = tickHealth(last);
  const local = server.hosting.mode === 'local';
  const uptimeS =
    running && server.last_started_at ? (now - Date.parse(server.last_started_at)) / 1000 : null;

  return (
    <div className="flex flex-col gap-4">
      {!running ? (
        <NotRunning server={server} />
      ) : isLoading ? (
        <div className="grid grid-cols-2 gap-4 xl:grid-cols-3">
          {Array.from({ length: 6 }, (_, i) => (
            <Skeleton key={i} className="h-36 rounded-2xl" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-4 xl:grid-cols-3" data-testid="overview-metrics">
          <Metric
            icon={<Users />}
            label={t('manage.tabs.players')}
            value={`${formatNumber(online.length, lang)} / ${formatNumber(server.max_players, lang)}`}
            testId="metric-players"
          >
            {online.length > 0 ? (
              <div className="flex items-center">
                {online.slice(0, 8).map((p) => (
                  <PlayerAvatar
                    key={p.uuid}
                    uuid={p.uuid}
                    name={p.name}
                    size="sm"
                    shape="circle"
                    className="ring-surface -mr-2 ring-2"
                  />
                ))}
                {online.length > 8 && (
                  <span className="text-fg-subtle ml-3 text-xs tabular-nums">
                    +{online.length - 8}
                  </span>
                )}
              </div>
            ) : (
              <p className="text-fg-subtle text-xs">{t('manage.overview.nobody')}</p>
            )}
          </Metric>
          <Metric
            icon={<Clock />}
            label={t('manage.overview.uptime')}
            value={uptimeS != null ? formatDuration(uptimeS, lang) : '–'}
            testId="metric-uptime"
          >
            {server.last_started_at && (
              <p className="text-fg-subtle text-xs">
                {t('manage.overview.startedAt', {
                  time: clockTime(server.last_started_at, lang, now),
                })}
              </p>
            )}
          </Metric>
          <Metric
            icon={<Cpu />}
            label={t('manage.overview.cpu')}
            value={
              last ? `${formatNumber(last.cpu_percent, lang, { maximumFractionDigits: 0 })} %` : '–'
            }
            tone={last && last.cpu_percent > 90 ? 'warning' : 'default'}
            testId="metric-cpu"
          >
            <LineChart
              series={[{ id: 'v', values: (stats ?? []).map((s) => s.cpu_percent) }]}
              max={100}
              label={t('manage.overview.cpuChart')}
              height={48}
            />
          </Metric>
          <Metric
            icon={<MemoryStick />}
            label={t('manage.overview.ram')}
            value={last ? formatMemory(last.ram_used_mb, lang) : '–'}
            sub={
              last
                ? t('manage.overview.of', { max: formatMemory(last.ram_max_mb, lang) })
                : undefined
            }
            testId="metric-ram"
          >
            <LineChart
              series={[
                {
                  id: 'v',
                  values: (stats ?? []).map((s) => s.ram_used_mb),
                  color: 'var(--rl-info)',
                },
              ]}
              max={last?.ram_max_mb ?? server.memory_mb}
              label={t('manage.overview.ramChart')}
              height={48}
            />
          </Metric>
          <Metric
            icon={<Gauge />}
            label={t('manage.overview.tps')}
            value={
              last
                ? formatNumber(last.tps, lang, {
                    minimumFractionDigits: 1,
                    maximumFractionDigits: 1,
                  })
                : '–'
            }
            sub={t(`manage.overview.health.${health}`)}
            tone={health === 'lagging' ? 'danger' : health === 'busy' ? 'warning' : 'default'}
            testId="metric-tps"
          >
            <LineChart
              series={[
                { id: 'v', values: (stats ?? []).map((s) => s.tps), color: 'var(--rl-success)' },
              ]}
              min={0}
              max={20}
              label={t('manage.overview.tpsChart')}
              height={48}
            />
          </Metric>
          <Metric
            icon={<Timer />}
            label={t('manage.overview.mspt')}
            value={last ? `${formatNumber(last.mspt, lang, { maximumFractionDigits: 1 })} ms` : '–'}
            sub={t('manage.overview.msptBudget')}
            tone={last && last.mspt > 50 ? 'danger' : 'default'}
            testId="metric-mspt"
          >
            <LineChart
              series={[
                { id: 'v', values: (stats ?? []).map((s) => s.mspt), color: 'var(--rl-warning)' },
              ]}
              min={0}
              max={Math.max(60, ...(stats ?? []).map((s) => s.mspt))}
              threshold={50}
              label={t('manage.overview.msptChart')}
              height={48}
            />
          </Metric>
        </div>
      )}
      <div
        className={cn('grid grid-cols-1 gap-4', running && 'lg:grid-cols-[minmax(0,1fr)_22rem]')}
      >
        {running && (
          <Panel title={t('manage.overview.onlineTitle', { count: online.length })}>
            {online.length === 0 && (
              <p className="text-fg-muted text-sm">{t('manage.overview.nobody')}</p>
            )}
            {online.length > 0 && (
              <ul className="grid gap-2 sm:grid-cols-2 2xl:grid-cols-3">
                {online.map((p) => (
                  <li
                    key={p.uuid}
                    className="border-border bg-surface-2/60 flex min-w-0 items-center gap-3 rounded-xl border p-2 pr-3"
                  >
                    <PlayerAvatar
                      uuid={p.uuid}
                      name={p.name}
                      size="md"
                      shape="square"
                      presence="in-game"
                      presenceLabel={t('manage.players.online')}
                    />
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="text-fg flex items-center gap-1.5 truncate text-sm font-medium">
                        {p.name}
                        {p.op && (
                          <Crown
                            className="text-warning-text size-3.5 shrink-0"
                            aria-label={t('manage.players.op')}
                          />
                        )}
                      </span>
                      {p.joined_at && (
                        <span className="text-fg-subtle truncate text-xs">
                          {formatRelative(p.joined_at, lang)}
                        </span>
                      )}
                    </span>
                    {p.ping_ms != null && (
                      <span className="text-2xs text-fg-subtle shrink-0 font-mono tabular-nums">
                        {p.ping_ms} ms
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </Panel>
        )}
        <Panel
          title={t('manage.overview.infoTitle')}
          action={
            local ? (
              <Button
                variant="secondary"
                size="sm"
                leftIcon={<FolderOpen />}
                title={server.folder}
                onClick={() => void invoke('app_open_path', { path: server.folder })}
              >
                {tc('actions.openFolder')}
              </Button>
            ) : undefined
          }
        >
          <dl
            className={cn(
              'grid grid-cols-2 gap-2 text-sm',
              !running && 'md:grid-cols-3 xl:grid-cols-6',
            )}
            data-testid="overview-details"
          >
            {(
              [
                [t('manage.overview.world'), server.world_name],
                [t('manage.overview.version'), shortVersion(server.horizon_version)],
                [t('manage.overview.port'), String(server.port)],
                [t('manage.overview.memory'), formatMemory(server.memory_mb, lang)],
                [t('manage.overview.created'), formatDate(server.created_at, lang)],
                ...(local ? [] : [[t('manage.overview.folder'), server.folder]]),
              ] as [string, string][]
            ).map(([k, v]) => (
              <div key={k} className="bg-surface-2/60 flex min-w-0 flex-col rounded-lg px-3 py-2">
                <dt className="text-fg-subtle text-xs">{k}</dt>
                <dd className="text-fg truncate font-medium" title={v}>
                  {v}
                </dd>
              </div>
            ))}
          </dl>
        </Panel>
      </div>
    </div>
  );
}
