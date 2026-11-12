import { render } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { RusticLogo, RusticMark, RUSTIC_ORANGE } from './RusticMark';

describe('RusticLogo', () => {
  it('is the official mark: same paths as assets/brand, white part themable, fixed orange bar', () => {
    const { container } = render(<RusticLogo size={24} title="Rustic" />);
    const paths = [...container.querySelectorAll('path')];
    expect(paths.map((p) => p.getAttribute('fill'))).toEqual([
      'currentColor',
      'currentColor',
      RUSTIC_ORANGE,
    ]);
    const brand = readFileSync(join(__dirname, '../../../../assets/brand/rustic-mark.svg'), 'utf8');
    for (const p of paths) expect(brand).toContain(p.getAttribute('d')!);
    expect(container.querySelector('svg')?.getAttribute('aria-label')).toBe('Rustic');
    expect(RusticMark).toBe(RusticLogo);
  });
});
