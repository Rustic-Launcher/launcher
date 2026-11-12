# 0006. Companion delivery: a prebuilt "Rustic edition" Horizon client, built and signed by our CI

- Status: **accepted (user decision, 2026-11-10)**
- Owners affected: content (companion), desktop (build manager), cloud (CDN/CI)

## Context

Rustic needs in-game features: Rustic cosmetics on players and name tags, HUD toasts, a settings page, networking
with Rustic Server. Horizon loads no code at run time: native code is compiled into the game
(`horizon/docs/native-mods.md:13,19`). Drawing meshes on player bones and adding name badges uses the engine-level
`horizon_hooks` (`player_layers`, `name_decoration`, `hud::register`), which only crates compiled into the
executable's `main` can register before `entry::main_with` seals the list (`crates/horizon-hooks/src/lib.rs:191`,
`registry.rs:23-40`, `docs/layers.md:255-285`). Catalog trait mods (`mod_api::Hooks`) cover networking and GUI.

Options compared:

| | 1. Local native build (Horizon's `native-build`) | 2. Prebuilt Rustic edition (CI) |
|---|---|---|
| Player download | Rust 1.99.1 + MinGW/LLVM-MinGW + ~63 MB sources + crates | One signed archive |
| First start | 1.5-30 min compile (`docs/native-mods.md:1845,2058`) | Seconds |
| Windows Smart App Control | Blocks the build (os error 4551) | Signed exe runs |
| Linux / SteamOS | Needs cc, pkg-config, ALSA, libudev headers; SteamOS unsupported | Works |
| Engine-level hooks | Not reachable from a `.hnmod`'s generated main | Our own `main` registers them |
| Reproducibility, QA | Per machine | Per build, tested in CI |
| Third-party native mods | Possible | Not in v1 |

## Decision

Option 2. CI matrix: Windows x64, Windows ARM64, macOS arm64, macOS x64, Linux x86-64. Bundle = the Horizon release
folder of the same commit (`horizon-prepare`, `research-lock.json`, `exporters/`) + `rustic-client[.exe]` +
`rustic-build.json` (Horizon commit, companion version, file hashes); `SHA256SUMS` signed with **our** Ed25519 key
(separate from Horizon's release key); Authenticode on Windows, Developer ID + notarisation on macOS. A new build
follows each Horizon bump we accept (weekly at most); builds stay on our CDN at least 90 days (Horizon deletes
Nightlies after 7: `docs/api/launch-guide.md:27-29`).

Horizon parts registered in our `main`: `horizon_friends::game::HOOKS` (Play with Friends joins). Not registered:
`horizon_heartbeat` (always-on ping to Horizon's telemetry, contrary to our consent-first policy),
`horizon_official_servers` (another project's server in the list), `horizon_discord` (Rustic presence later).
Horizon's in-engine cosmetics (`horizon-extras` feature, on by default) stay as Horizon ships them unless the user
decides otherwise (open question).

## Consequences

- One download, no compiler, no Smart App Control issue, reproducible builds.
- We redistribute a modified Horizon build: keep MIT `LICENSE` and `THIRD_PARTY_NOTICES.md`, add a Credits screen,
  call it "Rustic edition, based on Horizon" (never "Horizon" alone, never "Minecraft"), never ship Horizon's MSA
  client id (the client receives sessions; it does not sign in).
- No third-party native mods in v1. Curated native features (e.g. Horizon's official native mods, if their licences
  allow) may be compiled into the edition.
- The hook APIs are experimental: breakage shows up as CI compile errors, not on players' machines. Each bump is a
  CI and QA cost.

## Alternatives

Option 1 (rejected above). A long-lived Horizon fork with Rustic patched in (harder to rebase than an external `main`
depending on Horizon crates at a pinned revision).
