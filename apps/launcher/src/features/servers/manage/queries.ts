/** Queries and live subscriptions for the server management page. */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { invoke, listen } from '@/ipc';
import type { DomainBinding } from '@/ipc/types/dns';
import type { ConsoleLine, Server, ServerAccess, ServerStats } from '@/ipc/types/servers';
import { serverKeys } from '../list/queries';

export const mk = {
  server: serverKeys.one,
  stats: (id: string) => ['servers', id, 'stats'] as const,
  console: (id: string) => ['servers', id, 'console'] as const,
  players: (id: string) => ['servers', id, 'players'] as const,
  config: (id: string) => ['servers', id, 'config'] as const,
  raw: (id: string) => ['servers', id, 'config-raw'] as const,
  plugins: (id: string) => ['servers', id, 'plugins'] as const,
  datapacks: (id: string) => ['servers', id, 'datapacks'] as const,
  backups: (id: string) => ['servers', id, 'backups'] as const,
  schedule: (id: string) => ['servers', id, 'backup-schedule'] as const,
  access: (id: string) => ['servers', id, 'access'] as const,
  domain: (id: string) => ['servers', id, 'domain'] as const,
  porkbun: ['dns', 'porkbun'] as const,
  ownedDomains: ['dns', 'porkbun', 'domains'] as const,
};

export function useServer(id: string) {
  const qc = useQueryClient();
  const q = useQuery({ queryKey: mk.server(id), queryFn: () => invoke('server_get', { id }) });
  useEffect(() => {
    const offs = [
      listen('server-state', ({ server_id, state }) => {
        if (server_id !== id) return;
        qc.setQueryData<Server>(mk.server(id), (old) => (old ? { ...old, state } : old));
        void qc.invalidateQueries({ queryKey: mk.server(id), exact: true });
        void qc.invalidateQueries({ queryKey: mk.players(id) });
      }),
      listen('servers-changed', (list) => {
        const s = list.find((x) => x.id === id);
        if (s) qc.setQueryData(mk.server(id), s);
      }),
      listen('server-players-changed', ({ server_id }) => {
        if (server_id !== id) return;
        void qc.invalidateQueries({ queryKey: mk.players(id) });
        void qc.invalidateQueries({ queryKey: mk.server(id), exact: true });
      }),
      listen('dns-status', (b) => {
        if (b.server_id !== id) return;
        qc.setQueryData<DomainBinding | null>(mk.domain(id), b);
        if (b.status === 'verified')
          void qc.invalidateQueries({ queryKey: mk.server(id), exact: true });
      }),
    ];
    return () => offs.forEach((o) => o());
  }, [id, qc]);
  return q;
}

/** Last `minutes` of stats, kept live from `server-stats` events. */
export function useLiveStats(id: string, minutes = 2) {
  const qc = useQueryClient();
  const q = useQuery({
    queryKey: mk.stats(id),
    queryFn: () => invoke('server_stats_history', { server_id: id, minutes }),
  });
  useEffect(
    () =>
      listen('server-stats', (s) => {
        if (s.server_id !== id) return;
        qc.setQueryData<ServerStats[]>(mk.stats(id), (old) =>
          [...(old ?? []), s].slice(-minutes * 30),
        );
      }),
    [id, qc, minutes],
  );
  return q;
}

/** Console history + live lines. */
export function useConsole(id: string, limit = 500) {
  const qc = useQueryClient();
  const q = useQuery({
    queryKey: mk.console(id),
    queryFn: () => invoke('server_console_history', { server_id: id, limit }),
    staleTime: Infinity,
    // Lines are only appended while the console is open: catch up on what was logged meanwhile
    // (another tab, another page) when it opens again.
    refetchOnMount: 'always',
  });
  useEffect(
    () =>
      listen('server-console', (line) => {
        if (line.server_id !== id) return;
        qc.setQueryData<ConsoleLine[]>(mk.console(id), (old) =>
          [...(old ?? []), line].slice(-limit),
        );
      }),
    [id, qc, limit],
  );
  return q;
}

export const usePlayers = (id: string) =>
  useQuery({
    queryKey: mk.players(id),
    queryFn: () => invoke('server_players', { server_id: id }),
  });
export const useConfig = (id: string) =>
  useQuery({
    queryKey: mk.config(id),
    queryFn: () => invoke('server_config_get', { server_id: id }),
  });
export const useRawConfig = (id: string, enabled: boolean) =>
  useQuery({
    queryKey: mk.raw(id),
    queryFn: () => invoke('server_config_raw_get', { server_id: id }),
    enabled,
  });
export const usePlugins = (id: string, enabled = true) =>
  useQuery({
    queryKey: mk.plugins(id),
    queryFn: () => invoke('server_plugins_list', { server_id: id }),
    enabled,
  });
export const useDatapacks = (id: string) =>
  useQuery({
    queryKey: mk.datapacks(id),
    queryFn: () => invoke('server_datapacks_list', { server_id: id }),
  });
export const useBackups = (id: string) =>
  useQuery({
    queryKey: mk.backups(id),
    queryFn: () => invoke('server_backups_list', { server_id: id }),
  });
export const useSchedule = (id: string) =>
  useQuery({
    queryKey: mk.schedule(id),
    queryFn: () => invoke('server_backup_schedule_get', { server_id: id }),
  });
export const useAccess = (id: string) =>
  useQuery({
    queryKey: mk.access(id),
    queryFn: () => invoke('server_access_get', { server_id: id }),
  });
export const useDomainBinding = (id: string) =>
  useQuery({ queryKey: mk.domain(id), queryFn: () => invoke('dns_status', { server_id: id }) });
export const usePorkbun = () =>
  useQuery({ queryKey: mk.porkbun, queryFn: () => invoke('dns_porkbun_status') });

/** Mutation that writes the returned access object into the cache. */
export function useAccessMutation<A>(id: string, fn: (a: A) => Promise<ServerAccess>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (acc) => {
      qc.setQueryData(mk.access(id), acc);
      void qc.invalidateQueries({ queryKey: mk.server(id), exact: true });
    },
  });
}

/** Re-renders every `ms` (for "checked 3 s ago" texts). */
export function useNow(ms = 1000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), ms);
    return () => clearInterval(t);
  }, [ms]);
  return now;
}
