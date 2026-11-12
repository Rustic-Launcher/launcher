# 0040. Desktop crates and async runtime

- Status: accepted (desktop, 2026-11-10)

## Context
The Rust side of the launcher has to download, sign in, run Horizon's programs and talk to local
supervisors at once, and the Tauri shell already runs Tokio.

## Decision

- Four library crates with one job each: `rusticmc-launcher-core` (paths, versioned config, errors, tracing,
  the download manager, the shared HTTP client, `Secret`, `IpcError`), `rusticmc-auth` (accounts, owner gate,
  keychain, Rustic account seam), `rusticmc-instances` (builds, preparation, launch, instances),
  `rusticmc-servers` (local servers, plugins, access, domains). `apps/launcher/src-tauri` only maps IPC to them.
- Tokio everywhere (`tokio` 1, MIT); `async-trait` (MIT/Apache) for object-safe traits (`CloudLink`,
  `DnsProvider`, `DnsResolver`, `ReachabilityProbe`, the hosting crate's `HostingProvider`).
- Errors with `thiserror` 2 per crate, each with a stable `code()` and a `From` into
  `rusticmc_launcher_core::IpcError {code, message, details.reason}`.

## Consequences

Each crate is tested alone (wiremock, fake Horizon binaries). The Tauri crate stays thin.

## Alternatives

One big crate (slower builds, no isolation); `async-std` (Tauri is on Tokio).
