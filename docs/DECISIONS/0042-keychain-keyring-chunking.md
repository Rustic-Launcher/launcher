# 0042. Secrets: the OS keychain through keyring 3.6, split into pieces

- Status: accepted (desktop, 2026-11-10)

## Context
Tokens must live only in the OS keychain (launcher/AGENTS.md). Windows Credential Manager holds at most 2560
bytes per item, less than a sign-in's secrets (Horizon hit it: horizon/docs/microsoft-account.md).

## Decision

`keyring` 3.6 (MIT/Apache; `windows-native`, `apple-native`, `sync-secret-service` + `crypto-rust` on Linux).
A secret longer than 1000 characters is split: `<key>` holds `rustic-chunks:<n>`, pieces in `<key>#0..n-1`,
written before the header, old pieces removed after. Error 8 ("not enough memory") maps to `KeychainFull` with
the clean-up advice Horizon gives. Without a keychain the app keeps secrets in memory for the run and says so.
Items: `minecraft:<uuid>` (refresh token, Minecraft token, expiry, xuid, the ownership answer),
`rustic-cloud`, `porkbun`; the hosting crate keeps `hosting/<profile>/...` under its own service name.

## Consequences

The ownership answer sits with the tokens, so accounts.json can't open the owner gate. keyring 4 (a new
keyring-core API) is left for later; the hosting crate uses 3.6 too.

## Alternatives

An encrypted file (Horizon's Linux fallback): more code, and AGENTS.md says keychain only.
