import { describe, expect, it } from 'vitest';
import type { HorizonVersion } from '@/ipc/types/instances';
import { isRusticEdition } from './HorizonVersion';

const v = (id: string, rustic_edition: boolean) => ({ id, rustic_edition }) as HorizonVersion;

describe('isRusticEdition', () => {
  it('follows the backend version list', () => {
    const list = [v('nightly-2026-11-08-fdc87ae', false), v('v0.2.15', true)];
    expect(isRusticEdition('nightly-2026-11-08-fdc87ae', list)).toBe(false);
    expect(isRusticEdition('v0.2.15', list)).toBe(true);
  });
  it('never claims Rustic edition for an unknown build', () => {
    expect(isRusticEdition('v9.9.9', [])).toBe(false);
    expect(isRusticEdition('v9.9.9', undefined)).toBe(false);
  });
  it('lets an explicit answer win', () => {
    expect(isRusticEdition('x', [], true)).toBe(true);
  });
});
