# 0024. Typed IPC client with a browser mock

- Status: accepted (frontend, 2026-11-10)

## Context

The UI talks to the Rust core through Tauri commands and events (docs/IPC.md). The UI must also run in a plain
browser (design work, Storybook, Playwright) and be built before the Rust side exists.

## Decision

- One TypeScript map is the contract: `apps/launcher/src/ipc/commands.ts` (`IpcCommands`: name → [args, result];
  `IpcEvents`: event → payload), types under `ipc/types/`. docs/IPC.md mirrors it.
- Convention for Rust: every command takes **one argument named `args`** (a serde struct, snake_case fields) or no
  argument; it returns `Result<T, IpcError>` where `IpcError = { code, message, details? }`. Commands are
  `snake_case`, events `kebab-case`. This avoids Tauri's camelCase argument renaming entirely.
- `ipc/client.ts` exposes `invoke(cmd, args)` and `listen(event, handler)`. Inside Tauri
  (`window.__TAURI_INTERNALS__` present) it uses `@tauri-apps/api/core` / `event` (dynamically imported);
  otherwise it loads `ipc/mock` (also dynamically, so the mock is not in the native bundle's critical path).
- The mock is an in-memory backend with realistic data, simulated latency and long tasks (downloads, installs,
  DNS propagation, VPS install) that emit the same events as the real backend. Flags in `localStorage`
  (`rustic.mock.latency`, `rustic.mock.speed`, `rustic.mock.onboarded`) let Playwright control it.
- TanStack Query owns server state; push events update the cache (`instances-changed`, `accounts-changed`) or
  Zustand stores (`launch-state`, `download-progress`).

## Consequences

The desktop engineer implements the same names/types in `src-tauri`; any new command is added to `commands.ts`
and IPC.md together. Type drift between Rust and TS is caught in review until we generate TS from Rust
(ts-rs/specta) later.

## Alternatives

Tauri's specta bindings first (blocks the UI on the Rust side), MSW (HTTP-only; Tauri IPC is not HTTP).
