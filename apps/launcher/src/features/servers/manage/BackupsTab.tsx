import {
  Badge,
  Button,
  Dialog,
  EmptyState,
  formatBytes,
  IconButton,
  Input,
  Select,
  Skeleton,
  SwitchRow,
  toast,
} from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Archive, ArchiveRestore, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ListSkeleton } from '@/components/PageSkeleton';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { Backup, BackupSchedule, Server } from '@/ipc/types/servers';
import { formatDate, formatRelative } from '@/lib/format';
import { mk, useBackups, useSchedule } from './queries';
import { Panel } from './shared';

export function BackupsTab({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const lang = i18n.language;
  const qc = useQueryClient();
  const { data: backups, isLoading, isError, refetch } = useBackups(server.id);
  const [note, setNote] = useState('');
  const [restoring, setRestoring] = useState<Backup | null>(null);
  const [deleting, setDeleting] = useState<Backup | null>(null);
  const onError = (e: Error) =>
    toast({ tone: 'error', title: t('manage.backups.failed'), description: e.message });

  const create = useMutation({
    mutationFn: () =>
      invoke('server_backup_create', { server_id: server.id, note: note.trim() || undefined }),
    onSuccess: (b) => {
      qc.setQueryData<Backup[]>(mk.backups(server.id), (old) => [b, ...(old ?? [])]);
      setNote('');
      toast({ tone: 'success', title: t('manage.backups.created') });
    },
    onError,
  });
  const restore = useMutation({
    mutationFn: (id: string) =>
      invoke('server_backup_restore', { server_id: server.id, backup_id: id }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: mk.backups(server.id) });
      setRestoring(null);
      toast({ tone: 'success', title: t('manage.backups.restored') });
    },
    onError,
  });
  const remove = useMutation({
    mutationFn: (id: string) =>
      invoke('server_backup_delete', { server_id: server.id, backup_id: id }),
    onSuccess: (_, id) => {
      qc.setQueryData<Backup[]>(mk.backups(server.id), (old) => old?.filter((b) => b.id !== id));
      setDeleting(null);
    },
    onError,
  });

  return (
    <div className="grid grid-cols-1 gap-4 xl:grid-cols-[1fr_360px]">
      <div className="flex flex-col gap-4">
        <Panel title={t('manage.backups.nowTitle')} description={t('manage.backups.nowBody')}>
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              create.mutate();
            }}
          >
            <Input
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={t('manage.backups.notePlaceholder')}
              aria-label={t('manage.backups.note')}
              className="flex-1"
            />
            <Button
              type="submit"
              variant="primary"
              leftIcon={<Archive />}
              loading={create.isPending}
              data-testid="backup-now"
            >
              {t('manage.backups.now')}
            </Button>
          </form>
        </Panel>
        {isLoading && <ListSkeleton rows={4} />}
        {isError && <QueryError onRetry={() => void refetch()} />}
        {backups && backups.length === 0 && (
          <EmptyState icon={<Archive />} title={t('manage.backups.empty')} />
        )}
        {backups && backups.length > 0 && (
          <ul
            className="rx-surface divide-border flex flex-col divide-y overflow-hidden rounded-2xl"
            data-testid="backups-list"
          >
            {backups.map((b) => (
              <li key={b.id} className="flex items-center gap-3 px-4 py-3">
                <Archive className="text-fg-subtle size-5 shrink-0" aria-hidden />
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="flex items-center gap-2 text-sm font-medium">
                    {formatDate(b.created_at, lang, { dateStyle: 'medium', timeStyle: 'short' })}
                    <Badge
                      variant={
                        b.kind === 'manual'
                          ? 'accent'
                          : b.kind === 'pre_restore'
                            ? 'warning'
                            : 'neutral'
                      }
                    >
                      {t(`manage.backups.kinds.${b.kind}`)}
                    </Badge>
                  </span>
                  <span className="text-fg-muted truncate text-xs">
                    {formatRelative(b.created_at, lang)} · {formatBytes(b.size_bytes, lang)}
                    {b.note && ` · ${b.note}`}
                  </span>
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  leftIcon={<ArchiveRestore />}
                  onClick={() => setRestoring(b)}
                >
                  {t('manage.backups.restore')}
                </Button>
                <IconButton
                  label={t('manage.backups.delete')}
                  icon={<Trash2 />}
                  variant="danger"
                  onClick={() => setDeleting(b)}
                />
              </li>
            ))}
          </ul>
        )}
      </div>
      <ScheduleForm serverId={server.id} />
      <Dialog
        open={restoring != null}
        onOpenChange={(o) => !o && setRestoring(null)}
        tone="destructive"
        icon={<ArchiveRestore />}
        title={t('manage.backups.restoreTitle')}
        description={t('manage.backups.restoreBody', {
          date: restoring
            ? formatDate(restoring.created_at, lang, { dateStyle: 'medium', timeStyle: 'short' })
            : '',
        })}
        closeLabel={t('manage.close')}
        footer={
          <>
            <Button onClick={() => setRestoring(null)}>{t('manage.cancel')}</Button>
            <Button
              variant="danger"
              loading={restore.isPending}
              onClick={() => restoring && restore.mutate(restoring.id)}
            >
              {t('manage.backups.restore')}
            </Button>
          </>
        }
      />
      <Dialog
        open={deleting != null}
        onOpenChange={(o) => !o && setDeleting(null)}
        tone="destructive"
        icon={<Trash2 />}
        title={t('manage.backups.deleteTitle')}
        description={t('manage.backups.deleteBody')}
        closeLabel={t('manage.close')}
        footer={
          <>
            <Button onClick={() => setDeleting(null)}>{t('manage.cancel')}</Button>
            <Button
              variant="danger"
              loading={remove.isPending}
              onClick={() => deleting && remove.mutate(deleting.id)}
            >
              {t('manage.backups.delete')}
            </Button>
          </>
        }
      />
    </div>
  );
}

function ScheduleForm({ serverId }: { serverId: string }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const { data, isLoading } = useSchedule(serverId);
  const [draft, setDraft] = useState<BackupSchedule | null>(null);
  useEffect(() => {
    if (data) setDraft(data);
  }, [data]);
  const save = useMutation({
    mutationFn: (s: BackupSchedule) =>
      invoke('server_backup_schedule_set', { server_id: serverId, ...s }),
    onSuccess: (s) => {
      qc.setQueryData(mk.schedule(serverId), s);
      toast({ tone: 'success', title: t('manage.backups.scheduleSaved') });
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('manage.backups.failed'), description: e.message }),
  });
  if (isLoading || !draft) return <Skeleton className="h-64 rounded-2xl" />;
  const dirty = JSON.stringify(draft) !== JSON.stringify(data);
  return (
    <Panel
      title={t('manage.backups.scheduleTitle')}
      description={t('manage.backups.scheduleBody')}
      className="h-fit"
    >
      <div className="flex flex-col gap-3">
        <SwitchRow
          className="py-0"
          label={t('manage.backups.scheduleEnabled')}
          checked={draft.enabled}
          onCheckedChange={(v) => setDraft({ ...draft, enabled: v })}
        />
        <label className="flex flex-col gap-1.5 text-sm font-medium" htmlFor="bk-every">
          {t('manage.backups.every')}
          <Select
            id="bk-every"
            disabled={!draft.enabled}
            value={String(draft.interval_hours)}
            onValueChange={(v) => setDraft({ ...draft, interval_hours: Number(v) })}
            options={[1, 6, 12, 24, 48, 168].map((h) => ({
              value: String(h),
              label: t('manage.backups.hours', { count: h }),
            }))}
          />
        </label>
        <label className="flex flex-col gap-1.5 text-sm font-medium" htmlFor="bk-keep">
          {t('manage.backups.keep')}
          <Select
            id="bk-keep"
            disabled={!draft.enabled}
            value={String(draft.keep)}
            onValueChange={(v) => setDraft({ ...draft, keep: Number(v) })}
            options={[3, 5, 7, 14, 30].map((k) => ({
              value: String(k),
              label: t('manage.backups.keepCount', { count: k }),
            }))}
          />
        </label>
        <Button
          variant="primary"
          disabled={!dirty}
          loading={save.isPending}
          onClick={() => save.mutate(draft)}
          className="self-end"
        >
          {t('manage.save')}
        </Button>
      </div>
    </Panel>
  );
}
