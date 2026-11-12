# QA report: desktop launcher against the real local backend

Date: 2026-11-10. Build: branch `launcher`, debug desktop build (`--features custom-protocol`) and an
unsigned release build. Host: Windows 11 x64, WebView2. No mocks: the desktop app talked to the real
Rustic Cloud API and realtime gateway running locally, a real Horizon nightly, real Modrinth and the
official Mojang download servers.

## Set-up (how to reproduce)

| Piece | How it ran |
|---|---|
| Postgres 18, Valkey, Mailpit | `docker compose up -d postgres redis mailpit` in `services/` (Postgres on host port 55432) |
| Rustic Cloud API (`:8080`), realtime (`:8081`) | `target/debug/rusticmc-cloud-api.exe`, `target/debug/rusticmc-realtime.exe` from `services/`, with `services/.env` copied from `.env.example` (dev seeds generated, gitignored; Stripe keys left empty on purpose) |
| Relay | not run (optional; covered by its own tests) |
| Desktop app | `VITE_CLOUD_URL=http://localhost:8080 VITE_REALTIME_URL=ws://localhost:8081 pnpm build`, then `TAURI_CONFIG="$(cat apps/launcher/src-tauri/tauri.local.conf.json)" cargo build -p rustic-launcher --features custom-protocol` (the overlay only widens the CSP `connect-src` to the local services) |
| Run | `RUSTIC_LAUNCHER_HOME=<temp dir>` (isolated home), `RUSTIC_CLOUD_API=http://localhost:8080`, `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` |
| Driving | Playwright over CDP: `apps/launcher/scripts/qa/*.mjs` (`cdp.mjs` = helpers; `j1`...`j10` = journeys; `mailpit.mjs` reads the verification mail) |
| Second account | created and driven through the API only (sign-up, Mailpit token, verify, login, accept, DM) |

## Journeys

| # | Journey | Result | Real or gated | Screenshots (`docs/screenshots/`) |
|---|---|---|---|---|
| 1 | Onboarding | Pass up to the Microsoft step | Microsoft sign-in **gated**: this build has no Azure client id, the step now says so clearly (was a generic error, B2); the rest of onboarding was completed by setting `onboarding_completed` | `70-onboarding-1-welcome`, `70-onboarding-2-microsoft`, `70-onboarding-3-microsoft-gated` |
| 2 | Rustic account sign-up, email verification, login | **Pass** | Real: sign-up form in the app → real API → verification mail in Mailpit → verify → sign-in with password from the app, tokens in Windows Credential Manager. The mail links to `WEB_BASE_URL` (the website, not part of the launcher), so the click was replaced by `POST /v1/auth/verify-email` with the mailed token. A cold start after the 15-minute access token expired refreshes once and keeps working (after B13/B14) | `71-rustic-signup-form`, `71-rustic-signup-sent`, `71-mailpit-verification`, `72-rustic-signed-in`, `72-rustic-session-after-refresh` |
| 3 | Install a Horizon instance | **Pass** | Real: nightly `nightly-2026-11-08-fdc87ae` downloaded, minisign signature checked, instance created. Game-file preparation from the app is gated by Microsoft ownership (no Azure id); it was done outside the app with `horizon-prepare` on the official Mojang 26.3 client/server jars, libraries and assets (SHA-1 checked; downloaded into a scratch folder, never committed) | `73-instance-new`, `73-instance-installing` |
| 4 | Launch the game | **Gated** | The launch is refused before Java starts: "Sign in with a Microsoft account that owns Minecraft: Java Edition to play." Now shown under the button with an amber dot (was invisible, then green: B6, B16) | `74-launchpad-ready`, `74-launch-gated` |
| 5 | Explore: search Modrinth, install a resource pack | **Pass** | Real Modrinth search (resource packs), Faithful 64x installed into the instance (was refused as a Java mod, B8) | `75-explore-modrinth-resourcepacks`, `75-explore-installed`, `75-instance-resourcepack-installed` |
| 6 | Local server (Horizon engine): create, start, console, server-list ping, restart, stop, backup | **Pass** | Real `horizon-server` supervisor + engine on this PC (prepared files from journey 3). Console shows the game log and answers `list`; ping 14 ms, MOTD, 0/20, version 26.3 (was ~2 s, B12); restart, stop and a manual backup (134 KB) from the app | `76-server-wizard-basics`, `76-server-wizard-host`, `76-server-wizard-access`, `76-server-create-progress`, `76-server-running-overview`, `76-server-console`, `76-server-list-ping`, `76-server-restarted`, `76-server-stopped`, `76-server-backup` |
| 7 | Social with a second account | **Pass** | Real: friend request from the app (Social window), accepted by the second account through the API → appears live (after B15); removal also live; DMs from the second account arrive live in the open conversation; reply from the app received by the API. The Social window is a real second Tauri window (`#/social`, its own WebView) | `77-social-window-request-sent`, `77-social-friend-accepted`, `77-social-dm-received`, `77-social-dm-live`, `77-social-chat-reply` |
| 8 | Store with Stripe test mode | **Pass (as far as possible)** | No Stripe key: the API answers `503 payments_unavailable`; the app now says payments are not available and nothing was charged (was "did not go through, try again", B17). The catalogue shown is fixtures left in the dev database by the cloud integration tests (`Pack`, `Top Hat`, price 0 to 8 EUR) | `78-store`, `78-store-purchase-dialog`, `78-store-purchase-error` |
| 9 | Settings: theme and palette | **Pass** | Light/Dark, Ember/Verdigris/Heather applied at once, kept after a reload | `79-settings-light-verdigris`, `79-settings-dark-heather`, `79-settings-dark-ember` |

## Bugs found and fixed (each with a regression test)

| # | Bug | Fix | Test |
|---|---|---|---|
| B1 | The desktop app had no Rustic Cloud link at all (`UnavailableCloud` stub): no sign-in, no sign-up | `src-tauri/src/cloud_link.rs` (`ApiCloud` implements `CloudLink` over the API), commands `auth_rustic_signup`, `auth_rustic_login_password`, `auth_rustic_login_mfa`, `auth_rustic_resend_verification`; sign-in / create-account form in Settings and onboarding (`RusticAuthForm.tsx`) | `cloud_link.rs` tests; `rusticAuth.test.ts` |
| B2 | Microsoft sign-in without an Azure client id showed a generic network error | `not_configured` reason with its own message | `validation.test.ts` |
| B3 | `#/instances?new=1` crashed the route (numeric search param) | search param coerced to string | `routes.test.ts` |
| B4 | Every build was labelled "Rustic edition", even plain Horizon nightlies | label from the version list; unknown builds are plain Horizon | `HorizonVersion.test.ts` |
| B5 | Game-file preparation failures were swallowed (progress stalled, no message) | `prepareOutcome` → error toast / onboarding failure state | `prepare.test.ts` |
| B6 | A refused launch showed nothing | backend refusal shown as the status | `usePlay.test.ts` |
| B7 | Missing / invalid dates crashed lists (`formatRelative`, `formatDate`) | return `''` | `format.test.ts` |
| B8 | Modrinth resource packs were installed as Java mods and refused (loader `minecraft`) | `kind_of` infers resource/shader packs from loaders | `crates/rusticmc-content/tests/modrinth.rs` |
| B9 | Explore's version chips listed every snapshot | `shownVersions` (target first, short ids, max 6) | `shownVersions.test.ts` |
| B10 | Server uptime read "up -1h -1m" right after a start | `formatDuration` clamps negatives | `format.test.ts` |
| B11 | CPU and RAM of a local server read 0 % / 0 GB on Windows (Horizon's supervisor measures them only on Linux/macOS) | the launcher samples the engine process with `sysinfo` when the supervisor leaves them empty | `cmd::servers::tests` |
| B12 | Server-list ping to `localhost` reported ~2000 ms (time of the refused `::1` attempt counted) | latency = status round trip after connect | `rusticmc-hosting` `slp::tests::latency_is_the_status_round_trip` |
| B13 | Concurrent token requests at start-up refreshed twice with the same refresh token; the cloud saw reuse and **revoked the session** (every call 401 afterwards) | single-flight refresh in `RusticAccount` (re-read under a lock) | `cloud::tests::concurrent_callers_share_one_refresh` |
| B14 | A revoked session kept failing silently instead of showing "signed out" | 401 `unauthorized` → `ReauthRequired`; the stored session is dropped on refresh or status | `a_revoked_session_signs_out`, `cloud_link` tests |
| B15 | Accepted friend requests and removals never appeared until a restart (`friend_added` / `friend_removed` frames ignored) | `friends-changed` realtime event refetches friends and requests | `http.test.ts` |
| B16 | A DM arriving in the open conversation stayed unread; a refusal had the green "ready" dot | re-mark read on each new incoming message; `statusTone` | `lastIncoming.test.ts`, `usePlay.test.ts` |
| B17 | Store errors from the real cloud all read "payment did not go through, try again" (the cloud's error code was never read) | `purchaseErrorCode` reads `CloudError.code`; new "payments unavailable" message | `purchase.test.ts` |
| B18 | A real build showed made-up featured servers, player counts and release notes (mock fallback) | the real client returns no news / featured servers until the cloud has endpoints; empty states on Home | `http.test.ts` |
| B19 | The TPS chart drew across the whole window on a server's first sample (TPS 7356 against max 20) | `LineChart` clamps values to its range | `packages/ui/src/components/LineChart.test.tsx` |
| B20 | Uptime did not reset on restart (profile time instead of the game process) | start time from the supervisor's `engine_uptime_seconds` | `cmd::servers::tests::uptime_follows_the_game_process` |

## Checks

| Command | Result |
|---|---|
| `pnpm typecheck` | pass |
| `pnpm lint` | pass (0 warnings) |
| `pnpm test` | pass: launcher 186, ui 103 |
| `pnpm e2e` | pass: 48 (the `screenshots` project regenerated the standard `docs/screenshots/0x-5x` images) |
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` (with `DATABASE_URL`, `REDIS_URL`) | pass: 399 passed, 0 failed, 1 ignored (56 suites; the API and realtime binaries had to be stopped first: Windows locks a running `.exe`) |
| `node apps/launcher/scripts/gen-ipc-doc.mjs --check` | up to date (119 commands, 16 events) |
| `pnpm format:check` | pass on every file changed here; 5 files fail that were not touched (`package.json`, `apps/launcher/package.json`, `packages/ui/package.json`: CRLF in this Windows working tree only; `docker-compose.yml`, `client-mod/rustic-companion/ci/rustic-edition.yml`: pre-existing) |

## Release build

`pnpm tauri:build` (release profile, 10 min on this PC), unsigned:

| Artefact | Path | Size |
|---|---|---|
| NSIS installer (per user) | `target/release/bundle/nsis/Rustic Launcher_0.1.0_x64-setup.exe` | 9.15 MiB (9,589,555 bytes) |
| MSI (per machine) | `target/release/bundle/msi/Rustic Launcher_0.1.0_x64_en-US.msi` | 12.87 MiB (13,492,224 bytes) |
| Installed program | `rustic-launcher.exe` (+ `uninstall.exe`) | 34.5 MiB |

- Both bundles are produced, then the command **exits 1**: the updater public key is configured but
  `TAURI_SIGNING_PRIVATE_KEY` is not set, so the updater artefacts cannot be signed (TODO_HUMAN D2). Unsigned
  local builds can pass `--config '{"bundle":{"createUpdaterArtifacts":false}}'` instead.
- NSIS: `setup.exe /S /D=<temp dir>` → exit 0; the installed app started (isolated home), showed onboarding
  step 1 (`80-release-installed-first-run`), `app_info` = 0.1.0 / stable / windows x86_64; `uninstall.exe /S`
  → exit 0, folder empty, no uninstall registry entry left.
- MSI: `msiexec /i ... /qn` without elevation fails with error 1925 (per-machine install needs an administrator);
  not installed here (no UAC prompt possible in this session). To test: an elevated `msiexec /i ... /qn`.
- Built without `VITE_CLOUD_URL` (no production domain yet): see "Production cloud URL" below.

## Website, "Continue with Microsoft", motion and media (2026-11-11)

Checked on the same host against the real local API (`services/`, Postgres + Valkey + Mailpit in
Docker; `MICROSOFT_JWKS_URL=http://localhost:8099/keys` so the API trusts the test key of
`apps/web/e2e/jwks.setup.ts`; Microsoft itself is stubbed at the network layer, nothing reaches it).

| Check | Result |
|---|---|
| Website e2e (`pnpm --filter @rustic/web e2e`, desktop + 360 px) | 36 passed: sign-up + Mailpit link, session restore, reset, TOTP, Microsoft new player / returning player + launcher one-time code (single use) / password account not merged then linked, pages, FR, status pill, home sections, journey videos (lazy, paused off screen, posters under reduced motion), intro, reduced motion, scroll, back-office, news reaching the Launchpad API, featured servers, checkout without Stripe, no horizontal scroll at 360 px |
| Cloud API (`cargo test -p rusticmc-cloud-api` with `DATABASE_URL`) | all passed, incl. `tests/microsoft.rs` (valid token, wrong `aud`, wrong `iss`, expired, nonce mismatch, replayed nonce, garbage, email collision, link conflict, unlink rules, password set, single-use handoff) |
| Workspace (`cargo test --workspace`, `cargo clippy --all-targets -D warnings`, `cargo fmt --check`) | 408 tests passed, clippy and fmt clean |
| Front-end (`pnpm typecheck`, `pnpm lint`, unit tests) | clean; ui 109, web 36, launcher 188 tests |
| Launcher e2e (browser mock) | 45 passed (2 content tests flaky under 4 workers, green on rerun) |
| Website build | main chunk 230 KB gzip, three.js chunk 133 KB gzip (home only, after the intro), CSS 16 KB gzip |
| Lighthouse 12.8.2, production build, home | software GL (CI-like, as a machine without GPU): **desktop 95** / a11y 100 / best practices 100 / SEO 100 (FCP 0.6 s, LCP 1.2 s, TBT 80 ms, CLS 0.002); **mobile 59** / 100 / 100 / 100 (FCP 2.9 s, LCP 5.7 s, TBT 450 ms). With this laptop's NVIDIA GPU, desktop performance 59: the GPU process stalls ~2 s while Lighthouse screenshots every frame during the entrances (trace: `CrGpuMain` GPUTask blocking the renderer's first frames); the same page traced without Lighthouse on the same GPU has no main-thread task over 72 ms |
| Videos | four journeys, 1920 × 1080, 60 fps CFR: launch 3.9 MB MP4 / 4.4 MB WebM (43 s), mod 1.9 / 2.4 MB (28 s), server 4.8 / 5.6 MB (89 s), cosmetic 2.1 / 2.6 MB (52 s), WebP posters 40-85 KB |

Fixed while measuring: the hero title re-wrapped when the serif swapped in (CLS 0.09 → 0.002, the
first-screen fonts are now preloaded), the hero pulse animated `left` (now a transform), prohibited
`aria-label` on spans, dim statement words under 3:1, a 401 console error from the start-up refresh
for anonymous visitors (now only when a sign-in left a session hint), and, on machines without a GPU,
blur and WebGL are dropped (`html[data-gpu="software"]`).

Not re-checked this round: the desktop app itself. A release `rustic-launcher.exe` started from
Explorer was running on the host, and the single-instance guard forwards a second launch to it; it
was left alone.

## Mobile performance and launcher restyle (2026-11-11, second pass)

| Check | Result |
|---|---|
| Lighthouse 12.8.2, home, production build, software GL (3 runs each, docs/PERFORMANCE.md) | **mobile 98-99** / a11y 100 / best practices 100 / SEO 100 (FCP 1.2 s, LCP 2.0 s, TBT 40-100 ms, CLS 0); **desktop 96-97** / 100 / 100 / 100 (FCP 0.5 s, LCP 0.8-0.9 s, TBT 0-40 ms, CLS 0). Before: mobile 59, desktop 95 |
| How | first-screen JS 229 → 151 KB gzip (one language chunk, every page but the home lazy, GSAP / Lenis / toasts / animation library on demand, CSS entrances); phones get a prerendered, hydrated first screen (`scripts/prerender.mjs`, React server renderer at build time) and start the app after the hero poster has painted; no WebGL probe on the main thread (worker); no intro, WebGL or autoplaying video on phones and touch screens (posters, tap to play) |
| Website e2e | 36 passed on the dev server; the 7 mobile tests also against the prerendered production build (hydration without mismatch, menu, language copies, sections mounting ahead of the scroll) |
| Launcher restyle (ADR 0143) | eyebrows and serif-italic accent titles on every page, `Card` / `.rx-surface` premium surfaces, warm grain background and dot-grid panel, blur → sharp page entrances; `html[data-paused]` while a game runs (checked in `e2e/shell.spec.ts`: set when the game runs, the hero light pauses, cleared after Stop); 3D viewers stop rendering while paused or hidden |
| Launcher e2e | 45 passed; screenshots (all of docs/screenshots) and the four product videos re-recorded (the UI in them changed) |
| Workspace | `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test --workspace` with `DATABASE_URL`: 408 passed; typecheck, lint, format:check clean; unit tests ui 111, web 36, launcher 188 |

With this laptop's NVIDIA GPU instead of software GL, Lighthouse still reports multi-second TBT: the GPU
process stalls while Lighthouse screenshots every frame (unchanged diagnosis from the first pass).

## Remaining (TODO_HUMAN and follow-ups)

- **Microsoft sign-in** (D1): an Azure app id (`RUSTIC_MSA_CLIENT_ID`) unblocks onboarding step 2, in-app
  game-file preparation, launch and linking the Minecraft account. Journeys 1, 3 (preparation) and 4 stop there.
- **Stripe** (CL5): test keys + webhook secret to run a real checkout (journey 8 stops at `payments_unavailable`).
- **Website** (CL7): done after this QA (ADR 0140): `apps/web` serves `/verify-email?token=` and
  `/reset-password?token=` against the real API (Playwright e2e with Mailpit).
- **"Continue with Microsoft" (ADR 0090), real round trip: pending.** Verified so far with Microsoft
  stubbed at the network layer: API integration tests with a mocked JWKS (valid token, wrong `aud`,
  wrong `iss`, expired, nonce mismatch, replayed nonce, garbage, email collision, link conflict, unlink
  rules, single-use launcher handoff) and website e2e (`apps/web/e2e/microsoft.spec.ts`: new player
  completes the account, returning player signs in and the launcher gets a one-time code that works
  once, password account not merged then linked from settings). The real check needs the two SPA
  redirect URIs on the Azure app (TODO_HUMAN W10): open `http://localhost:5173/login`, "Continue with
  Microsoft", sign in with a personal account, expect "Complete your account" (first time) then
  `/account`; from the launcher, Settings → Accounts → "Continue with Microsoft" must come back through
  `rustic://auth?code=` and show the session.
- **Production cloud URL at build time**: since the domain decision (2026-11-10),
  `apps/launcher/.env.production` sets `VITE_CLOUD_URL=https://api.rustic-launcher.com` and
  `VITE_REALTIME_URL=wss://realtime.rustic-launcher.com` for `vite build` (env overrides). The installers
  listed above predate it and used the sample cloud.
- **News and featured servers**: done after this QA (ADR 0140): `GET /v1/news`,
  `GET /v1/featured-servers`, published from the website back-office; the Launchpad reads them.
- **Signing** (D2, D6): installers are unsigned (SmartScreen warns); the updater key is not set.
- **Upstream Horizon**: the supervisor's RSS/CPU on Windows (worked around, B11) and a TPS sample in the
  thousands right after start (charts clamp it, the number is shown as reported).
- **Dev database hygiene**: `cargo test` for the cloud runs against the dev database and leaves store fixtures
  that then appear in a local catalogue; a separate test database would avoid it.
- Modrinth `updated_at` is empty in Explore cards (content crate does not map it yet).
- The QA overlay `apps/launcher/src-tauri/tauri.local.conf.json` is for local runs only (CSP `connect-src` to
  `localhost:8080/8081`); production builds do not use it.

## Website redesign follow-up (2026-11-11, afternoon)

| Check | Result |
|---|---|
| Empty product windows | Root cause: the VP9 WebMs were full range (from the screencast JPEGs); Chrome's hardware VP9 decoder (D3D11) fails them with `PIPELINE_ERROR_DECODE` and a decode error does not fall through to the MP4 `<source>`, so the video went black. Re-encoded limited-range BT.709 (all six journeys), and `JourneyVideo` steps down WebM+MP4 → MP4 → poster on any media error. Checked on this laptop's GPU (D3D11): every video readyState 4, no error, in `vite dev` and `vite preview`; posters on phones, under reduced motion and in the prerendered HTML |
| New recordings | `social` (presence changes, a message arrives and is answered, party invite, join a friend) and `console` (a Rustic server's console and plugins: failed plugin updated and reloaded) |
| Website e2e (real API, Docker) | 38 passed (home sections incl. the 4 × 2 sources grid, social/console videos; new /server page test) |
| Launcher e2e (browser mock) | 45 tests: 43 passed in the 4-worker run, the 2 others (home launchpad, a domains test) are load flakes, green when rerun alone |
| Lighthouse (home, production build, software GL) | desktop 97-98, mobile 94-99 (one 94 with TBT 270 ms, others 99); a11y / best practices / SEO 100 |
| Launcher fixes found while recording | the console missed lines logged while another tab was open (refetch on mount); a chat message reaching the cache twice (send + realtime + refetch) showed twice (dedupe by id) |
