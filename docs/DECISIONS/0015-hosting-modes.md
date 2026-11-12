# 0015. Hosting modes: Local, Your VPS (SSH), Rustic Cloud (coming soon)

- Status: **accepted (user decisions, 2026-11-10)**; implementation owner: hosting (ADRs 0120+)

## Context

Players want to host with no infrastructure (their PC), on a VPS they rent, and later on Rustic machines. Horizon's
`horizon-server` supervisor exposes a non-interactive `init`, a localhost control API with a bearer token and a systemd
service installer (`horizon/docs/hosting.md`, `docs/api/server.md`); its Linux archive ships `horizon-engine` only for
Linux x86-64.

## Decision

- `HostingProvider` trait in `rusticmc-hosting` (ARCHITECTURE §6.1) with `Local`, `Vps` and `RusticCloud`
  (placeholder: UI card "Coming soon" + waitlist, API endpoints returning 501 with a waitlist link).
- **VPS**: SSH/SFTP (Rust `russh` 0.65.1, Apache-2.0, published 2026-11-03; to be confirmed by the hosting owner), launcher-generated Ed25519 key,
  host key pinned on first use, dedicated unprivileged `rustic` user, systemd unit, firewall rule for the game port only,
  control API reached only through an SSH local port-forward, game resources uploaded from the player's own prepared
  files. Supported targets v1: Debian 12/13, Ubuntu 22.04/24.04 on x86-64.
- **Local**: `horizon-server` in the user's data folder; exposure by LAN, temporary relay hostname, Rustic invite code
  (resolved by the launcher), or a custom domain via the relay.

## Consequences

Rustic Cloud hosting will be a paid service under our own terms (LEGAL §8). VPS support creates a support surface:
clear preflight errors and an "uninstall from VPS" action.

## Alternatives

Agent daemon on the VPS with its own port (extra attack surface): rejected in favour of SSH-only management.
