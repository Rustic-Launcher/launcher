# Hosting: this PC, your VPS, Rustic Cloud

Owner: hosting (`launcher/crates/rusticmc-hosting`, ADRs 0120-0125). Upstream facts come from Horizon at a84f637d:
`horizon/docs/hosting.md`, `docs/api/server.md`, `docs/api/server-reference.md`, `docs/play-with-friends.md`,
`packaging/server/install.sh`, `crates/horizon-server/src/{service,import_resources,engine,api,panel,cli}.rs`.
Nothing here relies on an API Horizon does not have.

## 1. The three modes

One trait, `HostingProvider` (`src/provider.rs`): install, start/stop/restart, status and metrics (RAM, CPU, TPS,
MSPT, players), live console + commands, logs, players and moderation, `server.toml` read/write, `.rplugin`
plugins, datapacks, backups (create/list/restore/download), engine update, uninstall.

| | **This PC** (`LocalProvider`) | **Your VPS** (`RemoteSshProvider`) | **Rustic Cloud** (`RusticCloudProvider`) |
|---|---|---|---|
| Status | adapter: forwards to the desktop's `rusticmc-servers` backend | **real**, tested | stub: typed `NotAvailableYet`, UI shows "coming soon" + waitlist |
| Where it runs | the player's PC (must stay on) | the user's Linux VPS (x86_64; aarch64 with a Horizon archive) | Rustic machines |
| How friends join | LAN address, temporary relay hostname, invite code (§6) | the VPS's public IP / a domain | Rustic hostname / domain |
| Cost | free | the user's VPS bill | paid plan (later) |
| Uptime | while the PC is on | 24/7, restarts at boot (systemd) | 24/7 |
| Game files | prepared locally by the launcher | uploaded from the user's own prepared files (§4) | uploaded the same way |
| Management channel | local `horizon-server` | SSH only (CLI over exec, control API over an SSH forward) | Rustic API |
| Who can join | any 26.3 client | any 26.3 client | any 26.3 client |

## 2. VPS requirements

- **Linux x86_64** (tested: Ubuntu 24.04 in Docker; Debian/Ubuntu recommended, others get a warning).
  **Linux aarch64**: Horizon publishes no ARM Linux release (`install.sh` dies with "no Horizon release for Linux
  aarch64 yet: build it from source"). The launcher accepts a **Horizon server archive** the user built
  (`RemoteInstallOptions.horizon_archive`, passed to `install.sh --archive`); without one, the install stops with
  `unsupported_platform` before touching anything. macOS/BSD/other architectures are refused.
- **RAM**: 1 GB minimum (blocking), 2 GB recommended (below that `engine.low_memory = true` is set). Horizon measures
  0.7 GB RSS with 1 player, 1.1 GB with 50.
- **Disk**: 2 GB free in the install tree (release ~100 MB, prepared game files ~180 MB, world and backups).
- **Tools**: `curl`, `tar`, `sha256sum` (blocking: Horizon's installer needs them); `openssl` 3 recommended (the
  installer then verifies Horizon's Ed25519 release signature, otherwise only GitHub's SHA-256).
- **Access**: SSH with a key (the launcher generates one) and either root, passwordless `sudo`, or neither (then a
  per-user install, see §3).
- **Port**: the game port (TCP, default 25565) must be free; the provider's own firewall/security group must allow
  it (the launcher can only open ufw/firewalld).

## 3. Security model (summary; SECURITY.md "Remote hosting" has the details)

- **Credentials**: the launcher generates an **Ed25519 key** per VPS. The password, if the user gives one, is used
  **once** to append the public key to `~/.ssh/authorized_keys`, then forgotten (`remote::bootstrap_key`). Private
  keys (generated or pasted) and passphrases live **only in the OS keychain** (`keyring`; Windows Credential Manager
  entries are chunked because one entry holds 2560 bytes). An existing key file (`~/.ssh/id_ed25519`) is read at
  connect time and never copied. Profiles (`registry.json`) hold no secret.
- **Host keys**: trust on first use with an explicit confirmation of the **SHA256 fingerprint**; nothing (no
  password, no signature) is sent to an unknown host. Pinned in `<data>/hosting/known_hosts.json`. A different key
  is a hard `host_key_mismatch` error; only an explicit "forget" (after the user checked the VPS was reinstalled)
  clears it.
- **Least privilege**: with root or sudo, a dedicated system account `rustic` (no login shell, home
  `/var/lib/rustic`, mode 0750) owns everything and runs the server under a hardened system unit
  (`NoNewPrivileges`, `PrivateTmp`, `ProtectSystem=strict`, `ProtectHome`, `ReadWritePaths=/var/lib/rustic`). Root
  is used only for: creating the account, writing the unit, the firewall rule, and moving uploaded files into the
  account's tree. Without sudo, everything goes under `~/rustic` as the login user.
- **No new listening port**: the control API stays on `127.0.0.1` (Horizon's default), reached through an SSH
  `direct-tcpip` channel (a local forward without a local listener). The web panel is turned off
  (`panel.bind = ""`); RCON and the management protocol stay off (Horizon defaults). The firewall opens only the
  game port, and only with the user's consent.
- **Commands**: every remote command is built with `shell::Cmd`/`shell::quote` (POSIX single quotes, NUL refused,
  round-trip tested through a real `sh`); server names, plugin ids, file names, config keys are validated before
  they get there.
- **Integrity**: Horizon's installer checks its release (signature or SHA-256); the `rustic-server` engine is checked
  against the expected SHA-256 **on the VPS** after upload or download, before it is moved into place.

## 4. What the installer does (`remote::steps`)

Each step checks the VPS to know if it is already done, so re-running `install` after a failure, a lost connection
or a closed launcher **resumes**. If a step fails, the steps applied **in this run** are undone in reverse order
(steps found already done belong to an earlier run and are kept). Every transition is a `ProgressEvent`.

| # | Step id | Done when | Does | Undo |
|---|---|---|---|---|
| 0 | `connect` | - | SSH, host key check, read-only probe (`uname -sm`, `/etc/os-release`, `id`, `sudo -n true`, tools, `/proc/meminfo`, `df`, `ss -ltn`) | - |
| 1 | `requirements` | never skipped | OS/arch, RAM, disk, port free, tools; warnings shown | - |
| 2 | `service-account` | `id -u rustic` | `useradd --system ... --shell /usr/sbin/nologin` (BusyBox: `adduser -S`) | `userdel`, remove home |
| 3 | `folders` | dirs exist | `servers/ engines/ staging/ bin/`, private upload dir `~/.cache/rustic-hosting/<server>` | - |
| 4 | `install-horizon` | `horizon/current/horizon-server` runs | downloads Horizon's `install.sh` (docs/hosting.md §1, URL configurable, SHA-256 shown/pinnable), runs it with `--dir <base>/horizon --bin <base>/bin [--version V | --archive F]` as the service account | remove the install |
| 5 | `install-engine` (Rustic only) | engine present with the right SHA-256 | upload `rustic-server` by SFTP **or** `curl` it from a release URL on the VPS; check SHA-256 on the VPS; `engines/<sha16>/rustic-server` | remove it |
| 6 | `init-server` | `server.toml` exists | `horizon-server --dir D init --eula --world W --port P --whitelist ... --yes` (only after the user accepted the EULA) | remove the folder (or only `server.toml` if the folder existed) |
| 7 | `configure` | always (idempotent) | `config set engine.path=... panel.bind= [engine.low_memory=true] <user settings> --json` | - |
| 8 | `import-resources` | remote manifest SHA-256 = local | pack the prepared folder into one `.tar` (uncompressed), SFTP, unpack as the service account, `horizon-server import-resources` into `D/resources` | remove `D/resources` |
| 9 | `service` | unit enabled | root/sudo: our system unit `rustic-<server>` (same `ExecStart`/stop timeout as Horizon's); no sudo: Horizon's own `service install` + `loginctl enable-linger`; no systemd: warning (background only) | disable + remove |
| 10 | `firewall` | rule present / firewall inactive | `ufw allow P/tcp` or `firewall-cmd --permanent --add-port`, only with consent | delete the rule |
| 11 | `start` | phase `running` | `systemctl start` (or `horizon-server start`); skipped with a warning if there are no game files yet | stop |
| 12 | `health-check` | never skipped | waits for `status --json` phase `running`, then a **status ping (server list ping) from this computer** to `host:port`; unreachable = warning about the provider's firewall | - |

Layout (dedicated account; `~/rustic` instead of `/var/lib/rustic` without sudo):

```text
/var/lib/rustic/horizon/          install.sh --dir (releases/, current -> releases/<tag>, game-files/)
/var/lib/rustic/bin/horizon-server install.sh launcher script (always runs current/)
/var/lib/rustic/engines/<sha16>/rustic-server
/var/lib/rustic/servers/<name>/   server.toml, world, plugins/, backups/, resources/
/var/lib/rustic/staging/          hand-over from the login user
~/.cache/rustic-hosting/<name>/   SFTP uploads (login user, 0700)
```

### How the game files reach a VPS (Horizon constraint)

A server reads Minecraft's data from files **prepared from the player's own game** (`horizon-prepare`, needs Java
25, the game installed once and its `server.jar`). Horizon's hosting doc says: on a headless machine, *prepare on your
own computer and copy the prepared `26.3` folder with `horizon-server import-resources DIR`* (it checks
`horizon-resources.json`, the version stamp, the derived files' hashes, and swaps atomically). The launcher does
exactly that: it uses the folder it prepared locally (`ServerSpec.prepared_resources`), uploads it (as one archive),
and runs `import-resources`. Nothing is ever downloaded from a third party, and the launcher never runs
`horizon-prepare` on the VPS (no Java, no game there). The folder must come from Horizon v2026.10.17.07 or newer
(older ones lack `registries.json`/`commands.json`). A server installed without game files is set up but not started;
`RemoteSshProvider::import_resources` adds them later.

`D/resources` is created before the import so the files land in the server folder: the engine looks there first
whatever `engine.path` is (`engine::resource_candidates`); the installer's shared `game-files/` would not be found
when `engine.path` points at `rustic-server` outside the release.

## 5. Managing a VPS server

- **CLI over SSH** (as the service account): `status --json`, `cmd`, `logs -n`, `players --json`,
  `kick|ban|pardon|ban-ip|pardon-ip|op|deop|whitelist ...`, `config --json`, `config set ... --json`,
  `backup [NAME]`, `backups --json`, `restore NAME --yes [--dimension ID]`, `update [--version TAG]`, `restart`.
  Start/stop go through systemd when the unit exists (`systemctl start|stop rustic-<name>`), else
  `horizon-server start|stop`. `restart` restarts the game only (the supervisor and its unit stay up).
- **Control API over an SSH forward**: `cat .horizon/api.json` as the service account (mode 0600), then
  `direct-tcpip` to its `127.0.0.1:<port>`, HTTP/1.1 with `Authorization: Bearer` and **`Host` = the API's own
  address** (Horizon refuses other hosts: `panel::host_allowed`). Used for the live console
  (`GET /api/v1/events?types=log,phase,join,leave,kick,chat,death`, server-sent events) and backup downloads
  (`GET /api/v1/backups/download?name=`; a stopped server's backup is streamed with `cat` instead). Backup names are
  checked against `backups --json` first.
- **Plugins**: `.rplugin` validated locally (zip, `rustic.plugin.toml` at the root, `[plugin] id` matching
  `[a-z0-9_-]{1,64}`, version), installed as `plugins/<id>.rplugin`. Disable = rename to `<id>.rplugin.disabled`
  (Rustic's discovery only loads `*.rplugin` and directories with a manifest, docs/PLUGIN_FORMAT.md §3); applies at
  the next start (or `/rustic reload|enable <id>` through the console).
- **Datapacks**: `.zip` into `<world>/datapacks/` (the world name read from `config --json`).
- **Update**: Horizon with `horizon-server update` (its own backup + automatic rollback); the Rustic engine by
  installing the new content-addressed binary, `config set engine.path=...`, restart. The previous engine stays on
  disk for a manual rollback.
- **Uninstall**: stop, remove the unit, delete the firewall rule, optionally delete the server folder, optionally
  remove Horizon and the `rustic` account when no other server is left. Each step idempotent.

## 6. Local servers: how friends join (ADR 0125, `src/access.rs`)

| Mode | What friends type | Who can join | Implemented by |
|---|---|---|---|
| **LAN** | `192.168.1.20:25565` (addresses listed, container/VM bridges and loopback hidden, tailnets marked VPN) | any client on the same network / tailnet | `access::local_addresses` (here) |
| **Temporary relay hostname** | `<words>.<temp domain>` | any client | `RelayTunnel` trait, implemented over `rusticmc-tunnel` (relay) |
| **Invite code** | a code in the launcher | see below | `InviteSharer` trait; Rustic's code resolves via the cloud to the relay name (ADR 0105) |

**Can Horizon's Play with Friends serve a dedicated server?** Studied in `horizon-friends` / `horizon-relay`:
the host side (`horizon_friends::Host::start(HostConfig { lan, .. })`) forwards *any* loopback TCP port behind an
end-to-end encrypted invite code (SPAKE2 + ChaCha20-Poly1305, direct with UPnP/PCP/NAT-PMP + STUN, optional
`horizon-relay` fallback), so technically yes. Limits: only clients with the `friend:` resolver
(`horizon-official` / the Rustic edition) can join, never stock clients; at most 8 sessions (`MAX_SESSIONS`); every
guest arrives from `127.0.0.1` (IP bans and per-IP limits stop working); codes change each time sharing restarts.
Decision: the default invite code is Rustic's own (cloud code → relay hostname, any client); a
`horizon-friends`-backed `InviteSharer` is an optional later path for Rustic-edition friends (HANDOFF).

## 7. Rustic Cloud (later)

`cloud.rs` documents the API the cloud owner will serve; every call answers `NotAvailableYet` today:
`GET /v1/hosting/plans` → `CloudPlan {id, name, ramMb, vcpus, diskGb, maxPlayers, priceCents (VAT incl.), currency}`;
`GET /v1/hosting/regions` → `CloudRegion {id, name, country, available}`; `POST /v1/hosting/servers`
(`AllocationRequest {plan, region, spec, immediateExecutionConsent}`) → `Allocation {id, state:
queued|provisioning|running|suspended|deleting, address}`; `POST /v1/hosting/waitlist`. Management afterwards reuses
the same `HostingProvider` calls.

## 8. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `host_key_unknown` | First connection: compare the fingerprint with the provider's console (`ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub`), then trust it. |
| `host_key_mismatch` | The VPS presents another key. If you did not reinstall it, **stop**: someone may be intercepting. If you did, forget the old key and trust the new one. |
| `auth_failed` "the server refused the credentials" | Key not in `~/.ssh/authorized_keys`, or `PasswordAuthentication no` when bootstrapping with a password: add the shown public line by hand in the provider's web console. |
| `unsupported_platform` aarch64 | Build `horizon-server` + `horizon-client` for ARM (docs/hosting.md), pack them as `.tar.gz`, choose it as the Horizon archive. |
| `requirements`: port in use | Another server listens on it: pick another port. |
| `install-horizon` fails | GitHub unreachable from the VPS, or the release signature/SHA-256 did not match (the installer refuses: report it to Horizon). |
| Server installed but not started | No prepared game files yet: prepare them in the launcher, then "Upload game files". |
| `registries.json not found next to the resources` | The prepared folder is from a Horizon release older than v2026.10.17.07: prepare again. |
| Health check: "can't reach host:port" | The game runs; your provider's firewall/security group blocks the port. Open TCP `<port>` there. |
| Server stops when you log out (no-sudo install) | `loginctl enable-linger` was refused: ask the provider, or reinstall with sudo. |
| No systemd (containers) | The server runs in the background but does not come back after a reboot. |
| Console: "the server is not running" | The console needs the supervisor's API; start the server first. |

## 9. Tests

- Unit (`cargo test -p rusticmc-hosting --lib`): shell quoting (incl. round trips through a real `sh`), probe /
  `os-release` / `uname` parsing, requirements, layout and privilege wrapping, systemd unit rendering and names,
  planner idempotency and reverse rollback (incl. rollback errors), host key TOFU/pinning/persistence, HTTP and SSE
  parsing, status ping (VarInt, fake server), `.rplugin` manifests, model parsing of Horizon's JSON, access modes,
  registry, keychain chunking (real keychain opt-in: `RUSTIC_KEYCHAIN_TEST=1`).
- Integration (`tests/ssh_inprocess.rs`): an in-process russh server on 127.0.0.1 with a temp folder served over
  SFTP and a fake executor recording commands: TOFU/mismatch/password failure on a real handshake, key bootstrap,
  the full install plan (order, uploads landing, unit content), idempotent re-run, rollback on failure, management
  calls, console and backup download through `direct-tcpip` to a fake control API, uninstall.
- End to end (`tests/docker_e2e.rs`, ignored): `RUSTIC_SSH_E2E=1 [RUSTIC_E2E_RESOURCES=<prepared 26.3>] cargo test
  -p rusticmc-hosting --test docker_e2e -- --ignored --nocapture` builds an Ubuntu 24.04 + OpenSSH image and runs the
  real installer against it.

## 10. Wiring in the app (IPC)

`docs/IPC.md` is generated from the TypeScript (`gen:ipc-doc`), so the mapping lives here. The commands are
implemented in `apps/launcher/src-tauri/src/cmd/hosting.rs` and `cmd/servers.rs` on top of this crate:

| IPC command / event | Implementation |
|---|---|
| `hosting_providers` | static list (local, vps available; rustic_cloud `coming_soon`) |
| `hosting_waitlist_join` | `unavailable` until the cloud waitlist endpoint exists |
| `hosting_vps_test` | `SshSession::connect` with the given credentials: unknown host key → `{known: false, fingerprint}` (nothing authenticated), pinned → `auth_ok`; a mismatch rejects with `conflict` / `host_key_mismatch` |
| `hosting_vps_trust` | checks the presented key equals the confirmed `fingerprint_sha256`, pins it; password → `remote::bootstrap_key` (generated key installed, password dropped); generated key (`key_path = "keychain:<id>"`) → moved to the profile's keychain entry, `AuthMethod::KeychainKey`; key file → `AuthMethod::KeyFile` + passphrase in the keychain; saved with `Registry` |
| `hosting_vps_generate_key` | `ssh::generate_key`; private key **in the keychain** (pending entry), the UI gets `key_path = "keychain:pending-<fp>"`, the public line and `authorize_command` (`ssh::authorize_key_script`) |
| `hosting_vps_list` / `_remove` / `_check` | `Registry` + pins / registry + keychain cleanup / `test_connection` + thresholds |
| `hosting_vps_install` `{connection_id, server, open_firewall?}` | `RemoteSshProvider::install_with(spec, RemoteInstallOptions { open_firewall, .. })` in a background task; `hosting-install` events built with `model::ui_step` (warnings keep the step state and show their text) |
| `server_*` (start/stop/restart, command, console, players, config, plugins, datapacks, backups, stats, delete) | the server's provider (`LocalProvider` backend or the cached `RemoteSshProvider`) through `HostingProvider` |

The wizard asks for the firewall consent ("Open the game port in the VPS firewall", off by default). The prepared
game files passed to a VPS install are the launcher's own prepared folder for the default Horizon build, only when
it holds `horizon-resources.json` (otherwise the server is installed and not started).
