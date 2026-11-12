# 0063. Content sources over the launcher's HTTP stack: Modrinth v2, Horizon's index, Rustic registry

- Status: accepted (content, 2026-11-10). Details spec ADR 0016.

## Decision

- HTTP through `rusticmc-launcher-core` (reqwest over rustls/ring, ADR 0041): one TLS stack. Modrinth requests carry
  `User-Agent: rustic-launcher/<version> (<contact>)` with the contact from config (TODO_HUMAN), honour
  `X-Ratelimit-Remaining`/`X-Ratelimit-Reset`, and retry 429, 5xx and network errors after the reset or an
  exponential backoff, capped.
- Horizon's registry is read as `horizon_mods::index` reads it (schema 1, unknown fields ignored, unknown kinds not
  offered) and every file passes the checks of Horizon's `check_file` (address shapes, size, SHA-256, revoked,
  minisign signature, manifest match). Minisign through `minisign-verify` 0.3, the version Horizon uses. Tested on
  Horizon's own sample index and signed files (MIT, copied into `tests/fixtures/horizon-index`).
- Rustic registry: a trait, an HTTP implementation over a proposed shape (HANDOFF), a disabled one without endpoint.
- Verdicts from `horizon-client --launch-api`'s `content` block (`HorizonEnv`), never from assumptions.
- Downloads through a small `Downloader` trait; `CoreDownloader` adapts the desktop owner's `DownloadManager` and
  then checks every hash the source listed. Tests use wiremock and an in-memory downloader: no live network.

## Crates

| Crate | Licence |
|---|---|
| `async-trait` 0.1 | MIT OR Apache-2.0 |
| `minisign-verify` 0.3 | MIT |
| `semver` 1, `serde`/`serde_json` 1, `sha1`/`sha2` 0.10, `hex` 0.4, `thiserror` 2, `tokio` 1, `tracing` 0.1, `toml` 0.9 | MIT OR Apache-2.0 / MIT |
| `zip` 4 | MIT |
| `wiremock` 0.6, `tempfile` 3 (dev) | MIT OR Apache-2.0 |

## Alternatives

CurseForge now (API key and redistribution terms: later, spec ADR 0016). Linking `horizon-mods` (not reachable from
the launcher workspace; its `index` feature pulls Horizon's shader checker).
