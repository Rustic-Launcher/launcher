/**
 * The update service's state (ADR 0162), mirrored from `update-state` events. `bindUpdates()` is
 * called once at startup.
 */
import { create } from 'zustand';
import { invoke, listen } from '@/ipc';
import type { UpdateStatus } from '@/ipc/types/app';

interface UpdatesState {
  status: UpdateStatus | null;
  set: (s: UpdateStatus) => void;
}

export const useUpdates = create<UpdatesState>()((set) => ({
  status: null,
  set: (status) => set({ status }),
}));

export function bindUpdates(): () => void {
  void invoke('app_update_status')
    .then((s) => useUpdates.getState().set(s))
    .catch(() => undefined);
  return listen('update-state', (s) => useUpdates.getState().set(s));
}

/** What the title bar shows: nothing while idle, checking or after a failure (Settings has it). */
export type UpdatePillKind = 'working' | 'restarting' | 'deferred' | 'apply' | null;

export function updatePill(s: UpdateStatus | null): UpdatePillKind {
  if (!s) return null;
  switch (s.phase) {
    case 'downloading':
    case 'installing':
      return 'working';
    case 'restarting':
      return 'restarting';
    case 'deferred':
      return 'deferred';
    case 'available':
      return s.launcher || s.edition ? 'apply' : null;
    default:
      return null;
  }
}
