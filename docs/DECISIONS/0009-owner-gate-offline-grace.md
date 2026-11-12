# 0009. Owner gate and offline grace

- Status: accepted (spec, 2026-11-10); the user may change the grace period (open question)

## Context

Owning the game is mandatory (launcher AGENTS rules). Horizon gates play on an owning Microsoft account and removed
offline names on 2026-11-09 (`horizon/docs/microsoft-account.md:77-109`). Players still need to play on a train or a
LAN without internet.

## Decision

- Play requires a Microsoft account whose entitlements include `game_minecraft` or `product_minecraft`.
- After a successful check the account's `verified_at` is stored (keychain-backed record). Offline play (singleplayer,
  LAN, local servers in offline mode) is allowed for **30 days** after the last successful check, then a fresh online
  check is required. Every online start re-checks silently.
- No free-text offline names. No Rustic account can play without a linked owning Microsoft account.
- Servers created by Rustic default to `online_mode = true`; switching it off shows a warning ("anyone can join with any
  name") and is refused for public (relay/domain) exposure.

## Consequences

Matches the "no auth bypass" rule; an offline server with relay exposure would let non-owners in, hence the refusal.

## Alternatives

7-day grace (stricter, annoying for travellers), unlimited (weak licence enforcement).
