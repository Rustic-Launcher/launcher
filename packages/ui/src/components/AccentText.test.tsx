import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AccentText, plainText } from './AccentText';

describe('AccentText', () => {
  it('turns *word* into the serif-italic accent and keeps the rest as text', () => {
    const { container } = render(
      <h1>
        <AccentText text="Your *locker*, today" />
      </h1>,
    );
    const em = container.querySelector('em.rx-accent');
    expect(em?.textContent).toBe('locker');
    expect(container.textContent).toBe('Your locker, today');
  });

  it('leaves text without markers alone and strips markers for plain uses', () => {
    const { container } = render(<AccentText text="Store" />);
    expect(container.querySelector('em')).toBeNull();
    expect(plainText('Welcome *back*,')).toBe('Welcome back,');
  });
});
