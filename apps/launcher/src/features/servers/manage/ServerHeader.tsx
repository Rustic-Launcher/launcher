import { Button, cn, toast } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { ChevronLeft, Clock, LogIn, Play, RotateCw, Square, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import { InstanceCover } from '@/features/instances/InstanceCover';
import { usePlay } from '@/features/play/usePlay';
import type { Server, ServerState } from '@/ipc/types/servers';
import { formatDuration } from '@/lib/format';
import { useActiveInstance } from '@/queries/core';
import { EngineBadge, HostingBadge, STATE_DOT } from '../list/ServerCard';
import { useServerPower } from '../list/queries';
import { useNow } from './queries';
import { CopyField } from './shared';

const PILL: Record<ServerState, string> = {
  running: 'border-success/40 bg-success/12 text-success-text',
  starting: 'border-warning/40 bg-warning/12 text-warning-text',
  stopping: 'border-warning/40 bg-warning/12 text-warning-text',
  installing: 'border-accent/40 bg-accent-soft text-accent-text',
  stopped: 'border-border bg-surface-2 text-fg-muted',
  crashed: 'border-danger/40 bg-danger/12 text-danger-text',
  unreachable: 'border-danger/40 bg-danger/12 text-danger-text',
};

/** The server's live state as a pill (a pulsing dot while it runs). */
export function StatePill({ state, className }: { state: ServerState; className?: string }) {
  const { t } = useTranslation('servers');
  return (
    <span
      className={cn(
        'inline-flex h-7 items-center gap-2 rounded-full border px-3 text-xs font-semibold',
        PILL[state],
        className,
      )}
      data-testid="server-state"
      role="status"
    >
      <span aria-hidden className="relative flex size-2">
        {state === 'running' && (
          <span className="bg-success absolute inset-0 animate-ping rounded-full opacity-60 motion-reduce:hidden" />
        )}
        <span className={cn('relative size-2 rounded-full', STATE_DOT[state])} />
      </span>
      {t(`manage.state.${state}`)}
    </span>
  );
}

/**
 * The server page's banner: the cover in the server's colour, its icon, name and live state, the
 * players and badges, the power controls and Join; then the address to share (copy) and the
 * folder (open) on a strip below.
 */
export function ServerHeader({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const power = useServerPower();
  const { instance } = useActiveInstance();
  const play = usePlay(instance);
  const now = useNow(30_000);
  const running = server.state === 'running';
  const busy =
    server.state === 'starting' || server.state === 'stopping' || server.state === 'installing';
  const uptime =
    running && server.last_started_at
      ? formatDuration((now - Date.parse(server.last_started_at)) / 1000, i18n.language)
      : null;

  const act = (action: 'start' | 'stop' | 'restart') =>
    power.mutate(
      { id: server.id, action },
      {
        onError: (e) =>
          toast({ tone: 'error', title: t('manage.header.powerFailed'), description: e.message }),
      },
    );

  return (
    <header className="flex flex-col gap-3">
      <Link
        to="/servers"
        className="rx-focus-ring text-fg-subtle hover:text-fg flex w-fit items-center gap-1 rounded text-xs font-medium"
      >
        <ChevronLeft className="size-3.5" aria-hidden />
        {t('manage.header.back')}
      </Link>
      <div className="rx-surface relative overflow-hidden rounded-2xl" data-testid="server-header">
        <div className="relative h-28 lg:h-32">
          <InstanceCover instance={server} wide />
          <span
            aria-hidden
            className="from-surface via-surface/35 absolute inset-0 bg-gradient-to-t to-transparent"
          />
        </div>
        <div className="relative -mt-14 flex flex-wrap items-end gap-x-5 gap-y-4 px-5 pb-5">
          <InstanceIcon
            icon={server.icon}
            size="xl"
            className="ring-surface relative shadow-lg ring-4"
          />
          <div className="flex min-w-[min(100%,18rem)] flex-1 flex-col gap-2">
            <h1 className="text-fg truncate text-2xl font-bold tracking-tight">{server.name}</h1>
            <div className="text-fg-muted flex flex-wrap items-center gap-x-3 gap-y-1.5 text-sm">
              <StatePill state={server.state} />
              <span className="text-fg flex items-center gap-1.5 font-medium tabular-nums">
                <Users className="text-fg-muted size-4" aria-hidden />
                {t('manage.header.players', {
                  online: server.players_online,
                  max: server.max_players,
                })}
              </span>
              {uptime && (
                <span className="flex items-center gap-1.5 tabular-nums">
                  <Clock className="size-4" aria-hidden />
                  {t('manage.header.uptime', { duration: uptime })}
                </span>
              )}
            </div>
            <div className="flex flex-wrap items-center gap-1.5">
              <EngineBadge engine={server.engine} />
              <HostingBadge hosting={server.hosting} />
              <HorizonVersion
                id={server.horizon_version}
                rusticEdition={false}
                className="border-border bg-surface-2 text-fg-muted text-2xs inline-flex h-5 items-center rounded-full border px-2 font-medium"
              />
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {running || server.state === 'stopping' ? (
              <>
                <Button
                  variant="secondary"
                  leftIcon={<RotateCw />}
                  onClick={() => act('restart')}
                  disabled={busy || power.isPending}
                  data-testid="server-restart"
                >
                  {t('manage.header.restart')}
                </Button>
                <Button
                  variant="secondary"
                  leftIcon={<Square />}
                  onClick={() => act('stop')}
                  loading={server.state === 'stopping'}
                  disabled={busy || power.isPending}
                  data-testid="server-stop"
                >
                  {t('manage.header.stop')}
                </Button>
              </>
            ) : (
              <Button
                variant="primary"
                size="lg"
                leftIcon={<Play />}
                onClick={() => act('start')}
                loading={server.state === 'starting' || power.isPending}
                disabled={server.state === 'installing'}
                data-testid="server-start"
              >
                {t('manage.header.start')}
              </Button>
            )}
            <Button
              variant={running ? 'primary' : 'secondary'}
              size={running ? 'lg' : 'md'}
              leftIcon={<LogIn />}
              disabled={!running || !instance}
              onClick={() => {
                const address = server.share_address ?? `localhost:${server.port}`;
                void play.play({ join: { address, name: server.name } });
                toast({ tone: 'info', title: tc('friends.joining', { server: server.name }) });
              }}
            >
              {t('manage.header.join')}
            </Button>
          </div>
        </div>
        {server.share_address && (
          <div className="border-border bg-surface-2/35 flex flex-wrap items-center gap-2 border-t px-5 py-3">
            <span className="text-fg-subtle shrink-0 text-xs font-medium">
              {t('manage.header.address')}
            </span>
            <CopyField
              value={server.share_address}
              label={t('manage.header.copyAddress')}
              testId="server-address"
              className="max-w-xl flex-1"
            />
          </div>
        )}
      </div>
    </header>
  );
}
