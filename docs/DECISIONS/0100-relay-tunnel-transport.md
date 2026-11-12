# 0100. Relay tunnel transport: TLS over TCP, yamux multiplexing

- Status: accepted (relay, 2026-11-10)

## Context

A server on a player's PC keeps one outbound connection to Rustic Relay. Each player connection needs its own
ordered byte stream through it, with flow control so a slow player can't stall the others. The tunnel must work
from home networks, schools and CGNAT (outbound TCP is the one thing that always works), and it must be
authenticated and private between the PC and the relay.

## Decision

- **TCP + TLS 1.2/1.3** (`rustls` 0.23 with the `ring` provider chosen explicitly, `tokio-rustls` 0.26), ALPN
  `rustic-relay/1`. The client trusts the OS store (`rustls-native-certs` 0.8) or pinned certificates.
- **yamux** (`yamux` 0.13, the libp2p/Parity implementation) on the TLS stream. Per-stream windows start at
  256 KiB and auto-tune, capped at 1 MiB per stream by our config. The stream limit is plan players + pings + 4.
  The client opens one control stream; the relay opens one data stream per player; the client sends a 1-byte ack
  after it reaches the local server. Wire format: `rusticmc-tunnel::wire` and RELAY.md §2.
- Authentication comes before multiplexing: a JSON `hello` frame carries the token, and the relay allocates
  yamux state only after the token verifies.

## Consequences

- All players of one tunnel share TCP-level head-of-line blocking. This is acceptable: one server's players
  already share the owner's uplink.
- yamux's API is poll-based, so a task we own drives it (`mux::spawn`). yamux announces a stream with its first
  frame, so the client writes a ping on the control stream at once.
- Crates checked on 2026-11-10: `yamux` 0.13.10 Apache-2.0 OR MIT, maintained in the libp2p ecosystem; `rustls`
  Apache-2.0 OR ISC OR MIT; `ring` 0.17 Apache-2.0 AND ISC; `tokio-rustls` MIT OR Apache-2.0;
  `rustls-native-certs` Apache-2.0 OR ISC OR MIT; `tokio` and `tokio-util` MIT; `futures` MIT OR Apache-2.0.

## Alternatives

- **QUIC (`quinn`)**: avoids TCP head-of-line blocking and has TLS 1.3 and streams built in. But some school and
  corporate networks block or throttle UDP, so this TCP design would be needed as a fallback anyway: twice the
  code. Revisit if many tunnels show head-of-line stalls.
- **`tokio-yamux`** (Nervos): tokio-native with a nicer API, but fewer maintainers than libp2p's `yamux`.
- **Custom framing**: simple framing is easy; correct per-stream flow control and window management are not.
- **One TCP connection per player** (the relay asks the client to dial back, like `horizon-relay`'s attach): no
  multiplexing, but a TLS handshake and a token check per player, and more connections through CGNAT.
