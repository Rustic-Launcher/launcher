# Horizon facts for Rustic Launcher

Owner: spec. Verified on 2026-11-09/10 against the read-only checkout
`<server checkout>\horizon` at commit **a84f637dd** (merge of PR #14282
`split/cosmetics-fx-api`). Paths below are relative to that `horizon/` folder unless they start with
`launcher/` **and** say "Rustic". When Horizon moves, re-verify before relying on a line number.

Horizon is MIT ("Copyright (c) 2026 Ludovic Marie and the Horizon contributors", `LICENSE:1-3`,
`Cargo.toml:59`). README ends with "NOT AN OFFICIAL MINECRAFT PRODUCT…" (`README.md:171`).

## 0. Mission assumptions checked

| # | The mission assumed | Reality at a84f637d | Evidence |
|---|---|---|---|
| A1 | Horizon mods are WASM components (`horizon:mods` WIT) | **False.** Code mods are **native**: horizon-mixins bundles (`.hnmod`) and **trait mods** (`horizon_client::mod_api::Hooks`) compiled into a game executable. The WASM runtime is gone from main and lives in the unofficial fork `arismoko/horizon`. `.hmod` files carrying `mod.wasm` are refused ("WebAssembly mods don't run any more, port the code to a native mod"). No `wasmtime` in `Cargo.lock`, no `.wit` file in the repo. | `docs/mods.md:3-8`; `docs/native-mods.md:13,19,743-749`; `crates/horizon-mods/src/index/native.rs:9`; `CONTRIBUTING.md:16-18`; `GOVERNANCE.md:313`; `docs/design/mod-hotload.md:155,168-170` |
| A2 | Shader packs were temporarily removed | **False.** Shader packs are supported: Iris/OptiFine-format `shaders/` packs and Horizon's own WGSL packs (`.hshader`, shader API 0.1 "experimental"), under Video Settings › Shader Packs; the launcher installs Horizon-registry shader packs into `shaderpacks/`. Modrinth shader packs are shown as not supported by the launcher. | `README.md:33,84-89`; `docs/shaders.md:1-17`; `launcher/src/browser.rs:36-38,283-284,325`; `launcher/src/packs.rs:7-10` |
| A3 | A mod can draw layers on players, HUD/overlays and do networking | **True, with two different mechanisms** (section 3): catalog hooks for native mods (`mob_layer`, `player_submit`, `render_name_tag`, `hud`, `gui_layer_*`, `net_*`) and the engine-level `horizon_hooks` registries (`player_layers`, `name_decoration`, `hud::register`) that only crates compiled into the executable can use. | `native-hooks.json`; `crates/horizon-hooks/src/{lib,players,hud}.rs` |
| A4 | Microsoft login = device code **+ PKCE** | **Mixed up.** The device-code grant has no PKCE. Horizon uses the device-code flow only (public client, `consumers` tenant, scopes `XboxLive.signin offline_access`). PKCE belongs to the authorization-code flow with a loopback redirect, which Rustic may add (ADR 0010). | `docs/microsoft-account.md:15-26,48-61`; `launcher/src/account.rs:25,61-73,168-217` |
| A5 | The session is passed by stdin | **Both exist.** Launch API: `--session-stdin` (JSON written to stdin, then closed) or `--session-file PATH` (read once and deleted, 0600, refused after 2 min). The official launcher uses `--session-file`. Never on the command line. | `docs/api/launch.md` ("The session JSON"); `launcher/src/account.rs:2008-2052`; `crates/horizon-client-render/src/auth.rs:15,90-106` |
| A6 | Relay routing "by handshake hostname" is how Horizon's Play with Friends works | **False.** Play with Friends is peer-to-peer by **invite code** (port mapping + STUN, SPAKE2 + ChaCha20-Poly1305); the optional `horizon-relay` routes by the code's **nameplate**, never by hostname, and Horizon runs no relay by default. Hostname routing is a Rustic design of its own. | `docs/play-with-friends.md:6-14,36-71,90-98,111-136`; `crates/horizon-relay/src/lib.rs:1-6` |
| A7 | Horizon has a friends service | **False.** `horizon-friends` is the invite-code tunnel only (no friends list, no presence). The only "presence" is the cosmetics service's `POST /v1/presence`. | `crates/horizon-friends/src/lib.rs:1-22`; `docs/play-with-friends.md:111-112` |
| A8 | A dedicated server has a plugins/mods folder | **False.** "Mods on a dedicated server" is "Not there yet"; native mods reach a server only through a standalone engine export (`native-build --server-output`) pointed at by `engine.path`. Rustic Server fills this gap with its own runtime (`rustic-server` engine). | `docs/api/server.md:79-81`; `docs/native-mods.md:2125-2150`; `docs/hosting.md:246-259` |
| A9 | Builds can be pinned forever | **False.** Nightlies are `nightly-<date>` GitHub pre-releases **kept for seven days** then deleted; "a signed release index and builds kept longer are not there yet". Only the Nightly is shipped since 2026-10-28. | `docs/api/launch-guide.md:27-29`; `docs/release-flow.md:523,548`; `launcher/README.md:246-256` |
| A10 | Lunar-style RAM / loader settings per instance apply | Only to **Java Edition** instances of the official launcher. Horizon instances only pin a Horizon version. | `launcher/src/instances.rs:135-185,207` |
| A11 | Datapacks are a full feature | Supported per world (`<world>/datapacks/`), list/add/remove; no enable/disable toggle in the launcher; docs disagree on which datapack features run (`docs/mod-browser.md:21` vs `launcher/src/browser.rs:353-357`). | `launcher/src/datapacks.rs:1-20,39-152`; `docs/datapack-upgrade.md:1-24` |

Each wrong assumption is also a row in [RISKS.md](RISKS.md) with impact and mitigation.

## 1. The official launcher (`horizon/launcher`)

- **Open source, MIT**, same repository, own Cargo workspace (`launcher/Cargo.toml:2-8`; root
  `Cargo.toml:2` excludes it; `launcher/README.md:6`). UI: **GPUI Fast** (fork of Zed's GPUI,
  `gpui-fast 0.1.13`, `launcher/Cargo.toml:13-16`). We may read it for behaviour and reuse MIT code with
  attribution; Rustic's UI is a separate Tauri/React app (ADR 0001).
- **Data**: launcher data `HORIZON_LAUNCHER_HOME` or `<data_dir>/horizon-launcher`; game home
  `HORIZON_HOME` or `<data_dir>/horizon` (`launcher/src/backend.rs:921-933,2652-2664`). `instances.json`,
  `settings.json` in the data folder.
- **Instances**: kinds `Default` (id `horizon`, the game home), `Own` (`<game_home>/instances/<slug>/`),
  `Port` (`ports/<pack>/`, imported Java packs), `Pack` (`horizon-packs/<id>/`) (`launcher/src/instances.rs:3-36,236,540`).
  `Meta`: name, icon, group, pinned `version` (None follows updates), play time, last played, launch
  options (pre-launch, wrapper, post-exit, env, gamemode/mangohud/dGPU) (`instances.rs:135-162`;
  `launcher/src/launch.rs:23-46`). Applying an instance sets `HORIZON_HOME=<instance>` and the working
  directory (`instances.rs:679-693`).
- **Preparation**: finds the player's own game files (Horizon's own Mojang download for an owner, or
  `.minecraft`, Prism, MultiMC, Modrinth App, ATLauncher, GDLauncher…, `launcher/src/sources.rs:1-6,114-151`),
  downloads the server jar for a signed-in owner only, then runs `horizon-prepare` in-process or as the
  version's binary (`backend.rs:2448-2589,2110-2128,2534-2555`). Output:
  `<versions>/<tag>/.local/horizon-client/<game_version>/` with `horizon-resources.json`
  (`backend.rs:1395-1403`). Java 25 is the pinned major (`backend.rs:2515-2533`).
- **Launch**: `versions/<tag>/horizon-client --resources <prepared>` + `--session-file`, env `HORIZON_HOME`,
  `HORIZON_LAUNCHER_EXE` (so the game can ask for a renewed session), removes `HORIZON_ACCOUNT_SESSIONS`
  (`backend.rs:2903-2954,3177-3213`). Renewal: `horizon-launcher account session <who>` prints a fresh
  read-once file (`launcher/src/cli.rs:121-123,1954-1962`).
- **Microsoft account**: device code, client id from `HORIZON_MSA_CLIENT_ID` (run time) or `option_env!`
  (build time), never hard-coded (`launcher/src/account.rs:41-47`); tokens in the OS keychain under
  `dev.playhorizon.launcher` (`keyring` 3 on Windows/macOS, `oo7` on Linux, encrypted-file fallback)
  (`account.rs:24,784,1023-1147,1382-1405`). **Forks and derived launchers must register their own Azure app
  and get their own Mojang approval; using Horizon's client id is forbidden** (`docs/forking.md`,
  "Microsoft sign-in in a fork").
- **Owner gate**: play only with an account that owns the game; offline names removed on 2026-11-09
  (`docs/microsoft-account.md:77-109`).
- **Content**: Horizon registry `https://horizon-rust.github.io/horizon-mods/index.json` (override
  `HORIZON_MOD_INDEX`), ratings `https://playhorizon.dev/api/mods/ratings` (`launcher/src/browser/horizon.rs:18-43`);
  Modrinth `https://api.modrinth.com/v2`; CurseForge code present but "coming soon", needs
  `HORIZON_CURSEFORGE_API_KEY` (`browser.rs:281-291`; `docs/mod-browser.md:226-235`). Downloads only from
  `cdn.modrinth.com`, `edge.forgecdn.net` and the registry's GitHub releases, hash-checked
  (`browser/download.rs:13-14`; `browser.rs:187-197`). Tabs: Mods, Modpacks, Resource packs, Shader packs,
  Data packs; compatibility labels **Works / Partly / Doesn't run** (`browser.rs:31-71,105-133,299-349`).
  Java mods never execute; their data/assets may be ported (1.21+) (`browser.rs:334-368`).
- **Registry index schema 1** (`crates/horizon-mods/src/index.rs:20-238`): kinds `mod` (.hmod),
  `native-mod` (.hnmod), `modpack` (.hpack), `resourcepack`, `shader` (.hshader), `shaderpack`, `datapack`;
  per version `api`, `side` (client/server/both), `capabilities`, `api_level`, `hooks`, `game`, `broken_on`,
  `requires`, `file{url,size,sha256,signature}`; `revoked[]`. Minisign `key` per project. Registry service
  (`crates/horizon-mod-registry`, axum, GitHub sign-in) at `playhorizon.dev/api/mods/*`.
- **Modpacks**: `.hpack` (zip + `horizon-modpack.toml`, `format=1`) and Java imports (`.mrpack`, CurseForge
  zip, Prism/MultiMC folders); export `.mrpack`/zip (`crates/horizon-mods/src/modpack.rs:1-40`;
  `launcher/src/modpacks.rs:1-8`). `docs/api/launcher.md` lists "a Horizon modpack manifest" as not there yet
  in its final form.
- **Updates and signing**: game and launcher from GitHub releases of `horizon-rust/horizon`;
  `SHA256SUMS` + Ed25519 `SHA256SUMS.sig` over `"horizon release <tag>\n"+sums`, key in
  `crates/horizon-release-sign/release-keys.txt` (`crates/horizon-release-sign/src/lib.rs:1-64`;
  `launcher/src/signed.rs:11-13`). Self-update tag `launcher@X.Y.Z` every 30 min (`backend.rs:23,846`).
- **Agent/CLI surface**: 46 commands, 43 MCP tools (`docs/api/launcher.md`, `docs/launcher-commands.md`).
- **Other UI**: Wardrobe + Shop (cosmetics), Servers page with Tailscale invites, "Join a friend" code field,
  a partner hosting banner (`launcher/src/partner.rs:1-13`), themes "Horizon" and "First Light".

## 2. Launch API for third-party launchers (what Rustic uses)

Contract: `docs/api/launch.md`, `docs/api/launch-guide.md`, `docs/api/launch-reference.md` (generated from
`crates/horizon-client/src/game/launch_api.rs`). Level: **experimental**, version 0.1.0.

1. **Owner only**: start Horizon only for a player whose Microsoft account owns Java Edition.
2. **Pick a build**: GitHub release archives `horizon-{windows-x64,windows-arm64,linux-x86_64,macos-arm64,macos-x64}`
   (zip/tar.gz). Verify `SHA256SUMS` + `SHA256SUMS.sig`. Keep each build whole in its own folder
   (`horizon-client`, `horizon-prepare`, `research-lock.json`, `exporters/`). macOS builds are ad-hoc signed.
3. **Ask the build**: `horizon-client --launch-api` → JSON `launch_api`, `options`, `content`; pass only listed
   options (others: exit 2). `horizon-prepare --launch-api` → pins (`minecraft`, `java_major`), `schema`,
   `output_version`.
4. **Prepare**: `horizon-prepare --prepare --output PREPARED --complete --progress json [--minecraft-dir |
   --version-json --client-jar --libraries-dir --assets-dir] [--server-jar JAR] [--java-home DIR]`; progress
   lines `{"message"}`, `{"fraction"}`, `{"error"}`; `--check --output PREPARED` exit 0 when current. Horizon
   downloads nothing; the launcher downloads Mojang files and the server jar only for a signed-in owner.
5. **Start**: `horizon-client --resources PREPARED --game-dir INSTANCE --home INSTANCE --width W --height H
   --session-stdin` and write `{"name","uuid","access_token","user_type":"msa","xuid"}` to stdin, then close it.
6. **Exit codes**: 0 ok; 2 refused option (stderr names it); 1 or 101 crash, last stderr line names the crash
   report.
7. **Not there yet**: finding a build through a signed release index; builds older than 7 days vanish.

Session renewal while playing goes through `HORIZON_LAUNCHER_EXE … account session` in the official
launcher; Rustic must either implement a compatible renewal entry point or accept that long sessions need a
restart to rejoin online servers (RISKS R-12).

## 3. Mods: what can draw, what can talk

### 3.1 Content and native mods

- Content mod `.hmod`: data + assets, no code, `horizon-mod.toml` (`id, name, version, api="0.2", side,
  capabilities, namespaces`), installed in `<instance>/horizon-mods/`; on/off in `horizon-mods.txt`
  (`docs/mods.md:23-29,90-117`). `side="both"` mods are checked against the server at join over
  `horizon:mod_list`; client mods never block a join.
- Native mod `.hnmod`: horizon-mixins bundle (source patches) compiled into a **new game executable on the
  player's machine** by the launcher: downloads Rust 1.99.1 + MinGW/LLVM-MinGW, fetches the pinned commit's
  source, `horizon-mixins apply`, `cargo build -p horizon-client`, installs version `native-<commit>-<key>`
  (`docs/native-mods.md:1840-1960`). First build "20 to 30 minutes" warning, measured 1.5-4.5 min, rebuild ~1 min.
  Windows Smart App Control blocks it (os error 4551). Not sandboxed: "native code with the player's rights".
- **Trait mod**: `cargo horizon-mod new <path> --template native --horizon <commit>`; the `[api]` package
  exports `pub fn hooks() -> Box<dyn horizon_client::mod_api::Hooks>`; the launcher generates a `main` that
  builds `vec![mod_0::hooks(), …]`, calls `horizon_official::register()` then
  `horizon_client::entry::main_with(mods)` (`docs/native-mods.md:5069-5102`;
  `launcher/src/native_mods/api.rs:140-172`; `crates/cargo-horizon-mod/template/native/lib.rs:1-18`;
  `crates/horizon-client/src/entry.rs:10,29-30`).
- Stability: native mods are **experimental**, pinned to Horizon + horizon-mixins revisions; hook points are
  only ever added (`docs/api/stability.md:20,71,91`).
- Official mods are all native (FPS counter, Keystrokes, Zoom, Minimap…) (`docs/mods.md:663-673`).

### 3.2 Catalog hooks (`native-hooks.json`, 434 hooks, 177 helpers)

62 hooks are `side: client`, 10 `both`, 270 `server`, 92 unspecified. Relevant to a companion mod
(signatures in `crates/horizon-client/src/game/native_hooks.rs`):

- **Player model**: `mob_layer` (client, after vanilla layers, `(mob, who: Option<&NativePlayerRender>, draw)`,
  `:3656`), `player_submit` (client, cancellable), `player_render_listened` (must be set or players are not
  posed for the mod), `render_hand`, `render_hand_pose`.
- **Name tag**: `render_name_tag` (client) can change visibility and text only (`:2901`), no icon/badge.
- **HUD/UI**: `hud` (`HudBatch`, `Corners`), `gui_layer_pre/post`, `gui_render_post`, `screen_*`,
  `screen_render_post`, `menu_button`, `settings`, `tooltip*`.
- **Input**: `key`, `input_key`, `mouse_*`; mod key bindings via helpers `mod_key`/`set_mod_key`
  (stored as `key.<mod>.<name>` in `horizon-mods.txt`); no key-mapping registration hook.
- **Chat**: `client_chat_received` (cancellable), helper `send_chat`.
- **Networking**: channel `horizon:mod_net` with `net_send_to_server` / `net_send_to_player` helpers and
  `net_from_server` / `net_from_client` hooks; channel names `mod_id:name`, ≤ 30 000 bytes, ≤ 32 msgs/tick,
  invisible to vanilla peers (`crates/horizon-protocol/src/mod_net.rs:15`; `docs/native-mods.md:4548-4561`).
  Raw plugin channels: `native_send_packet` (CustomPayload ≤ 32767 bytes) and `client_packet` (read-only view of
  every packet).
- **Lifecycle**: `client_start`, `client_stopping`, `client_disconnecting`, `client_level_load/unload`.

### 3.3 Engine-level hooks (`crates/horizon-hooks`, re-exported by `horizon-platform::hooks`)

For crates compiled into the executable (cosmetics, `horizon-official`), registered before the game starts
(`horizon_hooks::register(&'static dyn Hooks) -> Result<(), TooLate>`, sealed in `entry::main_with`;
`crates/horizon-hooks/src/lib.rs:191`, `registry.rs:23-40`; `docs/layers.md:255-285`):

- `player_layers(&self, model: &PlayerModel, draw: &mut dyn PlayerDraw)`: draw meshes on player bones; all hooks
  are called (`lib.rs:78,310-314`). `PlayerDraw` (since PR #14282): `pass`, `ticks`, `base`, `motion`, `view`,
  `bone`, `body_pose`, `sprite`, `draw(mesh, matrix, look, pose)`, `particles(uv, &[Particle], Light)`
  (`players.rs:431-456`). `Mesh {name, pose, quads, children}` (`players.rs:219-226`).
- `name_decoration(&self, model, sprites) -> Option<NameDecoration{mark, badge, top}>`: first answer wins
  (`lib.rs:58-64,301`; `players.rs:138-162`). Horizon's own cosmetics answer it when enabled, so a Rustic badge
  and a Horizon name-tag style would compete (RISKS R-09).
- `first_person`, `frame`, `tick`, `player_mark`, `camera`, `tunnel`, `started`, `published`, `activity`.
- `hud::register(&'static dyn hud::Layer)` with a `Canvas` (`text`, `fill`, `place(corner,w,h)`) (`hud.rs:18-50`);
  `pages::register` (settings pages), `settings::register` (key:value store) (`pages.rs:176`, `settings.rs:35`).

**Consequence for Rustic**: a companion that draws Rustic cosmetics on players and a name badge needs the
engine-level hooks, i.e. **it must be compiled into the executable's `main`**, not merely dropped in as a
`.hnmod`. That is what the "Rustic edition" client does (ADR 0006).

## 4. Cosmetics (Horizon's own)

- Format: one JSON per item, `format: 1`: `id`, `slot` (`head, back, torso, shoulder, name_tag, emote,
  companion, trail, aura`), `bone`, `attach`, base64 `texture.png`, `parts` (box tree, box UV like Blockbench
  "Modded Entity"), `animations` (swing, bob, spin); limits 512 vertices, 64×64 texture, 16 parts, 24 px reach;
  unknown fields refused; FX blocks for companion/trail/aura; emote keyframes (`docs/cosmetics.md:20-170`;
  `crates/horizon-cosmetics/src/item.rs:26-45`, `fx.rs:1-31`).
- Rules: only Horizon clients see them; no gameplay effect; **no capes or anything cape-like; no random
  draws**; shop without countdowns, scarcity, buyer counts, "limited" wording or loot boxes; no badge or pop-up
  points to the shop (`docs/cosmetics.md:10-18,450-453`).
- Service: `https://cosmetics.playhorizon.dev` (override `HORIZON_COSMETICS_API`), run by the Horizon project;
  **its server code is not in the repository**. Fetch others: `POST /v1/worn` (≤ 64 UUIDs, no sign-in), items
  `GET /v1/items/<id>?hash=`; auth by Mojang `join`/`hasJoined` (`/v1/auth/start`, `/v1/auth/finish`), the
  Minecraft token never reaches the service (`docs/cosmetics.md:230-258,390-398`; `account.rs:46-91`).
- Shop: `GET /v1/catalogue` in real currencies (CHF/GBP/EUR/USD), payment on the website through **Paddle**
  (`POST /v1/web/checkout` → `playhorizon.dev/shop/pay`), "Refunds within 14 days for any reason", free founder
  set (`docs/cosmetics.md:386-480`; `launcher/src/shop.rs:14-20`).
- Visibility default: on; others' cosmetics only shown if your account owns one; "Show me as Horizon" on by
  default (`docs/cosmetics.md:5-8`; `docs/telemetry.md:5-8`).
- In the game: `game/cosmetics.rs`, registered through `game/player_hooks.rs:21-26` when
  `game::cosmetics::enabled()` (the `horizon-extras` feature, on by default) (`docs/forking.md`).

**Implications for Rustic (reuse vs compete)** — see ADR 0007:
- Reuse the **format** (MIT crate `horizon-cosmetics`) as the base of Rustic's format so items convert both ways;
  reuse the **rules** (they match ours: no capes, no random draws, no dark patterns).
- Do **not** reuse Horizon's **service or shop**: it is Horizon's business (Paddle, their catalogue), not an open
  API for third parties; Rustic runs its own cosmetics service keyed by Minecraft UUID with the same
  join/hasJoined proof pattern.
- Rustic cosmetics are visible only to Rustic-edition players (as Horizon's are only to Horizon players).

## 5. Play with Friends, relay, friends

- Host: tunnel on TCP 25580 (`HORIZON_FRIENDS_PORT`), PCP → NAT-PMP → UPnP, public IP via STUN
  (Cloudflare, Google; `HORIZON_STUN_SERVERS`) (`docs/play-with-friends.md:9-44`).
- Invite code: Crockford base32 with version, 80-bit secret, port, endpoints, optional relay, checksum; join as
  `friend:<code>` resolved by `Hooks::tunnel` registered only by the official executable
  (`horizon-official`) (`:52-71`; `crates/horizon-friends/src/lib.rs:38-40`).
- Crypto: SPAKE2 (Ed25519) + HKDF-SHA256 → ChaCha20-Poly1305 per direction (`:90-98`).
- `horizon-relay`: optional, self-hostable, TCP 7000, JSON first line (`host/registered/join/bridged/attach/ping`),
  routes by **nameplate**, in-memory, `GET /health`, `/whoami`; no default relay (`:122-212`;
  `crates/horizon-relay/src/main.rs:7-23`).
- **Implication**: Rustic's "Local + invite code" mode can reuse `horizon-friends` as-is (it ships in the Rustic
  edition through `horizon_friends::game::HOOKS`), optionally pointing `HORIZON_RELAY_URL` at a Rustic-operated
  `horizon-relay`. The public-hostname mode (players type `play.example.com`) is a different product: the Rustic
  Relay routes by handshake hostname (ARCHITECTURE §6).

## 6. Servers: `horizon-server` supervisor

- Supervisor (no game code) spawning the engine `<program> --resources <abs> --control-socket
  <dir>/.horizon/engine.json --dedicated --nogui` (`crates/horizon-server/src/engine.rs:247-295`).
  Engine lookup: `[engine] path` (relative to the server folder) first, then `horizon-engine` /
  `horizon-client` beside the supervisor, the installer's current release, or PATH (`engine.rs:17,35-73`).
  **Rustic Server = `engine.path` pointing at `rustic-server`.**
- `server.toml`: `[server] port bind motd max_players online_mode whitelist …`, `[world]`, `[properties]`,
  `[engine] path resources restart_on_crash low_memory`, `[backups] every keep`, `[updates]`, `[panel] bind
  allow_public`, `[api] bind token allow_public`, `[transfer]`, `[discord]`; unknown keys refused
  (`docs/api/server-reference.md:76-170`).
- Non-interactive creation: `horizon-server init --eula --world --seed --port --whitelist --player --op
  --offline --yes` or `HORIZON_EULA/WORLD/SEED/PORT/WHITELIST/PLAYERS/OP/ONLINE` (`docs/hosting.md:111-118`;
  `crates/horizon-server/src/init.rs:33-59`); then `start` / `run`.
- Control API: HTTP JSON on 127.0.0.1, token in `.horizon/api.json` (0600), `Authorization: Bearer`, Host
  header check; 30 endpoints under `/api/v1` (status, metrics, logs, events SSE, command, start/stop/restart,
  backups, restore, update, players + kick/ban/op, whitelist, GET/PATCH config, panel/login); `/api/v1` declared
  stable (`docs/api/server.md:28-75`; `docs/api/server-reference.md:172-209`). PATCH config cannot touch
  `engine.*`, `panel.*`, `api.*`, RCON/query keys. Also `horizon-server mcp`.
- Game files: the owner's own copy is required (`[engine] resources`, `resources/`, or
  `.local/horizon-client/26.3`), `import-resources DIR` (`docs/server.md:438-459`). The server jar is never
  downloaded by Horizon.
- Engines are updated from GitHub releases, Ed25519-checked; a pinned `engine.path` is never auto-updated and
  there is no compatibility handshake yet (`docs/hosting.md:246-259`). Linux archive
  `horizon-server-linux-x86_64.tar.gz` ships `horizon-engine` (`docs/server.md:427-436`).
- No Docker image published; Pterodactyl egg and AMP templates in `packaging/server/` (`docs/hosting.md:36-37,632-651`).
- Velocity modern forwarding supported (`docs/velocity-forwarding.md`).

## 7. Telemetry and privacy (Horizon)

- Opt-in switches (performance, crashes, bugs, benchmarks), all off by default (`docs/telemetry.md:3`;
  `crates/horizon-telemetry/src/consent.rs:25-28`), endpoint `telemetry.playhorizon.dev` (Railway, europe-west4).
- **Exceptions that ignore consent**: the anonymous "I'm playing" heartbeat `POST /v1/live` once a minute
  (registered only by `horizon-official`), and cosmetics "Show me as Horizon" (`docs/telemetry.md:5-8,74-86`).
  The Rustic edition registers only the Horizon parts we choose (ADR 0006) so our privacy policy stays true.

## 8. SRV / handshake hostname (matters for custom domains)

`crates/horizon-protocol/src/resolver.rs:1-55`: on port 25565 the client queries `_minecraft._tcp.<host>`;
when a record exists, **login uses the SRV target host and port** (vanilla parity, tested by
`login_handshake_address_matches_vanilla`); status pings keep the original address. So a relay that routes by
handshake hostname sees the **SRV target**, not the name the player typed (ARCHITECTURE §7 designs around it).

## 9. Rustic Server side (from `<server checkout>`, branch main)

- `rustic-server` = Horizon's game linked with Rustic as a trait mod over Horizon's `Host` (root `AGENTS.md`);
  Rustic's own wasmtime runtime (`docs/DECISIONS/0002-own-wasm-runtime.md`); WIT contract
  `crates/rusticmc-api-wit/wit/` (`rustic:api@0.1.0`). `docs/PLUGIN_FORMAT.md` **does not exist yet**; the
  `.rplugin` layout used by the launcher is the one summarised in `launcher/AGENTS.md` (RISKS R-15).
