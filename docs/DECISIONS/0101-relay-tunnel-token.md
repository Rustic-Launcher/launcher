# 0101. Tunnel token: short-lived HS256 JWT issued by Rustic Cloud

- Status: accepted (relay, 2026-11-10); the claim format is shared with the cloud owner (HANDOFF)

## Context

The relay must know which server a tunnel belongs to, and which hostnames, quotas and temporary names it may use,
without calling the cloud on every connection. The cloud must also be able to revoke a tunnel.

## Decision

- A compact JWT (RFC 7519), **HS256 only**, with the key chosen by `kid`. Several keys are accepted at once for
  rotation, each at least 32 bytes. Claims: `iss`, `aud`, `sub` (server id), `iat`, `exp` (at most 1 h,
  10 min recommended), `jti`, `hosts`, `temp_hosts [{host, exp}]`, `port`, `plan`,
  `limits {max_players, rate_kib_s, daily_mib}` (RELAY.md §3).
- Verified by our own ~100 lines with `hmac` 0.12 + `sha2` 0.10 (constant-time `verify_slice`) and `base64` 0.22
  (all MIT OR Apache-2.0). The check is strict: `alg` must be `HS256`, exactly three parts, a size cap, and
  checks on issuer, audience, expiry, lifetime and host syntax.
- The client refreshes the token at 2/3 of its lifetime over the control stream. The relay closes a tunnel that
  has no valid token 60 s after `exp`. A refresh replaces the tunnel's routes: it renews temporary names and adds
  newly verified domains.

## Consequences

- Revocation takes up to one token lifetime (10 min or less recommended). The cloud revokes by no longer issuing.
- The relay needs no database and keeps working through a short cloud outage, until tokens expire.
- The key is symmetric, so the relay could mint tokens. This is acceptable while Rustic operates the relay. A
  third-party relay would need asymmetric tokens: add `alg: EdDSA` with a public key then.

## Alternatives

- The `jsonwebtoken` crate: fine, but v10 requires choosing a crypto backend and brings more than we use.
- Opaque tokens checked against the cloud on every connection: makes the cloud a hard runtime dependency.
- An mTLS client certificate per server: heavier to issue and to revoke.
