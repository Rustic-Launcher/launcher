# Rustic Launcher: architecture

Owner: spec (Phase 0). Implementation owners are in `launcher/AGENTS.md`. Facts about Horizon are cited in
[HORIZON_FACTS.md](HORIZON_FACTS.md) (HF §n); decisions in `DECISIONS/` (ADR nnnn).

## 1. System overview

```
┌───────────────────────────── Player's computer ─────────────────────────────┐
│  Rustic Launcher (Tauri v2)                                                  │
│  ┌──────────────────────┐ IPC (commands/events, docs/IPC.md) ┌────────────┐ │
│  │ React UI (WebView)   │◄──────────────────────────────────►│ src-tauri  │ │
│  │ apps/launcher/src    │                                    │ (Rust)     │ │
│  └──────────────────────┘                                    └─────┬──────┘ │
│    crates: launcher-core · auth · instances · content · servers ·  │        │
│            tunnel · hosting · api-client · cosmetics-format         │        │
│  OS keychain (tokens, Porkbun keys, SSH keys)                       │        │
│        │ spawn + stdin session            │ spawn                   │        │
│  ┌─────▼──────────────────┐   ┌───────────▼────────────┐            │        │
│  │ Rustic edition client  │   │ horizon-server (local) │            │        │
│  │ Horizon + companion    │   │ engine.path→rustic-    │◄─ tunnel ──┘        │
│  └─────┬──────────────────┘   │ server, plugins/       │   (rusticmc-tunnel) │
└────────┼──────────────────────┴───────────┬────────────┴─────────────────────┘
         │ HTTPS/WSS                         │ TLS tunnel (outbound)
┌────────▼───────────────────────────────────▼──────────── Rustic Cloud (EU) ──┐
│ api (axum)   realtime (WS)   relay (TCP 25565, routes by handshake host)      │
│ Postgres · Redis · object storage + CDN (cdn-pipeline output) · Stripe        │
└───────────────────────────────────────────────────────────────────────────────┘
External: Microsoft identity + Xbox Live + Minecraft Services · Mojang session server ·
GitHub releases of horizon-rust/horizon · Horizon registry · Modrinth · (CurseForge later) ·
Porkbun API (user's own keys) · user's VPS over SSH · Stripe · email provider.
```

## 2. Components

### 2.1 Desktop app

| Component | Path | Responsibility |
|---|---|---|
| UI | `apps/launcher/src` | React 19 + TS + Vite + Tailwind v4 + Radix + Motion, TanStack Router/Query, Zustand, i18next (ADR 0001-0003). Pure presentation + view state; every side effect through IPC or the Cloud API client. Runs in a browser with the IPC mock. |
| Design system | `packages/ui` | Tokens from [PALETTES.md](PALETTES.md), components, Storybook. |
| Tauri shell | `apps/launcher/src-tauri` | Commands/events (snake_case / kebab-case, `docs/IPC.md`), window, tray, deep links `rustic://`, single instance, automatic updates of the launcher and the Rustic edition (`update.rs`, ADR 0162), capability files (least privilege). |
| `rusticmc-launcher-core` | crates | Paths, settings, job/download manager (resumable, hash-checked, progress events), **Horizon build manager** (Rustic edition builds), `horizon-prepare` driver (`--progress json`), game process supervisor (spawn, stdin session, exit codes, crash report path). |
| `rusticmc-auth` | crates | Microsoft OAuth (auth code + PKCE loopback, device-code fallback), XBL → XSTS → `login_with_xbox` → entitlements → profile; refresh; keychain storage; owner gate + offline grace; Rustic account session (link via Mojang join/hasJoined). |
| `rusticmc-instances` | crates | Instances (folder per instance, `--game-dir/--home`), pinned build, content state, import/export. |
| `rusticmc-content` | crates | Sources (Horizon registry, Modrinth, Rustic catalogue; CurseForge later), search, compatibility, install/uninstall/update of `.hmod`, resource/shader/data packs, `.hpack`, `.rplugin`; hash and signature checks. |
| `rusticmc-servers` | crates | Server folders, `horizon-server init/start/stop`, control API client (`/api/v1`, token from `.horizon/api.json`), `server.toml` editing, `plugins/` (`.rplugin`) management, backups. |
| `rusticmc-tunnel` | crates | Outbound tunnel from a local server to the Rustic Relay (temporary hostname / custom domain modes). |
| `rusticmc-hosting` | crates | `HostingProvider` (Local, RusticCloud placeholder, Vps over SSH/SFTP), `DnsProvider` (Porkbun, Manual; Cloudflare later), domain purchase through the user's Porkbun account, DNS checker. Owner: "hosting". |
| `rusticmc-api-client` | crates | Generated from `services/api/openapi.json`. |
| `cosmetics-format` | `packages/cosmetics-format` | Rustic cosmetics format, validation, conversion to/from Horizon format 1 (ADR 0007). Shared by launcher, companion, cdn-pipeline. |

### 2.2 In-game: the Rustic edition client (ADR 0006)

`client-mod/rustic-companion` (own workspace, links `horizon-client` at a pinned Horizon commit with Horizon's
lockfile). CI produces **`rustic-client`**, an executable whose `main`:

1. registers engine-level parts with `horizon_platform::hooks::register` (HF §3.3): the companion's
   `player_layers` + `name_decoration` + `first_person` (Rustic cosmetics), `hud::register` (Rustic HUD:
   friend joined / party toasts), `pages::register` (a "Rustic" settings page);
   plus the chosen Horizon parts: `horizon_friends::game::HOOKS` (invite-code joins). Not registered:
   `horizon_heartbeat` (always-on ping to Horizon's telemetry; our privacy promise), `horizon_official_servers`
   (another project's server ad), `horizon_discord` (Rustic has its own presence later);
2. calls `horizon_client::entry::main_with(Arc::from([rustic_companion::hooks()]))` where `hooks()` is a
   trait mod (`mod_api::Hooks`) using catalog hooks: `client_start`, `client_level_load`, `net_from_server`
   (`rustic:*` channels on `horizon:mod_net` for Rustic Server features), `client_chat_received` (party chat
   routing), `screen_*`/`menu_button` (a "Rustic" button).

Every hook body is wrapped in `catch_unwind` (a panic in a hook kills the game, root AGENTS.md). The
companion talks to the launcher over a **local authenticated channel** (loopback WebSocket, one-time token
passed in the session hand-off environment, not argv) for presence (in-game, server address if the user allows),
and to Rustic Cloud with the Rustic session for cosmetics (`/v1/worn`-like batch fetch).

The release bundle per platform = the Horizon release folder of the same commit (`horizon-prepare`,
`research-lock.json`, `exporters/`) + `rustic-client[.exe]` + `rustic-build.json` (Horizon commit, companion
version, hashes) + our Ed25519 signature; Authenticode / notarised (TODO_HUMAN). Hosted on Rustic's CDN so
Horizon's 7-day Nightly deletion does not break pinned instances (HF A9).

### 2.3 Rustic Cloud

| Service | Path | Responsibility |
|---|---|---|
| `api` | `services/api` | axum + sqlx + Postgres; OpenAPI via utoipa. Accounts (email+password argon2id, "Continue with Microsoft" OIDC id_token verification, ADR 0090), MC identity link, sessions (short JWT access + rotating refresh), profiles, friends graph, blocks, reports/moderation, parental controls, cosmetics catalogue + ownership + equipped, store (Stripe Checkout, webhooks, invoices, refunds = revocation), server directory (servers a user owns, hostnames, domains, verification), GDPR export/delete, consent records, waitlist (Rustic Cloud hosting). |
| `realtime` | `services/realtime` | WebSocket gateway: presence, friend requests, invites, party, DMs (age-gated), notifications; Redis pub/sub fan-out; rate limits. |
| `relay` | `services/relay` | Public TCP 25565 listener; reads the handshake packet's server address and routes to the matching host tunnel; tunnel endpoint for `rusticmc-tunnel`; per-route limits; PROXY protocol v2 or Velocity-style forwarding of the real client IP to the server (relay owner's ADR). |
| `cdn-pipeline` | `services/cdn-pipeline` | Validates uploaded cosmetics (format + budgets), renders thumbnails, writes content-addressed files to object storage behind a CDN; builds the signed catalogue. |
| `migrations` | `services/migrations` | sqlx migrations. |
| Postgres 18, Redis 7/Valkey | infra | Durable data; presence, rate-limit buckets, sessions blacklist, pub/sub. |

## 3. Login and identity

### 3.1 Microsoft (game licence)

1. **Authorization code + PKCE** (RFC 7636) with a loopback redirect `http://127.0.0.1:<random>/callback`
   (RFC 8252) in the system browser, `consumers` tenant, scopes `XboxLive.signin offline_access`, `state` and
   `code_verifier` per attempt. **Fallback: device-code flow** (as Horizon, HF §1) when the browser is on another
   device or the loopback is blocked. Our own Azure app id (never Horizon's: `docs/forking.md`).
2. Xbox Live user auth (`user.auth.xboxlive.com`) → XSTS (`xsts.auth.xboxlive.com`, relying party
   `rp://api.minecraftservices.com/`); XSTS errors mapped to UX (2148916233 no Xbox profile, 2148916238 child
   account needs family consent, 2148916235 region).
3. `POST api.minecraftservices.com/authentication/login_with_xbox` → Minecraft access token (≈24 h).
4. `GET /entitlements/mcstore` (or `/entitlements`): require `game_minecraft` or `product_minecraft`
   (HF §1, `docs/microsoft-account.md:62-75`). Not owned → "You need Minecraft: Java Edition" screen, no play.
5. `GET /minecraft/profile` → name, UUID, skins.
6. Store: MS refresh token + MC token + expiry in the OS keychain (`keyring`), service `gg.rustic.launcher`
   (placeholder until the domain is chosen), one entry per account; `accounts.json` keeps only names/UUIDs and
   `verified_at`.
7. **Offline / LAN**: allowed only for an account with a successful entitlement check, for 30 days after the last
   check (revalidated silently at every online start). No offline names otherwise.

### 3.2 Rustic account and linking

- Rustic account = platform identity (friends, cosmetics, purchases, servers). Sign-up by email (verified),
  or "Continue with Microsoft" (ADR 0090). Birth date asked at sign-up (age gates, LEGAL §4).
- **Link a Minecraft profile** without ever sending the MC token to Rustic: `POST /v1/mc-link/start` → server id
  (random); launcher calls Mojang `sessionserver …/session/minecraft/join` with the MC token, the UUID and that
  server id; `POST /v1/mc-link/finish {username}` → the API calls `hasJoined?username&serverId` and binds the UUID
  (same pattern as Horizon's cosmetics, HF §4). One UUID ↔ one Rustic account; re-link requires the same proof.
- Rustic session: short-lived access JWT (15 min, EdDSA) + rotating refresh token (30 days, in keychain),
  revocable; device list in Settings › Security.

## 4. Launch ("Play in two clicks")

1. UI → `instance_launch {id}`.
2. core: owner gate (cached entitlement ≤ 30 days, else re-check) → refresh MC token if < 10 min left.
3. Ensure the Rustic edition build pinned by the instance is installed and verified (signature, hashes);
   otherwise download (progress in the bottom bar).
4. `horizon-prepare --check --output <prepared>`; if not current: locate the player's own game files
   (official launcher folder or the Mojang download for an owner), `--prepare --complete --progress json`
   (HF §2). Prepared resources are shared per Horizon commit (`<data>/prepared/<commit>/`).
5. `rustic-client --launch-api` once per build: verify options we pass are listed.
6. Spawn `rustic-client --resources <prepared> --game-dir <instance> --home <instance> --width --height
   --session-stdin [--server host:port]` with stdin piped; write the session JSON; close stdin. Env: the
   companion's local-channel token (`RUSTIC_COMPANION_TOKEN`), no tokens in argv, `HORIZON_LAUNCHER_EXE` **not**
   set (we are not horizon-launcher; RISKS R-12).
7. Watch stdout/stderr into the instance log; exit 2 → "incompatible option" bug report; 1/101 → crash dialog with
   the crash report path (last stderr line) and "Send report" (only with crash-report consent).
8. Presence: realtime "in game (instance X)"; with "share server" on, the server address for "Join".

## 5. Content install

1. Search fan-out to sources (Horizon registry index cached 10 min; Modrinth API with our User-Agent; Rustic
   catalogue). Results merged, deduplicated by source+id.
2. Compatibility badge computed locally (UX_SPEC §4.3): from Horizon index `side`, `api`, `game`, `broken_on`, kind,
   and Horizon's own Works/Partly/Doesn't-run logic for Java content (reimplemented from
   `launcher/src/browser.rs:299-368`, MIT, attributed).
3. Install: download from the allow-listed host only (registry GitHub releases, `cdn.modrinth.com`, Rustic CDN),
   size cap, SHA-256/512 check, minisign signature when the index gives a key, then unpack into the instance
   (`horizon-mods/`, `resourcepacks/`, `shaderpacks/`, `<world>/datapacks/`) or the server (`plugins/` for
   `.rplugin`). State recorded in the instance's `rustic-content.json` (source, id, version, hash).
4. Native mods (`.hnmod`) recompile the client: an instance that has any plays the Rustic edition compiled for it
   on the player's PC (`crates/rusticmc-native`, ADR 0157: trust notice, Rust, edition source archive, horizon-mixins,
   `cargo build --profile fast`, client in `<instance>/native/bin/`); the others keep the prebuilt client.

## 6. Servers

**Server directory (ADR 0158).** Besides the servers players host, the cloud keeps a public directory of
servers their owners publish from the website (`/v1/server-listings`, migration 0011): automatic checks then
published, verified badge, staff picks (the former featured servers), take-downs and reports, a status ping
from the cloud (public addresses only) and an anonymous "Play" counter for trending. The launcher's Servers page
(My servers / Discover / Browse, a server page, `rustic://server/<slug>`) and the website's `/servers` read it;
the in-game Discover screen will too.

### 6.1 Hosting modes (`HostingProvider`)

| Mode | Where the server runs | Reachability | Status |
|---|---|---|---|
| **Local** | Player's PC (must stay on) | LAN (local IP shown) · temporary relay hostname · invite code · custom domain via relay | v1 |
| **Your VPS** | User's Linux VPS over SSH/SFTP | Public IP directly, or custom domain (A/AAAA or SRV) | v1 |
| **Rustic Cloud** | Rustic machines | Rustic hostname / custom domain | Coming soon (UI entry + waitlist + API placeholders) |

```rust
// rusticmc-hosting (sketch for the hosting owner; final API in their ADR)
trait HostingProvider {
    fn kind(&self) -> HostingKind;                    // Local | Vps | RusticCloud
    async fn install(&self, spec: &ServerSpec, progress: Progress) -> Result<ServerHandle>;
    async fn control(&self, h: &ServerHandle) -> Result<ControlClient>; // horizon-server /api/v1
    async fn files(&self, h: &ServerHandle) -> Result<Box<dyn FileAccess>>; // plugins/, config, backups
    async fn uninstall(&self, h: &ServerHandle, keep_world: bool) -> Result<()>;
}
```

### 6.2 Creating a Rustic Server (local)

1. Download the Rustic Server bundle (signed): `horizon-server` (from the matching Horizon release, Ed25519
   verified) + `rustic-server` engine (built by Rustic Server CI from `main`).
2. `horizon-server init --dir <srv> --eula --world <name> --seed <seed> --port <p> [--whitelist] --player <owner>
   --op <owner> --yes` (HF §6); EULA acceptance shown to the user first (they accept Mojang's EULA themselves).
3. Edit `server.toml`: `[engine] path = "<abs>/rustic-server[.exe]"`, `resources` = the prepared resources of the
   same Horizon commit; `[api] bind = "127.0.0.1:0"`, token generated and stored in the keychain;
   `online_mode = true` by default.
4. Install chosen `.rplugin` files into `<srv>/plugins/` (manifest checked: `rustic.plugin.toml`, `api = "0.1"`,
   capabilities shown to the user).
5. `horizon-server start --dir <srv>`; read `.horizon/api.json`; poll `GET /api/v1/status`; stream
   `GET /api/v1/events` (SSE) and logs to the management page.
6. Vanilla Horizon server = same steps without `engine.path` (uses `horizon-engine`/`horizon-client`).

### 6.3 Your VPS (SSH/SFTP)

1. User enters host, port, user; auth by a **launcher-generated Ed25519 key** (recommended; we show the public key
   and a one-line command, or push it once with the password, which is never stored) or an existing key.
2. **Host key TOFU**: fingerprint shown on first connect, pinned in the hosting store; a change blocks with a
   warning (SECURITY §2.8).
3. Preflight over SSH: OS (Debian/Ubuntu x86-64 supported first, `horizon-server-linux-x86_64` only exists for
   Linux x86-64: HF §6), free disk/RAM, systemd, firewall tool, open ports.
4. Install as a dedicated **unprivileged user `rustic`** (sudo needed once for: create user, install systemd unit,
   open the game port in ufw/firewalld): `/opt/rustic/<server>/` with `horizon-server`, `rustic-server`, signed
   and hash-checked on the VPS too; **game resources uploaded from the player's own prepared files** over SFTP
   (Mojang files never come from us); `horizon-server init` non-interactive; systemd unit (`horizon-server
   service install` exists, HF §6; we template our own for the `rustic` user if needed).
5. Management: the control API stays bound to `127.0.0.1` on the VPS; the launcher reaches it **only through an
   SSH local port-forward**; panel `allow_public = false`; RCON off.
6. Updates: launcher uploads the new signed engine, `horizon-server` backup, restart; rollback keeps the previous
   engine.

### 6.4 Reachability and the Rustic Relay

- **Temporary hostname** (Local mode, optional feature, config `relay.public_suffix`, placeholder
  `play.rustic.example`): `<adjective-noun-1234>.<suffix>`, valid while the server runs; wildcard DNS
  `*.<suffix>` → relay IPs; no DNS changes for the user.
- **Invite code** (Local server): a short Rustic code (`RST-K7QM-3XPD` style, Crockford base32, 24 h or until
  revoked) issued by the API for a running server's relay route. Friends paste it in the launcher (or click the
  invite in the friends panel): the launcher resolves it via the API to the route's relay hostname and starts the
  game with `--server <host>`. In-game typing of a code is not possible: Horizon allows **one** tunnel resolver and
  only the `friend:` prefix (`crates/horizon-hooks/src/tunnel.rs:10-11`, `lib.rs:336-337`), already taken by
  Horizon's Play with Friends.
- **Horizon Play with Friends** (single-player worlds opened to friends, `friend:<code>`, peer-to-peer, HF §5)
  stays available in the Rustic edition through `horizon_friends::game::HOOKS`; optionally pointed at a
  Rustic-operated `horizon-relay` via `HORIZON_RELAY_URL`. It is the game's feature, not a server hosting mode.
- **Custom domain** (§7): the player's own domain points at the relay (Local) or at the VPS (VPS mode).
- **Relay routing**: the relay accepts TCP on 25565, parses the first packet (handshake: protocol version, server
  address, port, intent), lower-cases the address and strips a trailing `.` and any Forge/FML `\0` suffix, looks up
  `hostname → route` in Redis (written by the API only after DNS verification), and splices bytes to the host's
  tunnel. Unknown hostname → a status response "This server is offline" for status pings, close for logins.
  Rate limits per source IP and per route; connection caps per plan.

## 7. Domains (`DnsProvider`), three equal paths

| Path | Who changes DNS | Credentials | Verification |
|---|---|---|---|
| **Connect Porkbun** | Launcher via Porkbun API v3 | User's API key + secret (keychain); domain must have "API access" opted in by the user at Porkbun | Launcher + cloud (below) |
| **Buy at Porkbun** (in launcher) | Launcher after purchase | Same keys; purchase uses the user's Porkbun account credit/card | Same |
| **Bring your own domain** (any registrar) | The user, guided record by record with copy buttons and per-registrar guides | None | Same, live polling |

Records (computed per case and shown identically on all three paths):

| Case | Records |
|---|---|
| Relay, subdomain (recommended) | `play.example.com CNAME <route-id>.<relay-host>` (game port 25565) |
| Relay, apex (`example.com`) | `ALIAS/ANAME example.com → <route-id>.<relay-host>` where supported (Porkbun has ALIAS); else use a subdomain |
| Relay, SRV form | `_minecraft._tcp.play.example.com SRV 0 5 25565 <route-id>.<relay-host>.` **Note**: clients log in with the SRV *target* in the handshake (HF §8), so the relay routes on `<route-id>.<relay-host>`, a per-server name we issue, never on a shared target |
| VPS, direct | `play.example.com A <vps IPv4>` (+ `AAAA`), plus SRV if the port is not 25565 |
| Ownership (all) | `_rustic-challenge.play.example.com TXT "rustic-verify=<token>"` |

Note on the relay host name: `<route-id>.<relay-host>` needs a wildcard DNS name under a domain Rustic controls
(the relay's own hostname, placeholder `relay.rustic.example`). This is infrastructure, not a per-user subdomain
product; TODO_HUMAN lists it.

**Verification flow**

1. Launcher requests `POST /v1/servers/{id}/domains {fqdn}` → API returns the TXT token, the route id and the
   record set.
2. Porkbun paths: launcher creates/updates records (`/dns/create`, `/dns/editByNameType`, idempotent, `dryRun`
   preview shown first); BYO path: the user adds them.
3. Launcher polls public resolvers (DoH to two independent resolvers, plus the authoritative NS from the zone's
   SOA) every 10 s with backoff, and shows each record's status: **missing / wrong value / propagating (some
   resolvers OK) / OK**.
4. When all OK, launcher calls `POST /v1/servers/{id}/domains/{fqdn}/verify`; the **API re-checks from the cloud**
   (TXT token, CNAME/ALIAS/SRV target) and only then writes `fqdn → route` (and `<route-id>.<relay-host>` → route)
   into the relay's routing table. Re-verified daily; a removed TXT disables routing after 72 h grace with a
   notification.
5. VPS direct mode needs no relay route; verification only confirms the A/AAAA points at the VPS (UX feedback).

**Buying a domain** (Porkbun path 2): `POST /domain/checkDomain/{name}` (availability, price, renewal price;
10 checks / 10 s) → the launcher shows price **and renewal price** in USD as Porkbun quotes them →
`getRegistrationRequirements/{tld}` (some TLDs need extra fields or are not API-registrable) → explicit
confirmation dialog (Porkbun is the seller; Porkbun's terms; `agreeToTerms: "yes"` only after the user ticks it)
→ `POST /domain/create/{name}` with `cost` in cents (`COST_MISMATCH` on price change re-quotes), an
`Idempotency-Key`, payment from the user's Porkbun credit/saved card → then path 1. Constraints from Porkbun:
verified email/phone, $100 cap per API registration, 1 registration attempt/s.

## 8. Cosmetics propagation

1. Creator uploads (internal tool first) → `cdn-pipeline` validates (`cosmetics-format`, budgets like Horizon's:
   512 vertices, 64×64 texture, 16 parts), renders thumbnails, stores `items/<sha256>.json` on CDN, adds to the
   signed catalogue.
2. Purchase (Store) → Stripe Checkout in the system browser → webhook `checkout.session.completed` → API grants
   ownership (idempotent on event id) → realtime notifies the launcher.
3. Equip in Locker → `PUT /v1/me/equipped {slot,item}` (Rustic session).
4. In game, the companion batches the UUIDs it sees (≤ 64) → `POST /v1/worn` (no personal data beyond UUIDs) →
   `{player: [{slot,item,hash}]}`, TTL 60-3600 s → items fetched from CDN by hash, cached on disk → drawn via
   `player_layers` / `name_decoration`.
5. Locker and Store render the same items in the launcher's 3D viewer (react-three-fiber) from the same files.
6. Refund / chargeback → ownership revoked → next `/v1/worn` no longer lists it.

## 9. Security boundaries (details in [SECURITY.md](SECURITY.md))

| Boundary | Trust | Controls |
|---|---|---|
| WebView UI ↔ Rust core | UI is untrusted-ish (renders remote content) | Tauri capabilities allow-list, no shell/fs plugins exposed, strict CSP, all inputs validated in Rust, no secrets ever cross to JS (UI sees account names, never tokens/keys) |
| Launcher ↔ game | Same user | Session on stdin only; companion channel loopback + one-time token |
| Launcher ↔ Rustic Cloud | Internet | TLS, Rustic session, certificate pinning not used (CA + HSTS), signed catalogue/builds |
| Downloads | Untrusted | Allow-listed hosts, size caps, hashes, Ed25519/minisign signatures, Authenticode/notarisation |
| Relay | Internet-facing | No auth to play (it is a game port), per-IP/route limits, only verified hostnames routed, provider DDoS protection |
| Host tunnel ↔ relay | Authenticated | mTLS or signed route token issued by the API, per-server |
| Launcher ↔ VPS | User's machine | SSH key in keychain, pinned host key, control API only via SSH port-forward |
| Launcher ↔ Porkbun | Third party | User's keys in keychain, used only from the user's machine, never sent to Rustic Cloud |
| Plugins (`.rplugin`) | Untrusted code | Rustic Server's wasmtime sandbox with declared capabilities (root repo) |
| Content mods/packs | Data | Hash + source allow-list; no code execution (native mods: trust notice + local build, ADR 0157) |

## 10. Deployment

### Dev (`launcher/docker-compose.yml`, cloud owner)

`postgres:18`, `valkey:8` (Redis-compatible, BSD), `minio` (S3 for the CDN pipeline), `mailpit` (SMTP sink),
`api`, `realtime`, `relay` (ports 8080, 8081, 25565/25566), `stripe-cli` for webhooks (profile `payments`). Local
Postgres 18 can replace the container. Seed data: test accounts, a sample catalogue made of original placeholder
items.

### Production (sketch)

- EU region (GDPR): two app nodes (api + realtime) behind an HTTPS load balancer / CDN-WAF (e.g. Cloudflare),
  managed Postgres with PITR, managed Redis/Valkey, object storage + CDN for cosmetics and Rustic edition builds.
- **Relay**: separate nodes with dedicated IPs on a provider with game-grade (L4) DDoS mitigation; anycast or
  per-region relays later; health checks; the API pushes routes via Redis.
- CI: GitHub Actions; Rustic edition matrix builds (Windows x64/ARM64, macOS arm64/x64, Linux x64) on a Horizon
  commit bump; signing keys in protected environments.
- Observability: OpenTelemetry traces/metrics (self-hosted), structured logs with PII scrubbing, Sentry-compatible
  self-hosted crash intake for opt-in reports only.

## 11. Open architecture questions

Listed in [RISKS.md](RISKS.md) §3 and in the Phase 0 report: Horizon parts kept in the Rustic edition; session
renewal; local native-mod builds; relay DDoS provider; Stripe vs a merchant of record.
