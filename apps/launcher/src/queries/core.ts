/** TanStack Query hooks for the core IPC data (instances, accounts, settings, versions). */
import { QueryClient, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { invoke, listen } from '@/ipc';
import type { SettingsPatch } from '@/ipc/types/app';
import type { Instance } from '@/ipc/types/instances';
import { usePrefs } from '@/stores/prefs';

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, refetchOnWindowFocus: false, retry: 1 },
  },
});

export const qk = {
  instances: ['instances'] as const,
  instance: (id: string) => ['instances', id] as const,
  instanceContent: (id: string, kind?: string) =>
    ['instances', id, 'content', kind ?? 'all'] as const,
  instanceWorlds: (id: string) => ['instances', id, 'worlds'] as const,
  versions: ['horizon-versions'] as const,
  accounts: ['accounts'] as const,
  rustic: ['rustic-session'] as const,
  settings: ['settings'] as const,
  system: ['system'] as const,
  java: ['java-runtimes'] as const,
  prepare: ['game-prepare'] as const,
  rusticServer: ['rustic-server'] as const,
  appInfo: ['app-info'] as const,
};

export const useInstances = () =>
  useQuery({ queryKey: qk.instances, queryFn: () => invoke('instances_list') });
export const useHorizonVersions = () =>
  useQuery({
    queryKey: qk.versions,
    queryFn: () => invoke('horizon_versions_list'),
    staleTime: 5 * 60_000,
  });
export const useAccounts = () =>
  useQuery({ queryKey: qk.accounts, queryFn: () => invoke('auth_accounts_list') });
export const useRusticSession = () =>
  useQuery({ queryKey: qk.rustic, queryFn: () => invoke('auth_rustic_session') });
export const useSettings = () =>
  useQuery({ queryKey: qk.settings, queryFn: () => invoke('settings_get') });
export const useSystemInfo = () =>
  useQuery({ queryKey: qk.system, queryFn: () => invoke('system_info'), staleTime: Infinity });
export const useAppInfo = () =>
  useQuery({ queryKey: qk.appInfo, queryFn: () => invoke('app_info'), staleTime: Infinity });

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: SettingsPatch) => invoke('settings_set', patch),
    onSuccess: (s) => qc.setQueryData(qk.settings, s),
  });
}

/** Picks the instance shown in the bottom bar: the user's choice, else pinned, else last played. */
export function pickActiveInstance(
  list: Instance[],
  preferredId: string | null,
): Instance | undefined {
  const byId = preferredId ? list.find((i) => i.id === preferredId) : undefined;
  if (byId) return byId;
  const pinned = list.find((i) => i.pinned);
  if (pinned) return pinned;
  return [...list].sort((a, b) =>
    (b.last_played_at ?? '').localeCompare(a.last_played_at ?? ''),
  )[0];
}

export function useActiveInstance(): { instance: Instance | undefined; isLoading: boolean } {
  const { data, isLoading } = useInstances();
  const preferred = usePrefs((s) => s.activeInstanceId);
  const instance = useMemo(
    () => (data ? pickActiveInstance(data, preferred) : undefined),
    [data, preferred],
  );
  return { instance, isLoading };
}

/** Keeps the cache in sync with backend push events. */
export function bindCoreEvents(qc: QueryClient): () => void {
  const offs = [
    listen('instances-changed', (list) => {
      qc.setQueryData(qk.instances, list);
      void qc.invalidateQueries({ queryKey: qk.instances, refetchType: 'none' });
    }),
    listen('accounts-changed', (list) => qc.setQueryData(qk.accounts, list)),
  ];
  return () => offs.forEach((o) => o());
}
