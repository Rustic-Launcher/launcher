import type { DownloadKind, DownloadTask } from '../types/app';
import { emit, simulate, uid } from './runtime';

const tasks = new Map<string, DownloadTask>();
const cancels = new Map<string, () => void>();

function changed() {
  emit('downloads-changed', [...tasks.values()]);
}

export function listDownloads(): DownloadTask[] {
  return [...tasks.values()];
}

export interface StartDownload {
  kind: DownloadKind;
  label: string;
  total_bytes: number | null;
  duration_ms: number;
  instance_id?: string;
  server_id?: string;
  onProgress?: (p: number) => void;
  onDone?: () => void;
  onCancel?: () => void;
}

/** Starts a simulated background download that reports through the global progress events. */
export function startDownload(o: StartDownload): string {
  const id = uid('task');
  const task: DownloadTask = {
    id,
    kind: o.kind,
    label: o.label,
    instance_id: o.instance_id,
    server_id: o.server_id,
    received_bytes: 0,
    total_bytes: o.total_bytes,
    speed_bps: 0,
    state: 'running',
  };
  tasks.set(id, task);
  changed();
  let last = 0;
  let lastT = Date.now();
  const cancel = simulate(
    o.duration_ms,
    (p) => {
      const t = tasks.get(id);
      if (!t) return;
      const received = Math.round((o.total_bytes ?? 0) * p);
      const dt = Math.max(1, Date.now() - lastT) / 1000;
      t.speed_bps = Math.round((received - last) / dt) || t.speed_bps;
      last = received;
      lastT = Date.now();
      t.received_bytes = received;
      emit('download-progress', { ...t });
      o.onProgress?.(p);
    },
    () => {
      const t = tasks.get(id);
      if (!t) return;
      t.state = 'done';
      t.speed_bps = 0;
      emit('download-progress', { ...t });
      tasks.delete(id);
      cancels.delete(id);
      changed();
      o.onDone?.();
    },
  );
  cancels.set(id, () => {
    cancel();
    o.onCancel?.();
  });
  return id;
}

export function cancelDownload(id: string): void {
  const t = tasks.get(id);
  cancels.get(id)?.();
  cancels.delete(id);
  if (t) {
    t.state = 'cancelled';
    emit('download-progress', { ...t });
    tasks.delete(id);
    changed();
  }
}

export function cancelDownloadsFor(instanceId: string): void {
  for (const t of [...tasks.values()]) if (t.instance_id === instanceId) cancelDownload(t.id);
}
