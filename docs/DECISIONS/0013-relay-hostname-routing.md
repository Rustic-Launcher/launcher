# 0013. Rustic Relay: route by handshake hostname, only verified hostnames

- Status: accepted (spec, 2026-11-10); the relay owner designs the protocol (ADRs 0100+)

## Context

Local servers sit behind NAT. Horizon's Play with Friends is peer-to-peer by invite code with an optional relay routing
by nameplate (`horizon/docs/play-with-friends.md`), for single-player worlds only. Rustic wants players to type a host
name (temporary or the owner's domain) in a stock client.

## Decision

- The relay listens on TCP 25565, parses the handshake (server address, port, next state), normalises the address
  (lower-case, strip trailing dot and `\0`-suffixed forge markers), and routes to the host tunnel registered for that
  hostname.
- Only hostnames in the API-written routing table are routed: temporary hostnames the API issued, and custom domains
  verified by TXT + target check (ARCHITECTURE §7). Unknown hostname: status ping gets an "offline" MOTD, login is closed.
- SRV records are followed by clients and **the handshake then carries the SRV target** (`horizon-protocol`
  `resolver.rs:1-55`, vanilla parity). Each route therefore also has a unique `<route-id>.<relay-host>` name, used as
  CNAME/SRV target.
- Real client IP passed to the server (PROXY protocol v2 or forwarding) so bans work; per-IP and per-route limits.

## Consequences

Needs a wildcard DNS name under a Rustic-controlled domain for `<route-id>.<relay-host>` (infrastructure, TODO_HUMAN),
even though user-facing subdomains are optional.

## Alternatives

Port-per-server (no hostnames, ugly addresses, port exhaustion); invite codes only (not typeable by stock clients).
