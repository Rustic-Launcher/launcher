# 0142. Premium cards, product videos and screenshots

- Status: accepted (web + launcher + ui, 2026-11-11)

## Context

The user asked for one premium card system used everywhere, realistic images of the product (our UI
with believable sample data, original skins only, no in-game shots) and short product videos with a
smooth synthetic cursor, used in the website's hero window and feature rows.

## Decision

- **Cards**: `PremiumCard` and `PremiumGrid` in `@rustic/ui` (`premium-card.css`, in the
  `components` layer so utilities still win). Layers: glass (blur, gradient, SVG fractal-noise grain,
  inner highlight), spotlight + accent bloom, a 1 px gradient edge (accent → amber → transparent) with
  a conic sweep rotating on hover and a pointer hot spot; body. `PremiumGrid` writes `--mx` / `--my`
  to every card so neighbours share one spotlight. 3D tilt with `data-depth` layers only on fine
  pointers ≥ 900 px without reduced motion; a back-out "mechanical settle" on leave. Only transform
  and opacity animate (a unit test checks the stylesheet). Variants default, featured, accent plate,
  compact; amber focus glow. Functional pages use it with `interactive={false}`.
- **Videos**: Playwright journeys on the launcher's browser mock (`apps/launcher/e2e/
  journeys-video.spec.ts` + `video-kit.ts`): DevTools screencast of every painted frame at
  1920 × 1080, an injected cursor (original arrow) moved along eased Bézier curves, click ripples,
  human typing rhythm, zooms on details, title cards in the website's style. `scripts/
  encode-videos.mjs` (ffmpeg) holds each frame until the next paint, outputs 60 fps CFR H.264 MP4
  (`+faststart`) and VP9 WebM under 8 MB each (CRF raised if needed) and a WebP poster into
  `apps/web/public/media`. `pnpm --filter @rustic/launcher videos` redoes everything.
- **Screenshots**: `pnpm screenshots` (launcher mock), then `scripts/export-stills.mjs` exports a
  selection as AVIF + WebP at 720 / 1440 px for the website (`<picture>` with both sources).
- **Website use**: `JourneyVideo` (muted, loop, inline, sources attached only near the viewport,
  paused off screen, poster image only under reduced motion), in the hero window and three feature
  rows; a screenshot gallery on the download page. Every one is labelled as example data.
- **Original look**: the 3D player's default colours changed to Rustic's own (rust hoodie,
  charcoal trousers, light sneakers), unlike the game's default characters. No game textures,
  no in-game footage.

## Consequences

- About 14 MB of media in `apps/web/public/media` (four journeys, two codecs each, posters,
  stills); fetched only when scrolled near.
- Revised 2026-11-11 (empty windows): the screencast frames are full-range JPEGs and the VP9 WebMs
  were encoded full range; Chrome's hardware VP9 decoder (D3D11) rejects those with
  `PIPELINE_ERROR_DECODE`, and a decode error does not make the browser try the next `<source>`,
  so every product window turned black on machines with a GPU. `encode-videos.mjs` now converts
  to limited-range BT.709 and tags it; `JourneyVideo` steps down by itself on any media error
  (WebM + MP4 → MP4 → poster image) and starts after the title card. Two journeys were added:
  `social` (presence changes, a message arrives and gets an answer, a party invite, joining a
  friend; scripted through the browser mock's `window.__rusticSocialMock`) and `console` (a Rustic
  server's console and plugins: a plugin that failed, updated and reloaded while the server runs).
- The screencast emits frames when the page paints, so motion is 30-60 unique frames per second;
  the 60 fps output repeats frames during still moments.
- Re-recording is deterministic (seeded cursor paths, fixed mock data).

## Alternatives

- Playwright's built-in `recordVideo`: fixed ~25 fps, low bitrate VP8, no frame timestamps.
- Screen capture of the desktop app: needs a display, not reproducible in CI, shows the OS chrome.
- Lottie / hand-made animations of the UI: would drift from the real product.
