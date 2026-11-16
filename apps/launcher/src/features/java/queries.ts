/** Data of the Java Launchpad and the Bedrock panel (ADR 0172): queries, run states and logs. */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { create } from 'zustand';
import { invoke, listen } from '@/ipc';
import type { JavaLoader, JavaLogLine, JavaRunState } from '@/ipc/types/java';

export const jk = {
  profiles: ['java', 'profiles'] as const,
  versions: (snapshots: boolean, old: boolean) => ['java', 'versions', snapshots, old] as const,
  loaders: (loader: JavaLoader, game: string) => ['java', 'loaders', loader, game] as const,
  mods: (id: string) => ['java', 'mods', id] as const,
  modSearch: (id: string, q: string) => ['java', 'mod-search', id, q] as const,
  bedrock: ['bedrock'] as const,
};

export const useJavaProfiles = () =>
  useQuery({ queryKey: jk.profiles, queryFn: () => invoke('java_profiles_list') });

export const useJavaVersions = (snapshots: boolean, old: boolean) =>
  useQuery({
    queryKey: jk.versions(snapshots, old),
    queryFn: () => invoke('java_versions_list', { snapshots, old }),
    staleTime: 10 * 60_000,
  });

export const useLoaderVersions = (loader: JavaLoader, game: string) =>
  useQuery({
    queryKey: jk.loaders(loader, game),
    queryFn: () => invoke('java_loader_versions', { loader, game_version: game }),
    enabled: loader !== 'vanilla' && !!game,
    staleTime: 10 * 60_000,
  });

export const useJavaMods = (id: string | undefined, enabled: boolean) =>
  useQuery({
    queryKey: jk.mods(id ?? ''),
    queryFn: () => invoke('java_mods_list', { profile_id: id! }),
    enabled: !!id && enabled,
  });

export const useBedrock = () =>
  useQuery({ queryKey: jk.bedrock, queryFn: () => invoke('bedrock_status'), staleTime: 60_000 });

const LOG_CAP = 2000;

interface JavaLive {
  states: Record<string, JavaRunState>;
  logs: Record<string, JavaLogLine[]>;
  setState: (s: JavaRunState) => void;
  setStates: (list: JavaRunState[]) => void;
  pushLog: (l: JavaLogLine) => void;
  setLog: (id: string, lines: JavaLogLine[]) => void;
}

/** Run states and output lines, fed by `java-state` and `java-log`. */
export const useJavaLive = create<JavaLive>()((set) => ({
  states: {},
  logs: {},
  setState: (s) => set((o) => ({ states: { ...o.states, [s.profile_id]: s } })),
  setStates: (list) =>
    set((o) => ({
      states: { ...o.states, ...Object.fromEntries(list.map((s) => [s.profile_id, s])) },
    })),
  pushLog: (l) =>
    set((o) => {
      const prev = o.logs[l.profile_id] ?? [];
      const next = prev.length >= LOG_CAP ? [...prev.slice(1 - LOG_CAP), l] : [...prev, l];
      return { logs: { ...o.logs, [l.profile_id]: next } };
    }),
  setLog: (id, lines) => set((o) => ({ logs: { ...o.logs, [id]: lines.slice(-LOG_CAP) } })),
}));

/** Subscribes to the Java events while the Java Launchpad is shown (states reloaded on mount). */
export function useJavaEvents(selected: string | undefined) {
  const qc = useQueryClient();
  useEffect(() => {
    const { setState, setStates, pushLog } = useJavaLive.getState();
    void invoke('java_run_states').then(setStates);
    const offs = [
      listen('java-state', (s) => {
        setState(s);
        if (s.state === 'idle' || s.state === 'running' || s.state === 'crashed')
          void qc.invalidateQueries({ queryKey: jk.profiles });
      }),
      listen('java-log', pushLog),
      listen('java-profiles-changed', (list) => qc.setQueryData(jk.profiles, list)),
    ];
    return () => offs.forEach((f) => f());
  }, [qc]);
  useEffect(() => {
    if (!selected) return;
    void invoke('java_profile_log', { id: selected }).then((lines) =>
      useJavaLive.getState().setLog(selected, lines),
    );
  }, [selected]);
}
