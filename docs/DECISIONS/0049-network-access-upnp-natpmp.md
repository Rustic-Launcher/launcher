# 0049. LAN addresses, UPnP and NAT-PMP

- Status: accepted (desktop, 2026-11-10)

## Context
Local servers are reachable on the LAN, through a router mapping, through the relay, or with an invite
(launcher/AGENTS.md). Horizon's Play with Friends does PCP, NAT-PMP, UPnP.

## Decision

`netdev` 0.47 (MIT, `gateway` feature) for interfaces and the default gateway (container/VM bridges left out,
100.64/10 as VPN); `igd-next` 0.18 (MIT, `aio_tokio`) for UPnP IGD with a one-hour lease, permanent if the
router refuses timed ones; NAT-PMP (RFC 6886) written here (40 lines, tested against a fake router) because
the crates for it are sync or unmaintained. A `ReachabilityProbe` trait (HTTP checker contract for the cloud)
tests public reachability; without one the answer is "unknown".

## Consequences

PCP isn't implemented (NAT-PMP covers the same routers' fallback).

## Alternatives

`natpmp` crate (sync, older), `local-ip-address` (no gateway).
