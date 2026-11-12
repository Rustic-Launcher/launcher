import { Badge, Button, Card, cn, IconButton, toast, Tooltip } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { Copy, Globe, Laptop, Play, Puzzle, Square, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { InstanceIcon } from '@/components/InstanceIcon';
import type { Server, ServerState } from '@/ipc/types/servers';
import { shortVersion } from '@/lib/format';
import { copyText, useServerPower } from './queries';

export const STATE_DOT: Record<ServerState, string> = {
  running: 'bg-success',
  starting: 'bg-warning animate-pulse-soft',
  stopping: 'bg-warning animate-pulse-soft',
  installing: 'bg-accent animate-pulse-soft',
  stopped: 'bg-fg-subtle',
  crashed: 'bg-danger',
  unreachable: 'bg-danger',
};

export function EngineBadge({ engine }: { engine: Server['engine'] }) {
  const { t } = useTranslation('servers');
  return engine === 'rustic' ? (
    <Badge variant="accent" icon={<Puzzle />}>
      {t('list.engine.rustic')}
    </Badge>
  ) : (
    <Badge variant="neutral">{t('list.engine.horizon')}</Badge>
  );
}

export function HostingBadge({ hosting }: { hosting: Server['hosting'] }) {
  const { t } = useTranslation('servers');
  if (hosting.mode === 'vps') {
    return (
      <Badge variant="info" icon={<Globe />}>
        {t('list.hosting.remote', { host: hosting.host })}
      </Badge>
    );
  }
  if (hosting.mode === 'cloud') {
    return (
      <Badge variant="info" icon={<Globe />}>
        {t('list.hosting.cloud', { region: hosting.region })}
      </Badge>
    );
  }
  return (
    <Badge variant="neutral" icon={<Laptop />}>
      {t('list.hosting.local')}
    </Badge>
  );
}

export function ServerCard({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const power = useServerPower();
  const running = server.state === 'running' || server.state === 'starting';
  const busy = ['starting', 'stopping', 'installing'].includes(server.state);

  return (
    <Card
      variant="glass"
      interactive
      padding="none"
      className="group flex flex-col overflow-hidden"
      data-testid={`server-card-${server.id}`}
    >
      <div className="flex items-start gap-3 p-4 pb-3">
        <InstanceIcon icon={server.icon} size="lg" />
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <Link
            to="/servers/$serverId"
            params={{ serverId: server.id }}
            className="rx-focus-ring text-fg truncate rounded text-base font-semibold after:absolute after:inset-0 after:content-['']"
          >
            {server.name}
          </Link>
          <div className="flex flex-wrap gap-1.5">
            <EngineBadge engine={server.engine} />
            <HostingBadge hosting={server.hosting} />
          </div>
        </div>
      </div>
      <dl className="grid grid-cols-2 gap-x-3 gap-y-2 px-4 pb-3 text-xs">
        <div className="flex items-center gap-2">
          <dt className="sr-only">{t('list.state')}</dt>
          <span aria-hidden className={cn('size-2 rounded-full', STATE_DOT[server.state])} />
          <dd className="text-fg font-medium" data-testid="server-state">
            {t(`list.states.${server.state}`)}
          </dd>
        </div>
        <div className="text-fg-muted flex items-center gap-1.5">
          <Users className="size-3.5" aria-hidden />
          <dt className="sr-only">{t('list.players')}</dt>
          <dd className="tabular-nums">
            {t('list.playersCount', { online: server.players_online, max: server.max_players })}
          </dd>
        </div>
        <div className="text-fg-muted col-span-2 flex items-center gap-1.5">
          <dt className="sr-only">{t('list.version')}</dt>
          <dd>{tc('horizon.version', { version: shortVersion(server.horizon_version) })}</dd>
        </div>
      </dl>
      <div className="border-border mt-auto flex items-center gap-2 border-t px-4 py-2.5">
        <div className="relative z-10 flex min-w-0 flex-1 items-center gap-1">
          {server.share_address ? (
            <>
              <code className="text-fg-muted truncate font-mono text-xs">
                {server.share_address}
              </code>
              <IconButton
                size="sm"
                label={t('list.copyAddress')}
                icon={<Copy />}
                onClick={() =>
                  void copyText(server.share_address!).then((ok) =>
                    toast({
                      tone: ok ? 'success' : 'error',
                      title: ok ? tc('actions.copied') : t('list.copyFailed'),
                    }),
                  )
                }
              />
            </>
          ) : (
            <Tooltip content={t('list.notSharedHint')}>
              <span tabIndex={0} role="note" className="text-fg-subtle truncate text-xs">
                {t('list.notShared')}
              </span>
            </Tooltip>
          )}
        </div>
        <Button
          size="sm"
          variant={running ? 'secondary' : 'primary'}
          className="relative z-10"
          disabled={server.state === 'installing' || server.state === 'stopping'}
          loading={power.isPending || (busy && server.state !== 'installing')}
          leftIcon={running ? <Square /> : <Play />}
          onClick={() =>
            power.mutate(
              { id: server.id, action: running ? 'stop' : 'start' },
              {
                onError: (e) =>
                  toast({ tone: 'error', title: t('list.powerFailed'), description: e.message }),
              },
            )
          }
          aria-label={
            running
              ? t('list.stopAria', { name: server.name })
              : t('list.startAria', { name: server.name })
          }
        >
          {running ? t('list.stop') : t('list.start')}
        </Button>
      </div>
    </Card>
  );
}
