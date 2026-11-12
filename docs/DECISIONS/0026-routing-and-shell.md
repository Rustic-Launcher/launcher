# 0026. Routing and app shell: code-based TanStack Router, hash history, lazy pages

- Status: accepted (frontend, 2026-11-10)

## Context

The window must paint fast; pages are many (Home, Play/Instances, Content, Servers, Locker, Store, Settings,
onboarding) and owned by different engineers.

## Decision

- Code-based TanStack Router (no file-based generator step). `routes/base.tsx` defines the root and the `shell`
  layout route; each feature folder exports its routes from `features/<area>/routes.tsx`; `router.tsx` assembles the
  tree. Search params are validated per route (filters, tabs, settings section).
- **Hash history** (`#/servers/…`): works from Tauri's custom protocol and any static server without rewrites.
- Every page component is lazy (`lazyRouteComponent`), so the initial chunk holds only the shell; the pending
  component is a skeleton (shown after 120 ms, kept ≥ 200 ms to avoid flicker); links preload on intent.
- The shell (rail, top bar, bottom bar with PLAY, collapsible friends panel, Ctrl+K palette, toasts) is persistent
  across navigations; the first-launch flow is a sibling route outside the shell; the shell's `beforeLoad` redirects
  to `/onboarding` until `settings.onboarding_completed`.
- Keyboard: skip link, rail with roving focus (Up/Down/Home/End), Alt+1..6 and Alt+, shortcuts, Ctrl+K anywhere.

## Consequences

The 3D viewer (three/r3f) only loads with the Locker/Store pages. Deep links from the desktop (`rustic://…`) map
to hash routes.

## Alternatives

File-based routing (generator + more files to coordinate), browser history (needs SPA fallback in Tauri), React
Router 7 (weaker typed search params).
