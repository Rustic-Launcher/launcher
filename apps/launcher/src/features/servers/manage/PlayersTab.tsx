import {
  Avatar,
  Badge,
  Button,
  Dialog,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownSeparator,
  DropdownTrigger,
  EmptyState,
  IconButton,
  Input,
  SegmentedControl,
  SwitchRow,
  toast,
} from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Ban,
  Crown,
  DoorOpen,
  ListChecks,
  MoreHorizontal,
  ShieldCheck,
  ShieldOff,
  UserCheck,
  Users,
} from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { PlayerAction, Server, ServerPlayer } from '@/ipc/types/servers';
import { formatDuration } from '@/lib/format';
import { mk, useConfig, usePlayers } from './queries';
import { Panel } from './shared';

type View = 'online' | 'all' | 'ops' | 'whitelist' | 'banned';

export function filterPlayers(list: ServerPlayer[], view: View, query: string): ServerPlayer[] {
  const q = query.trim().toLowerCase();
  return list
    .filter((p) =>
      view === 'online'
        ? p.online
        : view === 'ops'
          ? p.op
          : view === 'whitelist'
            ? p.whitelisted
            : view === 'banned'
              ? p.banned
              : true,
    )
    .filter((p) => !q || p.name.toLowerCase().includes(q))
    .sort((a, b) => Number(b.online) - Number(a.online) || a.name.localeCompare(b.name));
}

export function PlayersTab({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const qc = useQueryClient();
  const { data: players, isLoading, isError, refetch } = usePlayers(server.id);
  const { data: config } = useConfig(server.id);
  const [view, setView] = useState<View>(server.state === 'running' ? 'online' : 'all');
  const [query, setQuery] = useState('');
  const [banTarget, setBanTarget] = useState<ServerPlayer | null>(null);
  const [reason, setReason] = useState('');
  const whitelistOn = Boolean(config?.fields.find((f) => f.key === 'white-list')?.value);

  const action = useMutation({
    mutationFn: (v: { player: string; action: PlayerAction; reason?: string }) =>
      invoke('server_player_action', { server_id: server.id, ...v }),
    onSuccess: (list, v) => {
      qc.setQueryData(mk.players(server.id), list);
      toast({ tone: 'success', title: t(`manage.players.done.${v.action}`, { name: v.player }) });
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('manage.players.failed'), description: e.message }),
  });
  const whitelist = useMutation({
    mutationFn: (enabled: boolean) =>
      invoke('server_whitelist_set_enabled', { server_id: server.id, enabled }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: mk.config(server.id) }),
  });

  const shown = useMemo(() => filterPlayers(players ?? [], view, query), [players, view, query]);
  const counts = useMemo(() => {
    const l = players ?? [];
    return {
      online: l.filter((p) => p.online).length,
      all: l.length,
      ops: l.filter((p) => p.op).length,
      whitelist: l.filter((p) => p.whitelisted).length,
      banned: l.filter((p) => p.banned).length,
    };
  }, [players]);

  return (
    <div className="flex flex-col gap-4">
      <Panel>
        <SwitchRow
          className="py-0"
          label={t('manage.players.whitelist')}
          description={t('manage.players.whitelistHint')}
          checked={whitelistOn}
          onCheckedChange={(v) => whitelist.mutate(v)}
          disabled={whitelist.isPending}
        />
      </Panel>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <SegmentedControl
          size="sm"
          label={t('manage.players.view')}
          value={view}
          onValueChange={(v) => setView(v as View)}
          options={(['online', 'all', 'ops', 'whitelist', 'banned'] as View[]).map((v) => ({
            value: v,
            label: `${t(`manage.players.views.${v}`)} · ${counts[v]}`,
          }))}
        />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t('manage.players.search')}
          aria-label={t('manage.players.search')}
          className="w-64"
        />
      </div>
      {isLoading && <ListSkeleton rows={5} />}
      {isError && <QueryError onRetry={() => void refetch()} />}
      {players && shown.length === 0 && (
        <EmptyState
          icon={<Users />}
          title={view === 'online' ? t('manage.players.emptyOnline') : t('manage.players.empty')}
          description={
            view === 'online' && server.state !== 'running'
              ? t('manage.players.stoppedHint')
              : undefined
          }
        />
      )}
      {shown.length > 0 && (
        <ul
          className="rx-surface divide-border flex flex-col divide-y overflow-hidden rounded-2xl"
          data-testid="players-list"
        >
          {shown.map((p) => (
            <li
              key={p.uuid}
              className="flex items-center gap-3 px-4 py-2.5"
              data-testid={`player-${p.name}`}
            >
              <Avatar
                name={p.name}
                size="sm"
                presence={p.online ? 'in-game' : 'offline'}
                presenceLabel={p.online ? t('manage.players.online') : t('manage.players.offline')}
              />
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="flex items-center gap-1.5 text-sm font-medium">
                  {p.name}
                  {p.op && (
                    <Badge variant="warning" icon={<Crown />}>
                      {t('manage.players.op')}
                    </Badge>
                  )}
                  {p.whitelisted && (
                    <Badge variant="neutral" icon={<ListChecks />}>
                      {t('manage.players.whitelisted')}
                    </Badge>
                  )}
                  {p.banned && (
                    <Badge variant="danger" icon={<Ban />}>
                      {t('manage.players.banned')}
                    </Badge>
                  )}
                </span>
                <span className="text-fg-subtle text-xs">
                  {p.online ? t('manage.players.online') : t('manage.players.offline')} ·{' '}
                  {t('manage.players.playtime', {
                    duration: formatDuration(p.playtime_s, i18n.language),
                  })}
                </span>
              </div>
              {p.online && p.ping_ms != null && (
                <span className="text-fg-muted font-mono text-xs tabular-nums">{p.ping_ms} ms</span>
              )}
              {p.online && (
                <Button
                  size="sm"
                  variant="ghost"
                  leftIcon={<DoorOpen />}
                  onClick={() => action.mutate({ player: p.name, action: 'kick' })}
                  aria-label={t('manage.players.kickAria', { name: p.name })}
                  data-testid={`kick-${p.name}`}
                >
                  {t('manage.players.kick')}
                </Button>
              )}
              <Dropdown>
                <DropdownTrigger asChild>
                  <IconButton
                    size="sm"
                    tooltip={false}
                    label={t('manage.players.more', { name: p.name })}
                    icon={<MoreHorizontal />}
                  />
                </DropdownTrigger>
                <DropdownContent>
                  {p.op ? (
                    <DropdownItem
                      icon={<ShieldOff />}
                      onSelect={() => action.mutate({ player: p.name, action: 'deop' })}
                    >
                      {t('manage.players.deop')}
                    </DropdownItem>
                  ) : (
                    <DropdownItem
                      icon={<ShieldCheck />}
                      onSelect={() => action.mutate({ player: p.name, action: 'op' })}
                    >
                      {t('manage.players.makeOp')}
                    </DropdownItem>
                  )}
                  {p.whitelisted ? (
                    <DropdownItem
                      icon={<ListChecks />}
                      onSelect={() => action.mutate({ player: p.name, action: 'whitelist_remove' })}
                    >
                      {t('manage.players.whitelistRemove')}
                    </DropdownItem>
                  ) : (
                    <DropdownItem
                      icon={<UserCheck />}
                      onSelect={() => action.mutate({ player: p.name, action: 'whitelist_add' })}
                    >
                      {t('manage.players.whitelistAdd')}
                    </DropdownItem>
                  )}
                  <DropdownSeparator />
                  {p.banned ? (
                    <DropdownItem
                      icon={<UserCheck />}
                      onSelect={() => action.mutate({ player: p.name, action: 'pardon' })}
                    >
                      {t('manage.players.pardon')}
                    </DropdownItem>
                  ) : (
                    <DropdownItem
                      icon={<Ban />}
                      destructive
                      onSelect={() => {
                        setReason('');
                        setBanTarget(p);
                      }}
                    >
                      {t('manage.players.ban')}
                    </DropdownItem>
                  )}
                </DropdownContent>
              </Dropdown>
            </li>
          ))}
        </ul>
      )}
      <Dialog
        open={banTarget != null}
        onOpenChange={(o) => !o && setBanTarget(null)}
        tone="destructive"
        icon={<Ban />}
        title={t('manage.players.banTitle', { name: banTarget?.name ?? '' })}
        description={t('manage.players.banBody')}
        closeLabel={t('manage.close')}
        footer={
          <>
            <Button onClick={() => setBanTarget(null)}>{t('manage.cancel')}</Button>
            <Button
              variant="danger"
              loading={action.isPending}
              onClick={() =>
                banTarget &&
                action.mutate(
                  { player: banTarget.name, action: 'ban', reason: reason.trim() || undefined },
                  { onSuccess: () => setBanTarget(null) },
                )
              }
            >
              {t('manage.players.ban')}
            </Button>
          </>
        }
      >
        <Input
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder={t('manage.players.reason')}
          aria-label={t('manage.players.reason')}
        />
      </Dialog>
    </div>
  );
}
