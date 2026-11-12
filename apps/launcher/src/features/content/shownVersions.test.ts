import { describe, expect, it } from 'vitest';
import { shownVersions } from './ContentPage';

describe('shownVersions (Explore › Horizon versions)', () => {
  const list = Array.from({ length: 30 }, (_, i) => ({ id: `nightly-${i}` }));
  it('caps a long real list', () => {
    expect(shownVersions(list, null)).toHaveLength(6);
  });
  it('keeps the target build visible', () => {
    const out = shownVersions(list, 'nightly-20');
    expect(out).toHaveLength(6);
    expect(out.at(-1)?.id).toBe('nightly-20');
  });
});
