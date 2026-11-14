import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { IconPicker } from './IconPicker';
import { PRESET_COLORS } from './InstanceIcon';

function setup() {
  const onPreset = vi.fn();
  const onColor = vi.fn();
  render(
    <IconPicker
      preset="grass"
      color={PRESET_COLORS[2]!}
      onPreset={onPreset}
      onColor={onColor}
      iconsLabel="Icon"
      colorsLabel="Colour"
      iconLabel={(p) => `icon ${p}`}
      colorLabel={(n) => `colour ${n}`}
    />,
  );
  return { onPreset, onColor };
}

afterEach(cleanup);

describe('IconPicker', () => {
  it('offers every glyph and colour as radios, with the current ones checked', () => {
    setup();
    const icons = screen.getByRole('radiogroup', { name: 'Icon' });
    expect(icons.querySelectorAll('[role="radio"]')).toHaveLength(10);
    expect(screen.getByRole('radio', { name: 'icon grass' })).toHaveAttribute(
      'aria-checked',
      'true',
    );
    expect(screen.getByRole('radio', { name: 'colour 3' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'colour 1' })).toHaveAttribute(
      'aria-checked',
      'false',
    );
  });

  it('paints only the chosen glyph in the chosen colour', () => {
    setup();
    expect(screen.getByRole('radio', { name: 'icon grass' }).getAttribute('style')).toContain(
      'linear-gradient',
    );
    expect(screen.getByRole('radio', { name: 'icon castle' }).getAttribute('style')).toBeNull();
  });

  it('reports a new glyph and a new colour', () => {
    const { onPreset, onColor } = setup();
    fireEvent.click(screen.getByRole('radio', { name: 'icon castle' }));
    expect(onPreset).toHaveBeenCalledWith('castle');
    fireEvent.click(screen.getByRole('radio', { name: 'colour 1' }));
    expect(onColor).toHaveBeenCalledWith(PRESET_COLORS[0]);
  });
});
