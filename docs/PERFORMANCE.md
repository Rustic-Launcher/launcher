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
| One language (one of the 23 complete bundles, never two; ADR 0161) | ≤ 17 KB (≤ 20 KB for non-Latin scripts: Cyrillic, Greek, Thai, CJK; raised from 15 KB on 2026-11-13, lead decision) | en 14.0, fr 15.5, de 15.9, es 15.4, it 15.1, pt-BR 15.2, pt-PT 15.4, pl 15.9, tr 15.4, nl 15.2, sv 15.1, cs 16.0, hu 16.5, ro 15.6, id 14.5, vi 15.8, ko 16.2, zh-CN 16.1, zh-TW 16.2, uk 18.2, ru 18.3 KB; **ja 17.1 and th 17.1 (over by 0.1 KB: Thai costs three bytes a letter before gzip; decide whether Thai and Japanese get the 20 KB of the non-Latin scripts)**. Measured 2026-11-13 after batch 2b (gzip -9 of the built chunk) | with the app (preloaded by an inline script) |
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
