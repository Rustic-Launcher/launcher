import { Button, EmptyState, IconButton, Switch, toast } from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { Package, Search, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { Server, ServerDatapack } from '@/ipc/types/servers';
import { mk, useDatapacks } from './queries';

export function DatapacksTab({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const { data, isLoading, isError, refetch } = useDatapacks(server.id);
  const set = (fn: (old: ServerDatapack[]) => ServerDatapack[]) =>
    qc.setQueryData<ServerDatapack[]>(mk.datapacks(server.id), (old) => fn(old ?? []));
  const onError = (e: Error) =>
    toast({ tone: 'error', title: t('manage.datapacks.failed'), description: e.message });
  const toggle = useMutation({
    mutationFn: (v: { id: string; enabled: boolean }) =>
      invoke('server_datapack_set_enabled', {
        server_id: server.id,
        datapack_id: v.id,
        enabled: v.enabled,
      }),
    onSuccess: (d) => set((old) => old.map((x) => (x.id === d.id ? d : x))),
    onError,
  });
  const remove = useMutation({
    mutationFn: (id: string) =>
      invoke('server_datapack_remove', { server_id: server.id, datapack_id: id }),
    onSuccess: (_, id) => set((old) => old.filter((x) => x.id !== id)),
    onError,
  });

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-fg-muted max-w-2xl text-sm">
          {t('manage.datapacks.intro', { world: server.world_name })}
        </p>
        <Button asChild variant="primary" leftIcon={<Search />}>
          <Link to="/content" search={{ kind: 'datapack', target: `server:${server.id}` }}>
            {t('manage.datapacks.find')}
          </Link>
        </Button>
      </div>
      {isLoading && <ListSkeleton rows={3} />}
      {isError && <QueryError onRetry={() => void refetch()} />}
      {data && data.length === 0 && (
        <EmptyState icon={<Package />} title={t('manage.datapacks.empty')} />
      )}
      {data && data.length > 0 && (
        <ul className="rx-surface divide-border flex flex-col divide-y overflow-hidden rounded-2xl">
          {data.map((d) => (
            <li key={d.id} className="flex items-center gap-3 px-4 py-3">
              <Package className="text-fg-subtle size-5 shrink-0" aria-hidden />
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="text-sm font-medium">
                  {d.name} <span className="text-fg-subtle font-mono text-xs">{d.version}</span>
                </span>
                <span className="text-fg-muted truncate text-xs">{d.description}</span>
              </div>
              <Switch
                checked={d.enabled}
                onCheckedChange={(v) => toggle.mutate({ id: d.id, enabled: v })}
                aria-label={t('manage.datapacks.toggle', { name: d.name })}
              />
              <IconButton
                label={t('manage.datapacks.remove', { name: d.name })}
                icon={<Trash2 />}
                variant="danger"
                onClick={() => remove.mutate(d.id)}
              />
            </li>
          ))}
        </ul>
      )}
      <p className="text-fg-subtle text-xs">{t('manage.datapacks.reloadHint')}</p>
    </div>
  );
}
