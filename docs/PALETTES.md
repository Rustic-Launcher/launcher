# Rustic Launcher: palettes and design tokens

Owner: spec. Status: **decided** (user, 2026-11-10): **Ember is the default**; Verdigris and Heather ship as
options in Settings › Appearance. The frontend implements these tokens in `launcher/packages/ui`
(ADR 0020+). Source of truth for colours: [`tools/palettes.json`](tools/palettes.json); contrast checked by
[`tools/contrast.py`](tools/contrast.py) (standard-library Python, WCAG 2.2 formula).

## Identity rules

- Deep dark theme only at launch (a light theme is a later, separate ADR). Backgrounds are *tinted*
  near-blacks, never pure `#000`, so glass surfaces and shadows read.
- The accent is **ours**: warm copper (Ember), patina green (Verdigris), heather rose (Heather). None of
  them is a blue, cyan or violet primary, which keeps us clearly apart from Lunar Client's look; no Lunar
  colour value, gradient or logo shape was used as a reference (we had no screenshots: see
  `docs/references/`, still empty).
- "Rust" is the brand metaphor: oxidised metal (copper, verdigris) and warm embers. Horizon's own launcher
  themes ("Horizon", "First Light", `horizon/launcher` `themes::THEMES`) were not copied either.
- Accent is used sparingly: the PLAY button, the active rail item, selection, progress, links. Large areas
  stay neutral.

## Token set (per palette)

| Token | Role |
|---|---|
| `bg.0` | Window background, behind everything (also the 2 px focus offset colour) |
| `bg.1` | Left icon rail, top bar, bottom bar |
| `bg.2` | Page background, friends panel |
| `bg.3` | Raised cards, list rows, inputs |
| `bg.4` | Hover on `bg.3`, pressed rows, skeleton shimmer base |
| `surface.glass` | Floating glass: popovers, tooltips, command palette (with `backdrop-filter: blur(16px) saturate(1.2)`) |
| `surface.glass-strong` | Modals, toasts, context menus |
| `surface.scrim` | Modal backdrop |
| `border.subtle` / `border.default` | Dividers (decorative, no contrast requirement) |
| `border.strong` | Boundaries of inputs, checkboxes, toggles (≥ 3:1, WCAG 1.4.11) |
| `text.primary` / `.secondary` / `.tertiary` | Text levels; all three are ≥ 4.5:1 on every background, so tertiary may carry real information |
| `text.disabled` | Disabled labels (exempt from contrast, never the only signal) |
| `text.on-accent` | Label on accent fills (a dark text: brighter accents keep AA) |
| `accent`, `accent.hover`, `accent.active` | Primary action fills |
| `accent.subtle` | Selected row / active rail item tint |
| `accent.text` | Accent used as text (links, "Installed" labels) |
| `success`, `warning`, `danger`, `info` | Status text, icons, badges (also used as 14 % tints for badge backgrounds) |
| `focus.ring` | 2 px ring, 2 px offset (`outline: 2px solid; outline-offset: 2px`), so it always sits on the page background, never on the control |

CSS: each token becomes `--rl-<token with dots as dashes>` (e.g. `--rl-accent-hover`), set on
`:root[data-palette="ember"]` etc. Tailwind v4 maps them in `@theme`.

## 1. Ember (default)

Warm copper-orange accent on a warm-tinted near-black.

| Token | Value | | Token | Value |
|---|---|---|---|---|
| `bg.0` | `#0E0B0A` | | `text.primary` | `#F5EFEA` |
| `bg.1` | `#141110` | | `text.secondary` | `#C9BDB4` |
| `bg.2` | `#1A1614` | | `text.tertiary` | `#A39489` |
| `bg.3` | `#231E1B` | | `text.disabled` | `#6A5F58` |
| `bg.4` | `#2D2723` | | `text.on-accent` | `#1A0E07` |
| `surface.glass` | `rgba(40,33,29,0.72)` | | `accent` | `#E8783A` |
| `surface.glass-strong` | `rgba(50,42,37,0.88)` | | `accent.hover` | `#F08C52` |
| `surface.scrim` | `rgba(8,6,5,0.64)` | | `accent.active` | `#CC6228` |
| `border.subtle` | `#2A2421` | | `accent.subtle` | `rgba(232,120,58,0.14)` |
| `border.default` | `#3A322D` | | `accent.text` | `#F29A66` |
| `border.strong` | `#7A6A60` | | `focus.ring` | `#FFB37A` |
| `success` | `#4CC38A` | | `warning` | `#E5B53E` |
| `danger` | `#F0645A` | | `info` | `#5AA9E6` |

## 2. Verdigris

Patina teal-green (oxidised copper) on a green-tinted near-black.

| Token | Value | | Token | Value |
|---|---|---|---|---|
| `bg.0` | `#090D0C` | | `text.primary` | `#ECF3F0` |
| `bg.1` | `#0E1312` | | `text.secondary` | `#B9C7C2` |
| `bg.2` | `#131A18` | | `text.tertiary` | `#8FA09A` |
| `bg.3` | `#1A2321` | | `text.disabled` | `#56635F` |
| `bg.4` | `#222D2A` | | `text.on-accent` | `#04140F` |
| `surface.glass` | `rgba(28,38,35,0.72)` | | `accent` | `#3FBF9F` |
| `surface.glass-strong` | `rgba(36,48,45,0.88)` | | `accent.hover` | `#5AD1B3` |
| `surface.scrim` | `rgba(4,7,6,0.64)` | | `accent.active` | `#2FA386` |
| `border.subtle` | `#1F2927` | | `accent.subtle` | `rgba(63,191,159,0.14)` |
| `border.default` | `#2E3B38` | | `accent.text` | `#6FD9BE` |
| `border.strong` | `#5F726D` | | `focus.ring` | `#8DEBD3` |
| `success` | `#7BC96F` | | `warning` | `#E3B341` |
| `danger` | `#EF6461` | | `info` | `#6AA8F0` |

Note: success is a yellow-green so it stays distinct from the teal accent; status badges always carry an
icon and a word as well, never colour alone.

## 3. Heather

Muted heather-rose on a violet-tinted near-black.

| Token | Value | | Token | Value |
|---|---|---|---|---|
| `bg.0` | `#0D0B0F` | | `text.primary` | `#F3EEF5` |
| `bg.1` | `#131016` | | `text.secondary` | `#C6BCCB` |
| `bg.2` | `#19151D` | | `text.tertiary` | `#9D92A3` |
| `bg.3` | `#221C27` | | `text.disabled` | `#625868` |
| `bg.4` | `#2B2431` | | `text.on-accent` | `#1A0710` |
| `surface.glass` | `rgba(36,30,41,0.72)` | | `accent` | `#D9668F` |
| `surface.glass-strong` | `rgba(46,39,52,0.88)` | | `accent.hover` | `#E57FA3` |
| `surface.scrim` | `rgba(6,5,8,0.64)` | | `accent.active` | `#CC5882` |
| `border.subtle` | `#28212D` | | `accent.subtle` | `rgba(217,102,143,0.14)` |
| `border.default` | `#382F3E` | | `accent.text` | `#EC92B2` |
| `border.strong` | `#766A7E` | | `focus.ring` | `#F2A9C4` |
| `success` | `#4FC08D` | | `warning` | `#E2B144` |
| `danger` | `#F26B5E` | | `info` | `#6EA6EE` |

Note: danger (coral red) and accent (rose) are close in hue: destructive actions in Heather always use the
danger fill **and** a trash/alert icon and explicit wording.

## Contrast results (WCAG 2.2 AA)

Minimums: 4.5:1 for text (all sizes, we do not rely on the large-text exception), 3:1 for focus indicators,
control boundaries and meaningful shapes (1.4.11). `text.disabled` and `border.default` are listed for
information only. Re-run after any change; CI may call it (exit 1 on failure).

<!-- contrast:begin -->
Generated by `python docs/tools/contrast.py --write`. All required pairs pass WCAG 2.2 AA.

#### Ember (`ember`) (default)

53 pairs, 0 failing. Translucent surfaces are composited over `bg.2`.

| Foreground | Background | Ratio | Min | Result | Use |
|---|---|---|---|---|---|
| `text.primary` | `bg.0` | 17.20:1 | 4.5 | pass | body text |
| `text.primary` | `bg.1` | 16.48:1 | 4.5 | pass | body text |
| `text.primary` | `bg.2` | 15.76:1 | 4.5 | pass | body text |
| `text.primary` | `bg.3` | 14.47:1 | 4.5 | pass | body text |
| `text.primary` | `bg.4` | 12.91:1 | 4.5 | pass | body text |
| `text.primary` | `surface.glass` | 14.44:1 | 4.5 | pass | body text |
| `text.primary` | `surface.glass-strong` | 12.76:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.0` | 10.67:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.1` | 10.22:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.2` | 9.77:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.3` | 8.97:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.4` | 8.01:1 | 4.5 | pass | body text |
| `text.secondary` | `surface.glass` | 8.95:1 | 4.5 | pass | body text |
| `text.secondary` | `surface.glass-strong` | 7.91:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.0` | 6.68:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.1` | 6.40:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.2` | 6.12:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.3` | 5.62:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.4` | 5.02:1 | 4.5 | pass | body text |
| `text.tertiary` | `surface.glass` | 5.61:1 | 4.5 | pass | body text |
| `text.tertiary` | `surface.glass-strong` | 4.96:1 | 4.5 | pass | body text |
| `accent.text` | `bg.0` | 8.95:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.1` | 8.58:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.2` | 8.20:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.3` | 7.53:1 | 4.5 | pass | links, accent text |
| `accent.text` | `surface.glass` | 7.52:1 | 4.5 | pass | links, accent text |
| `text.on-accent` | `accent` | 6.47:1 | 4.5 | pass | label on accent button |
| `text.on-accent` | `accent.hover` | 7.73:1 | 4.5 | pass | label on accent button |
| `text.on-accent` | `accent.active` | 4.82:1 | 4.5 | pass | label on accent button |
| `success` | `bg.2` | 8.11:1 | 4.5 | pass | status text and icons |
| `success` | `bg.3` | 7.45:1 | 4.5 | pass | status text and icons |
| `success` | `surface.glass` | 7.43:1 | 4.5 | pass | status text and icons |
| `warning` | `bg.2` | 9.42:1 | 4.5 | pass | status text and icons |
| `warning` | `bg.3` | 8.65:1 | 4.5 | pass | status text and icons |
| `warning` | `surface.glass` | 8.63:1 | 4.5 | pass | status text and icons |
| `danger` | `bg.2` | 5.71:1 | 4.5 | pass | status text and icons |
| `danger` | `bg.3` | 5.24:1 | 4.5 | pass | status text and icons |
| `danger` | `surface.glass` | 5.23:1 | 4.5 | pass | status text and icons |
| `info` | `bg.2` | 7.06:1 | 4.5 | pass | status text and icons |
| `info` | `bg.3` | 6.48:1 | 4.5 | pass | status text and icons |
| `info` | `surface.glass` | 6.47:1 | 4.5 | pass | status text and icons |
| `focus.ring` | `bg.0` | 11.19:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.1` | 10.72:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.2` | 10.25:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.3` | 9.41:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.4` | 8.40:1 | 3 | pass | focus indicator (1.4.11) |
| `border.strong` | `bg.2` | 3.47:1 | 3 | pass | input / control boundary (1.4.11) |
| `border.strong` | `bg.3` | 3.19:1 | 3 | pass | input / control boundary (1.4.11) |
| `accent` | `bg.0` | 6.70:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `accent` | `bg.2` | 6.14:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `accent` | `bg.3` | 5.64:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `text.disabled` | `bg.2` | 2.90:1 | 0 | info | disabled (exempt, informative) |
| `border.default` | `bg.2` | 1.43:1 | 0 | info | decorative divider (informative) |

#### Verdigris (`verdigris`)

53 pairs, 0 failing. Translucent surfaces are composited over `bg.2`.

| Foreground | Background | Ratio | Min | Result | Use |
|---|---|---|---|---|---|
| `text.primary` | `bg.0` | 17.35:1 | 4.5 | pass | body text |
| `text.primary` | `bg.1` | 16.64:1 | 4.5 | pass | body text |
| `text.primary` | `bg.2` | 15.69:1 | 4.5 | pass | body text |
| `text.primary` | `bg.3` | 14.27:1 | 4.5 | pass | body text |
| `text.primary` | `bg.4` | 12.63:1 | 4.5 | pass | body text |
| `text.primary` | `surface.glass` | 14.35:1 | 4.5 | pass | body text |
| `text.primary` | `surface.glass-strong` | 12.57:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.0` | 11.18:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.1` | 10.72:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.2` | 10.11:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.3` | 9.19:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.4` | 8.13:1 | 4.5 | pass | body text |
| `text.secondary` | `surface.glass` | 9.24:1 | 4.5 | pass | body text |
| `text.secondary` | `surface.glass-strong` | 8.10:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.0` | 7.13:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.1` | 6.84:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.2` | 6.45:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.3` | 5.87:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.4` | 5.19:1 | 4.5 | pass | body text |
| `text.tertiary` | `surface.glass` | 5.90:1 | 4.5 | pass | body text |
| `text.tertiary` | `surface.glass-strong` | 5.17:1 | 4.5 | pass | body text |
| `accent.text` | `bg.0` | 11.49:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.1` | 11.02:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.2` | 10.39:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.3` | 9.45:1 | 4.5 | pass | links, accent text |
| `accent.text` | `surface.glass` | 9.50:1 | 4.5 | pass | links, accent text |
| `text.on-accent` | `accent` | 8.24:1 | 4.5 | pass | label on accent button |
| `text.on-accent` | `accent.hover` | 10.08:1 | 4.5 | pass | label on accent button |
| `text.on-accent` | `accent.active` | 6.03:1 | 4.5 | pass | label on accent button |
| `success` | `bg.2` | 8.77:1 | 4.5 | pass | status text and icons |
| `success` | `bg.3` | 7.98:1 | 4.5 | pass | status text and icons |
| `success` | `surface.glass` | 8.02:1 | 4.5 | pass | status text and icons |
| `warning` | `bg.2` | 9.08:1 | 4.5 | pass | status text and icons |
| `warning` | `bg.3` | 8.26:1 | 4.5 | pass | status text and icons |
| `warning` | `surface.glass` | 8.30:1 | 4.5 | pass | status text and icons |
| `danger` | `bg.2` | 5.61:1 | 4.5 | pass | status text and icons |
| `danger` | `bg.3` | 5.10:1 | 4.5 | pass | status text and icons |
| `danger` | `surface.glass` | 5.13:1 | 4.5 | pass | status text and icons |
| `info` | `bg.2` | 7.13:1 | 4.5 | pass | status text and icons |
| `info` | `bg.3` | 6.49:1 | 4.5 | pass | status text and icons |
| `info` | `surface.glass` | 6.52:1 | 4.5 | pass | status text and icons |
| `focus.ring` | `bg.0` | 13.92:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.1` | 13.35:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.2` | 12.58:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.3` | 11.45:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.4` | 10.13:1 | 3 | pass | focus indicator (1.4.11) |
| `border.strong` | `bg.2` | 3.46:1 | 3 | pass | input / control boundary (1.4.11) |
| `border.strong` | `bg.3` | 3.15:1 | 3 | pass | input / control boundary (1.4.11) |
| `accent` | `bg.0` | 8.53:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `accent` | `bg.2` | 7.71:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `accent` | `bg.3` | 7.02:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `text.disabled` | `bg.2` | 2.81:1 | 0 | info | disabled (exempt, informative) |
| `border.default` | `bg.2` | 1.51:1 | 0 | info | decorative divider (informative) |

#### Heather (`heather`)

53 pairs, 0 failing. Translucent surfaces are composited over `bg.2`.

| Foreground | Background | Ratio | Min | Result | Use |
|---|---|---|---|---|---|
| `text.primary` | `bg.0` | 17.13:1 | 4.5 | pass | body text |
| `text.primary` | `bg.1` | 16.49:1 | 4.5 | pass | body text |
| `text.primary` | `bg.2` | 15.74:1 | 4.5 | pass | body text |
| `text.primary` | `bg.3` | 14.53:1 | 4.5 | pass | body text |
| `text.primary` | `bg.4` | 13.12:1 | 4.5 | pass | body text |
| `text.primary` | `surface.glass` | 14.65:1 | 4.5 | pass | body text |
| `text.primary` | `surface.glass-strong` | 13.00:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.0` | 10.69:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.1` | 10.29:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.2` | 9.82:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.3` | 9.07:1 | 4.5 | pass | body text |
| `text.secondary` | `bg.4` | 8.19:1 | 4.5 | pass | body text |
| `text.secondary` | `surface.glass` | 9.14:1 | 4.5 | pass | body text |
| `text.secondary` | `surface.glass-strong` | 8.11:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.0` | 6.60:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.1` | 6.35:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.2` | 6.07:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.3` | 5.60:1 | 4.5 | pass | body text |
| `text.tertiary` | `bg.4` | 5.05:1 | 4.5 | pass | body text |
| `text.tertiary` | `surface.glass` | 5.65:1 | 4.5 | pass | body text |
| `text.tertiary` | `surface.glass-strong` | 5.01:1 | 4.5 | pass | body text |
| `accent.text` | `bg.0` | 8.70:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.1` | 8.37:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.2` | 7.99:1 | 4.5 | pass | links, accent text |
| `accent.text` | `bg.3` | 7.38:1 | 4.5 | pass | links, accent text |
| `accent.text` | `surface.glass` | 7.44:1 | 4.5 | pass | links, accent text |
| `text.on-accent` | `accent` | 5.78:1 | 4.5 | pass | label on accent button |
| `text.on-accent` | `accent.hover` | 7.30:1 | 4.5 | pass | label on accent button |
| `text.on-accent` | `accent.active` | 4.89:1 | 4.5 | pass | label on accent button |
| `success` | `bg.2` | 7.94:1 | 4.5 | pass | status text and icons |
| `success` | `bg.3` | 7.33:1 | 4.5 | pass | status text and icons |
| `success` | `surface.glass` | 7.39:1 | 4.5 | pass | status text and icons |
| `warning` | `bg.2` | 9.09:1 | 4.5 | pass | status text and icons |
| `warning` | `bg.3` | 8.39:1 | 4.5 | pass | status text and icons |
| `warning` | `surface.glass` | 8.46:1 | 4.5 | pass | status text and icons |
| `danger` | `bg.2` | 6.04:1 | 4.5 | pass | status text and icons |
| `danger` | `bg.3` | 5.57:1 | 4.5 | pass | status text and icons |
| `danger` | `surface.glass` | 5.62:1 | 4.5 | pass | status text and icons |
| `info` | `bg.2` | 7.16:1 | 4.5 | pass | status text and icons |
| `info` | `bg.3` | 6.61:1 | 4.5 | pass | status text and icons |
| `info` | `surface.glass` | 6.66:1 | 4.5 | pass | status text and icons |
| `focus.ring` | `bg.0` | 10.49:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.1` | 10.10:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.2` | 9.64:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.3` | 8.90:1 | 3 | pass | focus indicator (1.4.11) |
| `focus.ring` | `bg.4` | 8.04:1 | 3 | pass | focus indicator (1.4.11) |
| `border.strong` | `bg.2` | 3.54:1 | 3 | pass | input / control boundary (1.4.11) |
| `border.strong` | `bg.3` | 3.27:1 | 3 | pass | input / control boundary (1.4.11) |
| `accent` | `bg.0` | 5.83:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `accent` | `bg.2` | 5.36:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `accent` | `bg.3` | 4.94:1 | 3 | pass | accent shapes, PLAY button (1.4.11) |
| `text.disabled` | `bg.2` | 2.67:1 | 0 | info | disabled (exempt, informative) |
| `border.default` | `bg.2` | 1.41:1 | 0 | info | decorative divider (informative) |

<!-- contrast:end -->

## Typography

Font: **Inter** (variable, SIL OFL 1.1, bundled via `@fontsource-variable/inter`, no network fetch), with
`font-feature-settings: "cv11", "ss01", "tnum"` on numbers (counters, prices, versions). Monospace:
**JetBrains Mono** (OFL 1.1) for console, logs, IPs and codes. Geist (also OFL) was considered; Inter wins
for its wider language coverage (Cyrillic, Greek, Vietnamese) and hinting at 12-13 px on Windows.

| Token | Size / line height | Weight | Tracking | Use |
|---|---|---|---|---|
| `text.display` | 32 / 40 | 700 | -0.02em | Home hero title, wizard step titles |
| `text.h1` | 24 / 32 | 650 | -0.015em | Page titles |
| `text.h2` | 20 / 28 | 600 | -0.01em | Section titles |
| `text.h3` | 16 / 24 | 600 | 0 | Card titles, dialog titles |
| `text.body` | 14 / 20 | 400 | 0 | Default text |
| `text.body-strong` | 14 / 20 | 550 | 0 | Emphasis, button labels |
| `text.small` | 13 / 18 | 400 | 0 | Secondary lines, table cells |
| `text.caption` | 12 / 16 | 500 | 0.01em | Badges, metadata, tooltips |
| `text.overline` | 11 / 16 | 600 | 0.06em, uppercase | Group labels in rail tooltips, section kickers |
| `text.mono` | 13 / 20 | 400 | 0 | Console, codes, addresses |
| `text.play` | 18 / 24 | 700 | 0.04em, uppercase | The PLAY button only |

Minimum rendered size: 12 px (11 px only for the uppercase overline). The UI honours the OS text scaling
and a Settings › Accessibility "Text size" (90 %-130 %).

## Spacing (4 px grid)

| Token | px | | Token | px |
|---|---|---|---|---|
| `space.0` | 0 | | `space.6` | 24 |
| `space.0_5` | 2 | | `space.8` | 32 |
| `space.1` | 4 | | `space.10` | 40 |
| `space.2` | 8 | | `space.12` | 48 |
| `space.3` | 12 | | `space.16` | 64 |
| `space.4` | 16 | | `space.20` | 80 |
| `space.5` | 20 | | | |

Layout constants: icon rail 72 px wide; top bar 56 px; bottom bar 72 px; friends panel 296 px open / 0
collapsed (a 44 px edge tab remains); page gutter 32 px (24 px under 1100 px window width); card gap 16 px;
minimum window 1024 × 640.

## Radii

| Token | px | Use |
|---|---|---|
| `radius.xs` | 6 | Badges, tags, small chips |
| `radius.sm` | 8 | Checkboxes, small buttons, tooltips |
| `radius.md` | 12 | Buttons, inputs, list rows, cards (default) |
| `radius.lg` | 14 | Panels, popovers, friends panel cards |
| `radius.xl` | 16 | Modals, hero cards, the PLAY button |
| `radius.full` | 9999 | Avatars, status dots, pills |

## Elevation (shadows, dark-tuned)

| Token | Value |
|---|---|
| `shadow.1` | `0 1px 2px rgba(0,0,0,.40)` |
| `shadow.2` | `0 4px 12px rgba(0,0,0,.45)` |
| `shadow.3` | `0 12px 32px rgba(0,0,0,.55)` |
| `shadow.accent-glow` | `0 0 0 1px var(--rl-accent), 0 8px 24px color-mix(in srgb, var(--rl-accent) 35%, transparent)` (PLAY button hover only) |

## Motion

| Token | Value | Use |
|---|---|---|
| `duration.instant` | 0 ms | Reduced motion replacement |
| `duration.fast` | 150 ms | Hover, press, toggles, tooltips in |
| `duration.base` | 200 ms | Popovers, dropdowns, tab switches, toasts |
| `duration.slow` | 250 ms | Page transitions, modals, friends panel slide |
| `ease.standard` | `cubic-bezier(0.2, 0, 0, 1)` | Enter / move |
| `ease.exit` | `cubic-bezier(0.4, 0, 1, 1)` | Leave (exit runs at 80 % of the enter duration) |
| `ease.spring` | Framer Motion `{ type: "spring", stiffness: 420, damping: 34 }` | Locker model turn, drag snap |

Nothing animates longer than 250 ms except progress bars and the 3D model idle. Under
`prefers-reduced-motion: reduce` (or Settings › Accessibility › Reduce motion): no translation, scale or
parallax; opacity fades only, at most 100 ms; the Locker model does not auto-rotate; skeleton shimmer
becomes a static tint.

## Z-index

| Token | Value | Layer |
|---|---|---|
| `z.base` | 0 | Page content |
| `z.raised` | 10 | Sticky table headers, hovered cards |
| `z.bars` | 100 | Top bar, bottom bar |
| `z.rail` | 110 | Left icon rail |
| `z.friends` | 120 | Friends panel (overlay mode under 1280 px) |
| `z.dropdown` | 1000 | Selects, menus |
| `z.popover` | 1100 | Popovers, notification centre |
| `z.scrim` | 1300 | Modal backdrop |
| `z.modal` | 1400 | Dialogs, wizard, command palette (Ctrl+K) |
| `z.modal-popover` | 1450 | Menus opened inside a modal |
| `z.toast` | 1600 | Toasts |
| `z.tooltip` | 1700 | Tooltips (always on top) |
