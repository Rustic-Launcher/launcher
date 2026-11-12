/** Instance-area queries and mutations (IPC). */
import { toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { ContentKind } from '@/ipc/types/content';
import type {
  CreateInstanceRequest,
  ImportInstanceRequest,
  Instance,
  InstanceFolder,
  InstancePatch,
} from '@/ipc/types/instances';
import { qk, useInstances } from '@/queries/core';

/** One instance, read from the list cache (kept fresh by `instances-changed`). */
export function useInstance(id: string) {
  const list = useInstances();
  const fromList = list.data?.find((i) => i.id === id);
  const single = useQuery({
    queryKey: qk.instance(id),
    queryFn: () => invoke('instance_get', { id }),
    enabled: !!list.data && !fromList,
    retry: false,
  });
  return {
    data: fromList ?? single.data,
    isLoading: list.isLoading || (single.isLoading && single.fetchStatus !== 'idle'),
    isError: list.isError || single.isError,
    refetch: () => void list.refetch(),
  };
}

export const useInstanceContent = (id: string, kind?: ContentKind) =>
  useQuery({
    queryKey: qk.instanceContent(id, kind),
    queryFn: () => invoke('instance_content_list', { instance_id: id, kind }),
  });

export const useInstanceWorlds = (id: string) =>
  useQuery({
    queryKey: qk.instanceWorlds(id),
    queryFn: () => invoke('instance_worlds_list', { instance_id: id }),
  });

function useErrorToast() {
  const { t } = useTranslation('instances');
  return (e: unknown) =>
    toast({ tone: 'error', title: t('errors.action'), description: (e as Error).message });
}

export function useInstanceMutations() {
  const qc = useQueryClient();
  const onError = useErrorToast();
  const refresh = () => qc.invalidateQueries({ queryKey: qk.instances });
  const setOne = (inst: Instance) =>
    qc.setQueryData<Instance[]>(qk.instances, (old) =>
      old?.map((i) => (i.id === inst.id ? inst : i)),
    );
  return {
    create: useMutation({
      mutationFn: (req: CreateInstanceRequest) => invoke('instance_create', req),
      onSuccess: refresh,
      onError,
    }),
    importInstance: useMutation({
      mutationFn: (req: ImportInstanceRequest) => invoke('instance_import', req),
      onSuccess: refresh,
      onError,
    }),
    update: useMutation({
      mutationFn: (v: { id: string; patch: InstancePatch }) => invoke('instance_update', v),
      onSuccess: setOne,
      onError,
    }),
    remove: useMutation({
      mutationFn: (id: string) => invoke('instance_delete', { id }),
      onSuccess: refresh,
      onError,
    }),
    duplicate: useMutation({
      mutationFn: (id: string) => invoke('instance_duplicate', { id }),
      onSuccess: refresh,
      onError,
    }),
    setVersion: useMutation({
      mutationFn: (v: { id: string; horizon_version: string }) => invoke('instance_set_version', v),
      onSuccess: refresh,
      onError,
    }),
    openFolder: useMutation({
      mutationFn: (v: { id: string; folder: InstanceFolder }) => invoke('instance_open_folder', v),
      onError,
    }),
  };
}

export function useContentMutations(instanceId: string) {
  const qc = useQueryClient();
  const onError = useErrorToast();
  const refresh = () => {
    void qc.invalidateQueries({ queryKey: ['instances', instanceId, 'content'] });
  };
  return {
    setEnabled: useMutation({
      mutationFn: (v: { content_id: string; enabled: boolean }) =>
        invoke('instance_content_set_enabled', { instance_id: instanceId, ...v }),
      onSuccess: refresh,
      onError,
    }),
    remove: useMutation({
      mutationFn: (content_id: string) =>
        invoke('instance_content_remove', { instance_id: instanceId, content_id }),
      onSuccess: refresh,
      onError,
    }),
    update: useMutation({
      mutationFn: (content_id?: string) =>
        invoke('instance_content_update', { instance_id: instanceId, content_id }),
      onSuccess: () => setTimeout(refresh, 2500),
      onError,
    }),
    reorder: useMutation({
      mutationFn: (ordered_ids: string[]) =>
        invoke('instance_resource_packs_reorder', { instance_id: instanceId, ordered_ids }),
      onSuccess: (list) => qc.setQueryData(qk.instanceContent(instanceId, 'resourcepack'), list),
      onError,
    }),
  };
}
