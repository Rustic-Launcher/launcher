# 0050. Domain records and verification

- Status: accepted (desktop, 2026-11-10)

## Context
ARCHITECTURE.md §7 fixes the record shapes (CNAME to `<route-id>.<relay-host>`, ALIAS at the apex, SRV form,
A/AAAA for a VPS, `_rustic-challenge.<host> TXT "rustic-verify=<token>"`) and asks for checks against public
resolvers, not the system's cache.

## Decision

`required_records(zone, sub, mode, verification)` builds the plan (Relay / Srv / Direct modes); `verify`
checks each record through a `DnsResolver` trait. `PublicResolver` uses `hickory-resolver` 0.26 (MIT/Apache,
tokio only, no system config) against 1.1.1.1, 8.8.8.8 and 9.9.9.9, each asked apart so the UI shows
"propagating (k of n)". An ALIAS is checked by comparing the apex's A records with the target's.

## Consequences

Tests use a fake resolver. The ownership token comes from Rustic Cloud (`OwnershipTokens` trait); until its
client exists the plan has routing records only and says so.

## Alternatives

DNS-over-HTTPS JSON (vendor-specific); the system resolver (stale caches).
