# 0143. The launcher in the website's design language

- Status: accepted (launcher + ui, 2026-11-11)
- Builds on: ADR 0028 (app shell), web ADR 0141 (motion), ADR 0142 (premium cards).

## Context

The website got its motion-design language (ember accent, amber, warm near-black, serif-italic
accent words, mono eyebrows, premium cards, grain). The user asked for the launcher to speak the same
language while keeping its Lunar-like organisation (rail, Launchpad, Versions, Explore, Servers,
Locker, Store, Settings), with animations that rest while a game runs or the window is hidden.

## Decision

- **Shared in `@rustic/ui` (`language.css`)**, used by both apps:
  - `.rx-surface`: the premium surface in pure CSS (1 px gradient edge accent → amber → transparent,
    layered glass, film grain, inner highlight, soft shadow, a pointer spotlight through `--mx` /
    `--my` set by one delegated listener, `installSurfaceSpotlight()`). `Card` (solid / glass) now
    renders it, so every card in the launcher and on the website's functional pages is one.
    `PremiumCard` (sweep, tilt) stays for showcase grids.
  - `.rx-eyebrow` (mono, uppercase, tracked, accent dot), `.rx-accent` + `AccentText` (`*word*` in a
    translation becomes the serif-italic accent; Instrument Serif, OFL-1.1, bundled).
  - `.rx-ambient` (warm grain background with an ember glow), `.rx-panel` (content panel with a
    faint dot grid).
  - `.rx-page-enter`: blocks enter blur → sharp in 420 ms, 40 ms apart.
  - `html[data-paused]` pauses every CSS animation (spinners excepted) and skips entrances;
    `useAnimationsPaused()` (also true while the window is hidden) stops the 3D viewers
    (`frameloop="never"`); the Launchpad light stream pauses on its own rules.
- **Launcher**: every page title has a mono eyebrow and an accent word (Launchpad "Welcome *back*,",
  Versions "Your *profiles*", Explore "Find your next *mod*", Servers "Play on *servers*", Locker
  "Your *locker*", Store "The *store*", Settings "Your *settings*", new server "Create a *server*");
  the toolbar pages get a compact visible title (they had a screen-reader-only one). Card-like
  containers (settings cards, server tabs, wizard steps, instance header, locker panels, store
  tiles, party card) use `.rx-surface`. The shell sets `data-paused` while a game runs.
- Organisation, navigation and test ids are unchanged.

## Consequences

- One look across site and app; three palettes keep working (the surface derives from tokens).
- `Card` changed appearance everywhere at once (intended); its `interactive` variant no longer
  animates borders or shadows (transform only).
- Screenshots and the four product videos were redone.

## Alternatives

- Wrapping every container in `PremiumCard`: extra DOM and a body wrapper that breaks the callers'
  layouts; the CSS surface gives the same look where tilt is not wanted.
- A launcher-only stylesheet: the website's functional pages would drift from the app.
