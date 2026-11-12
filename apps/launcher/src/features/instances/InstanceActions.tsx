import {
  Button,
  Dialog,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownSeparator,
  DropdownTrigger,
  IconButton,
  toast,
} from '@rustic/ui';
import { useNavigate } from '@tanstack/react-router';
import { Copy, FolderOpen, MoreHorizontal, Pin, PinOff, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Instance } from '@/ipc/types/instances';
import { usePrefs } from '@/stores/prefs';
import { useInstanceMutations } from './queries';

/** "More" menu of an instance: open folder, pin, duplicate, delete (with confirmation). */
export function InstanceActions({
  instance,
  afterDelete,
}: {
  instance: Instance;
  afterDelete?: () => void;
}) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const m = useInstanceMutations();
  const navigate = useNavigate();
  const activeId = usePrefs((s) => s.activeInstanceId);
  const setActive = usePrefs((s) => s.setActiveInstance);
  const [confirm, setConfirm] = useState(false);

  return (
    <>
      <Dropdown>
        <DropdownTrigger asChild>
          <IconButton
            size="sm"
            tooltip={false}
            label={t('card.more', { name: instance.name })}
            icon={<MoreHorizontal />}
            data-testid={`instance-menu-${instance.id}`}
          />
        </DropdownTrigger>
        <DropdownContent>
          <DropdownItem
            icon={<FolderOpen />}
            onSelect={() => m.openFolder.mutate({ id: instance.id, folder: 'root' })}
          >
            {tc('actions.openFolder')}
          </DropdownItem>
          <DropdownItem
            icon={instance.pinned ? <PinOff /> : <Pin />}
            onSelect={() =>
              m.update.mutate({ id: instance.id, patch: { pinned: !instance.pinned } })
            }
          >
            {instance.pinned ? t('card.unpin') : t('card.pin')}
          </DropdownItem>
          <DropdownItem
            icon={<Copy />}
            onSelect={() =>
              m.duplicate.mutate(instance.id, {
                onSuccess: (copy) =>
                  toast({ tone: 'success', title: t('card.duplicated', { name: copy.name }) }),
              })
            }
          >
            {t('card.duplicate')}
          </DropdownItem>
          <DropdownSeparator />
          <DropdownItem icon={<Trash2 />} destructive onSelect={() => setConfirm(true)}>
            {tc('actions.delete')}
          </DropdownItem>
        </DropdownContent>
      </Dropdown>
      <Dialog
        open={confirm}
        onOpenChange={setConfirm}
        tone="destructive"
        icon={<Trash2 />}
        title={t('delete.title', { name: instance.name })}
        description={t('delete.body')}
        closeLabel={tc('actions.close')}
        footer={
          <>
            <Button onClick={() => setConfirm(false)}>{tc('actions.cancel')}</Button>
            <Button
              variant="danger"
              loading={m.remove.isPending}
              data-testid="confirm-delete-instance"
              onClick={() =>
                m.remove.mutate(instance.id, {
                  onSuccess: () => {
                    setConfirm(false);
                    if (activeId === instance.id) setActive(null);
                    toast({ tone: 'success', title: t('delete.done', { name: instance.name }) });
                    if (afterDelete) afterDelete();
                    else void navigate({ to: '/instances' });
                  },
                })
              }
            >
              {t('delete.confirm')}
            </Button>
          </>
        }
      />
    </>
  );
}
