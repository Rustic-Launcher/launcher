# 0025. Translations: namespaces per area, typed keys, parity test

- Status: accepted (frontend, 2026-11-10)
- Implements: ADR 0002 (i18next, FR + EN)

## Context

All UI text must be translated (FR + EN at launch) with no hardcoded strings, and several engineers write pages in
parallel.

## Decision

- One JSON per namespace and language: `apps/launcher/src/i18n/locales/{en,fr}/{common,home,instances,content,
  servers,locker,store,settings,onboarding}.json`, bundled statically (small) and loaded at startup.
- Keys are typed (`i18next.d.ts` augments `CustomTypeOptions` with the English resources): a missing key is a
  TypeScript error.
- `locales.test.ts` checks FR and EN have the same keys (plural suffixes normalised; French uses
  `_one/_many/_other`), the same `{{placeholders}}` and no empty strings.
- `@rustic/ui` contains no text: every label (aria-labels, tooltips, button text) is a prop.
- Numbers, dates, durations, money and relative times use `Intl` through `src/lib/format.ts`.
- Language: saved preference, else the OS language (fr → French, otherwise English); switchable live.

## Consequences

Adding a language = one folder of JSON files + one line in `i18n/index.ts`. Proper names from data (server names,
cosmetics) are not translated; the mock provides FR/EN variants where it matters.

## Alternatives

ICU MessageFormat (heavier), Lingui (extraction step), lazy-loaded namespaces (not needed at this size).
