/**
 * Discreet toasts for the update service (ADR 0162), and the launch an update restart
 * interrupted, started again once the new launcher's UI is up.
 */
import { toast } from '@rustic/ui';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, listen } from '@/ipc';
import type { UpdateStatus } from '@/ipc/types/app';
import { useUpdates } from '@/stores/updates';

/** The toast a change of state deserves, if any (pure: tested). */
export function updateNotice(
  prev: UpdateStatus | null,
  next: UpdateStatus,
): { key: string; tone: 'info' | 'success' | 'warning'; version?: string } | null {
  const was = prev?.phase;
  if (next.phase === 'downloading' && was !== 'downloading' && was !== 'installing') {
    return next.working_on === 'edition'
      ? { key: 'updates.toast.edition', tone: 'info', version: next.edition?.version }
      : { key: 'updates.toast.launcher', tone: 'info', version: next.launcher?.version };
  }
  if (next.phase === 'deferred' && was !== 'deferred') {
    return { key: 'updates.toast.deferred', tone: 'info' };
  }
  if (next.phase === 'restarting' && was !== 'restarting') {
    return { key: 'updates.toast.restarting', tone: 'info' };
  }
  if (next.phase === 'error' && (was === 'downloading' || was === 'installing')) {
    return { key: 'updates.toast.failed', tone: 'warning' };
  }
  return null;
}

export function useUpdateNotices(): void {
  const { t } = useTranslation();
  const prev = useRef<UpdateStatus | null>(null);
  const greeted = useRef(false);

  useEffect(
    () =>
      useUpdates.subscribe(({ status }) => {
        if (!status) return;
        if (!greeted.current) {
          greeted.current = true;
          if (status.updated_from)
            toast({
              tone: 'success',
              title: t('updates.toast.updated', { version: status.current_version }),
            });
        }
        const n = updateNotice(prev.current, status);
        prev.current = status;
        if (n)
          toast({
            tone: n.tone,
            title: t(n.key as 'updates.toast.launcher', { version: n.version ?? '' }),
          });
      }),
    [t],
  );

  useEffect(
    () =>
      listen('update-installed', (e) =>
        toast({ tone: 'success', title: t('updates.toast.editionDone', { version: e.version }) }),
      ),
    [t],
  );

  // A launch the update restart interrupted: start it again (once).
  useEffect(() => {
    invoke('app_take_pending_launch')
      .then((p) => {
        if (!p) return;
        toast({ tone: 'info', title: t('updates.toast.resuming') });
        return invoke('instance_launch', {
          instance_id: p.instance_id,
          options: p.options ?? undefined,
        });
      })
      .catch(() => undefined);
  }, [t]);
}
