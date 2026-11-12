# 0082. API clients: shared wire types, methods generated from openapi.json

- Status: accepted (cloud, 2026-11-10)

## Context

The launcher (Rust/Tauri) and the web app (TypeScript) need typed clients that cannot drift from the
server. utoipa emits OpenAPI **3.1**; progenitor, the usual Rust generator, reads 3.0 only and its
current line is an alpha.

## Decision

- `rusticmc-api-client::types` holds every JSON body type. The server depends on the crate with the
  `openapi` feature (derives `utoipa::ToSchema`), so schema names in `openapi.json` equal the Rust
  type names and server and client share one definition of each type.
- One async method per operation is **generated from `openapi.json`** (operationId → method name,
  path/query parameters → arguments, `$ref` bodies/responses → shared types) into
  `src/generated.rs` by `cargo run -p rusticmc-cloud-api --bin gen-openapi` (generator in
  `services/api/src/openapi.rs`). The transport (`Client`, `ClientError`, bearer token) is
  hand-written (~150 lines, reqwest + rustls).
- Tests fail when `openapi.json` or `generated.rs` is stale, and check that every operation
  documents the error envelope and has the expected security.
- TypeScript: `openapi-typescript` + `openapi-fetch` (MIT) from the same file (API.md).

## Consequences

Adding an endpoint = handler + `#[utoipa::path]` + `routes!` line + regenerate. Two operations are
deliberately absent from the Rust client: the OAuth browser callback and the Stripe webhook.

## Alternatives

progenitor (3.0 only), openapi-generator (Java tooling, verbose output), a fully hand-written client
(drifts).
