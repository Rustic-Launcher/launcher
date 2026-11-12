# 0010. Microsoft sign-in: authorization code + PKCE (loopback), device code as fallback

- Status: accepted (spec, 2026-11-10)

## Context

The mission said "device code + PKCE": these are two different OAuth grants (the device-code grant has no PKCE).
Horizon uses the device-code flow only (`horizon/docs/microsoft-account.md:15-26`). Device code needs the player to type
a code on another page; a loopback redirect is one click in the system browser (RFC 8252). Our own Azure app and Mojang
approval are mandatory (`horizon/docs/forking.md`, TODO_HUMAN).

## Decision

1. Primary: authorization code + PKCE (S256), `consumers` tenant, scopes `XboxLive.signin offline_access`, redirect
   `http://localhost` (Azure "Mobile and desktop applications" platform; the actual port is random per attempt), system
   browser, `state` checked, listener closed after one hit or 5 minutes.
2. Fallback: device-code grant ("Sign in on another device"), shown with the code and a QR code.
3. Then Xbox user → XSTS → `login_with_xbox` → entitlements → profile (ARCHITECTURE §3.1).
4. Client id injected at build time (`option_env!("RUSTIC_MSA_CLIENT_ID")`), overridable by env in dev builds only.

## Consequences

Azure app needs "Allow public client flows" (device code) and a localhost redirect URI. Both flows share token storage.

## Alternatives

Embedded WebView login (phishing-shaped, Microsoft discourages it). Rejected.
