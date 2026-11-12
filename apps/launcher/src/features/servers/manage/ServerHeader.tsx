import { Button, cn, toast } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { ChevronLeft, LogIn, Play, RotateCw, Square, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import { usePlay } from '@/features/play/usePlay';
import type { Server } from '@/ipc/types/servers';
import { formatDuration } from '@/lib/format';
import { useActiveInstance } from '@/queries/core';
import { EngineBadge, HostingBadge, STATE_DOT } from '../list/ServerCard';
import { useServerPower } from '../list/queries';
import { useNow } from './queries';
import { CopyField } from './shared';

/** Status header: identity, state, address, power controls and Join. */
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
    <header className="flex flex-col gap-4">
      <Link
        to="/servers"
        className="rx-focus-ring text-fg-subtle hover:text-fg flex w-fit items-center gap-1 rounded text-xs font-medium"
      >
        <ChevronLeft className="size-3.5" aria-hidden />
        {t('manage.header.back')}
      </Link>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 items-start gap-4">
          <InstanceIcon icon={server.icon} size="lg" />
          <div className="flex min-w-0 flex-col gap-1.5">
            <h1 className="text-fg truncate text-2xl font-bold tracking-tight">{server.name}</h1>
            <div className="text-fg-muted flex flex-wrap items-center gap-x-3 gap-y-1.5 text-sm">
              <span
                className="text-fg flex items-center gap-1.5 font-medium"
                data-testid="server-state"
              >
                <span aria-hidden className={cn('size-2 rounded-full', STATE_DOT[server.state])} />
                {t(`manage.state.${server.state}`)}
              </span>
              <span className="flex items-center gap-1 tabular-nums">
                <Users className="size-3.5" aria-hidden />
                {t('manage.header.players', {
                  online: server.players_online,
                  max: server.max_players,
                })}
              </span>
              {uptime && <span>{t('manage.header.uptime', { duration: uptime })}</span>}
              <EngineBadge engine={server.engine} />
              <HostingBadge hosting={server.hosting} />
              <HorizonVersion
                id={server.horizon_version}
                rusticEdition={false}
                className="text-xs"
              />
            </div>
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
        <div className="flex max-w-xl items-center gap-2">
          <span className="text-fg-subtle shrink-0 text-xs font-medium">
            {t('manage.header.address')}
          </span>
          <CopyField
            value={server.share_address}
            label={t('manage.header.copyAddress')}
            testId="server-address"
            className="flex-1"
          />
        </div>
      )}
    </header>
  );
}
