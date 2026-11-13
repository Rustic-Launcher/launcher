import {
  Button,
  Dialog,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownSeparator,
  DropdownTrigger,
  cn,
  IconButton,
  toast,
} from '@rustic/ui';
import { useNavigate } from '@tanstack/react-router';
import {
  CircleCheck,
  Copy,
  FolderOpen,
  MoreHorizontal,
  Pin,
  PinOff,
  Settings2,
  Trash2,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Instance } from '@/ipc/types/instances';
import { useActiveInstance } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { useInstanceMutations } from './queries';

/**
 * "More" menu of an instance: open folder, edit its settings, make it the Launchpad's, pin,
 * duplicate, delete (with confirmation). Export has no IPC command yet, so it is not offered.
 */
export function InstanceActions({
  instance,
  afterDelete,
  triggerClassName,
  edit = true,
}: {
  instance: Instance;
  afterDelete?: () => void;
  /** Extra classes for the "..." button (over a picture on the Profiles cards). */
  triggerClassName?: string;
  /** Offer "Edit settings" (off on the settings page itself). */
  edit?: boolean;
}) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const m = useInstanceMutations();
  const navigate = useNavigate();
  const activeId = usePrefs((s) => s.activeInstanceId);
  const setActive = usePrefs((s) => s.setActiveInstance);
  const [confirm, setConfirm] = useState(false);
  const isActive = useActiveInstance().instance?.id === instance.id;

  return (
    <>
      <Dropdown>
        <DropdownTrigger asChild>
          <IconButton
            size="sm"
            tooltip={false}
            label={t('card.more', { name: instance.name })}
            icon={<MoreHorizontal />}
            className={cn(triggerClassName)}
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
          {edit && (
            <DropdownItem
              icon={<Settings2 />}
              data-testid={`instance-edit-${instance.id}`}
              onSelect={() =>
                void navigate({
                  to: '/instances/$instanceId',
                  params: { instanceId: instance.id },
                  search: { tab: 'settings' },
                })
              }
            >
              {t('profiles.menu.edit')}
            </DropdownItem>
          )}
          <DropdownItem
            icon={<CircleCheck />}
            disabled={isActive}
            data-testid={`instance-activate-${instance.id}`}
            onSelect={() => {
              setActive(instance.id);
              toast({
                tone: 'success',
                title: t('profiles.menu.activated', { name: instance.name }),
              });
            }}
          >
            {isActive ? t('profiles.menu.isActive') : t('profiles.menu.activate')}
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
