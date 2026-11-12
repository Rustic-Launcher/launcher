import { Button, Checkbox, Dialog, Input, toast } from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { Trash2, TriangleAlert } from 'lucide-react';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { Server } from '@/ipc/types/servers';
import { serverKeys } from '../list/queries';
import { Panel } from './shared';

export function DangerTab({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [deleteFiles, setDeleteFiles] = useState(false);
  const [confirm, setConfirm] = useState('');
  const checkId = useId();
  const vps = server.hosting.mode === 'vps';
  const del = useMutation({
    mutationFn: () => invoke('server_delete', { id: server.id, delete_files: deleteFiles }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: serverKeys.all });
      toast({ tone: 'success', title: t('manage.danger.deleted', { name: server.name }) });
      void navigate({ to: '/servers' });
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('manage.danger.failed'), description: e.message }),
  });
  return (
    <Panel
      className="border-danger/30"
      title={vps ? t('manage.danger.removeVpsTitle') : t('manage.danger.deleteTitle')}
      description={
        vps
          ? t('manage.danger.removeVpsBody', {
              host: server.hosting.mode === 'vps' ? server.hosting.host : '',
            })
          : t('manage.danger.deleteBody')
      }
      action={
        <Button variant="danger" leftIcon={<Trash2 />} onClick={() => setOpen(true)}>
          {vps ? t('manage.danger.removeVps') : t('manage.danger.delete')}
        </Button>
      }
    >
      <Dialog
        open={open}
        onOpenChange={setOpen}
        tone="destructive"
        icon={<TriangleAlert />}
        title={t('manage.danger.confirmTitle', { name: server.name })}
        description={t('manage.danger.confirmBody')}
        closeLabel={t('manage.close')}
        footer={
          <>
            <Button onClick={() => setOpen(false)}>{t('manage.cancel')}</Button>
            <Button
              variant="danger"
              disabled={confirm !== server.name}
              loading={del.isPending}
              onClick={() => del.mutate()}
            >
              {vps ? t('manage.danger.removeVps') : t('manage.danger.delete')}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          <div className="flex items-start gap-2.5">
            <Checkbox
              id={checkId}
              checked={deleteFiles}
              onCheckedChange={(v) => setDeleteFiles(v === true)}
            />
            <label htmlFor={checkId} className="text-fg text-sm">
              {t('manage.danger.deleteFiles')}
            </label>
          </div>
          <Input
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            placeholder={server.name}
            aria-label={t('manage.danger.typeName', { name: server.name })}
          />
          <p className="text-fg-subtle text-xs">
            {t('manage.danger.typeName', { name: server.name })}
          </p>
        </div>
      </Dialog>
    </Panel>
  );
}
