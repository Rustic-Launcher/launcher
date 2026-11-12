# 0152. Where the launcher finds `rustic-server`: the build folder first, a Settings override

Date: 2026-11-12 · Status: accepted · Builds on ADRs 0047, 0048, 0070, 0148

## Context

A Rustic Server is Horizon's supervisor (`horizon-server`) with `engine.path` pointing at
`rustic-server`, Horizon's game linked with Rustic's trait mod (ADR 0047; Rustic-MC
`crates/rusticmc-server`). The launcher looked for that program in three places only: the
`[rustic_server] executable` of `launcher.toml`, beside the launcher, and the PATH. No player has
either of the last two, and nothing in the UI wrote the first one, so "Create a server" with the
Rustic Server engine always failed with "The rustic-server program was not found. Set its path in
Settings > Rustic Server." (a setting that did not exist).

`rustic-server` reads the prepared game resources of the server's build (`engine.resources`), so it
has to come from the **same Horizon commit** (and the same Rustic patches) as that build: an engine
picked from elsewhere can refuse the resources or behave differently. The Rustic edition bundle
(ADR 0070) is already built from Rustic's patched Horizon at a given commit, from the same
repositories `rustic-server` comes from.

## Decision

- **The Rustic edition bundle ships `rustic-server[.exe]`** beside `rustic-client` and
  `horizon-server`, built from Rustic-MC `main` (`crates/rusticmc-server`, `--release --locked`) on
  the same Horizon commit with the same `patches/*.patch`. The CI sketch
  (`client-mod/rustic-companion/ci/rustic-edition.yml`) checks out Rustic-MC `main`, fails when
  `main` pins another Horizon commit than the one requested, builds both programs, runs
  `rustic-server --rustic-version` as a smoke test, signs both and records the Rustic-MC commit in
  `rustic-build.json`.
- **Lookup order** (`rusticmc-servers::engine`, used by `ServerManager::engine_settings` at creation,
  at every start and on a build change, by the Settings row and by VPS installs):
  1. the server's own engine (`profile.engine`, an uploaded or downloaded one);
  2. the manual override, `[rustic_server] executable` in `launcher.toml`;
  3. **the server's build folder**, `<builds>/<profile.build>/rustic-server[.exe]`;
  4. beside the launcher's program;
  5. the PATH.

  The first two are choices someone made: when that file is gone the lookup **fails** (the error
  points to Settings) instead of silently running another engine. An empty value means automatic.
  VPS installs look in the default build's folder, the build whose prepared resources they upload
  (`spec_for`), so engine and resources match there too.
- **Settings › Game & instances › Rustic Server** shows the program in use, where it comes from
  (server / chosen manually / shipped with build X / beside the launcher / PATH) and its
  `--rustic-version` line, or "not found" (and "the chosen file no longer exists" for a stale
  override). "Choose…" picks a file, which is accepted only when it answers `--rustic-version`
  (rustic-server's own flag: one line `Rustic <ver> (rustic:api <api>) on Horizon (Minecraft 26.3)`,
  exit 0; 30 s timeout, no console window) and saved to `launcher.toml`; "Use automatic" clears it.
  The change applies at once (`ServerManager::set_rustic_server`), for servers at their next start.
  IPC: `rustic_server_status`, `rustic_server_set { path | null }` → `RusticServerStatus`
  (docs/IPC.md); a refused file is `invalid_argument` with reason `server.not_rustic_server`.
- The creation error keeps its reason `server.no_rustic_server`; the wizard words it itself
  (French first) and its toast's action opens `/settings?section=game&focus=rustic-server`, which
  scrolls the row into view.

## Consequences

- A player with a Rustic edition build creates Rustic Servers with no setup; a plain Horizon build
  still needs a chosen program (the error says where).
- Changing a server's build also changes its engine to the new build's `rustic-server` (unless an
  override or a per-server engine is set), so engine and resources move together.
- Bundles grow by the engine (about 120 MB uncompressed on Windows in the `fast` profile; less with
  `release` + LTO). Release signing now covers two programs.
- The check against the prepared resources is the engine's own: the launcher only checks the program
  answers `--rustic-version`. A matching commit is guaranteed by the bundle, not by the override.
- The edition CI now depends on Rustic-MC `main` pinning the requested Horizon commit.

## Alternatives

- **Keep the launcher-wide setting only**: every player would have to find and point at a program
  that must match the build; that is the bug.
- **Download `rustic-server` separately** (a Rustic Server release channel, `EngineSource::RusticRelease`):
  a second artefact to version against each build; possible later for plain Horizon builds.
- **Fall back to the next place when a chosen file is missing**: quieter, but it could run an engine
  from another Horizon commit than the one the player picked, against resources it does not match.
