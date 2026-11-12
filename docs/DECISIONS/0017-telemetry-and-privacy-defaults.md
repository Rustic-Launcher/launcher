# 0017. Telemetry and privacy defaults

- Status: accepted (spec, 2026-11-10)

## Context

GDPR and ePrivacy (Art. 5(3)) require consent for non-essential telemetry; Horizon's opt-in model has two consent-free
exceptions (heartbeat ping, "Show me as Horizon") (`horizon/docs/telemetry.md:3-8,74-86`).

## Decision

- Telemetry categories: **usage analytics**, **performance**, **crash reports**; all **off by default**, each a
  separate toggle at first launch and in Settings › Privacy, consent versioned and stored server-side when signed in.
- Strictly necessary processing (account, purchases, friends/presence the user enabled, security logs) is not telemetry
  and is explained in the privacy policy.
- The Rustic edition does **not** register Horizon's heartbeat (ADR 0006). If Horizon's cosmetics stay enabled, its
  "Show me as Horizon" presence is documented and its toggle surfaced in Settings › Privacy.
- Self-hosted analytics in the EU (no third-party trackers in the launcher); IPs truncated; crash reports scrubbed of
  usernames, paths, tokens.
- Presence visibility: friends only by default; "appear offline" available; server address shared only when the user
  enables "Let friends join me".

## Consequences

Product analytics will be sparse; plan dashboards on consenting users only.

## Alternatives

Opt-out telemetry (unlawful without consent in the EU for non-essential data).
