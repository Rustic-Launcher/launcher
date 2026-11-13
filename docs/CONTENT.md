# Content: sources, compatibility, installs, packs, plugins

Owner: content. Code: `launcher/crates/rusticmc-content`. Decisions: ADR 0063 (sources and HTTP), 0064 (installs,
plugins), 0065 (`.mrpack` import and Rustic packs), 0066 (fair-play filter), spec ADR 0016. Every Horizon fact below
is from `<server checkout>\horizon` at a84f637d (`docs/api/content.md`, `docs/api/mod-index.md`,
`docs/mods.md`, `docs/native-mods.md`, `docs/shaders.md`, `docs/modpacks.md`, `docs/mod-browser.md`).

## The model

| Type | What |
|---|---|
| `ContentKind` | `mod` (Horizon `.hmod`; a Modrinth jar is a Java mod), `native_mod` (`.hnmod`), `modpack` (`.hpack`, `.mrpack`, Rustic pack), `resourcepack`, `shader` (Horizon `.hshader`, WGSL), `shaderpack` (Iris/OptiFine `shaders/` zip), `datapack`, `plugin` (Rustic Server `.rplugin`) |
| `SourceKind` | `modrinth`, `horizon` (registry `index.json`), `rustic` (Rustic Cloud), `local` |
| `ContentVersion` | id, version, game versions, loaders, files (URL, name, size, sha1/sha256/sha512, minisign signature), dependencies (required/optional/incompatible/embedded), Horizon facts (API line, `hooks`, side, `broken_on`, project key) |
| `Target` | `instance {game_dir}`, `server {server_dir}`, `world {game_dir, world}`, `new_worlds {game_dir}` |
| `Compatibility` | `verdict` (`compatible`, `partial`, `needs_build`, `incompatible`, `blocked`) + `badge` + reasons |

`HorizonEnv` is what the instance's build loads, read from `horizon-client --launch-api` (`content` block): content
mod API lines (`0.2`, `0.3`), `NATIVE_HOOKS_API` (415 at the pin), shader API, resource pack format `[97, 1]`, data
pack format `[121, 0]`, `.hpack` format 1, index schema 1. Verdicts never guess past it.

## Sources

- **Modrinth API v2**: search with facets (`project_type:*`, `versions:26.3`, any extra group), projects, versions
  (filtered by loaders and game versions, newest first), files with hashes, dependencies. Every request carries
  `User-Agent: rustic-launcher/<version> (<contact>)`, the contact from the launcher's config. Rate limits:
  `X-Ratelimit-Remaining: 0` makes the next request wait for `X-Ratelimit-Reset`; a 429 is retried after the reset
  (else exponential backoff), as are 5xx and network errors, at most `max_retries` times. Files only from Modrinth's
  documented hosts (`cdn.modrinth.com`, `github.com`, `raw.githubusercontent.com`, `gitlab.com`).
- **Horizon's registry**: `https://horizon-rust.github.io/horizon-mods/index.json`, schema 1 (a newer schema is
  refused: "update"), unknown fields ignored, unknown kinds kept and not offered. Before install, the checks
  `horizon_mods::index::check_file` makes: the address is a registry release asset or a release asset of the
  project's own repository; size at most 64 MiB and equal to the entry's; SHA-256 equal; not revoked (by hash, or
  every version of an id); minisign signature verified with the project's key (unsigned projects say so); the file's
  own manifest matches the entry (`horizon-mod.toml`, `horizon-native.json`, `horizon-modpack.toml`,
  `horizon-shaders.toml`, `pack.mcmeta`).
- **Rustic registry** (Rustic's catalogue): a `RusticRegistry` trait; `HttpRusticRegistry` over
  `GET /v1/content/search` and `GET /v1/content/projects/{id}/versions` at the endpoint from config;
  `DisabledRusticRegistry` when none is set. See "Rustic catalogue and marketplace" below.

## What runs on Horizon (badges)

| Kind | Verdict | Why |
|---|---|---|
| Horizon content mod (`.hmod`) | **Works** when its manifest `api` is listed by the build, else **Doesn't run** | Horizon reads content at start (`docs/api/content.md`) |
| Horizon native mod (`.hnmod`) | **Builds on your Horizon** (verdict `needs_build`) when its `hooks` <= the build's `NATIVE_HOOKS_API` (or its pin is the instance's commit); **Needs newer Horizon** otherwise; **Doesn't build on your Horizon** when `broken_on` names the instance's commit | See "Native mods" |
| Java mod (Fabric, Forge, NeoForge, Quilt jar) | **Needs port**, never installed | Horizon has no JVM; data and assets can be carried by `horizon-port-pack` |
| Resource pack | **Works** made for 26.3 (pack format 97 in its range), else **Partly** | loads, marked too old/new as vanilla does |
| Horizon shader pack (`.hshader`) | **Works** when its shader API is listed | WGSL against Horizon's stages, checked by the game |
| Iris/OptiFine shader pack | **Partly** | Horizon's own GLSL loader (`horizon-shaders`), not every feature; Horizon's launcher offers only registry-reviewed ones |
| Data pack | **Partly**, into a world or new worlds only | Horizon runs part of a data pack (functions, function tags, predicates and item modifiers per its mod browser; worldgen in part) |
| `.hpack` | **Works** when its format is the build's | its mods come from the index, pinned by SHA-256 |
| `.mrpack` | **Partly** | see "`.mrpack` import" |
| Rustic Server plugin | **Works** on a Rustic Server, **Wrong target** elsewhere | engine `rustic-server` |
| Bukkit/Paper plugin (Modrinth) | **Doesn't run** | Java |

### Native mods and a third-party launcher

Horizon's native mods are Rust source compiled *into the game* by Horizon's own launcher (`horizon-launcher
native-build --trust-native-code`: it downloads Rust 1.99.1, horizon-mixins and the Horizon source of the pinned
commit, then builds a `native-<commit>-<key>` version; 1.5 to 30 minutes, about 2 GB). Horizon's Content API says a
launcher that doesn't build Horizon "doesn't offer them". Rustic Launcher does not compile Horizon (ADR 0006: the
Rustic edition is prebuilt by CI), so it shows native mods with the badge their `hooks` earns against the instance's
`NATIVE_HOOKS_API` and the reason, and installs their file into `horizon-native-mods/` only with `allow_native`
(an expert's choice): they run only after the player builds them with Horizon's launcher. The Rustic edition client
itself never contains third-party native mods (v1).

## Where content goes (Horizon's Content API)

| Kind | Folder | Switch |
|---|---|---|
| Content mod | `horizon-mods/` | `horizon-mods.txt`: `mod.<id>:true` (the id from the file's `horizon-mod.toml`) |
| Native mod | `horizon-native-mods/` | `mod.<id>:true` chooses it for the next build |
| Resource pack | `resourcepacks/` | `options.txt` `resourcePacks:["vanilla","file/<name>"]` (appended: highest priority) |
| Shader pack (both formats) | `shaderpacks/` | `horizon-graphics.txt` `pickedShaderPack:<name>` |
| Data pack, one world | `saves/<world>/datapacks/` | `saves/<world>/datapacks.json` `enabled`/`disabled` (`file/<name>`) |
| Data pack, new worlds | `new-world-datapacks/` | the game copies the folder into every world it creates (`docs/modpacks.md`) |
| Plugin | `<server>/plugins/<id>.rplugin` | loaded when present |

Change these only while the game is stopped. Downloads go to `.rustic-staging/`, are checked, then copied in place;
a file that fails never reaches the game folder.

`rustic-content.json` (the **ledger**, in the game or server folder) remembers what Rustic installed: source,
project, version, files with hashes, target, enabled, the game id, who needed it, the address. Uninstall, enable/
disable, update checks and Rustic pack export read it; the game never does.

- **Dependencies**: `resolve::resolve` installs required dependencies first (depth first, each once, at most 64),
  skips what is installed, stops on an installed `incompatible` project, leaves optional ones to the player.
- **Enable/disable**: switches only; nothing deleted.
- **Uninstall**: files and switch removed; refused while an installed project needs it.
- **Updates**: Horizon registry (newest installable for the instance's game and API) and Modrinth (newest compatible
  for 26.3); never native mods, packs inside one world, or a modpack's pinned mods.

Downloads go through the `Downloader` trait: `CoreDownloader` wraps the desktop owner's `DownloadManager`
(`rusticmc-launcher-core`: resumable, cached, verifying the strongest hash) and then verifies every other hash the
source listed; `MemoryDownloader` serves tests.

## Rustic catalogue and marketplace (ADR 0155)

Rustic Cloud's catalogue holds admin-curated entries and, since the marketplace, projects creators
publish from the website: mods (`.hmod`), native mods (`.hnmod`, or a native `.hmod`), modpacks
(`.hpack` or `.rusticpack`), resource packs, Horizon shader packs (`.hshader`) and data packs.
(`shaderpack` and `plugin` stay curated.) What the launcher sees:

- **Search** (`GET /v1/content/search`) lists public, published projects only; it also takes
  `category`, `tag`, `sort` (`relevance`, `downloads`, `newest`, `updated`), `price` (`free`,
  `paid`), `verified=true`, and answers `has_more` and `total`. Hits keep the `ProjectSummary` JSON;
  extra card fields (`icon_url`, `gallery`, `owner`, `updated_at`, `size`, `verified`, `price`,
  `latest_version`) are optional on the wire.
- **Versions** keep the `ContentVersion` JSON and add `horizon` (the manifest facts as
  `HorizonFacts`: a content mod's API line, a native mod's `native-1/<horizon>/<mixins>` and
  `hooks`, a modpack's format, a shader pack's shader API, a pack's pack format), so `compat::check`
  gives real verdicts for Rustic content. Pending (native, under review), rejected and yanked versions
  are never listed to the launcher.
- **Files**: `files[].url` is the API's download address (`https://api.rustic-launcher.com/v1/content/
  files/<version id>/<file>`), which counts the download and redirects to
  `https://cdn.rustic-launcher.com/marketplace/files/...`; `size`, `sha256` and `sha512` are given, so
  the downloader's checks apply as for any source (https only, size, hash).
- **Checks before anything is published**, server side: archive safety (size, entries, real unpacked
  size, decompression bombs, traversal, links, encryption), no programs in any kind, the kind's
  manifest parsed as Horizon parses it, id and version consistent with the form, images sniffed and
  re-encoded, and the same fair-play policy as `FairPlayPolicy` (the API links this crate). Native
  mods are reviewed by an admin before they become public.
- **Unlisted** projects are reachable by id (`GET /v1/content/projects/{id}`, `.../versions`) but never
  searched; **private** ones answer 404 to the launcher (it sends no token).
- **Install link** from the website: `rustic://content/rustic/<project>[?version=<version id>]`
  (spec for the launcher in `HANDOFF.md`).

## Fair play

`FairPlayPolicy` (ADR 0066, public policy `FAIR_PLAY.md`): a deny list of project ids/slugs and terms matched on
whole words in name, slug, categories and tags (X-ray, auto-clickers, kill-aura, aim bots, ESP, tracers, ore finders,
fly/speed hacks, scaffold, dupes, crashers, session stealers...) **block** a project: never promoted, never installed
through Rustic. A term only in the summary, or a feature fair in single player only (freecam, fullbright), **flags**
it: not promoted, installable with a notice. Protections ("anti-xray") never match. `promotable()` filters featured
and recommended lists.

## `.mrpack` import

`mrpack::import(pack, game_dir, downloader, port, opts)` reads `modrinth.index.json` (format 1, game `minecraft`) and
the overrides (`overrides/`, then `client-overrides/`, which wins; `server-overrides/` skipped), and imports only what
runs, with a line per file (`imported`, `imported_partly`, `needs_port`, `skipped`, `failed`) and its reason:

- `resourcepacks/*`: imported (index files downloaded from Modrinth's allowed hosts, checked against SHA-512 and
  SHA-1; overrides copied).
- `shaderpacks/*`: imported, partly (Iris/OptiFine packs).
- `datapacks/*`, `global_packs/*_data/*`: into `new-world-datapacks/`, partly.
- `mods/*.jar`: **needs port**. Their code cannot run; when the player has Horizon's `horizon-port-pack`, it is run
  (`port <pack> <work>`) to carry their data and assets as Horizon's own launcher does; Rustic does not reimplement
  the carry (upgrade to 26.3, registrations from the Java oracle).
- `config/`, `defaultconfigs/`, `kubejs/`, `scripts/`, `options.txt`, server-only files: skipped with the reason.
- Unsafe paths, unknown hosts, files over 512 MiB: failed. Overrides at most 65,536 files and 512 MiB.
- A pack for another game version is called out (older resource packs show the compatibility warning; data packs
  may need `horizon-port-pack upgrade-datapack`).

## Rustic pack format (`.rusticpack`)

A zip holding `rustic-pack.json` and, optionally, `overrides/`:

```json
{
  "format": 1,
  "name": "Cozy",
  "version": "1.0.0",
  "description": "",
  "game": "26.3",
  "horizon": { "mod_api": ["0.2", "0.3"], "native_hooks": 415 },
  "content": [
    { "source": "modrinth", "project_id": "RP1", "version_id": "abc", "version": "1.2.0", "name": "Faithful",
      "kind": "resourcepack", "target": "instance", "enabled": true,
      "file": { "name": "Faithful.zip", "url": "https://cdn.modrinth.com/...", "size": 1234,
                "hashes": { "sha1": "...", "sha512": "..." } } }
  ],
  "bundled": ["horizon-graphics.txt"]
}
```

- Content from a source is **listed, never copied** (address + hashes): a pack carries nobody else's files.
- `overrides/` may hold only `options.txt`, `horizon-graphics.txt`, `horizon-extras.txt` (when the exporter chose
  "settings") and the player's own files added by hand under `resourcepacks/`, `shaderpacks/`,
  `new-world-datapacks/` (when chosen: their licence is the player's responsibility).
- Not exported: content installed in a single world.
- Import installs each listed file through the normal installer (source checks, fair play, compatibility), then the
  allowed overrides. Unknown fields are refused; a newer format asks to update.

## Rustic Server plugins

`plugins.rs` mirrors `docs/PLUGIN_FORMAT.md` (main branch, sections 1 to 3): `rustic.plugin.toml` with strict
validation (id `[a-z0-9_-]{1,64}` not reserved, semver, `api` compatible with the server's `0.1`, authors, SPDX
licence, entry path ending in `.wasm`, dependency keys and Cargo version requirements, no required+optional or
dependency+`load_before` overlap, network `false` or host names only, limits), archive caps (manifest 64 KiB,
`config.toml` 1 MiB valid TOML, component 256 MiB with the `\0asm` magic, resources 64 MiB, 10,000 entries, no
unsafe names), and the server's resolution (duplicate ids refuse every copy; missing, mismatched or refused required
dependencies refuse transitively; required cycles refused; optional and `load_before` only order, soft cycles broken
with a warning; ties by id). `install` refuses a vanilla Horizon server (engine not `rustic-server`) and a plugin
the server would not load, replaces an older version, and returns the load order.

## In the app (Tauri commands, `apps/launcher/src-tauri/src/cmd/content.rs`)

| Command (docs/IPC.md) | What it does |
|---|---|
| `content_search` | Horizon registry (index fetched at most every 10 min, searched locally), Modrinth (live search with facets), Rustic catalogue (Rustic Cloud `/v1/content`, empty until the cloud serves it); verdicts against the target's `--launch-api` (Horizon defaults without a target); fair-play **blocked** projects never listed, **featured** only for clean projects; `installed_in` from the target's ledger. Ids: `horizon:<id>`, `rustic:<id>`, Modrinth ids bare. |
| `content_get` | Details per source: versions with verdicts, dependencies with names and installed state, gallery, links, side, changelog (Horizon). |
| `content_install` | Resolves dependencies, installs through `Installer` (all checks) into an instance, a world, new worlds (a data pack aimed at an instance) or a server; a Modrinth modpack is downloaded and imported as an `.mrpack`; a Horizon `.hpack` through the index; a Rustic `.rplugin` through the server's plugin manager after `plugins::read_rplugin`. |
| `instance_import { modpack }` | A catalogue modpack into a new instance (same path). |
| `instance_content_list / set_enabled / remove` | Files on disk merged with the ledger: Rustic-installed content switches with Horizon's own files and uninstalls with its dependants checked; hand-added files keep the move-to-`.disabled/` behaviour. |
| `instance_content_update` | Horizon registry and Modrinth updates of the ledger, installed in a background task. |
| `server_plugin_install { project_id }` | A catalogue plugin (Rustic catalogue); Modrinth plugins are Bukkit/Paper and refused. |

Verified against the real sources on 2026-11-10 (`cargo run -p rusticmc-content --example live_install -- <dir>`):
Horizon's live index (schema 1, 158 projects, 3 revoked) read; `horizon:abelha_skull` 1.0.0 installed with
Horizon's checks; Modrinth's Fresh Animations 1.11.2 (26.1+) found, judged **Works** (pack format 84..=999
includes 97), installed with SHA-512/SHA-1 checks and switched on in `options.txt`.
