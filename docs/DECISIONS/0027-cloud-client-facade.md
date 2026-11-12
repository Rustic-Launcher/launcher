# 0027. Rustic Cloud client facade and mock

- Status: accepted (frontend, 2026-11-10)

## Context

Friends, presence, party, notifications, news, featured servers, cosmetics, store and account compliance come from
the Rustic Cloud API, whose OpenAPI document (`services/api/openapi.json`) does not exist yet. The UI must not wait.

## Decision

- Pages depend on one interface, `CloudClient` (`apps/launcher/src/cloud/client.ts`), and on hand-written types
  (`cloud/types.ts`) describing the shapes the UI needs. `cloud()` returns the implementation.
- Until the generated client exists, `cloud()` returns the mock (`cloud/mock/*`: social with simulated presence
  changes and party joins, account/parental/GDPR, cosmetics and store with per-currency prices and purchase
  guards).
- The backend published `services/api/openapi.json`: `pnpm gen:cloud` generates `src/cloud/generated/schema.d.ts`
  (openapi-typescript 7.13) and `src/cloud/http.ts` implements `CloudClient` over it with openapi-fetch 0.17,
  enabled by `VITE_CLOUD_URL` (bearer from the `auth_rustic_access_token` IPC command, or `VITE_CLOUD_TOKEN` in a
  browser). The mock stays the default. Endpoints the API lacks (news, featured servers, PIN parental gate) fall
  back to the mock; differences are reported in HANDOFF.md.
- The UI never sends payment data: purchases send the product, currency and the two explicit EU consents
  (immediate execution, loss of the withdrawal right) and, when parental control requires it, the parental PIN;
  payment itself is Stripe Checkout opened by the desktop side (ADR 0012).

## Consequences

The "expected API" section of docs/IPC.md is the frontend's request to the cloud engineer.

## Alternatives

Waiting for OpenAPI (blocks UI work), calling fetch() directly from pages (no seam for tests/mocks).
