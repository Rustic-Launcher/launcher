import { beforeEach, describe, expect, it } from 'vitest';
import type { DownloadTask } from '@/ipc/types/app';
import { summarizeDownloads, useActivity } from './activity';

const task = (p: Partial<DownloadTask>): DownloadTask => ({
  id: 't',
  kind: 'game',
  label: 'x',
  received_bytes: 0,
  total_bytes: 100,
  speed_bps: 0,
  state: 'running',
  ...p,
});

describe('summarizeDownloads', () => {
  it('is empty when nothing runs', () => {
    expect(summarizeDownloads([])).toMatchObject({ count: 0, progress: null });
  });
  it('aggregates byte progress and speed', () => {
    const s = summarizeDownloads([
      task({ id: 'a', received_bytes: 50, total_bytes: 100, speed_bps: 10 }),
      task({ id: 'b', received_bytes: 0, total_bytes: 100, speed_bps: 5 }),
    ]);
    expect(s.count).toBe(2);
    expect(s.progress).toBeCloseTo(0.25);
    expect(s.speed_bps).toBe(15);
  });
  it('is indeterminate when a total is unknown', () => {
    expect(summarizeDownloads([task({ total_bytes: null })]).progress).toBeNull();
  });
  it('ignores finished tasks', () => {
    expect(summarizeDownloads([task({ state: 'done' })]).count).toBe(0);
  });
});

describe('useActivity', () => {
  beforeEach(() => useActivity.setState({ launches: {}, downloads: {} }));
  it('removes downloads once done', () => {
    const { upsertDownload } = useActivity.getState();
    upsertDownload(task({ id: 'a' }));
    expect(Object.keys(useActivity.getState().downloads)).toEqual(['a']);
    upsertDownload(task({ id: 'a', state: 'done' }));
    expect(useActivity.getState().downloads).toEqual({});
  });
  it('tracks launch state per instance', () => {
    useActivity.getState().setLaunch({ instance_id: 'i1', phase: 'running', progress: null });
    expect(useActivity.getState().launches.i1?.phase).toBe('running');
  });
});
