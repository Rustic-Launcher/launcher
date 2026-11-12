# 0018. Design tokens and the Ember default palette

- Status: **accepted (user decision, 2026-11-10)**

## Context

Three original palettes were proposed (docs/PALETTES.md) with WCAG contrast computed by `docs/tools/contrast.py`.

## Decision

Ember (warm copper accent `#E8783A` on tinted near-black `#0E0B0A`…`#2D2723`) is the default; Verdigris and Heather are
selectable in Settings › Appearance. Typography Inter, spacing 4 px grid, radii 6/8/12/14/16, durations 150/200/250 ms,
z-index scale: as tabled in PALETTES.md. `docs/tools/palettes.json` is the source of truth; the frontend generates CSS
variables `--rl-*` from it and CI runs the contrast script.

## Consequences

Any palette change must keep `contrast.py` green (53 pairs per palette, AA).

## Alternatives

Single fixed theme (less personal); blue/violet accents (too close to Lunar Client and Horizon's look).
