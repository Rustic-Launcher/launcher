import { Badge, Button, Dialog, EmptyState, Switch, toast, Tooltip } from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import {
  ArrowUpCircle,
  FolderOpen,
  Globe,
  HardDrive,
  Puzzle,
  Search,
  ShieldCheck,
  Trash2,
  TriangleAlert,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { Server, ServerPlugin } from '@/ipc/types/servers';
import { mk, usePlugins } from './queries';

export function PluginsTab({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const rustic = server.engine === 'rustic';
  const { data, isLoading, isError, refetch } = usePlugins(server.id, rustic);
  const [removing, setRemoving] = useState<ServerPlugin | null>(null);

  const setPlugin = (p: ServerPlugin) =>
    qc.setQueryData<ServerPlugin[]>(mk.plugins(server.id), (old) =>
      old?.map((x) => (x.id === p.id ? p : x)),
    );
  const onError = (e: Error) =>
    toast({ tone: 'error', title: t('manage.plugins.failed'), description: e.message });

  const toggle = useMutation({
    mutationFn: (v: { id: string; enabled: boolean }) =>
      invoke('server_plugin_set_enabled', {
        server_id: server.id,
        plugin_id: v.id,
        enabled: v.enabled,
      }),
    onSuccess: setPlugin,
    onError,
  });
  const update = useMutation({
    mutationFn: (id: string) =>
      invoke('server_plugin_update', { server_id: server.id, plugin_id: id }),
    onSuccess: (p) => {
      setPlugin(p);
      toast({
        tone: 'success',
        title: t('manage.plugins.updated', { name: p.name, version: p.version }),
      });
    },
    onError,
  });
  const remove = useMutation({
    mutationFn: (id: string) =>
      invoke('server_plugin_remove', { server_id: server.id, plugin_id: id }),
    onSuccess: (_, id) => {
      qc.setQueryData<ServerPlugin[]>(mk.plugins(server.id), (old) =>
        old?.filter((x) => x.id !== id),
      );
      setRemoving(null);
    },
    onError,
  });
  const upload = useMutation({
    mutationFn: async () => {
      const path = await invoke('app_pick_path', {
        kind: 'file',
        title: t('manage.plugins.uploadTitle'),
        filters: [{ name: 'Rustic plugin', extensions: ['rplugin'] }],
      });
      if (!path) return null;
      return invoke('server_plugin_install', { server_id: server.id, path });
    },
    onSuccess: (p) => {
      if (!p) return;
      void qc.invalidateQueries({ queryKey: mk.plugins(server.id) });
      toast({ tone: 'success', title: t('manage.plugins.installed', { name: p.name }) });
    },
    onError,
  });

  if (!rustic) {
    return (
      <EmptyState
        icon={<Puzzle />}
        title={t('manage.plugins.vanillaTitle')}
        description={t('manage.plugins.vanillaBody')}
        action={
          <Button asChild variant="secondary">
            <Link to="/servers/new">{t('manage.plugins.createRustic')}</Link>
          </Button>
        }
      />
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-fg-muted max-w-2xl text-sm">{t('manage.plugins.intro')}</p>
        <div className="flex gap-2">
          <Button
            variant="secondary"
            leftIcon={<FolderOpen />}
            loading={upload.isPending}
            onClick={() => upload.mutate()}
          >
            {t('manage.plugins.upload')}
          </Button>
          <Button asChild variant="primary" leftIcon={<Search />}>
            <Link to="/content" search={{ kind: 'plugin', target: `server:${server.id}` }}>
              {t('manage.plugins.find')}
            </Link>
          </Button>
        </div>
      </div>
      {isLoading && <ListSkeleton rows={4} />}
      {isError && <QueryError onRetry={() => void refetch()} />}
      {data && data.length === 0 && (
        <EmptyState
          icon={<Puzzle />}
          title={t('manage.plugins.empty')}
          description={t('manage.plugins.emptyBody')}
        />
      )}
      {data && data.length > 0 && (
        <ul className="grid grid-cols-1 gap-3 xl:grid-cols-2" data-testid="plugins-list">
          {data.map((p) => (
            <li key={p.id} className="rx-surface flex flex-col gap-3 rounded-2xl p-4">
              <div className="flex items-start gap-3">
                <span className="bg-accent-soft text-accent-text grid size-10 shrink-0 place-items-center rounded-xl">
                  <Puzzle className="size-5" aria-hidden />
                </span>
                <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="flex flex-wrap items-center gap-1.5">
                    <span className="text-fg text-sm font-semibold">{p.name}</span>
                    <span className="text-fg-subtle font-mono text-xs">{p.version}</span>
                    {p.state === 'errored' && (
                      <Badge variant="danger">{t('manage.plugins.errored')}</Badge>
                    )}
                    {p.state === 'disabled' && (
                      <Badge variant="neutral">{t('manage.plugins.disabled')}</Badge>
                    )}
                  </span>
                  <span className="text-fg-muted text-xs">
                    {p.description || t('manage.plugins.noDescription')} · {p.authors.join(', ')} ·{' '}
                    {p.license}
                  </span>
                </div>
                <Switch
                  checked={p.state === 'enabled'}
                  onCheckedChange={(v) => toggle.mutate({ id: p.id, enabled: v })}
                  aria-label={t('manage.plugins.toggle', { name: p.name })}
                />
              </div>
              {p.error && (
                <p className="bg-danger/10 text-danger-text flex items-start gap-2 rounded-lg px-3 py-2 text-xs">
                  <TriangleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden />
                  {p.error}
                </p>
              )}
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="text-2xs text-fg-subtle font-semibold tracking-wider uppercase">
                  {t('manage.plugins.capabilities')}
                </span>
                {p.capabilities.network === false ? (
                  <Badge variant="success" icon={<ShieldCheck />}>
                    {t('manage.plugins.noNetwork')}
                  </Badge>
                ) : (
                  <Tooltip content={p.capabilities.network.join(', ')}>
                    <Badge variant="warning" icon={<Globe />} tabIndex={0}>
                      {t('manage.plugins.network', { domains: p.capabilities.network.join(', ') })}
                    </Badge>
                  </Tooltip>
                )}
                <Badge
                  variant={p.capabilities.filesystem === 'none' ? 'success' : 'info'}
                  icon={<HardDrive />}
                >
                  {t(`manage.plugins.fs.${p.capabilities.filesystem}`)}
                </Badge>
                <span className="text-2xs text-fg-subtle">· API {p.api}</span>
                <span className="ml-auto flex gap-1.5">
                  {p.update_available && (
                    <Button
                      size="sm"
                      variant="soft"
                      leftIcon={<ArrowUpCircle />}
                      loading={update.isPending && update.variables === p.id}
                      onClick={() => update.mutate(p.id)}
                    >
                      {t('manage.plugins.update', { version: p.update_available })}
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    leftIcon={<Trash2 />}
                    onClick={() => setRemoving(p)}
                  >
                    {t('manage.remove')}
                  </Button>
                </span>
              </div>
            </li>
          ))}
        </ul>
      )}
      <Dialog
        open={removing != null}
        onOpenChange={(o) => !o && setRemoving(null)}
        tone="destructive"
        icon={<Trash2 />}
        title={t('manage.plugins.removeTitle', { name: removing?.name ?? '' })}
        description={t('manage.plugins.removeBody')}
        closeLabel={t('manage.close')}
        footer={
          <>
            <Button onClick={() => setRemoving(null)}>{t('manage.cancel')}</Button>
            <Button
              variant="danger"
              loading={remove.isPending}
              onClick={() => removing && remove.mutate(removing.id)}
            >
              {t('manage.remove')}
            </Button>
          </>
        }
      />
    </div>
  );
}
