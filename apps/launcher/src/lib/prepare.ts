import type { PrepareProgress } from '@/ipc/types/app';

/**
 * Outcome of a `prepare-progress` event. The desktop side ends a failed preparation with
 * `{ step: 'done', message: <error> }` and a progress below 1: that is a failure, not a success.
 */
export function prepareOutcome(p: PrepareProgress): 'running' | 'ok' | 'failed' {
  if (p.step !== 'done') return 'running';
  return p.message && p.progress < 1 ? 'failed' : 'ok';
}
