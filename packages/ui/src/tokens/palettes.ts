/**
 * Colour palettes of Rustic Launcher (ADR 0021).
 *
 * Three original palettes (Ember default, Verdigris, Heather) on shared neutral anthracite
 * surfaces (ADR 0028). Accent values are launcher/docs/PALETTES.md; accents are never a blue, cyan
 * or violet primary. Every text token is checked for WCAG AA contrast in `palettes.test.ts`.
 */

export const PALETTE_IDS = ['ember', 'verdigris', 'heather'] as const;
export type PaletteId = (typeof PALETTE_IDS)[number];
export type ThemeMode = 'dark' | 'light';
export type ThemePreference = ThemeMode | 'system';

/**
 * Semantic colour tokens, mapped from PALETTES.md: bg = bg.0, bgElevated = bg.1, surface = bg.2,
 * surface2 = bg.3, surface3 = bg.4, border = border.subtle, borderDefault = border.default,
 * fg = text.primary, fgMuted = text.secondary, fgSubtle = text.tertiary.
 */
export interface ColorTokens {
  /** bg.0: window background (behind everything, focus-ring offset colour). */
  bg: string;
  /** bg.1: rail, top bar, bottom bar. */
  bgElevated: string;
  /** bg.2: panels, cards, friends panel. */
  surface: string;
  /** bg.3: inputs, raised rows, secondary buttons. */
  surface2: string;
  /** bg.4: hover on surface2, pressed rows, skeleton base. */
  surface3: string;
  /** border.subtle: hairline dividers. */
  border: string;
  borderDefault: string;
  /** border.strong: inputs and control boundaries (>= 3:1). */
  borderStrong: string;
  fg: string;
  fgMuted: string;
  fgSubtle: string;
  fgDisabled: string;
  accent: string;
  accentHover: string;
  accentActive: string;
  /** text.on-accent */
  accentFg: string;
  /** accent.text: accent used as text or icon. */
  accentText: string;
  /** accent.subtle */
  accentSoft: string;
  focusRing: string;
  glass: string;
  glassStrong: string;
  scrim: string;
  success: string;
  warning: string;
  danger: string;
  info: string;
  /** Text-safe variants of the status colours (identical in dark: they already pass AA). */
  successText: string;
  warningText: string;
  dangerText: string;
  infoText: string;
}

export interface PaletteDefinition {
  id: PaletteId;
  /** Swatch shown in Settings (the dark accent). */
  swatch: string;
  dark: ColorTokens;
  light: ColorTokens;
}

function status(success: string, warning: string, danger: string, info: string) {
  return {
    success,
    warning,
    danger,
    info,
    successText: success,
    warningText: warning,
    dangerText: danger,
    infoText: info,
  };
}

/**
 * Light variants are a frontend addition (ADR 0021): PALETTES.md ships dark only at launch; light
 * stays an Appearance option and is AA-checked by palettes.test.ts like the dark ones.
 */
const lightStatus = {
  success: '#1F7A52',
  warning: '#9A6400',
  danger: '#C2342C',
  info: '#2F5FB3',
  successText: '#1B6B48',
  warningText: '#8A5900',
  dangerText: '#B02E27',
  infoText: '#2A55A1',
};

/**
 * Neutral surfaces (step 1b redesign, ADR 0028): very dark blue-grey / anthracite, flat, shared by
 * every palette; only the accent family changes between Ember, Verdigris and Heather.
 */
const darkNeutrals = {
  // Motion-design redesign (2026-11-10): warm near-black surfaces shared by every palette.
  bg: '#050505',
  bgElevated: '#090908',
  surface: '#0D0D0C',
  surface2: '#151412',
  surface3: '#1E1C1A',
  border: '#1A1917',
  borderDefault: '#262421',
  borderStrong: '#6A665F',
  fg: '#F3F1EE',
  fgMuted: '#8C8882',
  fgSubtle: '#85817B',
  fgDisabled: '#4C4844',
  glass: 'rgba(13,13,12,0.70)',
  glassStrong: 'rgba(21,20,18,0.94)',
  scrim: 'rgba(0,0,0,0.66)',
};

const lightNeutrals = {
  bg: '#E9EBEF',
  bgElevated: '#E9EBEF',
  surface: '#FFFFFF',
  surface2: '#F3F4F6',
  surface3: '#E6E8EC',
  border: '#E1E3E8',
  borderDefault: '#D3D6DC',
  borderStrong: '#7C808B',
  fg: '#15171C',
  fgMuted: '#4D515B',
  fgSubtle: '#5F636D',
  fgDisabled: '#A3A7B0',
  glass: 'rgba(255,255,255,0.82)',
  glassStrong: 'rgba(255,255,255,0.95)',
  scrim: 'rgba(15,17,22,0.36)',
};

export const palettes: Record<PaletteId, PaletteDefinition> = {
  ember: {
    id: 'ember',
    swatch: '#FF6A00',
    dark: {
      ...darkNeutrals,
      // The logo orange; amber is the secondary accent (focus ring, highlights).
      accent: '#FF6A00',
      accentHover: '#FF8124',
      accentActive: '#E05A00',
      accentFg: '#0B0603',
      accentText: '#FF8A3D',
      accentSoft: 'rgba(255,106,0,0.14)',
      focusRing: '#FFB020',
      ...status('#4CC38A', '#E5B53E', '#F0645A', '#5AA9E6'),
    },
    light: {
      ...lightNeutrals,
      accent: '#B84A00',
      accentHover: '#A34100',
      accentActive: '#8C3700',
      accentFg: '#FFFFFF',
      accentText: '#A84400',
      accentSoft: 'rgba(184,74,0,0.10)',
      focusRing: '#A84B16',
      ...lightStatus,
    },
  },
  verdigris: {
    id: 'verdigris',
    swatch: '#3FBF9F',
    dark: {
      ...darkNeutrals,
      accent: '#3FBF9F',
      accentHover: '#5AD1B3',
      accentActive: '#2FA386',
      accentFg: '#04140F',
      accentText: '#6FD9BE',
      accentSoft: 'rgba(63,191,159,0.14)',
      focusRing: '#8DEBD3',
      ...status('#7BC96F', '#E3B341', '#EF6461', '#6AA8F0'),
    },
    light: {
      ...lightNeutrals,
      accent: '#17735A',
      accentHover: '#125F4A',
      accentActive: '#0E4F3D',
      accentFg: '#FFFFFF',
      accentText: '#156B54',
      accentSoft: 'rgba(23,115,90,0.10)',
      focusRing: '#17735A',
      ...lightStatus,
    },
  },
  heather: {
    id: 'heather',
    swatch: '#D9668F',
    dark: {
      ...darkNeutrals,
      accent: '#D9668F',
      accentHover: '#E57FA3',
      accentActive: '#CC5882',
      accentFg: '#1A0710',
      accentText: '#EC92B2',
      accentSoft: 'rgba(217,102,143,0.14)',
      focusRing: '#F2A9C4',
      ...status('#4FC08D', '#E2B144', '#F26B5E', '#6EA6EE'),
    },
    light: {
      ...lightNeutrals,
      accent: '#A8385F',
      accentHover: '#922E51',
      accentActive: '#7C2645',
      accentFg: '#FFFFFF',
      accentText: '#9C3358',
      accentSoft: 'rgba(168,56,95,0.10)',
      focusRing: '#A8385F',
      ...lightStatus,
    },
  },
};

export const DEFAULT_PALETTE: PaletteId = 'ember';

const cssVarName: Record<keyof ColorTokens, string> = {
  bg: '--rl-bg',
  bgElevated: '--rl-bg-elevated',
  surface: '--rl-surface',
  surface2: '--rl-surface-2',
  surface3: '--rl-surface-3',
  border: '--rl-border',
  borderDefault: '--rl-border-default',
  borderStrong: '--rl-border-strong',
  fg: '--rl-fg',
  fgMuted: '--rl-fg-muted',
  fgSubtle: '--rl-fg-subtle',
  fgDisabled: '--rl-fg-disabled',
  glass: '--rl-glass',
  glassStrong: '--rl-glass-strong',
  scrim: '--rl-overlay',
  accent: '--rl-accent',
  accentHover: '--rl-accent-hover',
  accentActive: '--rl-accent-active',
  accentFg: '--rl-accent-fg',
  accentText: '--rl-accent-text',
  accentSoft: '--rl-accent-soft',
  focusRing: '--rl-focus',
  success: '--rl-success',
  warning: '--rl-warning',
  danger: '--rl-danger',
  info: '--rl-info',
  successText: '--rl-success-text',
  warningText: '--rl-warning-text',
  dangerText: '--rl-danger-text',
  infoText: '--rl-info-text',
};

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '');
  const full =
    h.length === 3
      ? h
          .split('')
          .map((c) => c + c)
          .join('')
      : h;
  const n = Number.parseInt(full, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function channel(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

export function relativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** WCAG 2.x contrast ratio between two hex colours. */
export function contrastRatio(a: string, b: string): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

export function resolveTheme(pref: ThemePreference): ThemeMode {
  if (pref !== 'system') return pref;
  if (typeof window === 'undefined' || !window.matchMedia) return 'dark';
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

/** CSS custom properties for a palette and mode, ready for `style` or `setProperty`. */
export function themeVariables(palette: PaletteId, mode: ThemeMode): Record<string, string> {
  const tokens = palettes[palette][mode];
  const vars: Record<string, string> = {};
  for (const key of Object.keys(cssVarName) as (keyof ColorTokens)[]) {
    vars[cssVarName[key]] = tokens[key];
  }
  const [ar, ag, ab] = hexToRgb(tokens.accent);
  vars['--rl-glass-border'] = mode === 'dark' ? 'rgba(255, 255, 255, 0.07)' : 'rgba(0, 0, 0, 0.07)';
  vars['--rl-accent-glow'] = `rgba(${ar}, ${ag}, ${ab}, 0.35)`;
  vars['--rl-shadow-sm'] =
    mode === 'dark' ? '0 1px 2px rgba(0,0,0,0.40)' : '0 1px 2px rgba(20,16,14,0.08)';
  vars['--rl-shadow-md'] =
    mode === 'dark'
      ? '0 4px 12px rgba(0,0,0,0.45)'
      : '0 8px 24px -8px rgba(20,16,14,0.16), 0 2px 6px rgba(20,16,14,0.06)';
  vars['--rl-shadow-lg'] =
    mode === 'dark'
      ? '0 12px 32px rgba(0,0,0,0.55)'
      : '0 24px 64px -16px rgba(20,16,14,0.24), 0 4px 16px rgba(20,16,14,0.08)';
  return vars;
}

/** Applies a palette and theme to an element (the document root by default). */
export function applyTheme(
  palette: PaletteId,
  pref: ThemePreference,
  el: HTMLElement = document.documentElement,
): ThemeMode {
  const safe: PaletteId = PALETTE_IDS.includes(palette) ? palette : DEFAULT_PALETTE;
  const mode = resolveTheme(pref);
  const vars = themeVariables(safe, mode);
  for (const [k, v] of Object.entries(vars)) el.style.setProperty(k, v);
  el.dataset.palette = safe;
  el.dataset.theme = mode;
  el.style.colorScheme = mode;
  return mode;
}
