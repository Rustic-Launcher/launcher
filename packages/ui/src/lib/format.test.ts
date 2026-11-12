import { describe, expect, it } from 'vitest';
import { clamp, formatBytes } from './format';
import { cn } from './cn';

describe('formatBytes', () => {
  it('formats with binary units', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(4 * 1024 ** 3)).toBe('4 GB');
  });
  it('uses the locale decimal separator', () => {
    expect(formatBytes(1536, 'fr')).toBe('1,5 KB');
  });
});

describe('clamp', () => {
  it('clamps', () => {
    expect(clamp(5, 0, 3)).toBe(3);
    expect(clamp(-1, 0, 3)).toBe(0);
    expect(clamp(2, 0, 3)).toBe(2);
  });
});

describe('cn', () => {
  it('merges conflicting tailwind classes', () => {
    const hidden = false;
    expect(cn('px-2 text-sm', 'px-4', hidden && 'hidden')).toBe('text-sm px-4');
    expect(cn('text-xs', 'text-fg')).toBe('text-xs text-fg');
  });
});
