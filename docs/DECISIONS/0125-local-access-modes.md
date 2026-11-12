# 0125. Local server access: LAN, relay hostname, invite code

- Status: accepted (hosting, 2026-11-10); complements ADR 0105 (relay)

## Decision

- `access::local_addresses` lists usable IPv4s (private = LAN; `100.64/10` or VPN interfaces = VPN; loopback,
  link-local, Docker/WSL/VM bridges hidden).
- Relay hostnames go through a `RelayTunnel` trait implemented over `rusticmc-tunnel` (relay owner).
- Invite codes go through an `InviteSharer` trait. Rustic's default code resolves via the cloud to the relay route
  (ADR 0105), so every client can join. Horizon's `horizon-friends` host side *can* front a dedicated server (it
  forwards any loopback port, end-to-end encrypted, P2P with relay fallback), but only `friend:`-aware clients
  (horizon-official / the Rustic edition) can join, 8 sessions at most, and every guest appears as 127.0.0.1 (IP
  bans useless). It stays an optional later `InviteSharer` reusing Horizon's crate unchanged (MIT).

## Consequences

Until the relay and cloud pieces are wired, `AccessMode::Invite` answers `not_available_yet` and `Relay` answers
`unsupported`; LAN works now.
