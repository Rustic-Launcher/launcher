import { describe, expect, it } from 'vitest';
import { contrastRatio, palettes, PALETTE_IDS, themeVariables, type ThemeMode } from './palettes';

const AA_TEXT = 4.5;
const AA_UI = 3;

describe('palettes', () => {
  for (const id of PALETTE_IDS) {
    for (const mode of ['dark', 'light'] as ThemeMode[]) {
      const t = palettes[id][mode];
      describe(`${id} / ${mode}`, () => {
        it.each([
          ['fg on bg', t.fg, t.bg],
          ['fg on surface3', t.fg, t.surface3],
          ['fgMuted on bg', t.fgMuted, t.bg],
          ['fgMuted on surface2', t.fgMuted, t.surface2],
          ['fgSubtle on bg', t.fgSubtle, t.bg],
          ['fgSubtle on surface', t.fgSubtle, t.surface],
          ['accentFg on accent', t.accentFg, t.accent],
          ['accentFg on accentHover', t.accentFg, t.accentHover],
          ['accentFg on accentActive', t.accentFg, t.accentActive],
          ['accentText on bg', t.accentText, t.bg],
          ['accentText on surface2', t.accentText, t.surface2],
          ['successText on surface', t.successText, t.surface],
          ['warningText on surface', t.warningText, t.surface],
          ['dangerText on surface', t.dangerText, t.surface],
          ['infoText on surface', t.infoText, t.surface],
        ])('%s meets AA (4.5:1)', (_name, fg, bg) => {
          expect(contrastRatio(fg, bg)).toBeGreaterThanOrEqual(AA_TEXT);
        });

        it('component borders and accent meet 3:1 against surface', () => {
          expect(contrastRatio(t.borderStrong, t.surface)).toBeGreaterThanOrEqual(AA_UI);
          expect(contrastRatio(t.accent, t.surface)).toBeGreaterThanOrEqual(AA_UI);
          expect(contrastRatio(t.focusRing, t.bg)).toBeGreaterThanOrEqual(AA_UI);
          expect(contrastRatio(t.focusRing, t.surface2)).toBeGreaterThanOrEqual(AA_UI);
        });
      });
    }
  }

  it('exposes every token as a CSS variable', () => {
    const vars = themeVariables('ember', 'dark');
    expect(vars['--rl-accent']).toBe('#FF6A00');
    expect(vars['--rl-glass']).toMatch(/^rgba\(/);
    expect(vars['--rl-focus']).toBe('#FFB020');
  });

  it('computes known contrast ratios', () => {
    expect(contrastRatio('#000000', '#FFFFFF')).toBeCloseTo(21, 0);
    expect(contrastRatio('#777777', '#777777')).toBeCloseTo(1, 5);
  });
});
