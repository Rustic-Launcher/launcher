/** Queries for the servers list, multiplayer list and hosting connections. */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { invoke, listen } from '@/ipc';
import type { MultiplayerEntry } from '@/ipc/types/multiplayer';
import type { Server } from '@/ipc/types/servers';

export const serverKeys = {
  all: ['servers'] as const,
  one: (id: string) => ['servers', id] as const,
  multiplayer: ['multiplayer'] as const,
  ping: (address: string) => ['multiplayer', 'ping', address] as const,
  vps: ['hosting', 'vps'] as const,
  providers: ['hosting', 'providers'] as const,
};

export const useServers = () =>
  useQuery({ queryKey: serverKeys.all, queryFn: () => invoke('servers_list') });

/** Keeps the servers cache live from backend events. */
export function useServersLive(): void {
  const qc = useQueryClient();
  useEffect(() => {
    const offs = [
      listen('servers-changed', (list) => {
        qc.setQueryData(serverKeys.all, list);
        for (const s of list) qc.setQueryData(serverKeys.one(s.id), s);
      }),
      listen('server-state', ({ server_id, state }) => {
        qc.setQueryData<Server[]>(serverKeys.all, (old) =>
          old?.map((s) => (s.id === server_id ? { ...s, state } : s)),
        );
      }),
    ];
    return () => offs.forEach((o) => o());
  }, [qc]);
}

export function useServerPower() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, action }: { id: string; action: 'start' | 'stop' | 'restart' }) =>
      invoke(
        action === 'start' ? 'server_start' : action === 'stop' ? 'server_stop' : 'server_restart',
        { id },
      ),
    onSuccess: (s) => {
      qc.setQueryData<Server[]>(serverKeys.all, (old) => old?.map((x) => (x.id === s.id ? s : x)));
    },
  });
}

export const useMultiplayer = () =>
  useQuery({ queryKey: serverKeys.multiplayer, queryFn: () => invoke('multiplayer_list') });

export const usePing = (address: string) =>
  useQuery({
    queryKey: serverKeys.ping(address),
    queryFn: () => invoke('multiplayer_ping', { address }),
    staleTime: 60_000,
    retry: false,
  });

export function useMultiplayerActions() {
  const qc = useQueryClient();
  const set = (fn: (old: MultiplayerEntry[]) => MultiplayerEntry[]) =>
    qc.setQueryData<MultiplayerEntry[]>(serverKeys.multiplayer, (old) => fn(old ?? []));
  return {
    add: useMutation({
      mutationFn: (v: { name: string; address: string }) => invoke('multiplayer_add', v),
      onSuccess: (e) => set((old) => [...old, e]),
    }),
    update: useMutation({
      mutationFn: (v: {
        id: string;
        patch: Partial<Pick<MultiplayerEntry, 'name' | 'address' | 'favorite'>>;
      }) => invoke('multiplayer_update', v),
      onMutate: (v) => set((old) => old.map((e) => (e.id === v.id ? { ...e, ...v.patch } : e))),
      onSuccess: (e) => set((old) => old.map((x) => (x.id === e.id ? e : x))),
      onError: () => void qc.invalidateQueries({ queryKey: serverKeys.multiplayer }),
    }),
    remove: useMutation({
      mutationFn: (id: string) => invoke('multiplayer_remove', { id }),
      onMutate: (id) => set((old) => old.filter((e) => e.id !== id)),
    }),
  };
}

/** Favourites first, then most recently joined, then name. */
export function sortMultiplayer(list: MultiplayerEntry[]): MultiplayerEntry[] {
  return [...list].sort(
    (a, b) =>
      Number(b.favorite) - Number(a.favorite) ||
      (b.last_joined_at ?? '').localeCompare(a.last_joined_at ?? '') ||
      a.name.localeCompare(b.name),
  );
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
