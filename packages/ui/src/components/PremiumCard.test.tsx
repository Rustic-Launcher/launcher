import { fireEvent, render } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PremiumCard, PremiumGrid } from './PremiumCard';

const raf = (cb: FrameRequestCallback) => {
  cb(0);
  return 1;
};

afterEach(() => vi.restoreAllMocks());

function mockMedia(matches: boolean) {
  vi.spyOn(window, 'matchMedia').mockImplementation(
    (q: string) =>
      ({
        matches: q.includes('reduce') ? false : matches,
        media: q,
        addEventListener() {},
        removeEventListener() {},
      }) as unknown as MediaQueryList,
  );
}

describe('PremiumCard', () => {
  it('renders the layers, the variant and the body', () => {
    const { container, getByText } = render(
      <PremiumCard variant="featured" as="article">
        hello
      </PremiumCard>,
    );
    const card = container.firstElementChild as HTMLElement;
    expect(card.tagName).toBe('ARTICLE');
    expect(card.dataset.variant).toBe('featured');
    expect(card.hasAttribute('data-interactive')).toBe(true);
    expect([...card.children].map((c) => c.className)).toEqual([
      'rx-pcard-glass',
      'rx-pcard-spot',
      'rx-pcard-edge',
      'rx-pcard-body',
    ]);
    // Decorative layers stay out of the accessibility tree.
    expect(card.querySelectorAll('[aria-hidden]')).toHaveLength(3);
    expect(getByText('hello').className).toBe('rx-pcard-body');
  });

  it('follows the pointer (spotlight + tilt) and settles back on leave', () => {
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(raf);
    mockMedia(true);
    const { container } = render(<PremiumCard tilt={10}>x</PremiumCard>);
    const card = container.firstElementChild as HTMLElement;
    vi.spyOn(card, 'getBoundingClientRect').mockReturnValue({
      left: 100,
      top: 50,
      width: 200,
      height: 100,
    } as DOMRect);
    fireEvent.pointerMove(card, { clientX: 300, clientY: 50 });
    expect(card.style.getPropertyValue('--mx')).toBe('200px');
    expect(card.style.getPropertyValue('--my')).toBe('0px');
    expect(card.style.getPropertyValue('--ry')).toBe('5.00deg');
    expect(card.style.getPropertyValue('--rx')).toBe('5.00deg');
    expect(card.hasAttribute('data-tilting')).toBe(true);
    fireEvent.pointerLeave(card);
    expect(card.style.getPropertyValue('--rx')).toBe('0deg');
    expect(card.hasAttribute('data-tilting')).toBe(false);
  });

  it('does not tilt on touch / small screens / reduced motion, and not when static', () => {
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(raf);
    mockMedia(false);
    const { container } = render(<PremiumCard>x</PremiumCard>);
    const card = container.firstElementChild as HTMLElement;
    fireEvent.pointerMove(card, { clientX: 10, clientY: 10 });
    expect(card.hasAttribute('data-tilting')).toBe(false);
    const still = render(<PremiumCard interactive={false}>y</PremiumCard>).container
      .firstElementChild as HTMLElement;
    fireEvent.pointerMove(still, { clientX: 10, clientY: 10 });
    expect(still.style.getPropertyValue('--mx')).toBe('');
  });

  it('a grid shares one spotlight across its cards', () => {
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(raf);
    const { container } = render(
      <PremiumGrid>
        <PremiumCard>a</PremiumCard>
        <PremiumCard>b</PremiumCard>
      </PremiumGrid>,
    );
    const grid = container.firstElementChild as HTMLElement;
    fireEvent.pointerMove(grid, { clientX: 40, clientY: 30 });
    expect(grid.hasAttribute('data-lit')).toBe(true);
    const cards = grid.querySelectorAll<HTMLElement>('.rx-pcard');
    cards.forEach((c) => expect(c.style.getPropertyValue('--mx')).toBe('40px'));
    fireEvent.pointerLeave(grid);
    expect(grid.hasAttribute('data-lit')).toBe(false);
  });

  it('animates only transform and opacity, and honours reduced motion', () => {
    const css = readFileSync(join(__dirname, '../premium-card.css'), 'utf8');
    const transitions = [...css.matchAll(/transition:\s*([^;]+);/g)].map((m) => m[1]!);
    for (const t of transitions) {
      if (t.trim() === 'none') continue;
      expect(t).toMatch(/^(transform|opacity)\b/);
    }
    expect(css).toMatch(/prefers-reduced-motion: reduce/);
  });
});
