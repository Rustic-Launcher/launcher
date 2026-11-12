# 0014. Domains: three equal paths (connect Porkbun, buy at Porkbun, bring your own), `DnsProvider` trait

- Status: **accepted (user decisions, 2026-11-10)**; the hosting owner implements (ADRs 0120+)

## Context

No free Rustic-owned subdomain product for now. Users bring their own domain. Porkbun API v3
(`https://api.porkbun.com/api/json/v3`, docs read 2026-11-10): keys `apikey`/`secretapikey` (body) or `X-API-Key` /
`X-Secret-API-Key` headers; `dns/create|retrieve|editByNameType|deleteByNameType` (types A, AAAA, CNAME, ALIAS, TXT,
SRV…); `domain/checkDomain` (avail, price, renewal; 10 per 10 s); `domain/getRegistrationRequirements/{tld}`;
`domain/create/{domain}` with `cost` in cents (`COST_MISMATCH`), `agreeToTerms: "yes"`, `payWith` card/usdc or account
credit, `Idempotency-Key`, `$100` cap, verified email/phone required; per-domain "API access" opt-in by the account
holder (`API_ACCESS_DISABLED`); keys can be restricted to domains.

## Decision

- `DnsProvider` trait: `verify_credentials`, `list_domains`, `get_records`, `upsert_records(plan, dry_run)`,
  `delete_records`; optional `DomainRegistrar`: `check(name)`, `requirements(tld)`, `register(name, quote, consent)`.
  Implementations: `Porkbun` (v1), `Manual` (BYO: produces the plan for the user), `Cloudflare` (later).
- The record plan is computed once (ARCHITECTURE §7) and shown identically on all paths; BYO is a first-class path with
  copy buttons, per-registrar guides and live per-record status (missing / wrong value / propagating / OK).
- Porkbun keys are stored only in the OS keychain on the user's machine and never sent to Rustic Cloud.
- Buying: Porkbun is the seller; price and renewal price shown; explicit confirmation; `agreeToTerms` sent only after the
  user ticks Porkbun's terms; dry-run quote first; idempotency key per attempt.
- The relay-subdomain mode (`<name>.<rustic suffix>`) stays an optional, configurable feature (placeholder domain).

## Consequences

The cloud still verifies TXT + target itself before routing (users can't route someone else's hostname).

## Alternatives

Rustic as reseller/registrar (regulatory and support burden: rejected for now); Cloudflare first (Porkbun chosen by the
user).
