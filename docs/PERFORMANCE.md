# Website performance (apps/web)

Targets: Lighthouse performance **≥ 90 on mobile** and **≥ 95 on desktop**, with accessibility,
best practices and SEO at 100, on the production build of the home page. ADR 0141 covers the motion
stack; this page holds the loading budget and how it is met.

## Budget (gzip, production build)

| What | Budget | Now (2026-11-11) | Loaded |
|---|---|---|---|
| HTML of the home (static snapshot of the whole page, one language, ADR 0151) | ≤ 10 KB | **16.4 KB (over)**: the full content is in the HTML for crawlers; raise to 20 KB or trim | first |
| CSS | ≤ 20 KB | **25.4 KB (over)** | first (render-blocking) |
| Fonts preloaded (Inter latin, Instrument Serif italic) | ≤ 75 KB | 71 KB | first |
| Anton (condensed titles of the Rustic Server panels, ADR 0144) | n/a | 18.6 KB | with the home's lower sections, not preloaded |
| Site backdrop (badlands scene: CSS sky first, canvases on idle) | ≤ 1 ms per frame | 0.13–0.35 ms average (Playwright, 1440 × 900 and 390 × 844) | lazy chunk after idle |
| App JavaScript of the first screen (entry + shared + router + runtime) | ≤ 160 KB | **161.7 KB (over)** | phones: after the hero has painted; desktop: at once (the footer is its own chunk) |
| One language (one of the 30 complete bundles, never two; ADR 0161) | ≤ 17 KB (≤ 20 KB for non-Latin scripts: Cyrillic, Greek, Thai, CJK, Arabic, Hebrew, Devanagari; raised from 15 KB on 2026-11-13, lead decision) | Measured 2026-11-15 after batch 2c (gzip -9 of the built chunk; every language grew since batch 2b with the new texts): en 14.7, fr 16.4, de 16.7, da 15.7, fi 16.3, nb 15.8, he 16.6, ar 17.7, ja 17.8, th 18.2, hi 18.9, uk 19.1, ru 19.2 KB, **el 20.0 KB (at the limit)** | with the app (preloaded by an inline script) |
| Language menu (registry of the game's 143 locales + `LanguageSelect`) and its flags | n/a | 4.5 KB + 4 KB (flags) | when idle (a globe button stands in), never on the first screen |
| Below the hero (`home/Rest` + the Rustic Server parts it shares with /server) | ≤ 10 KB | 6.3 + 3.0 KB | when idle; its sections mount two screens ahead of the scroll |
| GSAP + ScrollTrigger | ≤ 45 KB | 44 KB | desktop full show only (intro, pinned scenes), on demand |
| Light stream (three.js) | ≤ 135 KB | not loaded any more (the home's background is the procedural scene, ADR 0150) | n/a |
| Toasts + animation library | n/a | 53 KB | when idle; animated UI parts come with the pages that use them |

`pnpm --filter @rustic/web build` prints the sizes; the Lighthouse numbers below are redone after any
change to the first screen.

## How the first screen stays light

- **Static snapshot of every public page** (ADR 0151). `scripts/prerender.mjs` (run by `pnpm build`)
  renders each public page in English and French with the built app in Playwright's Chromium and
  writes its content and head into `dist/<page>.html` / `dist/fr/<page>.html` (the home:
  `index.html`, `fr/index.html`). The browser paints the page from the HTML alone; the app starts
  over it and replaces it in one commit once the route's code is loaded (`main.tsx`). Decorative
  scenery (the home's layers, the miner, inline textures) is left out of the snapshot: the app
  draws it. Every other URL gets the empty shell (`app.html`). Without Playwright the build keeps a
  plain `index.html`.
- **No video without a GPU.** The product videos wait for the GPU probe's answer before fetching
  anything; on software rendering they are posters with a play button (a first run that let the
  hero video decode on a software-GL machine measured TBT 5.3 s on desktop).
- **Full show only where it fits.** `fullMotion()` = no reduced motion, ≥ 900 px, fine pointer and a
  GPU. Elsewhere: no intro, no WebGL (static gradient), no smooth scrolling, no pinned scenes, posters
  with a play button instead of autoplaying videos.
- **No GPU probe on the main thread.** Whether the renderer is software (SwiftShader, llvmpipe...) is
  asked in a worker with an `OffscreenCanvas`, only on wide fine-pointer screens, and remembered.
  Without a GPU the page drops blurs and WebGL (`html[data-gpu="software"]`).
- **Entrances are CSS.** `[data-reveal]` and split titles use CSS transitions started by an
  IntersectionObserver; what is on screen at load is never hidden.
- **Code splitting.** Only the home page is in the entry chunk; every other page, the account area,
  the store and the back-office load on demand. Strings used by one lazy page only (the /server page)
  ship with that page (`addPageStrings` in `src/i18n`), not in the language chunk.
- **Fonts without shift.** The fonts of the hero title are preloaded; the mono of small labels is not.

## Lighthouse 12.8.2 (home, production build, local API, software GL like PageSpeed servers)

| Run | Performance | Accessibility | Best practices | SEO | FCP | LCP | TBT | CLS | SI |
|---|---|---|---|---|---|---|---|---|---|
| Mobile | 98-99 | 100 | 100 | 100 | 1.2 s | 2.0 s | 40-100 ms | 0 | 1.7-2.0 s |
| Desktop | 96-97 | 100 | 100 | 100 | 0.5 s | 0.8-0.9 s | 0-40 ms | 0 | 1.6-1.8 s |
| Mobile, after the website redesign (2026-11-11 pm) | 94-99 | 100 | 100 | 100 | 1.2 s | 2.0 s | 10-270 ms | 0 | 1.8-2.1 s |
| Desktop, after the website redesign (2026-11-11 pm) | 97-98 | 100 | 100 | 100 | 0.5 s | 0.8-0.9 s | 0-10 ms | 0-0.002 | 1.5-1.6 s |

Three consecutive runs each, after the launcher restyle (2026-11-11). With this laptop's own GPU
instead of software GL, Lighthouse's frame screenshots stall the GPU process (see QA_REPORT): those
runs are not comparable. Before this work: mobile 59, desktop 95 (2026-11-11 morning).
Command: `npx lighthouse@12.8.2 http://localhost:5173/ [--preset=desktop] --chrome-flags="--headless=new
--use-angle=swiftshader --enable-unsafe-swiftshader"` against `vite preview` of a build made with
`VITE_API_URL=http://localhost:8080`.

## Block-world look (ADR 0170, 2026-11-15)

Production build, `vite preview`, Playwright Chromium (no throttling), median of 3 runs; "before"
is the same measurement on a build of the previous commit (7f92388).

| What | Before | After |
|---|---|---|
| CSS (gzip -9, render-blocking) | 29.1 KB | 31.2 KB (+2.1 KB: `blocks.css`) |
| Entry JavaScript (gzip) | 78.9 KB | 78.9 KB (+0.07 KB: the motion toggle) |
| Block textures (`public/media/blocks`, fetched by the stylesheet, not preloaded) | n/a | ~5 KB, 27 PNGs |
| Home, desktop: LCP / CLS on load / CLS after scrolling the page | 672 ms / 0.009 / 1.947 | 668 ms / 0.000 / 0.000 |
| /download, desktop | 280 ms / 0.048 / 0.048 | 492 ms / 0.045 / 0.045 |
| /server, desktop | 408 ms / 0 / 0 | 336 ms / 0 / 0 |
| /store, desktop | 256 ms / 0 / 0 | 452 ms / 0 / 0 |
| Home, phone (390 × 844) | 320 ms / 0 / 4.881 | 356 ms / 0 / 0 |
| /download, /server, /store, phone | 272 / 280 / 236 ms, CLS 0 | 432 / 292 / 404 ms, CLS ≤ 0.002 |

- The CLS after scrolling (sections mounting under the home's strata) is gone: the strata layers
  are now placed with transforms, which never count as layout shifts.
- LCP on /download and /store grows by about 0.2 s: the landing strata, now behind every page, is
  the largest paint there. Still far under the 2.5 s "good" threshold.
- The portal costs 0.6–1.6 ms per frame at 10 frames a second (Playwright, desktop) and draws
  nothing off screen; hover particles draw only while some are alive.
