# 0028. Lunar-like organisation: Launchpad, Social window, frameless title bar, neutral surfaces

- Status: accepted (frontend, 2026-11-10, step 1b)
- Supersedes parts of: ADR 0021 (surface colours), 0026 (shell layout)

## Context

The user reviewed the first UI and asked for an organisation much closer to Lunar Client's (layout and
interaction patterns only; never its name, logo, artwork, texts, icons or green accent; brief at the end of
`launcher/AGENTS.md`, references in `docs/references/lunar-*.png`, local and gitignored).

## Decision

- **Surfaces**: every palette now shares neutral very dark blue-grey / anthracite surfaces
  (`#0C0D10` → `#25262D`, flat 1 px borders, no warm tint, no ambient glow); only the accent family changes
  (Ember default, Verdigris, Heather from PALETTES.md). AA is re-checked by `palettes.test.ts` for all palettes
  and both modes.
- **Chrome**: no OS decorations (`decorations: false` on the `main` window). A 44 px custom title bar holds the
  product switcher ("Rustic"), back/forward, a route breadcrumb, then search, notifications, social (unread badge),
  the account pill, the "N instances running" pill (running games + background downloads, with the global
  progress line) and the window controls. Empty areas carry `data-tauri-drag-region`. In the browser build the
  window controls are hidden.
- **Rail**: plain outline icons with tooltips (Launchpad, Versions, Explore, Servers, Locker, Store), a separator,
  then "+" (new profile) and Settings; the active item is a small accent bar on the rail's left edge.
- **Pages**: the content area is one rounded panel. Launchpad = greeting + online pill, hero card with original
  SVG art and the big centred two-line LAUNCH button (caret to switch instance, gear to instance settings),
  "Latest news (N unread)" grid; right column with featured server icons and the friends strip ("Open social").
  Versions = fused source/search bar + sort + "New profile", portrait covers drawn in code, right detail panel.
  Explore = type dropdown fused with search, rows with View/Install, right column with source tabs, category
  chips, compatibility and Horizon versions. Servers = view dropdown fused with search, sort, ping refresh,
  grid/list, "Create server" card, banner cards with view/join icons.
- **The bottom bar and the docked friends panel are removed.** PLAY lives on the Launchpad hero and on every
  Versions cover/detail; long work is surfaced by the running pill.
- **Social window**: friends, requests, party and chats (Friends / Chats tabs, conversation pane with
  "Select a conversation" empty state). Desktop: a separate frameless `WebviewWindow` labelled `social` loading
  `index.html#/social` (no shell). Browser build: an in-app overlay with the same content. `openSocial()`
  chooses, and falls back to the overlay if window creation is refused.
- **Capabilities (least privilege)**: `default.json` (window `main`) adds only
  `core:window:allow-minimize | allow-toggle-maximize | allow-close | allow-start-dragging | allow-unminimize |
  allow-set-focus` and `core:webview:allow-create-webview-window` (to open the Social window). `social.json`
  (window `social`) gets `core:default` plus the same minimise/maximise/close/drag permissions, nothing else.
  No fs, shell, http, opener or dialog permission is added to either.

## Consequences

- The first-launch flow, Locker, Store, Settings, server management, domains and VPS screens keep their
  structure and pick up the new surfaces automatically (tokens).
- Chat is a new `CloudClient` surface (`conversations`, `messages`, `sendMessage`, `markConversationRead`);
  `http.ts` maps `/v1/messages/{user_id}`; a conversation list and read receipts are requested from the cloud
  engineer (HANDOFF).
- E2E journeys address PLAY through the Launchpad (`play-button`) or a Versions cover (`play-<id>`).

## Alternatives

Keep OS decorations (inconsistent with the requested look), keep the docked friends panel everywhere (rejected by
the user), an in-app drawer instead of a window on desktop (no detach to a second screen).
