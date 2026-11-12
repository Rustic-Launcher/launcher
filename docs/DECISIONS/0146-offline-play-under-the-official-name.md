# 0146. Offline play is the owner's own account, under its official name

Date: 2026-11-12 · Status: accepted (product owner) · Supersedes the offline profiles of ADR 0043

## Context

ADR 0043 let a player who owns the game create free-name "offline profiles" for LAN parties and
playing without internet, unlocked for 30 days after a successful ownership check. Two problems:

- Horizon's Launch API takes no offline name, so an offline launch started **without a session**
  and the game picked its own offline identity: the profile's name never reached the game (RISKS R-42).
- The product owner wants the player to always be their Microsoft account: the official name (and
  its skin and UUID) even offline. Horizon itself dropped free-name offline profiles on 2026-11-09
  (`docs/microsoft-account.md`, "The account that plays owns the game").

Horizon reads the session file as `{name, uuid, access_token, user_type: "msa", xuid}` and only uses
the token for online calls; when they fail it falls back to offline properties
(`horizon-client-render/src/auth.rs`).

## Decision

- **No free-name profiles.** Only Microsoft accounts that own the game play. The `offline_profiles`
  of older `accounts.json` files are ignored and dropped at the next save; a selection pointing at one
  falls back to the first account. The IPC commands `auth_offline_profile_create` / `_delete` are gone,
  and `Account.kind` is always `microsoft`.
- **Offline is the same account.** Without network, an account whose own last answer "owns" is within
  the window (30 days) still launches with a session: its official name and UUID, the last access token
  and XUID from the keychain (`LaunchAuth::Offline { session }`). The game goes offline by itself; LAN
  servers see the real name and UUID. A "doesn't own" answer still blocks at once.
- Settings › Accounts shows an "Offline play" card: the owner's avatar, name and the date until which
  offline play stays open (`ownership.next_check_at`), or why it is locked.

## Consequences

- The name in game is the official one in every case; skins and the avatar match the account.
- An expired token handed to the game is never logged (it stays a `Secret`) and is only sent to
  Mojang's own services by the game, as an online launch would.
- Players who relied on several LAN names can't anymore (as in Horizon). R-42 is closed.

## Alternatives

- Keep free names and ask upstream for an offline-name option: contrary to the product owner's request
  and to Horizon's own direction.
- Launch offline without a session (the old behaviour): the game shows a name the player never chose.
