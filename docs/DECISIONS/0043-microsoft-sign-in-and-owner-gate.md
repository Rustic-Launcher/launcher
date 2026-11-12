# 0043. Microsoft sign-in and the owner gate

- Status: accepted (desktop, 2026-11-10)

## Context
ADR 0009/0010 decide PKCE loopback with device-code fallback and a 30-day offline grace. Horizon documents the
chain (microsoft-account.md) and its launcher tries `launcher/login` before `authentication/login_with_xbox`
(a new account's token from the latter names no profile).

## Decision

- `AuthApi`: device code (`/devicecode`, polling with `authorization_pending`/`slow_down`/declined/expired),
  authorization code + PKCE S256 with a one-shot `127.0.0.1` listener (`http://localhost:<port>` redirect,
  state checked, other requests 404), refresh (`invalid_grant` → sign in again), XBL, XSTS (XErr messages),
  `launcher/login` then `login_with_xbox`, entitlements (`game_minecraft`/`product_minecraft`), profile.
- `AccountManager`: several accounts, one selected (or an offline profile); an account that doesn't own the
  game is refused before anything is stored; a re-sign-in for a UUID refuses another account; renewal when
  less than an hour is left, the rotated refresh token saved at once; entitlements and profile asked at every
  launch; without network an account whose own last answer (within the window) was "owns" starts offline.
- Offline profiles (`OwnershipPolicy`): allowed only while some account's keychain answer "owns" is younger
  than `ownership.revalidation_days` (default 30, clamped 1..90); future timestamps beyond 5 minutes ignored.
  Enforced at creation, selection and every offline launch.
- `VerifiedOwner`: a proof only the auth crate makes; downloading the server jar for the preparer needs it.

> **2026-11-12:** offline profiles were replaced by offline play under the official name (ADR 0146).

## Consequences

Sign-in needs the Azure client id (TODO_HUMAN D1) and Mojang's approval; without it every flow answers
`auth.not_configured`. Horizon's Launch API has no offline-name option, so an offline profile starts the game
without a session (Horizon picks the offline identity).

## Alternatives

MSAL libraries (no Rust one that fits); device code only (worse UX).
