# 0052. Invite codes for local servers

- Status: accepted (desktop, 2026-11-10)

## Context
Local access includes "an invite code in the spirit of Horizon's Play with Friends" (AGENTS.md). Horizon's
codes carry an 80-bit secret for an end-to-end tunnel to an integrated server; ADR 0105 (relay) plans
cloud-resolved codes for the relay route.

## Decision

A Rustic invite is self-contained: Crockford base32 in groups of five (input forgives case, spaces, dashes,
O/0, I/L/1), version 1, default port, endpoints (IPv4, IPv6, hostname, each with its own port when it differs)
and the first two bytes of SHA-256 as a checksum. No secret: the server speaks the plain game protocol and
checks accounts itself. The joining launcher dials every endpoint at once and starts the game with
`--quickPlayMultiplayer` on the first that answers.

## Consequences

Not compatible with Horizon's `friend:` codes; a cloud-resolved code (ADR 0105) can coexist.

## Alternatives

Reusing horizon-friends codes (needs the Rustic edition on both sides and a tunnel per guest).
