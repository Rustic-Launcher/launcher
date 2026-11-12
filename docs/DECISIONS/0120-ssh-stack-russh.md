# 0120. SSH/SFTP stack: russh + russh-sftp (pure Rust, ring backend)

- Status: accepted (hosting, 2026-11-10)

## Context

`RemoteSshProvider` needs SSH exec, SFTP and `direct-tcpip` forwards from a Windows/macOS/Linux desktop app, plus
an SSH **server** for in-process tests. The workspace already uses rustls with the `ring` provider and avoids C
toolchains (no cmake/nasm on the dev box).

## Decision

- `russh` 0.65.1 (Apache-2.0, warp-tech, actively released in 2026) with `default-features = false,
  features = ["ring", "rsa", "flate2"]`: client and server, OpenSSH/PEM/PuTTY keys via `ssh-key` 0.7 (ed25519,
  ECDSA, RSA with `rsa-sha2-*`), passphrase-protected keys.
- `russh-sftp` 3.0.1 (Apache-2.0, AspectUnk, maintained alongside russh): client and server sides.
- `tar` 0.4 (MIT/Apache) to upload the prepared game files as one archive (uncompressed: PNG and class files barely shrink); `zip` 2 to read `.rplugin`
  manifests; `if-addrs` 0.15 (MIT/BSD-3) for LAN addresses; `async-trait` (MIT/Apache).

## Consequences

- Pure Rust, no OpenSSL/libssh2 to ship or patch; the same crate gives an in-process SSH server for tests.
- `ssh-key` 0.7 is a release candidate pinned by russh (`=0.7.0-rc.11`); follow russh's updates.
- No ssh-agent / `~/.ssh/config` integration in v1 (keys are passed explicitly).

## Alternatives

- `ssh2` (libssh2 + OpenSSL, MIT/Apache): mature but C dependencies, blocking API, no server side for tests.
- Shelling out to the system `ssh`: inconsistent on Windows, host keys and secrets outside our control.
