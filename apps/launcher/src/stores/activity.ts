/**
 * Live backend activity: launch states per instance and background downloads, fed by IPC events.
 * `bindActivity()` is called once at startup.
 */
import { create } from 'zustand';
import { invoke, listen } from '@/ipc';
import type { DownloadTask } from '@/ipc/types/app';
import type { LaunchStatus } from '@/ipc/types/instances';

interface ActivityState {
  launches: Record<string, LaunchStatus>;
  downloads: Record<string, DownloadTask>;
  setLaunch: (s: LaunchStatus) => void;
  upsertDownload: (t: DownloadTask) => void;
  setDownloads: (list: DownloadTask[]) => void;
}

export const useActivity = create<ActivityState>()((set) => ({
  launches: {},
  downloads: {},
  setLaunch: (s) => set((st) => ({ launches: { ...st.launches, [s.instance_id]: s } })),
  upsertDownload: (t) =>
    set((st) => {
      const next = { ...st.downloads };
      if (t.state === 'done' || t.state === 'cancelled') delete next[t.id];
      else next[t.id] = t;
      return { downloads: next };
    }),
  setDownloads: (list) =>
    set({
      downloads: Object.fromEntries(list.filter((t) => t.state !== 'done').map((t) => [t.id, t])),
    }),
}));

export interface DownloadSummary {
  count: number;
  /** 0..1 or null when some totals are unknown. */
  progress: number | null;
  speed_bps: number;
  received: number;
  total: number;
}

/** Aggregates every running download for the global progress bar. */
export function summarizeDownloads(tasks: DownloadTask[]): DownloadSummary {
  const running = tasks.filter((t) => t.state === 'running' || t.state === 'queued');
  const known = running.filter((t) => t.total_bytes != null && t.total_bytes > 0);
  const total = known.reduce((a, t) => a + (t.total_bytes ?? 0), 0);
  const received = known.reduce((a, t) => a + t.received_bytes, 0);
  return {
    count: running.length,
    progress:
      running.length === 0
        ? null
        : known.length === running.length && total > 0
          ? received / total
          : null,
    speed_bps: running.reduce((a, t) => a + t.speed_bps, 0),
    received,
    total,
  };
}

export function bindActivity(): () => void {
  const st = useActivity.getState();
  void invoke('launch_status').then((list) => list.forEach((s) => st.setLaunch(s)));
  void invoke('downloads_list').then((list) => st.setDownloads(list));
  const offs = [
    listen('launch-state', (s) => useActivity.getState().setLaunch(s)),
    listen('download-progress', (t) => useActivity.getState().upsertDownload(t)),
    listen('downloads-changed', (list) => useActivity.getState().setDownloads(list)),
  ];
  return () => offs.forEach((o) => o());
}
