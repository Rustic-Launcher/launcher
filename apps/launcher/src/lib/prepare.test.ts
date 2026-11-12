import { describe, expect, it } from 'vitest';
import { prepareOutcome } from './prepare';

describe('prepareOutcome', () => {
  it('reads running, success and failure', () => {
    expect(prepareOutcome({ step: 'export', progress: 0.4 })).toBe('running');
    expect(prepareOutcome({ step: 'done', progress: 1 })).toBe('ok');
    expect(prepareOutcome({ step: 'done', progress: 0, message: 'no verified owner' })).toBe(
      'failed',
    );
  });
});
