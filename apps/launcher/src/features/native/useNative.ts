/**
 * Native mods (ADR 0157): queries and actions over `native_*` commands, and the build task an
 * instance has in the downloads list.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { invoke, listen } from '@/ipc';
import type { DownloadTask } from '@/ipc/types/app';
import { useActivity } from '@/stores/activity';

export const nk = {
  all: ['native'] as const,
  status: (instanceId: string) => ['native', 'status', instanceId] as const,
  cache: ['native', 'cache'] as const,
};

/** Refetches native statuses when a build or Rust's installation ends. */
function useNativeEvents() {
  const qc = useQueryClient();
  useEffect(
    () => listen('native-changed', () => void qc.invalidateQueries({ queryKey: nk.all })),
    [qc],
  );
}

export function useNativeStatus(instanceId: string | undefined) {
  useNativeEvents();
  return useQuery({
    queryKey: nk.status(instanceId ?? ''),
    queryFn: () => invoke('native_status', { instance_id: instanceId! }),
    enabled: !!instanceId,
  });
}

export function useNativeCache() {
  useNativeEvents();
  return useQuery({ queryKey: nk.cache, queryFn: () => invoke('native_cache_info') });
}

const live = (t: DownloadTask) => t.state === 'running' || t.state === 'queued';

/** The instance's native build task: running, or the last one that failed. */
export function useNativeTask(instanceId: string | undefined): DownloadTask | undefined {
  return useActivity((s) => {
    if (!instanceId) return undefined;
    const mine = Object.values(s.downloads).filter(
      (d) => d.kind === 'native-build' && d.instance_id === instanceId,
    );
    return mine.find(live) ?? mine.find((d) => d.state === 'failed');
  });
}

/** The launcher's Rust installation task, while it runs or after it failed. */
export function useToolchainTask(): DownloadTask | undefined {
  return useActivity((s) => {
    const all = Object.values(s.downloads).filter((d) => d.kind === 'native-toolchain');
    return all.find(live) ?? all.find((d) => d.state === 'failed');
  });
}

export function taskFraction(t: DownloadTask | undefined): number | null {
  if (!t || !t.total_bytes) return null;
  return Math.min(1, t.received_bytes / t.total_bytes);
}

/**
 * Resolves when the task `taskId` ends: `true` done, `false` failed or cancelled. Finished tasks
 * leave the activity store (done, cancelled) or stay with `failed`.
 */
export function waitForTask(taskId: string): Promise<boolean> {
  return new Promise((resolve) => {
    let seen = false;
    const check = (): boolean => {
      const t = useActivity.getState().downloads[taskId];
      if (t) {
        seen = true;
        if (t.state === 'failed' || t.state === 'cancelled') {
          resolve(false);
          return true;
        }
        return false;
      }
      if (seen) {
        resolve(true);
        return true;
      }
      return false;
    };
    if (check()) return;
    const off = useActivity.subscribe(() => {
      if (check()) off();
    });
    // A task that ended before the store heard of it: its final event says how.
    const stop = listen('download-progress', (t) => {
      if (t.id !== taskId) return;
      if (t.state === 'done') {
        stop();
        off();
        resolve(true);
      } else if (t.state === 'failed' || t.state === 'cancelled') {
        stop();
        off();
        resolve(false);
      }
    });
  });
}

export function useNativeActions() {
  const qc = useQueryClient();
  const refresh = () => void qc.invalidateQueries({ queryKey: nk.all });
  return {
    consent: useMutation({
      mutationFn: (v: { instanceId: string; accept: boolean }) =>
        invoke('native_consent', { instance_id: v.instanceId, accept: v.accept }),
      onSuccess: (s) => qc.setQueryData(nk.status(s.instance_id), s),
    }),
    build: useMutation({
      mutationFn: (instanceId: string) => invoke('native_build', { instance_id: instanceId }),
      onSettled: refresh,
    }),
    installToolchain: useMutation({
      mutationFn: () => invoke('native_toolchain_install'),
      onSettled: refresh,
    }),
    cancel: useMutation({
      mutationFn: (taskId: string) => invoke('download_cancel', { id: taskId }),
      onSettled: refresh,
    }),
    clearCache: useMutation({
      mutationFn: () => invoke('native_cache_clear'),
      onSettled: refresh,
    }),
    setLimit: useMutation({
      mutationFn: (gib: number) => invoke('native_cache_set_limit', { cache_limit_gib: gib }),
      onSuccess: (info) => qc.setQueryData(nk.cache, info),
    }),
  };
}
