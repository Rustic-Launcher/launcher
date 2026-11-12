# Rustic Launcher: risks, unknowns, wrong assumptions

Owner: spec. Facts: [HORIZON_FACTS.md](HORIZON_FACTS.md). Impact: H/M/L. Likelihood: H/M/L.

## 1. Wrong assumptions found in the mission (Phase 0 research)

| # | Assumption | Reality | Impact | Mitigation / decision |
|---|---|---|---|---|
| A1 | Horizon mods are WASM (`horizon:mods` WIT) | Native mixin bundles and trait mods compiled into the game; WASM runtime moved to the unofficial fork `arismoko/horizon`; `.hmod` with `mod.wasm` refused | H | Companion = compiled into a prebuilt Rustic edition (ADR 0006); no WASM client mods; third-party native mods unsupported in v1 |
| A2 | Shader packs temporarily removed | Supported (Iris/OptiFine `shaders/` and Horizon WGSL `.hshader`); Modrinth shader packs not runnable via the launcher | M | Shaders tab ships in v1, sources Horizon registry (+ Rustic); Modrinth shader packs badged "Doesn't run" |
| A3 | A mod can draw on players/HUD and network | True, via two mechanisms; engine-level `player_layers`/`name_decoration`/`hud::register` need compile-time registration in `main` | M | Rustic edition registers them in its own `main` |
| A4 | MS login = device code + PKCE | Two different grants; Horizon uses device code only | L | Auth code + PKCE loopback primary, device code fallback (ADR 0010) |
| A5 | Session passed via stdin | `--session-stdin` exists; the official launcher uses `--session-file` | L | Use stdin (ADR 0008) |
| A6 | Relay routing by handshake hostname is Horizon's model | Horizon routes by invite-code nameplate, P2P first, no default relay | M | Rustic Relay is our own design (ADR 0013); invite codes resolved by the launcher |
| A7 | Horizon has a friends service | No friends/presence service; only invite-code tunnels | M | Rustic realtime service (ADR 0019) |
| A8 | Dedicated servers have a plugins folder | Not in Horizon; only native-mod engine exports | M | Rustic Server (`rustic-server` engine + `plugins/`) fills it |
| A9 | Builds can be pinned forever | Nightlies deleted after 7 days; no signed release index yet | H | We host Rustic edition bundles ≥ 90 days on our CDN |
| A10 | Per-instance RAM / loader settings | Only for Java-Edition instances of Horizon's launcher | L | No RAM/loader fields; resolution/launch options only |
| A11 | Datapacks fully supported | Per-world install; doc vs code disagree on what runs; no enable toggle in the official launcher | L | Badge "Partly" when unsure; per-world UI; re-verify per bump |
| A12 | "Free relay subdomains for everyone" (initial idea) | User decided: users bring/buy their own domain; relay subdomain optional | M | Three domain paths (ADR 0014); route-id wildcard still needs one Rustic-controlled domain |

## 2. Risks

| ID | Risk | I | L | Mitigation |
|---|---|---|---|---|
| R-01 | Mojang App ID review for our Azure app is slow or refused; without it `login_with_xbox` returns 403 | H | M | Apply on day 1 (TODO_HUMAN); dev with test accounts once approved; clear "sign-in not configured" state; never borrow another client id |
| R-02 | Launch API is experimental (0.1.0) and may change | M | M | Pin Horizon commit per Rustic edition; CI snapshot of `--launch-api`; probe options at run time |
| R-03 | Engine-level hooks (`horizon_hooks`, `PlayerDraw` changed in PR #14282) and `mod_api` change between commits | M | H | Companion pinned to a commit; CI build per bump; small adapter layer |
| R-04 | Horizon deletes Nightlies after 7 days; only Nightly is shipped | H | H | Mirror the Horizon parts we redistribute inside our bundles; keep ≥ 90 days |
| R-05 | Players expect Java mods / Lunar-style mod lists; most don't run on Horizon | M | H | Honest badges, "Works only" default filter, curated content |
| R-06 | Rustic Server plugin API (rustic:api 0.1) immature; `docs/PLUGIN_FORMAT.md` missing | M | M | Launcher validates only the manifest fields in launcher/AGENTS.md; R-15 |
| R-07 | Relay DDoS and bandwidth cost | H | M | Provider DDoS protection, per-route limits, plan quotas, VPS direct mode for big servers |
| R-08 | SRV records: clients log in with the SRV **target** as handshake host (`horizon-protocol/src/resolver.rs`) | H | H (if ignored) | CNAME preferred; SRV target is a per-route `<route-id>.<relay-host>`; relay routes on both |
| R-09 | Horizon's own cosmetics and ours both answer `name_decoration` (first wins) | L | M | Register companion first, return `None` when no Rustic name tag; or build without Horizon cosmetics (open question) |
| R-10 | Relationship with the Horizon project (we redistribute a modified client; forking rules on MSA id; brand) | M | L | MIT notices, "based on Horizon" wording, our own client id, upstream fixes contributed back, contact the maintainers early |
| R-11 | Mojang usage guidelines on selling cosmetics through a client could be interpreted strictly | H | L | Legal review; cosmetic-only, no capes, no random draws, disclaimer; precedent of other cosmetic clients |
| R-12 | Minecraft access token (~24 h) expires during a long session; the official renewal path (`HORIZON_LAUNCHER_EXE … account session`) is specific to horizon-launcher | M | M | Investigate whether `rustic-client` can be given a renewal command we implement (same CLI shape) without pretending to be horizon-launcher; else restart prompt |
| R-13 | New code-signing certificates have no SmartScreen reputation; macOS notarisation needed | M | H | EV/Trusted Signing, early beta to build reputation, notarise in CI |
| R-14 | Dev machine disk is tight (~20 GB); Horizon builds are large | M | H | Shared target dirs, CI does Rustic edition builds, check `df` |
| R-15 | `.rplugin` format spec lives only in AGENTS summaries | M | M | Request `docs/PLUGIN_FORMAT.md` from the server effort (HANDOFF) |
| R-16 | VPS mode: variety of distros, user misconfig, security of user boxes | M | H | Support matrix (Debian/Ubuntu x86-64), preflight, hardening defaults, SSH-only management |
| R-17 | Porkbun: API access must be enabled per domain by the user; key friction; purchase disputes; API changes | M | M | Clear guide, ping/diagnose errors (`API_ACCESS_DISABLED`, `DOMAIN_NOT_ALLOWED`), seller disclosure, dry-run quotes, `DnsProvider` abstraction |
| R-18 | BYO DNS: propagation delays, apex CNAME impossible at many registrars, CNAME conflicts with existing records | M | H | Per-record live status, subdomain default, ALIAS guidance, conflict detection |
| R-19 | Minors: regulatory exposure (GDPR Art. 8, DSA Art. 28, consumer law) | H | M | Age gate, parental controls, chat restrictions, moderation tooling in MVP, DPIA |
| R-20 | VAT outside the EU (UK, CH, US) | M | M | EU-only launch, then registrations or a merchant of record |
| R-21 | `horizon-engine` server archive only for Linux x86-64; ARM VPS unsupported | M | M | Preflight refuses ARM with explanation; ask Rustic Server for aarch64 builds later |
| R-22 | In-game invite codes impossible: one tunnel resolver, prefix `friend:` fixed, taken by Play with Friends | L | H | Codes resolved in the launcher; typed hostnames for stock clients |
| R-23 | Look-alike accusations (Lunar Client) | M | L | Own palettes (no blue/violet accent), own copy, own icons, design review against references (folder empty: TODO_HUMAN) |
| R-24 | Privacy promises vs Horizon parts compiled into the edition (heartbeat, "Show me as Horizon") | M | M | Do not register heartbeat; document Horizon cosmetics behaviour; open question on building without them |
| R-25 | Horizon docs inconsistent in places (datapacks coverage; release-signature "not yet live" in `docs/security.md:41` vs live in `docs/server.md:705-707`; README tag prefix) | L | M | Trust code over docs; verify per bump |
| R-26 | Real client IPs behind the relay: Horizon's dedicated engine has no PROXY protocol support (only `horizon-relay` reads PROXY v1; game supports Velocity forwarding) | M | H | Ask Rustic Server to accept PROXY v2 from the tunnel (HANDOFF) or use Velocity-style forwarding; until then IP bans don't work behind the relay (UUID bans do) |
| R-27 | Third-party native mods wanted by power users | L | M | Later: optional local build path reusing Horizon's pipeline (open question) |
| R-28 | Linux without Secret Service | L | M | Encrypted file fallback with warning |
| R-29 | Stripe merchant liability (we are the seller, consumer law, chargebacks) | M | M | Clear terms, revocation, fraud rules; MoR alternative (open question) |
| R-30 | Reference screenshots missing (`docs/references/` empty) | L | H | Frontend works from wireframes/tokens; human adds references (TODO_HUMAN) |
| R-31 | Rustic's cosmetic validator mirrors Horizon's `Item::parse` rules (not linked): a Horizon bump that changes them makes converted items refused by Horizon | L | M | Tests pin both on Horizon's documented example; re-check `horizon-cosmetics/src/item.rs` at each bump (ADR 0060) |
| R-32 | Plugin rules implemented twice (`rusticmc-content::plugins` mirrors `docs/PLUGIN_FORMAT.md` / `rusticmc-core`) | M | M | Rule-by-rule tests; HANDOFF asks for a shared `rusticmc-plugin-format` crate (ADR 0064) |
| R-33 | Companion uses pending (unstamped) hook fragments: `client_tick_post`, `add_toast`, `session_state` fields; a later Horizon may stamp or reshape them | M | M | Pinned Horizon commit per edition, CI compile per bump (ADR 0070) |
| R-34 | No in-game way to connect to a server from a mod: accepting an invite needs the launcher to restart the game | L | H | Cloud event to the launcher; quick-play relaunch (`--quickPlayMultiplayer`) |
| R-35 | Horizon docs disagree on data pack coverage (mod-browser.md: functions/predicates/item modifiers only; datapack-upgrade.md: most worldgen) | L | M | Badge "Partly" with both statements; verify per bump |
| R-36 | Fair-play term matching has false positives/negatives | L | M | Cloud deny/allow lists authoritative; flagged items stay installable (ADR 0066) |

## 3. Unknowns / open questions for the user

1. Keep Horizon's own cosmetics (and its "Show me as Horizon" presence) inside the Rustic edition, or build without
   `horizon-extras` cosmetics? (Default chosen: keep, per Horizon's defaults.)
2. Payments: Stripe + Stripe Tax with the Company as seller (chosen), or a merchant of record (Paddle/Lemon Squeezy)?
3. Company entity and country (drives OSS, legal notices, age rules, supervisory authority).
4. Product domain and the Rustic-controlled relay domain (needed for `<route-id>.<relay-host>` even without user
   subdomains).
5. Offline grace period: 30 days (chosen) or shorter?
6. Launch languages: EN + FR (chosen)?
7. Local native-mod builds as a later power-user feature: yes/no?
8. Rustic account minimum age 13 everywhere (chosen) and purchases for under-18 "ask a parent" by default (chosen)?
9. Contact the Horizon maintainers before public launch (recommended)?

### Added by desktop (2026-11-10)

| Id | Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|---|
| R-40 | Horizon's launch-guide.md says Nightlies are checked with the channel signature (`SHA256SUMS.sig`, `horizon release nightly`), but horizon-release-sign and Horizon's launcher require `SHA256SUMS.v2.sig` signed for `nightly-<date>-<commit>` | L | H | The launcher follows the code (v2, strict title parsing); releases from before signing (legacy list) are not offered |
| R-41 | RAM settings can't be applied: Horizon is native and Launch API 0.1 / server.toml have no memory option | L | H | Kept and shown, not passed; revisit if Horizon adds one |
| R-42 | ~~Offline profiles can't carry their name into the game~~ **Closed (ADR 0146)**: no free-name profiles; an offline launch passes the owner's session (official name and UUID) and the game goes offline itself | – | – | – |
| R-43 | The control API has no command completion: completion uses the prepared `commands.json` (vanilla tree) plus online players; plugin commands registered at run time are missing | L | H | Ask Horizon for a completion endpoint |
| R-44 | Porkbun's spec doesn't state the SRV content layout: the launcher sends `prio` apart and `weight port target` as content (Porkbun's DNS editor layout) | M | M | Verify on a real Porkbun domain and adjust `PorkbunDns::content_of` |
| R-45 | `.rplugin` rules exist in three places (rusticmc-core on main, rusticmc-content, rusticmc-servers): rusticmc-core can't be a dependency of the launcher branch (its path is outside this checkout) | M | M | Conformance test on PLUGIN_FORMAT.md's example and rule table; shared crate after the merge (ADR 0048) |
| R-46 | World game mode in the instance world list is not read (level.dat is NBT) | L | H | Shown as survival; add an NBT reader later |
| R-47 | The multiplayer list is the launcher's own (`multiplayer.json`), not the game's `servers.dat` (NBT, per game folder) | L | M | Join buttons pass `--quickPlayMultiplayer`; sync with servers.dat later |
| R-48 | Relay and ownership TXT need the Rustic Cloud client (tunnel tokens, domain tokens): turning the relay on answers "not available yet"; domain plans carry routing records only | M | H | Wire the cloud client (HANDOFF desktop → cloud) |
| R-49 | `horizon-server start` spawns the supervisor in the background: reading its output through pipes would hang on Windows (handle inheritance) | M | L | The launcher sends `start` output to `.rustic/start.log` (ADR 0047) |
| R-50 | The Launchpad hero video is third-party footage showing Minecraft textures (docs/THIRD_PARTY.md): a licence dispute, or Mojang's Usage Guidelines read as forbidding game assets inside a commercial launcher | M | L | Written permission from the creator on file; re-read the Usage Guidelines before launch (⚖); one-file swap (`HERO_VIDEO` in `HeroVideo.tsx`), and the original SVG illustration is the built-in fallback |
