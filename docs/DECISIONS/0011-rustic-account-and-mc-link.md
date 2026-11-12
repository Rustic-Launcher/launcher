# 0011. Rustic account and Minecraft profile linking

- Status: accepted (spec, 2026-11-10)

## Context

The Rustic account carries friends, cosmetics, purchases and servers; it never replaces the game licence. The Rustic
Cloud must know which Minecraft UUID a Rustic account plays as, without ever receiving a Microsoft or Minecraft token.

## Decision

- Sign-up: email + password (argon2id, breached-password check via k-anonymity range API optional), or "Continue with
  Microsoft" (ADR 0090 replaced the Discord / Google OAuth planned here); email verification; birth date (age gates, LEGAL §4); optional 2FA (TOTP) at launch, passkeys later.
- Link: `POST /v1/mc-link/start` → random server id; the launcher calls Mojang's session `join` with the MC token,
  UUID and that id; `POST /v1/mc-link/finish {username}` → the API calls `hasJoined` and binds the returned UUID.
  Same pattern as Horizon's cosmetics service (`horizon/docs/cosmetics.md:390-394`). One UUID per Rustic account;
  several Microsoft accounts per Rustic account are not allowed in v1 (simpler moderation).
- Sessions: access JWT (EdDSA, 15 min) + rotating refresh token (30 days, keychain), device list, revoke all.

## Consequences

The API holds UUID + username history only. Unlinking keeps purchases on the Rustic account; relinking another UUID is
rate-limited (once per 30 days) to stop cosmetic sharing.

## Alternatives

Using the Microsoft account as the Rustic identity (we would hold MS tokens server-side: rejected for risk and
Microsoft terms).
