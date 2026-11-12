# 0053. The Tauri shell

- Status: accepted (desktop, 2026-11-10)

## Context
ADR 0001 picks Tauri v2. The app must start without the frontend dist in development and expose every
IPC.md command and event.

## Decision

- Tauri 2.12 (`tauri-build` 2.7); plugins: single-instance (with deep-link forwarding), deep-link (`rustic://`
  `join/<address-or-invite>`, `auth?ticket=`), opener, dialog, notification, window-state, log (`skip_logger`:
  tracing owns the logger), updater (signed updates; public key placeholder, TODO_HUMAN).
- `devUrl` http://localhost:1420, `frontendDist` ../dist; `custom-protocol` feature for release builds.
- Least privilege: one capability for the `main` window with `core:default`, `log:default`,
  `deep-link:default`; no fs/shell/http/opener/dialog from JS (the Rust commands open links, files and dialogs
  after their own checks). CSP: self scripts, inline styles (motion), images from the texture/CDN hosts,
  connections to IPC and Rustic Cloud only, no objects, frames or forms.
- One shared `AppState`; long work returns a task id and reports through `downloads-changed` /
  `download-progress` / `prepare-progress` / `hosting-install` / `dns-status`.

## Consequences

Placeholder icons until TODO_HUMAN provides the real ones; updater inactive until its key exists.

## Alternatives

Electron (ADR 0001).
