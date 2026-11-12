# 0071. Companion session and cloud calls

- Status: proposed to the cloud and desktop owners (content, 2026-11-10). Follows spec ADR 0008.

## Decision

- Session: the launcher starts the game with `RUSTIC_COMPANION_ENDPOINT` (`http://127.0.0.1:<port>`) and a one-time
  `RUSTIC_COMPANION_TOKEN` in the environment (never on the command line, never on disk). The companion calls
  `POST <endpoint>/v1/companion/session` with `Authorization: Bearer <companion token>` and gets
  `{token, expires_in, uuid}`: a short-lived Rustic access token kept only in its network thread's memory. Only a
  loopback endpoint is accepted.
- Cloud calls (HTTPS): `POST /v1/cosmetics/worn` (no auth, at most 64 UUIDs; `{ttl, players: {uuid: [{id, version,
  hash}]}, playing: {uuid: {item, ms}}}`), bundles from the CDN by content address
  (`<cdn>/cosmetics/<id>/<version>/<sha256>.json`, SHA-256 checked, then the format's validation), `GET /v1/social`
  (friends, invites, messages, owned emotes), `POST /v1/social/invites {to, address}`,
  `POST /v1/social/invites/{id}/accept`, `POST /v1/emotes/playing {emote}`. Answers cached for their TTL (1 min to
  1 h), batches of 64, backoff 15 s doubling to 10 min, 4096 players remembered.
- HTTP with `ureq` 2 (MIT OR Apache-2.0), already in Horizon's lockfile (the game's own client): no second TLS stack
  in the game.

## Consequences

The cloud owner adds these routes to the OpenAPI and the desktop owner serves the loopback route (HANDOFF). Until
then the companion runs signed out: others' cosmetics still show (no auth needed); the overlay asks to sign in.
