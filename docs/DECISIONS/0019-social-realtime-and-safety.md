# 0019. Social features: realtime gateway, age gates and moderation baseline

- Status: accepted (spec, 2026-11-10); the cloud owner implements (ADRs 0080+)

## Context

Friends panel on every page, presence, invites, parties and DMs. Horizon has no friends service (HORIZON_FACTS A7).
Minors are part of the audience (LEGAL §4).

## Decision

- `services/realtime`: authenticated WebSocket (Rustic access token), events `presence`, `friend-request`, `invite`,
  `party`, `dm`, `notification`; Redis pub/sub fan-out; per-user rate limits; server-side validation of every message.
- Friend requests by Rustic username or Minecraft name; requests can be limited to "friends of friends" or turned off.
- **Chat (DMs, party chat)**: available from the minimum age in the country (default 13; 16 where national law sets
  the digital consent age higher unless a parent consents, LEGAL §4), off by default under 16, blocked words filter,
  report and block on every message, messages retained 90 days for moderation then deleted.
- Moderation: reports queue with categories, actions (warn, mute, suspend, ban), appeal by email, audit log;
  DSA-style statement of reasons for account restrictions.
- No public profiles of minors; presence visible to friends only.

## Consequences

Moderation tooling is part of the MVP backlog, not a later add-on.

## Alternatives

Using Discord for social (loses in-launcher experience, Discord's own age rules apply).
