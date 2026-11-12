# 0021. Design tokens implementation: runtime palettes, Tailwind v4 theme, light variant

- Status: accepted (frontend, 2026-11-10)
- Implements: ADR 0018 and docs/PALETTES.md (spec)

## Context

PALETTES.md defines three dark palettes (Ember default, Verdigris, Heather) with AA-checked pairs, plus type,
spacing, radii, elevation, motion and z-index scales. The palette is switchable live in Settings and must be
right on the very first paint (no flash).

## Decision

- **Source in TypeScript**: `packages/ui/src/tokens/palettes.ts` holds every colour token per palette and mode;
  dark values are PALETTES.md verbatim (`bg.0..4`, `text.*`, `accent.*`, `surface.glass*`, `focus.ring`,
  status colours). `applyTheme(palette, preference)` writes CSS custom properties on `<html>` and sets
  `data-palette` / `data-theme`. It runs synchronously before React renders (`bindTheme()` in `main.tsx`); the
  default Ember values are also in `tokens.css` so even the HTML shell paints correctly.
- **Tailwind v4 mapping**: `styles.css` resets the default palette (`--color-*: initial`) and maps semantic
  tokens in `@theme inline` (`bg-surface`, `text-fg-muted`, `bg-accent`, `text-accent-text`, `border-border-strong`,
  `bg-glass`…), so components cannot use off-palette colours by accident.
- **Non-colour tokens** (type scale, 4 px spacing, radii 6/8/12/14/16, shadows, durations 150/200/250 ms,
  `ease.standard`, z-index, shell metrics: rail 72, top bar 56, bottom bar 72, friends 296) live in
  `tokens.css` and `tokens.ts` (for Motion).
- **Contrast in CI via Vitest**: `palettes.test.ts` recomputes WCAG ratios for every text/background pair,
  on-accent labels (hover and active), control borders and the focus ring, for all palettes and modes. It
  complements `docs/tools/contrast.py` (spec), which remains the source check for PALETTES.md.
- **Light variant**: PALETTES.md is dark-first. The brief asked for a light theme, so each palette gets a light
  variant (darker accents with white labels), exposed as Appearance › Theme (dark default, light, system). It is
  held to the same AA test. A future spec ADR may refine or drop it.
- **Variable prefix**: CSS variables are `--rl-*` as requested in HANDOFF (spec → frontend), utility classes
  `rx-glass`, `rx-glass-strong`, `rx-focus-ring`, `rx-skeleton`, `rx-ambient`.
- **Reduced motion**: global CSS clamps transitions/animations under `prefers-reduced-motion`; the app wraps
  everything in `<MotionConfig reducedMotion="user">`; carousel autoplay and 3D auto-rotate stop.

## Consequences

Changing a colour means editing `palettes.json` (spec) and `palettes.ts` together; the Vitest contrast test
fails the build on regressions. Palette preference is stored per webview (`localStorage`, key `rustic.prefs`).

## Alternatives

Pure CSS `[data-palette]` blocks (no single typed source, no unit test), CSS-in-JS theming (runtime cost).
