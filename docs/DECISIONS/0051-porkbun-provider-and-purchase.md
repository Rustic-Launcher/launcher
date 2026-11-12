# 0051. Porkbun: connect, configure, buy

- Status: accepted (desktop, 2026-11-10)

## Context
User decisions (2026-11-10): connect your own Porkbun account or buy a domain through it from the launcher;
keys in the OS keychain only. Porkbun API v3 (OpenAPI spec read 2026-11-10).

## Decision

- `PorkbunDns` implements `DnsProvider`: `/ping` (credentialsValid) before keeping the keys (`porkbun` item),
  `/domain/listAll` (paged by 1000, `apiAccess`), `/dns/retrieve|create|edit|delete` with idempotent apply
  (unchanged / edit in place / create; our TXT records carry the note "Rustic Launcher" so other TXT records at
  the name are left alone). SRV: `prio` apart, content `weight port target` (RISKS).
- Buying: `checkDomain` (price, renewal, premium), a dry-run quote (`domain/create` with `dryRun`, `cost: 0`)
  kept 10 minutes, then `domain/create` with the exact `cost` and `agreeToTerms: "yes"` only with a confirmation
  matching the quote (domain, cents, currency) and the terms ticked; the quote id is the Idempotency-Key; paid
  from Porkbun credit (`INSUFFICIENT_FUNDS` names the top-up page). Premium names: website only.
- `ManualDns` for any other registrar: the same plan shown for copy, verified the same way.

## Consequences

Keys never reach the cloud or a log (requests are logged by path; bodies never).

## Alternatives

x402/USDC or card holds through the API (agent payment flows, not for a desktop UI).
