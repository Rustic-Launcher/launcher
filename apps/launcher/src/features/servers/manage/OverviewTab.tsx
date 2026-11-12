import { Avatar, EmptyState, LineChart, Skeleton } from '@rustic/ui';
import { Activity, Cpu, Gauge, MemoryStick, Timer } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Server, ServerStats } from '@/ipc/types/servers';
import { formatDate, formatMemory, formatNumber, shortVersion } from '@/lib/format';
import { useLiveStats, usePlayers } from './queries';
import { Panel } from './shared';

function Metric({
  icon,
  label,
  value,
  sub,
  children,
  testId,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  sub?: string;
  children: ReactNode;
  testId?: string;
}) {
  return (
    <div className="rx-surface flex flex-col gap-3 rounded-2xl p-4" data-testid={testId}>
      <div className="flex items-start justify-between gap-2">
        <span className="text-fg-muted flex items-center gap-2 text-xs font-medium [&_svg]:size-4">
          {icon}
          {label}
        </span>
        <span className="text-right">
          <span className="text-fg block font-mono text-lg font-semibold tabular-nums">
            {value}
          </span>
          {sub && <span className="text-2xs text-fg-subtle block tabular-nums">{sub}</span>}
        </span>
      </div>
      {children}
    </div>
  );
}

/** Health status of the tick loop from TPS/MSPT. */
export function tickHealth(s: ServerStats | undefined): 'good' | 'busy' | 'lagging' {
  if (!s) return 'good';
  if (s.mspt > 50 || s.tps < 18) return 'lagging';
  if (s.mspt > 35) return 'busy';
  return 'good';
}

export function OverviewTab({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const lang = i18n.language;
  const { data: stats, isLoading } = useLiveStats(server.id);
  const { data: players } = usePlayers(server.id);
  const running = server.state === 'running';
  const last = stats?.at(-1);
  const series = (pick: (s: ServerStats) => number) => [
    { id: 'v', values: (stats ?? []).map(pick) },
  ];
  const online = players?.filter((p) => p.online) ?? [];
  const health = tickHealth(last);

  return (
    <div className="flex flex-col gap-4">
      {!running ? (
        <EmptyState
          icon={<Activity />}
          title={t('manage.overview.stoppedTitle')}
          description={t('manage.overview.stoppedBody')}
        />
      ) : isLoading ? (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          {Array.from({ length: 4 }, (_, i) => (
            <Skeleton key={i} className="h-40 rounded-2xl" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          <Metric
            icon={<Cpu />}
            label={t('manage.overview.cpu')}
            value={
              last ? `${formatNumber(last.cpu_percent, lang, { maximumFractionDigits: 0 })} %` : '–'
            }
            testId="metric-cpu"
          >
            <LineChart
              series={series((s) => s.cpu_percent)}
              max={100}
              label={t('manage.overview.cpuChart')}
              height={64}
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
              height={64}
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
            testId="metric-tps"
          >
            <LineChart
              series={[
                { id: 'v', values: (stats ?? []).map((s) => s.tps), color: 'var(--rl-success)' },
              ]}
              min={0}
              max={20}
              label={t('manage.overview.tpsChart')}
              height={64}
            />
          </Metric>
          <Metric
            icon={<Timer />}
            label={t('manage.overview.mspt')}
            value={last ? `${formatNumber(last.mspt, lang, { maximumFractionDigits: 1 })} ms` : '–'}
            sub={t('manage.overview.msptBudget')}
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
              height={64}
            />
          </Metric>
        </div>
      )}
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_380px]">
        <Panel
          title={t('manage.overview.onlineTitle', { count: running ? online.length : 0 })}
          description={running ? undefined : t('manage.overview.onlineStopped')}
        >
          {running && online.length === 0 && (
            <p className="text-fg-muted text-sm">{t('manage.overview.nobody')}</p>
          )}
          {running && online.length > 0 && (
            <ul className="flex flex-wrap gap-2">
              {online.map((p) => (
                <li
                  key={p.uuid}
                  className="border-border bg-surface-2 flex items-center gap-2 rounded-full border py-1 pr-3 pl-1"
                >
                  <Avatar
                    name={p.name}
                    size="xs"
                    shape="circle"
                    presence="in-game"
                    presenceLabel={t('manage.players.online')}
                  />
                  <span className="text-sm">{p.name}</span>
                  {p.ping_ms != null && (
                    <span className="text-2xs text-fg-subtle font-mono">{p.ping_ms} ms</span>
                  )}
                </li>
              ))}
            </ul>
          )}
        </Panel>
        <Panel title={t('manage.overview.infoTitle')}>
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
            <dt className="text-fg-subtle">{t('manage.overview.world')}</dt>
            <dd className="text-fg truncate">{server.world_name}</dd>
            <dt className="text-fg-subtle">{t('manage.overview.version')}</dt>
            <dd className="text-fg">{shortVersion(server.horizon_version)}</dd>
            <dt className="text-fg-subtle">{t('manage.overview.port')}</dt>
            <dd className="text-fg font-mono">{server.port}</dd>
            <dt className="text-fg-subtle">{t('manage.overview.memory')}</dt>
            <dd className="text-fg">{formatMemory(server.memory_mb, lang)}</dd>
            <dt className="text-fg-subtle">{t('manage.overview.created')}</dt>
            <dd className="text-fg">{formatDate(server.created_at, lang)}</dd>
            <dt className="text-fg-subtle">{t('manage.overview.folder')}</dt>
            <dd className="text-fg-muted truncate font-mono text-xs" title={server.folder}>
              {server.folder}
            </dd>
          </dl>
        </Panel>
      </div>
    </div>
  );
}
