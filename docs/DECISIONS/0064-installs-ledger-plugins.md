# 0064. Installs follow Horizon's Content API; a ledger; plugins checked as Rustic Server checks them

- Status: accepted (content, 2026-11-10)

## Decision

- Files go where `horizon/docs/api/content.md` says and are switched on the way Horizon reads them
  (`horizon-mods.txt`, `options.txt` `resourcePacks`, `horizon-graphics.txt` `pickedShaderPack`,
  `saves/<world>/datapacks.json`). Data packs for every new world go to `new-world-datapacks/`, which Horizon copies
  into each world it creates (`docs/modpacks.md`). Downloads are staged and checked, then moved: nothing unchecked
  reaches the game folder.
- `rustic-content.json` records what Rustic installed (uninstall, enable/disable, updates, Rustic packs).
- `.hpack`: its mods from the index, pinned by SHA-256, `on` honoured; `overrides/` limited to Horizon's list and
  sizes (anything else refuses the pack before a file is written).
- Native mods: shown with their badge, never built by Rustic; files copied only with `allow_native`.
- Plugins: `docs/PLUGIN_FORMAT.md` on `main` (final) is mirrored in `plugins.rs` rather than depending on
  `rusticmc-core`: the launcher worktree has no stable relative path to `main`'s crates, and `rusticmc-core` brings the
  runtime's dependencies (zip 9, permissions, timings). The mirror is tested rule by rule; HANDOFF asks for a small
  shared `rusticmc-plugin-format` crate.

## Consequences

Two implementations of the plugin rules to keep in step (RISKS). A server is Rustic Server when `server.toml`'s
`[engine] path` names the `rustic-server` executable.
