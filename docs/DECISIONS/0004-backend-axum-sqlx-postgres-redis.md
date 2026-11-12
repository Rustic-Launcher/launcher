# 0004. Rustic Cloud backend: axum + sqlx + Postgres + Redis/Valkey

- Status: accepted (spec, 2026-11-10); the cloud owner details it in ADRs 0080+.

## Context

Accounts, friends, cosmetics, store, server directory, DNS verification and GDPR tooling need a transactional store,
a fast ephemeral store (presence, rate limits, relay routes) and OpenAPI for generated clients.

## Decision

Checked on crates.io, 2026-11-10: `axum` 0.8.9, `tokio` 1.54.1, `tower-http` 0.7.1, `sqlx` 0.9.0 (Postgres,
compile-time checked queries, migrations), `utoipa` 6.0.0 (OpenAPI 3.1), `argon2` 0.6.0 (argon2id), `jsonwebtoken`
11.1.0 (EdDSA access tokens), `oauth2` 5.0.0 (Discord/Google sign-in; removed by ADR 0090, Microsoft id_tokens are checked with `jsonwebtoken`), `governor` 0.10.4 (rate limits), `reqwest`
0.13.5, `tracing` 0.1.44. Redis client: `fred` 10.1.0 or `redis` 1.8.1 (2.0 is an rc: avoid). Stripe: `async-stripe`
0.41 stable (1.0 is an rc) or `reqwest` + our own webhook signature check. All MIT and/or Apache-2.0.
Postgres 18; Valkey 8 (BSD-3, Redis-compatible) in dev and prod.

## Consequences

One language from launcher to backend; `rusticmc-api-client` generated from the OpenAPI document.

## Alternatives

Node/NestJS (two languages), Supabase/Firebase (lock-in, data residency), SeaORM (heavier than sqlx).
