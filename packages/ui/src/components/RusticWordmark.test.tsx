import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { RUSTIC_WORDMARK_RATIO, RusticWordmark } from './RusticWordmark';

describe('RusticWordmark', () => {
  it('keeps the wordmark proportions and is named for assistive technology', () => {
    const { container } = render(<RusticWordmark height={20} />);
    const root = container.querySelector<HTMLElement>('[data-rustic-wordmark]')!;
    expect(root.getAttribute('role')).toBe('img');
    expect(root.getAttribute('aria-label')).toBe('Rustic');
    expect(root.style.height).toBe('20px');
    expect(root.style.width).toBe(`${Math.round(20 * RUSTIC_WORDMARK_RATIO)}px`);
  });

  it('paints the letters with the text colour and keeps the orange layer as is', () => {
    const { container } = render(<RusticWordmark />);
    expect(container.querySelector('[data-wordmark-letters]')?.className).toContain('bg-current');
    expect(container.querySelector('[data-wordmark-accent]')?.getAttribute('alt')).toBe('');
  });

  it('can be hidden from assistive technology when its parent is labelled', () => {
    const { container } = render(<RusticWordmark title={null} />);
    const root = container.querySelector('[data-rustic-wordmark]')!;
    expect(root.getAttribute('aria-hidden')).toBe('true');
    expect(root.getAttribute('role')).toBeNull();
  });
});
