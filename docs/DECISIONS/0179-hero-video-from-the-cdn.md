# 0179 — The hero video: free stock footage, served from the CDN, chosen in the back-office

Status: accepted (2026-11-16). Owner: web + cloud + desktop. Replaces the clip of ADR 0170's account
pages and of the Launchpad (docs/THIRD_PARTY.md, RISKS R-50).

## Context

The owner found the looping forest video of the launcher's Launchpad and of the website's sign-in
and sign-up pages blurry: a 736 × 414 clip, bundled in both apps. It was also a Pinterest download
of unknown authorship showing a shader render of the game (project rule: no game footage, no
assets of unclear rights). The owner asked for a 1080p to 2160p video that is truly free to use,
close in look to the old one (lush, misty forest, light shafts, slow motion, dark green mood under
the orange "LANCER LE JEU" button), and lighter for the launcher by coming "from the API / the CDN"
so it can change without a release.

## Decision

1. **The clip.** "The sun shines through the trees in a forest" by Matthias Groeneveld on Pexels
   (3840 × 2160, 29.97 fps, 43 s, H.264 at 61 Mbit/s; Pexels License: free, commercial use, no
   attribution required, modification allowed; terms recorded in docs/THIRD_PARTY.md). Chosen
   among about 70 Pexels results and 8 downloaded 4K candidates (frames compared at 1:1) for:
   dark green woodland, low sun through the trunks with shafts of light on the grass and mist in
   the background, a **slow** lateral camera move (calm behind the UI), no people, no brand, high
   bitrate source. The runner-up (Pexels 19190654, more volumetric rays, greener) pans about five
   times faster with a birch close to the lens: too busy behind a launch button, and it would
   betray a loop.
2. **The loop.** 11 s (330 frames from 0.5 s) played forward then backward without repeating the
   end frames: a 658-frame, 21.96 s **ping-pong** loop with no seam and no crossfade ghosting (a
   crossfade of a moving camera blends two different views; reversing a slow slider move reads as
   the slider going back). Light grade (gamma 0.95, saturation 1.08, contrast 1.03) to sit a touch
   darker under the UI's own gradients; no vignette (both pages already darken their edges), no
   audio, BT.709 limited range.
3. **The renditions**, in a content-addressed CDN folder `media/hero/<id>/` (id: the first 12 hex
   of the SHA-256 of the sorted `file:sha256` lines): 480p, 720p, 1080p, 1440p, 2160p, each in
   **AV1** (SVT-AV1, MP4), **VP9** (WebM) and **H.264** (High, MP4, `+faststart`), quality-targeted
   (CRF) rather than size-targeted, because foliage in motion is the worst case for every codec;
   posters (first frame, WebP,
   720p to 2160p); a `manifest.json` listing every file with its size, type with codecs string and
   SHA-256, the duration and the credit. nginx serves the folder as immutable
   (`location ^~ /media/hero/`). The first set is `7eb30672eb0b` (21.96 s, 658 frames each):

   | Height | AV1 (CRF 50/48) | VP9 (CRF 50/48) | H.264 (CRF 30/28) |
   |---|---|---|---|
   | 480p | 1.8 MB | 1.7 MB | 2.6 MB |
   | 720p | 4.9 MB | 5.7 MB | 7.9 MB |
   | 1080p | 10.4 MB | 12.6 MB | 16.2 MB |
   | 1440p | 16.5 MB | 19.8 MB | 25.2 MB |
   | 2160p | 29.7 MB | 34.2 MB | 47.3 MB |

   Posters: 164 KB (720p) to 743 KB (2160p). The whole set is 228 MB on the CDN; a client
   downloads one file.
4. **A bundled first paint.** A tiny 480p clip (two-pass, about 200 kbit/s: ≈ 0.55 MB in VP9 and in
   H.264) and a 960 × 540 WebP poster (67 KB) ship in the launcher (`public/media/hero-480.*`,
   `hero-poster.webp`) and the website (`public/media/auth/`): they play at once and offline.
5. **The API.** `GET /v1/media/hero` (public, `Cache-Control: max-age=300`) answers the active set
   with absolute CDN URLs. The active set is a **site setting** (`site_settings`, migration 0023,
   key `hero_media`, the checked manifest as JSON); without a row the set built into the API
   (`routes/backoffice/hero_default.json`) is served. Admins change it in the back-office
   (`/admin/hero`: name a set by id or by its manifest URL, preview it, save with a reason,
   audited `media.hero.changed`; "Restore the default"). Inputs never leave the CDN: an id is
   12-64 hex, a URL must be the CDN's own origin and path `media/hero/<id>/manifest.json`; the
   manifest is read from the CDN folder when the API sees it (then every file must be there with
   its size) or fetched from the CDN (5 s, 64 KB at most), and checked: version, id = folder,
   1-120 s, 1-32 renditions with plain file names, codec/container/codecs-string whitelists (AV1
   in MP4/WebM, VP9 in WebM, H.264 in MP4), at least one H.264, sizes (200 MiB per rendition,
   5 MiB per poster), SHA-256, https credit links.
6. **Picking a rendition** (`@rustic/ui` `heroMedia.ts`, shared): AV1, then VP9, then H.264, the
   first the engine plays (`MediaSource.isTypeSupported`, else `canPlayType`); then the smallest
   size whose height covers the box under `object-fit: cover` at the device pixel ratio (capped at
   2; 5 % under is accepted). `HeroLoop` plays the bundled clip and crossfades (1.2 s) to the HD
   one once it actually plays; a failing HD clip leaves the bundled one.
7. **The launcher** caches the picked file: `hero_media_fetch` (Tauri) downloads it into
   `<cache>/hero-media/<sha256>.<ext>` with the launcher's resumable, checksummed downloader (own
   queue, out of the game downloads' progress), keeps only that file, and the webview plays it
   through the **asset protocol**, enabled with an empty static scope and allowed at run time for
   that one folder; CSP `media-src` gains `asset: http://asset.localhost`. Only
   `https://cdn.rustic-launcher.com/media/hero/<id>/<file>.mp4|webm` is accepted. No download
   offline or with data saver (`navigator.connection`); the last set seen is remembered so a cached
   file still plays offline. Reduced motion shows the set's poster (the CDN), else the bundled one.
8. **The website** (account pages, desktop only as before) asks the API for the set and gives the
   HD `<source>`s in codec order at the panel's size (the browser takes the first type it plays,
   HTTP caching does the rest); phones and reduced motion keep a still.

## Consequences

- The launcher gets the full quality for its window (often 1440p or 2160p on HiDPI screens) once
  per computer, then plays it from disk; the installer only carries ≈ 0.6 MB of video.
- Changing the video is an upload plus a back-office form, no release; an old set stays valid as
  long as its folder exists (sets are immutable), so clients with a cached manifest keep working.
- One more public endpoint, one table, one Tauri command, the asset protocol (scoped to one folder)
  and an nginx block. Tauri's `protocol-asset` feature adds one crate, `http-range` 0.1.6 (MIT,
  maintained with Tauri; range requests for the asset protocol); no other dependency. The dev API serves `mp4`/`webm`/`avif` from its local CDN folder with the
  right types.
- Sizes are larger than a size-capped encode would give (foliage): 1080p ≈ 10 MB in AV1; the
  launcher downloads once, the website streams only on desktop.

## Alternatives

- **Stream from the CDN in the launcher** without a cache: re-downloads every start, and the CSP
  already allowed it; caching costs a command and the asset protocol but saves bandwidth both ways.
- **A blob URL over IPC** instead of the asset protocol: 10-40 MB through IPC at each start, and no
  range requests.
- **Size-capped encodes** (1080p ≤ 6-8 MB): visible smearing of the leaves at that rate; the HD
  file exists to be sharp.
- **A crossfade loop** of a longer excerpt: ghosting of trunks during the fade with a moving camera.
- **Mixkit, Pixabay, Coverr**: Pexels had the best match and the highest source bitrate; the
  Pixabay ("Ray of light", CC0) and other 4K candidates were brighter, greyer or busier.
