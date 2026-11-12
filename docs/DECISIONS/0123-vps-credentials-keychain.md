# 0123. VPS credentials: generated Ed25519 key in the OS keychain, password used once

- Status: accepted (hosting, 2026-11-10)

## Decision

- `keyring` 3.6 (MIT/Apache; `windows-native`, `apple-native`, Linux `linux-native-async-persistent` +
  `crypto-rust`, no libdbus) behind a `SecretStore` trait (`MemoryStore` for tests). Service
  `gg.rustic.launcher.hosting`, entries `hosting/<profile>/{private-key,passphrase}`. Values over 1000 characters are
  chunked (`<name>#i` + a header): Windows Credential Manager caps a blob at 2560 bytes (an RSA-4096 PEM is larger).
- Recommended flow: `remote::bootstrap_key` connects once with the password, generates an Ed25519 key, appends its
  public line to `~/.ssh/authorized_keys` idempotently, stores the private key in the keychain, then checks the key
  alone logs in. The password is never stored. Alternatives: a pasted key (keychain) or an existing key file (only
  its passphrase in the keychain; the file is read at connect time, never copied).
- Host keys: TOFU with explicit SHA256 confirmation, pins in `<data>/hosting/known_hosts.json`, mismatch = hard
  error; the handshake stops before authentication when the key is not trusted.

## Consequences

No secret in config files, logs or IPC replies (`Secret` redacts `Debug`). If desktop standardises on keyring 4,
align versions (HANDOFF).

## Alternatives

Encrypted key file with a keychain-held passphrase (more moving parts); storing passwords (refused by SECURITY §2.8).
