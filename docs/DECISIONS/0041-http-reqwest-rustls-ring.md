# 0041. HTTP: reqwest 0.13 over rustls with the ring provider

- Status: accepted (desktop, 2026-11-10)

## Context
reqwest 0.13's default TLS is rustls with aws-lc-rs, which needs cmake/NASM on Windows. The Tauri updater
plugin already pulls reqwest 0.13 with `rustls-no-provider`.

## Decision

`reqwest` 0.13 (MIT/Apache) with `rustls-no-provider`, `rustls` 0.23 with `ring` (Apache-2.0 AND ISC),
system roots through `rustls-platform-verifier`. `rusticmc_launcher_core::http::client()` installs the ring
provider once and sets the user agent; every crate uses that client (core re-exports `reqwest`).

## Consequences

One TLS stack, no C toolchain beyond what ring needs. Other owners' crates still on reqwest 0.12
(`rusticmc-api-client`) compile a second copy until they move.

## Alternatives

native-tls (OpenSSL on Linux), aws-lc-rs (build tooling), ureq (blocking; Horizon's choice).
