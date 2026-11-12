# 0020. Frontend workspace, toolchain and pinned packages

- Status: accepted (frontend, 2026-11-10)
- Builds on: ADR 0001, 0002, 0003 (spec)

## Context

ADR 0001-0003 chose the stack. The frontend needs exact, mutually compatible versions on the dev box
(Windows 11, Node 22.14, pnpm 10.28, ~20 GB free disk shared with other efforts) and a layout that lets
several engineers work in parallel: a design system and an app.

## Decision

pnpm workspace in `launcher/` (`pnpm-workspace.yaml`): `packages/ui` (`@rustic/ui`, design system consumed as
TypeScript source, no build step) and `apps/launcher` (`@rustic/launcher`, Vite app, Tauri front end).
Exact versions (no `^`) are pinned; every package was checked with `npm view <pkg> version license` on
2026-11-10.

| Package | Version | Licence | Where |
|---|---|---|---|
| react, react-dom | 19.3.1 | MIT | ui (dev/peer), app |
| @types/react, @types/react-dom | 19.3.0 | MIT | both |
| typescript | **5.9.3** | Apache-2.0 | root |
| vite | 8.3.7 | MIT | both |
| @vitejs/plugin-react | 6.3.0 | MIT | both |
| tailwindcss, @tailwindcss/vite | 4.3.5 | MIT | both |
| radix-ui (unified primitives) | 1.8.1 | MIT | ui |
| motion | 14.11.1 | MIT | ui, app |
| class-variance-authority | 0.7.1 | Apache-2.0 | ui |
| clsx | 2.1.1 | MIT | ui |
| tailwind-merge | 3.7.0 | MIT | ui |
| cmdk (command palette) | 1.1.1 | MIT | ui |
| lucide-react | 1.70.0 | ISC | ui, app |
| @fontsource-variable/inter | 5.3.0 | OFL-1.1 (font) | ui |
| @fontsource-variable/jetbrains-mono | 5.3.0 | OFL-1.1 (font) | ui |
| three, @types/three | 0.187.0 | MIT | ui (peer), app |
| @react-three/fiber | 9.8.2 | MIT | ui (peer), app |
| @tanstack/react-router | 1.170.46 | MIT | app |
| @tanstack/react-query | 5.107.1 | MIT | app |
| zustand | 5.1.0 | MIT | app |
| i18next / react-i18next | 27.1.0 / 18.0.1 | MIT | app |
| @tauri-apps/api | 2.12.4 | Apache-2.0 OR MIT | app |
| storybook, @storybook/react-vite, addon-a11y, addon-docs | 10.6.6 | MIT | ui |
| eslint, @eslint/js | **9.39.5** | MIT | root |
| typescript-eslint | 8.72.2 | MIT | root |
| eslint-plugin-react-hooks / -react-refresh / -jsx-a11y / -storybook | 7.1.1 / 0.5.7 / 6.10.2 / 10.6.6 | MIT | root |
| eslint-config-prettier, globals | 10.1.8, 17.14.0 | MIT | root |
| prettier, prettier-plugin-tailwindcss | 3.9.10, 0.8.1 | MIT | root |
| vitest | 5.1.0 | MIT | both |
| happy-dom | **21.0.4** | MIT | both (test DOM) |
| @testing-library/react / jest-dom / user-event | 16.3.3 / 7.0.1 / 14.6.8 | MIT | both |
| @playwright/test | 1.65.0 (Chromium only) | Apache-2.0 | app |
| @types/node | 22.20.5 | MIT | app |
| openapi-typescript (codegen, dev) | 7.13.0 | MIT | app (`pnpm gen:cloud`) |
| openapi-fetch | 0.17.0 | MIT | app (real cloud client, ADR 0027) |

Deliberate pins below "latest":

- **TypeScript 5.9.3, not 7.0.2**: `typescript-eslint` 8.72 declares `typescript >=4.8.4 <6.1.0`; TS 7 (the
  native compiler) also has no stable JS API yet for Storybook docgen. Revisit when typescript-eslint supports it.
- **ESLint 9.39.5, not 10**: `eslint-plugin-jsx-a11y` 6.10.2 peers on ESLint ≤ 9.
- **happy-dom instead of jsdom 31**: jsdom 31 requires Node ≥ 22.22; the box runs 22.14.
- **No `@react-three/drei`**: see ADR 0023.

Not added (kept lean): a chart library (a 60-line SVG `LineChart` in `@rustic/ui` covers the server metrics), a
carousel library (own `Carousel`), a toast library (Radix Toast + tiny store), a drag-and-drop library
(`motion`'s `Reorder` for resource-pack ordering).

Scripts (root `package.json`): `pnpm dev`, `pnpm build`, `pnpm lint` (ESLint flat config, `--max-warnings 0`),
`pnpm typecheck`, `pnpm test` (Vitest in both packages), `pnpm e2e`, `pnpm screenshots`, `pnpm storybook`,
`pnpm build-storybook`, `pnpm format`.

## Consequences

- The UI package has no build: apps import `@rustic/ui` source; Tailwind scans it with `@source`.
- Upgrades are explicit edits of pinned versions plus an update of this table.
- OFL font licence texts must ship with the app (fontsource packages include them; the bundler copies the
  font files, the about page links the notices).

## Alternatives

npm/yarn workspaces (pnpm saves disk with one content-addressed store), a built `ui` package (slower loop),
Jest (slower, no native ESM), Cypress (heavier, less parallel).
