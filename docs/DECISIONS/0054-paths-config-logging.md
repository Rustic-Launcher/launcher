# 0054. Paths, configuration and logs

- Status: accepted (desktop, 2026-11-10)

## Context
The launcher keeps data, config and caches per OS, needs a configuration that survives upgrades, and logs.

## Decision

`directories` 6 (MIT/Apache) `ProjectDirs("gg","Rustic","Rustic Launcher")`, or everything under
`RUSTIC_LAUNCHER_HOME` (portable installs, tests). `launcher.toml` with `version = 1`, every section
defaulted, older versions migrated in memory, newer ones refused. `tracing` + `tracing-subscriber` (env filter,
console) + `tracing-appender` (daily files, newest 7 kept); `log` records forwarded. Secrets are `Secret`
values that print `[redacted]`.

## Consequences

UI-only settings live in `settings.json` beside it; what Rust uses is mirrored into launcher.toml.

## Alternatives

A JSON config (no comments for hand edits); `log4rs` (heavier).
