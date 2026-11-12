# 0121. One `HostingProvider` trait for local, VPS and Rustic Cloud

- Status: accepted (hosting, 2026-11-10); refines ADR 0015 and ARCHITECTURE §6.1

## Context

The UI manages servers the same way wherever they run. Desktop owns local servers (`rusticmc-servers`, empty when
this was written); cloud hosting is "coming soon".

## Decision

- `rusticmc_hosting::HostingProvider` (async-trait, object safe): `install` (with progress), `start/stop/restart`,
  `status`/`metrics`, `console_subscribe` (mpsc of `ConsoleEvent`, task aborted on drop), `send_command`, `logs`,
  `players`/`player_action`, `config_read`/`config_write`, plugins (list/upload/enable/disable/remove), datapacks,
  backups (create/list/restore/download), `update_engine`, `uninstall`. Methods take the server name; errors are one
  `HostingError` with a stable `code()` for the UI.
- `LocalProvider` is an adapter over an `Arc<dyn HostingProvider>` the desktop registers (the same trait, no second
  interface). Helpers (`PlayerAction::horizon_args`, `ServerStatus::from_horizon`, `PlayersSnapshot::from_horizon`)
  work for a local `horizon-server` too.
- `RusticCloudProvider` returns `NotAvailableYet` everywhere and documents the future API shape.
- ARCHITECTURE's `control()`/`files()` accessors are folded into concrete methods: a VPS cannot hand out raw file
  access without breaking least privilege.

## Consequences

Desktop implements one trait; the IPC layer routes `server_*` commands by provider.

## Alternatives

A separate `LocalBackend` trait (duplicate surface); exposing the raw control API to the UI layer.
