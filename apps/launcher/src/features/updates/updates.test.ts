import { describe, expect, it } from 'vitest';
import type { UpdateStatus } from '@/ipc/types/app';
import { scenarioStatus } from '@/ipc/mock/handlers/updates';
import { updatePill } from '@/stores/updates';
import { updateNotice } from './useUpdateNotices';

const idle = scenarioStatus('none');
const at = (s: Partial<UpdateStatus>): UpdateStatus => ({ ...idle, ...s });

describe('update pill', () => {
  it('stays hidden unless something is happening or waiting', () => {
    expect(updatePill(null)).toBeNull();
    expect(updatePill(idle)).toBeNull();
    expect(updatePill(at({ phase: 'checking' }))).toBeNull();
    expect(updatePill(scenarioStatus('error'))).toBeNull();
  });

  it('shows progress, the restart, a deferral and the manual button', () => {
    expect(updatePill(scenarioStatus('downloading'))).toBe('working');
    expect(updatePill(at({ phase: 'installing', working_on: 'edition' }))).toBe('working');
    expect(updatePill(scenarioStatus('restarting'))).toBe('restarting');
    expect(updatePill(scenarioStatus('deferred'))).toBe('deferred');
    expect(updatePill(scenarioStatus('available'))).toBe('apply');
    expect(updatePill(at({ phase: 'available' }))).toBeNull();
  });
});

describe('update notices', () => {
  it('announces each step once', () => {
    const down = scenarioStatus('downloading');
    expect(updateNotice(idle, down)).toEqual({
      key: 'updates.toast.launcher',
      tone: 'info',
      version: '0.2.1',
    });
    expect(updateNotice(down, { ...down, progress: 0.9 })).toBeNull();
    expect(updateNotice(down, scenarioStatus('restarting'))?.key).toBe('updates.toast.restarting');
    expect(updateNotice(idle, scenarioStatus('deferred'))?.key).toBe('updates.toast.deferred');
    expect(updateNotice(scenarioStatus('deferred'), scenarioStatus('deferred'))).toBeNull();
  });

  it('names the edition build and warns when an install fails', () => {
    const edition = at({
      phase: 'downloading',
      working_on: 'edition',
      edition: { version: 'rustic-edition-x', notes: null, size: null, published_at: null },
    });
    expect(updateNotice(idle, edition)?.key).toBe('updates.toast.edition');
    expect(updateNotice(edition, at({ phase: 'error', error: 'x' }))?.tone).toBe('warning');
    // A failed background check is not worth a toast (Settings shows it).
    expect(updateNotice(at({ phase: 'checking' }), at({ phase: 'error', error: 'x' }))).toBeNull();
  });
});
