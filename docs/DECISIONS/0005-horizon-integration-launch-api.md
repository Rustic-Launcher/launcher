# 0005. Integrate Horizon through its Launch API, not through horizon-launcher

- Status: accepted (spec, 2026-11-10)

## Context

Horizon documents a contract for other launchers (`horizon/docs/api/launch.md`, `launch-guide.md`,
`launch-reference.md`; experimental, 0.1.0): `horizon-client --launch-api`, `horizon-prepare --prepare --output
--complete --progress json`, `--check`, `--session-stdin`, exit codes. `horizon-launcher` also has a CLI/MCP surface,
but it is a whole product with its own data folders and Horizon's Microsoft client id.

## Decision

Rustic drives `horizon-prepare` and the Rustic-edition client **directly** through the Launch API. We never run
`horizon-launcher`, never share its data folders, never use its MSA client id (`horizon/docs/forking.md`). We probe
`--launch-api` per build and pass only listed options. We reimplement (or reuse, MIT, attributed) what the official
launcher does around it: finding the player's game files (`horizon/launcher/src/sources.rs`), downloading Mojang files
and the server jar only for a signed-in owner.

## Consequences

- The Launch API is experimental: each Rustic-edition build pins a Horizon commit and CI snapshots its
  `--launch-api` JSON.
- No session renewal through `HORIZON_LAUNCHER_EXE` (RISKS R-12).

## Alternatives

Wrapping the `horizon-launcher` CLI: couples us to its UX and its client id (forbidden). Rejected.
