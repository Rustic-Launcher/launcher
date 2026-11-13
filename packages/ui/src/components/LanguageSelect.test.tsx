import { loadFlags, type TranslationStatus } from '@rustic/locales';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { Flag, LanguageSelect } from './LanguageSelect';

beforeAll(async () => {
  // cmdk measures and scrolls; happy-dom has neither.
  globalThis.ResizeObserver ??= class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
  Element.prototype.scrollIntoView ??= () => {};
  await loadFlags();
});
afterEach(cleanup);

const labels = {
  label: 'Language',
  search: 'Search languages',
  empty: 'No language matches',
  translated: 'Translated',
  others: 'Other languages',
  partial: 'Partial',
  english: 'in English for now',
};
const status = (code: string): TranslationStatus =>
  code.startsWith('fr') || code.startsWith('en')
    ? 'complete'
    : code === 'de_de'
      ? 'partial'
      : 'none';

describe('Flag', () => {
  it('draws the game flag, or a neutral tile with the code', async () => {
    const { container } = render(
      <>
        <Flag code="fr_fr" />
        <Flag code="tok" />
        <Flag code="en_ud" />
      </>,
    );
    await waitFor(() => expect(container.querySelector('[data-flag="fr_fr"] rect')).toBeTruthy());
    const fr = [...container.querySelectorAll('[data-flag="fr_fr"] rect')].map((r) =>
      r.getAttribute('fill'),
    );
    expect(fr).toEqual(['#0055a4', '#ffffff', '#ef4135']);
    expect(container.querySelector('[data-flag="tok"]')).toHaveTextContent('TOK');
    // Upside Down English is turned over.
    expect(container.querySelector('[data-flag="en_ud"] g')).toHaveAttribute(
      'transform',
      'matrix(1 0 0 -1 0 20)',
    );
  });
});

describe('LanguageSelect', () => {
  it('lists every locale, translated first, and searches without accents', async () => {
    const onChange = vi.fn();
    render(<LanguageSelect value="fr_fr" onChange={onChange} status={status} labels={labels} />);
    const button = screen.getByTestId('language-select');
    expect(button).toHaveAccessibleName('Language: Français (France)');
    await act(async () => {
      fireEvent.click(button);
    });
    const list = await screen.findByTestId('language-select-list');
    expect(list.querySelectorAll('[cmdk-item]')).toHaveLength(143);
    const headings = [...list.querySelectorAll('[cmdk-group-heading]')].map((h) => h.textContent);
    expect(headings).toEqual(['Translated', 'Other languages']);
    // The partial one comes first among the others, with its badge.
    const others = list.querySelectorAll('[cmdk-group]')[1]!;
    expect(others.querySelector('[cmdk-item]')).toHaveAttribute('data-code', 'de_de');
    expect(screen.getByTestId('language-select-de_de')).toHaveTextContent('Partial');
    expect(screen.getByTestId('language-select-ja_jp')).toHaveTextContent('in English for now');
    // Right-to-left names keep their direction.
    expect(screen.getByTestId('language-select-ar_sa').querySelector('[dir="rtl"]')).toBeTruthy();

    fireEvent.change(screen.getByTestId('language-select-search'), {
      target: { value: 'portugues brasil' },
    });
    await waitFor(() => expect(list.querySelectorAll('[cmdk-item]')).toHaveLength(1));
    fireEvent.click(screen.getByTestId('language-select-pt_br'));
    expect(onChange).toHaveBeenCalledWith('pt_br');
  });

  it('says so when nothing matches', async () => {
    render(<LanguageSelect value="en_us" onChange={() => {}} status={status} labels={labels} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId('language-select'));
    });
    fireEvent.change(await screen.findByTestId('language-select-search'), {
      target: { value: 'zzzz' },
    });
    expect(await screen.findByText('No language matches')).toBeInTheDocument();
  });
});
