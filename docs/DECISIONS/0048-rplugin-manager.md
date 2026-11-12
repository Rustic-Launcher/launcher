# 0048. Rustic Server plugins in the launcher

- Status: accepted (desktop, 2026-11-10)

## Context
docs/PLUGIN_FORMAT.md (main) is final; rusticmc-core implements it. The coordinator asked to depend on
rusticmc-core by path if it builds cleanly in the launcher workspace.

## Decision

Keep the launcher's own reader (`rusticmc_servers::plugins`), mirroring every rule of the spec, with a
conformance test on the spec's example manifest and its rule table. rusticmc-core lives only on `main`
(`C:/.../Rustic-MC/crates`), not in the launcher branch: a path dependency would be an absolute,
machine-specific path that breaks every other checkout and CI, and it pulls a second `zip` (9) and `toml` (1).
Enable/disable moves the file (or dev directory) to and from `plugins/.disabled/`; installs validate first and
replace an older version (never a silent downgrade, never over a dev directory); dependency checks report
missing, disabled, non-matching (`*` matches pre-releases), duplicates and required cycles.

## Consequences

Rules exist three times (core, content, servers): RISKS R-32. Switch to a shared `rusticmc-plugin-format`
crate when main and launcher merge (HANDOFF).

## Alternatives

Path dependency on rusticmc-core (see above); the content crate's reader (different shape, same risk).
