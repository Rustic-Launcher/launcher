# 0002. Frontend routing, data, state, i18n and fonts

- Status: accepted (spec, 2026-11-10)

## Context

The UI needs typed routing (pages, nested detail pages, filters in search params), cached server state (Cloud API,
content sources, IPC queries), light client state (active instance, friends panel, jobs) and translations.

## Decision

| Package | Version | Licence | Last publish | Role |
|---|---|---|---|---|
| `@tanstack/react-router` | 1.170.46 | MIT | 2026-10-22 | Typed routes, search-param state for filters |
| `@tanstack/react-query` | 5.107.1 | MIT | 2026-11-06 | Cloud API and IPC queries, cache, retries, optimistic equip |
| `zustand` | 5.1.0 | MIT | 2026-11-06 | UI state, mirror of IPC job events |
| `i18next` / `react-i18next` | 27.1.0 / 18.0.1 | MIT | 2026-11-09 / 2026-11-08 | Translations, ICU plurals, namespaces per page |
| `@fontsource-variable/inter` | 5.3.0 | OFL-1.1 | 2026-07-19 | UI font, bundled (no Google Fonts request: GDPR) |
| JetBrains Mono (fontsource) | current | OFL-1.1 | | Console, codes, addresses |

Languages at launch: **English (source) and French**. No string concatenation; dates and numbers through `Intl`.
OFL fonts may be bundled with an MIT/Apache app; ship their licence texts.

## Consequences

IPC events feed Zustand stores; query keys `['cloud', resource, params]` and `['ipc', command, params]`.

## Alternatives

React Router 7 (weaker typed search params), Redux Toolkit (heavier), Lingui. Geist (OFL, 1.9.2): fewer scripts.
