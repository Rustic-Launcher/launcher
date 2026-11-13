# Rustic Launcher: security and threat model

Owner: spec. Applies to every owner. Architecture context: [ARCHITECTURE.md](ARCHITECTURE.md) §9.

## 1. Assets and adversaries

| Asset | Why it matters |
|---|---|
| Microsoft refresh token / Minecraft access token | Full control of the player's game account (and Microsoft consumer identity scopes we requested) |
| Rustic refresh token | Purchases, friends, servers, domains of the user |
| Porkbun API key + secret | Control of the user's domains and the ability to **buy domains with their money** |
| SSH private key / VPS access | Root-equivalent control of the user's server (after sudo) |
| Server control API token | Console (op commands), config, backups |
| Rustic signing keys (Ed25519 builds, Authenticode, Apple) | Supply chain of every installation |
| Stripe secret + webhook secret | Payments, refunds |
| User data (PII, minors, messages) | GDPR, safety |
| Relay capacity | Availability of everyone's public servers |

Adversaries: malware/other local users on the player's PC (out of scope beyond OS protections), malicious content
authors (mods, packs, plugins, cosmetics), network attackers, malicious websites/links trying to drive the launcher
(deep links), abusive players (DDoS on relay, spam, harassment), credential stuffers on the API, compromised CI or
dependency.

## 2. Desktop app

### 2.1 Secrets storage

- All secrets in the **OS keychain** via `keyring` (Windows Credential Manager, macOS Keychain, Linux Secret Service);
  one entry per secret, service name `<reverse-domain>.launcher` (decided with the domain). Linux without a Secret
  Service: encrypted file (ChaCha20-Poly1305, key in a 0600 file) as Horizon does
  (`horizon/launcher/src/account.rs:1023-1044`), with a visible warning.
- Never on disk in plain text, never in logs (a `Secret<T>` newtype with redacted `Debug`, `zeroize` on drop), never in
  argv, never sent to the WebView (the UI receives account names and "connected" booleans only).
- Game session over **stdin** (ADR 0008). No session files.

### 2.2 IPC surface (Tauri)

- Commands listed in `docs/IPC.md` only; Tauri v2 **capabilities** allow exactly those commands for the main window;
  no `shell`, `fs`, `http` plugin scopes exposed to JS (Rust does all I/O). `opener` limited to `https:` URLs after an
  allow-list check (or a confirm dialog for unknown hosts).
- Every command validates inputs in Rust (ids are UUIDs or slugs `[a-z0-9-]{1,64}`, paths are resolved under managed
  roots and checked against traversal, URLs parsed and host allow-listed).
- Long-running commands are jobs with ids; cancellation by id; no command takes arbitrary command lines (launch options
  "pre-launch/wrapper" are advanced user settings stored by Rust and shown with a warning; never settable from deep
  links or remote content).
- CSP: `default-src 'self'; img-src 'self' https://cdn.<rustic> https://cdn.modrinth.com data: blob:; connect-src
  'self' ipc: https://api.<rustic> wss://rt.<rustic>; script-src 'self'; style-src 'self' 'unsafe-inline'` (Tailwind
  runtime styles, revisit); no remote scripts; Markdown from content sources sanitised (no raw HTML, links rewritten to
  open externally).
- Deep links `rustic://join/<code>`, `rustic://content/<source>/<id>`, `rustic://server/<slug>` (ADR 0158): parsed
  strictly, always lead to a confirmation UI, never perform an action directly (no install, no purchase, no server
  change, no join from a server link).
- Server directory pings (ADR 0158): the cloud only connects to listed servers whose every resolved address is
  public (no private, loopback, link-local, CGNAT, multicast, documentation or reserved range, IPv4 or IPv6), on
  ports 1024 and up, to the address it checked (no second DNS lookup), with a 4 s budget and a 64 KiB answer cap.

### 2.3 Updates and downloads

- Launcher self-update: `tauri-plugin-updater` with its minisign/Ed25519 signature (key offline, CI signs in a protected
  environment), HTTPS, plus Authenticode (Windows) and notarisation (macOS).
- Rustic edition builds and server bundles: `SHA256SUMS` signed with our Ed25519 key, verified before unpacking; per-file
  hashes from `rustic-build.json`; unpack into a fresh directory with zip-slip/symlink protection; atomic swap.
- Horizon artefacts we fetch directly (`horizon-server` for VPS/local): verify Horizon's `SHA256SUMS.sig` with the key in
  `horizon/crates/horizon-release-sign/release-keys.txt` (pinned in our code).
- Content: allow-listed hosts only (Horizon registry GitHub releases, `cdn.modrinth.com`, Rustic CDN), size caps,
  SHA-256/512, minisign when provided; archives unpacked with path checks.

### 2.4 Game and companion

- The companion's loopback channel: `127.0.0.1` random port, one-time token in env, origin check, only allows
  presence and short-lived Rustic access tokens (scoped `cosmetics:read`); no purchase or account operations from the
  game.
- `catch_unwind` around every hook; cosmetics files from the CDN are parsed with strict schemas and budgets (DoS by a
  huge model is impossible).

### 2.5 Mods, packs, plugins trust

- Content mods/packs are data. Native mods are unsandboxed native code (`horizon/docs/native-mods.md:19`), compiled
  on the player's PC since ADR 0157: a trust notice per instance the player ticks and accepts (versioned, stored in
  `<instance>/native/consent.json`; no command, deep link or agent can accept it), only Horizon's registry (SHA-256,
  minisign key, `revoked.toml`, its review) or admin-approved marketplace versions, the `.hnmod` re-checked by the
  builder (Rust/TOML/data files under `mixins/<id>/` only, limits, no links), rustup-init checked against its
  published SHA-256, WinLibs and the edition source pinned by SHA-256, `cargo build --locked` (only the edition
  lockfile's crates), the player's Cargo/rustc settings dropped. The built game itself is not sandboxed.
- `.rplugin` plugins run in Rustic Server's wasmtime sandbox with fuel/epoch/memory limits and declared capabilities
  (`network` domains, `filesystem` data folder); the launcher shows capabilities before install and on update when they
  grow.
- Fair-play deny-list prevents promoted cheats (FAIR_PLAY.md).

### 2.6 Porkbun keys

- Stored only in the keychain; used only from the user's machine; never sent to Rustic Cloud or logs.
- Encourage **domain-restricted keys** (Porkbun lets a key be limited to listed domains) in the connect guide.
- Every DNS change previewed (dry run) and confirmed; purchases need an explicit confirmation dialog with price and two
  ticks; an `Idempotency-Key` per attempt prevents double purchases; no background purchase ever.
- Removal in Settings › Connections deletes the keychain entries.

### 2.7 DNS verification

- The cloud, not the launcher, decides routing: it re-checks the TXT token and target itself before writing a route
  (a modified launcher cannot hijack hostnames). Tokens are random 128-bit, per server+hostname; re-verified daily.

### 2.8 SSH / VPS

- Launcher-generated **Ed25519** key per VPS (private key in keychain or an encrypted file with a keychain-held
  passphrase); passwords used once to install the public key, never stored.
- **Host key TOFU + pinning**: fingerprint shown on first connect (SHA256 form), stored; a mismatch aborts with a clear
  warning; re-pin only after explicit confirmation.
- **Least privilege**: install creates a `rustic` system user owning `/opt/rustic`; sudo is used only during install
  (user creation, systemd unit, firewall rule) with the exact commands shown; the service runs as `rustic` with systemd
  hardening (`NoNewPrivileges`, `ProtectSystem=strict`, `ReadWritePaths=/opt/rustic`, `PrivateTmp`).
- Management API (`horizon-server` control API) bound to `127.0.0.1` on the VPS, token in `.horizon/api.json` (0600);
  reached **only through an SSH local port-forward**; panel and RCON off; firewall opens only the game port (and SSH).
- Uploaded binaries are hash-checked on the VPS after transfer.

## 3. Rustic Cloud (OWASP Top 10, 2021 numbering)

| Risk | Mitigation |
|---|---|
| A01 Broken access control | Every handler checks ownership (`user_id` from the token, never from the body); row-level checks in queries; admin routes on a separate router with role checks and audit log; IDs are UUIDv7, not guessable |
| A02 Cryptographic failures | TLS 1.2+ everywhere, HSTS; argon2id for passwords; EdDSA JWTs with key rotation (kid); secrets encrypted at rest by the provider; no secrets in URLs |
| A03 Injection | sqlx bound parameters only; no string-built SQL; HTML never rendered from user input on the web (escaping), Markdown sanitised |
| A04 Insecure design | Threat model per feature (this file), abuse cases (gift spam, invite spam, refund abuse), rate limits designed in |
| A05 Security misconfiguration | Minimal containers (distroless), no debug endpoints in prod, CORS allow-list (launcher origin `tauri://localhost` / `https://tauri.localhost`, web origin), security headers |
| A06 Vulnerable components | `cargo deny` (licences + advisories), `cargo audit`, `pnpm audit`, Dependabot/Renovate, pinned lockfiles |
| A07 Identification and authentication failures | Rate-limited login, credential-stuffing protection (IP + account buckets), optional TOTP, refresh-token rotation with reuse detection (revoke family), email verification, secure password reset (single-use, 30 min) |
| A08 Software and data integrity failures | Signed builds and catalogue; Stripe webhooks verified by signature + idempotency on event id; CI with protected environments and required reviews for release workflows |
| A09 Logging and monitoring failures | Structured logs (PII-scrubbed), auth and admin audit events, alerts on anomalies (refund spikes, login failures, relay floods) |
| A10 SSRF | The API fetches only fixed hosts (Mojang session server, Stripe, DoH resolvers); DNS verification uses DoH to fixed resolvers, never user-supplied URLs |

Other: GDPR export/delete endpoints require recent re-authentication; account enumeration avoided in sign-up/reset
responses; uploads (cosmetics) only by creators, validated by the pipeline in a sandboxed worker.

## 4. Relay

- Public TCP 25565 is unauthenticated by nature: per-source-IP connection rate and concurrency limits, per-route
  limits, handshake read timeout (5 s) and max size, only verified hostnames routed, unknown hostnames answered cheaply.
- L3/L4 DDoS: hosted on a provider with game-grade DDoS mitigation; anycast later; separate IPs from the API.
- Tunnels from hosts: outbound TLS from `rusticmc-tunnel`, authenticated with a per-server route token issued by the
  API (short-lived, rotating); a host can only receive traffic for its own routes.
- Real client IP forwarded to the server (PROXY protocol v2 / forwarding) so bans work; the relay never terminates the
  game protocol encryption (online-mode encryption stays end-to-end between client and server).
- Abuse handling: hostnames can be suspended by moderation; relay logs kept 7 days.

## 5. Secrets management

- Production secrets in the cloud provider's secret manager (or SOPS-encrypted files with age keys held by two people);
  never in the repo; `.env.example` only.
- Signing keys: Ed25519 build key offline/hardware-backed, used through a protected CI environment with required
  approval; Authenticode via a cloud HSM-backed certificate (EV/OV) or Azure Trusted Signing; Apple Developer ID in the
  CI keychain of a protected job.
- Rotation runbooks for JWT keys, Stripe keys, webhook secrets, relay route-token keys.

## 6. Incident response

Security contact (`security@<domain>`, `SECURITY.md` at repo root later), triage within 72 h, coordinated disclosure,
revocation paths: revoke all Rustic sessions, rotate build key (launcher ships two trusted keys to allow rotation),
yank a Rustic edition build (signed revocation list checked at start).

## Remote hosting (owner: hosting, 2026-11-10)

Implementation of §2.8 in `launcher/crates/rusticmc-hosting` (HOSTING.md §3, ADRs 0120-0124). Deviations from §2.8
are marked **(deviation)**.

- **Credentials**: launcher-generated Ed25519 key per VPS, private key in the OS keychain
  (`gg.rustic.launcher.hosting`, `hosting/<profile>/private-key`), chunked under Windows' 2560-byte limit. A password
  is used once (`remote::bootstrap_key`) to append the public key to `authorized_keys`, then dropped; the key alone
  is checked to log in before the profile is saved. Existing key files are read at connect time, never copied; only
  their passphrase is stored (keychain). `Secret` redacts `Debug`; profiles and the registry hold no secret.
- **Host keys**: TOFU with explicit SHA256 confirmation; the handshake stops before authentication when the key is not
  pinned (no password or signature reaches an unknown host); mismatch is a hard error; re-pin only after an explicit
  "forget". Host certificates are refused (no CA support yet).
- **Least privilege**: dedicated `rustic` system account (nologin) owns `/var/lib/rustic` **(deviation: not
  `/opt/rustic`, FHS variable state; ADR 0122)**; the unit runs as it with `NoNewPrivileges`, `PrivateTmp`,
  `ProtectSystem=strict`, `ProtectHome`, `ReadWritePaths=/var/lib/rustic`. Root (sudo `-n`, never a sudo password)
  is used for: account creation, the unit file, the firewall rule, moving staged uploads in with
  `install -o rustic`. Every root command is a `ProgressEvent` detail the UI can show. Without sudo the server runs
  as the login user under `~/rustic` (Horizon's user unit).
- **Commands**: one string per `exec`, built only with `shell::Cmd` / `shell::quote` (POSIX single quotes, NUL refused,
  tested by round trips through `sh`); names (`[a-z][a-z0-9-]{0,31}`), plugin ids (`[a-z0-9_-]{1,64}`), file names,
  config keys and one-line values are validated first. Backup names must be in the supervisor's own list.
- **Management surface**: no new port on the VPS. The control API stays on `127.0.0.1` with its token in
  `.horizon/api.json` (0600, read as the service account over SSH); reached through an SSH `direct-tcpip` channel
  (no local listener either), with `Host` = the API's address as Horizon requires. Web panel turned off
  (`panel.bind = ""`), RCON and the management protocol left off. The firewall opens only the game port, only with
  the user's consent; the provider's own security group is the user's job (shown as a warning).
- **Supply chain**: Horizon is installed by Horizon's own `install.sh` (release Ed25519 signature with OpenSSL 3,
  else GitHub SHA-256); the script's SHA-256 is shown and can be pinned (TODO_HUMAN H2). The `rustic-server` engine
  is checked against the expected SHA-256 on the VPS before it is moved into place; content-addressed paths keep the
  previous engine for rollback.
- **Game files**: only the player's own prepared files go to the VPS (`import-resources`); nothing is downloaded
  from third parties.
- **Residual risks**: a compromised VPS can lie to the launcher (status, logs) but cannot reach the user's PC: the
  launcher never runs anything the VPS sends, and console text is rendered as text. A user with passwordless sudo
  gives the launcher (and anyone with their SSH key) root on the VPS; the key is per-VPS and revocable by removing
  its `authorized_keys` line.

## Marketplace uploads (ADR 0155)

Creator uploads are untrusted input that ends up in players' games. The API never trusts the client:
it streams the bytes to a private staging folder (size counted while streaming), reads the archive
entry by entry counting the real unpacked sizes (decompression bombs, entry counts), refuses unsafe
paths (traversal, absolute, `\`, `:`, links, devices, encryption), refuses programs in every kind
(extensions and executable magic bytes; native mods carry Rust source only), parses each kind's
manifest as Horizon does, sniffs and re-encodes images (metadata and appended data dropped), and runs
the fair-play policy. Native mods (code compiled on the player's PC) are private until an admin has
read and approved each version. Private, pending and taken-down files never sit in the CDN folder;
the API streams them to their owner or an admin (bearer token or a 10-minute HMAC-signed link derived
from `DATA_ENCRYPTION_KEY`). Not done server side: compiling native mods, the full content reader and
the naga shader validation (the game runs them before loading content).
