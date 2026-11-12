# 0001. Desktop stack: Tauri v2 + React 19 + TypeScript + Vite + Tailwind v4 + Radix + Motion

- Status: accepted (spec, 2026-11-10)
- Owners affected: frontend, desktop

## Context

The launcher needs a premium, animated, glassy UI with a 3D model viewer, fast iteration, a browser-runnable UI
(mock IPC) for design work and Playwright, and a Rust core that reuses Horizon crates and the OS keychain.
Horizon's own launcher uses GPUI Fast (`horizon/launcher/Cargo.toml:13-16`), which is Rust-only and offers no
web tooling (Storybook, Playwright, Radix accessibility primitives).

## Decision

Versions checked on 2026-11-10 (`npm view <pkg> version license time.modified`, crates.io API):

| Package | Version | Licence | Last publish | Note |
|---|---|---|---|---|
| `tauri` (crate) | 2.12.5 stable | Apache-2.0 OR MIT | 2026-11-06 | **3.0.0-alpha.6 exists: do not use an alpha**; pin `tauri = "2.12"` |
| `@tauri-apps/api`, `@tauri-apps/cli` | 2.12.4 | Apache-2.0 OR MIT | 2026-10-14 / 11-06 | CLI via pnpm, not cargo install |
| `tauri-plugin-updater` / `-deep-link` / `-single-instance` | 2.14.1 / 2.6.x / 2.x stable | MIT OR Apache-2.0 | 2026-10-28 | 3.x alphas exist, ignore |
| `react`, `react-dom` | 19.3.1 | MIT | 2026-11-09 | |
| `typescript` | 7.0.2 | Apache-2.0 | 2026-11-09 | TS 7 is the native compiler; if a tool in the chain lags, pin 6.x and record it in a frontend ADR |
| `vite` | 8.3.7 | MIT | 2026-11-09 | |
| `tailwindcss`, `@tailwindcss/vite` | 4.3.5 | MIT | 2026-11-09 | CSS-first `@theme` from PALETTES.md tokens |
| `radix-ui` (unified package) | 1.8.1 | MIT | 2026-10-14 | Accessible primitives (Dialog, Popover, Tooltip, Tabs, Select, Toggle…) |
| `motion` (current name of `framer-motion`, same version) | 14.11.1 | MIT | 2026-11-09 | Import from `motion/react` |
| `lucide-react` | 1.70.0 | ISC | 2026-11-06 | Icons |
| `storybook` | 10.6.6 | MIT | 2026-11-09 | |
| `@playwright/test` | 1.65.0 | Apache-2.0 | 2026-11-09 | Critical journeys against the mock IPC |

All maintained (published in the last 4 weeks) and compatible with MIT OR Apache-2.0.

## Consequences

- WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux): test Linux `backdrop-filter` performance early;
  Settings › Accessibility "Reduce transparency" swaps glass for solid `bg.3`.
- Tauri capabilities must be least-privilege (SECURITY §2.2).

## Alternatives

- GPUI Fast (Horizon's): Rust-only, no web tooling, would look like Horizon's launcher. Rejected.
- Electron: heavier, Node main process instead of Rust. Rejected.
- Tauri 3 alpha: unstable. Revisit at 3.0 stable.
