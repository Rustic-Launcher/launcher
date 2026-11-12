# 0016. Content sources and compatibility badges

- Status: accepted (spec, 2026-11-10)

## Context

Horizon runs content mods (`.hmod`), resource packs, Iris/OptiFine and WGSL shader packs, datapacks, `.hpack`
modpacks; native mods need a rebuild; Java mods never run, only their data/assets may be ported
(`horizon/docs/mods.md`, `launcher/src/browser.rs:299-368`). Rustic Server adds `.rplugin` plugins.

## Decision

- Sources v1: **Horizon registry** (`index.json`, schema 1), **Modrinth** (API v2, our User-Agent), **Rustic catalogue**
  (plugins and Rustic-curated content). **CurseForge later**: needs an API key, and its terms restrict distribution
  of files whose authors opted out of third-party distribution (TODO_HUMAN).
- Badges (computed locally, shown everywhere content appears): **Works** (native Horizon content for this build),
  **Partly** (ported Java data/assets with gaps; tooltip lists what does not run), **Doesn't run** (Java code mods,
  unsupported kinds), **Needs native build** (`.hnmod`, not supported in v1), **Server only** / **Client only** /
  **Both** (`side`), **Plugin for Rustic Server** (`.rplugin`, API version check against the server's `rustic-server`).
- Fair play: the catalogue hides/blocks items in the deny-list (FAIR_PLAY.md); a "Not promoted" state for allowed but
  unfeatured content.
- Downloads only from allow-listed hosts with hash checks; minisign check when the index has a key.

## Consequences

Content owner reimplements Horizon's compatibility logic (MIT, attributed) and keeps it in sync per Horizon bump.

## Alternatives

Only Horizon registry (too little content); executing Java mods (impossible on Horizon).
