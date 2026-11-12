# 0008. Session hand-off to the game over stdin

- Status: accepted (spec, 2026-11-10)

## Context

The Launch API accepts `--session-stdin` (JSON written to stdin, then closed) or `--session-file PATH` (read once,
deleted, 0600, refused after 2 minutes) (`horizon/docs/api/launch.md`, "The session JSON"). The official launcher uses
the file (`horizon/launcher/src/account.rs:2008-2052`). Root rule: tokens never on a command line.

## Decision

Rustic always uses `--session-stdin`: the core reads the Minecraft token from the keychain, spawns the game with a
piped stdin, writes `{"name","uuid","access_token","user_type":"msa","xuid"}`, closes stdin, and zeroes its buffer
(`zeroize`). No file ever touches the disk. The companion's local-channel token goes in an environment variable
(`RUSTIC_COMPANION_TOKEN`, random, single use, readable only by the same user's processes); no Rustic refresh token is
passed to the game: the companion asks the launcher for a short-lived Rustic access token over the loopback channel.

## Consequences

No temp-file race or leftover file; works identically on Windows. Stdin can no longer be used for anything else.
Renewal of an expired Minecraft token during a long session: RISKS R-12.

## Alternatives

`--session-file` (disk exposure window, Windows ACL subtleties). Rejected.
