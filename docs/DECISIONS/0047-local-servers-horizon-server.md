# 0047. Local servers through horizon-server

- Status: accepted (desktop, 2026-11-10)

## Context
horizon-server is Horizon's supervisor (CLI + `/api/v1`, server-reference.md). A Rustic Server is
horizon-server with `engine.path` pointing at `rustic-server`.

## Decision

- One folder per server (`--dir`), the launcher's `.rustic/profile.toml` beside server.toml. Creation runs
  `init --eula --yes ...` (the EULA ticked in the UI), then `config set engine.resources=<prepared build>` and
  `engine.path` (Rustic Server) through the CLI (the API's PATCH refuses `engine.*`).
- Running servers are driven through `/api/v1` with the token of `.horizon/api.json`: status/metrics, logs,
  server-sent events (resumed after the last id), commands, moderation, backups, restore, config, crash
  reports. Stopped servers use the CLI (backups, list edits, restore).
- `start` runs with its output in `.rustic/start.log`, never pipes: the background supervisor would inherit a
  pipe handle on Windows and a read to EOF would wait for the server to exit.
- Console completion from the prepared `commands.json` (Brigadier dump) plus online players: the API has no
  completion endpoint.
- `LocalServers` implements the hosting engineer's `rusticmc_hosting::HostingProvider`, so the app treats local
  and VPS servers alike; build changes back the world up first.

## Consequences

Tested end to end with a fake horizon-server binary that follows the CLI and API.

## Alternatives

Running `horizon-client --dedicated` ourselves (duplicates the supervisor's restart, backups, API).
