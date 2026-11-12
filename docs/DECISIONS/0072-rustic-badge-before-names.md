# 0072. The Rustic badge: the logo left of Rustic users' names, in place of Horizon's mark

- Status: accepted (content, 2026-11-12). Builds on ADRs 0070 (the companion and the edition), 0071
  (the companion's cloud calls) and 0148 (the edition's look). Needs Rustic's Horizon patch U14
  (Rustic-MC `patches/0012-rustic-name_picture-hook-a-picture-in-the-place-of-the-mark.patch`,
  Rustic-MC ADR 0012).

## Context

Players who use Rustic should be recognisable in game, on every server, without anything to set up:
the Rustic logo left of their name, always visible, before the whole display name (a rank prefix a
server sets through a team or a list name included), never replacing it.

Horizon already draws **its own mark** (a small pixel glyph) before the names of Horizon players, in
the player list (`horizon_hooks::Hooks::player_mark`) and on name tags (`NameDecoration::mark` from
`name_decoration`), from Horizon's cosmetics service (`POST /v1/worn` answers `horizon: [uuid]`). A
`Mark` is static filled rectangles (pixel art at GUI resolution), both hooks are "first answer wins",
and `name_decoration` also carries Horizon's name tag style and lift. Chat lines are a `Text`, in
which Horizon draws an inline sprite (`InlineObject::Atlas`) from the GUI atlas.

**User decision (2026-11-12)**: for a Rustic user the Rustic logo **takes the place** of Horizon's
badge (one badge only, Rustic's, in the same position); a player who uses Horizon (Horizon's own
launcher or client) and not Rustic keeps Horizon's badge exactly as today; players using neither get
no badge.

## Decision

### Where it shows

- **Player list (Tab)** and **name tags over heads**: Rustic's patch U14 adds the platform hook
  `name_picture(uuid) -> Option<sprite id>`. The companion's player hooks (`mod/src/players.rs`,
  `mod/src/badge.rs`) answer the badge's sprite for Rustic users; Horizon then draws it in the mark's
  8x8 box (after the head in the list, a pixel left of the tag's background over heads), with the
  text's shadow in the list, and **leaves Horizon's mark out for that player only**. Horizon's mark
  logic is untouched for everyone else. The badge sits where the mark sits: left of the whole
  display name, so a team prefix (`[Admin] `) or a list name stays after it, unchanged.
- **Chat**: in `client_chat_received` (trait mod), a **player** chat line (not a system line, not a
  disguised one: the sender's UUID is known) from a Rustic user gets the badge prepended as an inline
  sprite span (`\u{fffc}`, atlas object `hooks/<hex>`, white, "Rustic" on hover). Only the shown
  spans change: the line's component (narration, the log, copying) stays as the server sent it.
  System lines (most chat plugins on Paper-like servers send decorated chat as system messages, with
  no sender) get no badge: the companion cannot tell whose they are. The badge goes before the whole
  line, so before the decorated name and its rank.

### Who is a Rustic user

- `POST <api>/v1/public/minecraft/lookup` (existing, public, no auth, at most 100 UUIDs a request,
  rate-limited to 120 a minute per address): `player` is non-null when the Minecraft UUID is linked
  to an **active** Rustic account. That is the definition used: "has a Rustic account linked to this
  Minecraft profile". The worn-cosmetics call (`/v1/cosmetics/worn`) cannot answer it: it leaves out
  players who wear nothing.
- Asked for the listed players (`Host::client_profiles`, read every 2 s), the players in view and
  recent chat senders, in batches of at most 100, **one request in flight and at most one every
  2 s**, on the companion's network thread (never the render thread or the game tick); answers cached
  15 minutes for "yes", 5 minutes for "no" (`core/src/badge.rs` `BadgeCache`), the last answer kept
  while a new one is asked, at most 4096 players remembered; failures back off 15 s doubling to 10
  minutes (`Backoff`). Offline or unreachable: **no badge** for anyone the cloud has not answered for.
- The window's own player gets the badge while the Rustic session (`/v1/companion/session`, ADR 0071)
  is theirs: the session's UUID is the profile's. Signed out: only if the cloud says so.
- The seam is `Endpoints::rustic_lookup` + `LookupAnswer::rustic()` (`core/src/protocol.rs`): a
  leaner endpoint (proposed in `docs/HANDOFF.md`, 2026-11-12) replaces those two functions only.

### Privacy

- The badge reveals **only "uses Rustic"**: no account id, username or display name is drawn,
  logged or kept (the companion keeps a boolean per UUID in memory, nothing on disk).
- The endpoint itself returns more than the badge needs: the linked account's public profile
  (`user.id`, `username`, `display_name`) and what it has equipped. It is an existing public endpoint
  (anyone can already query it), but every Rustic client now asks it about every player it sees, so a
  minimal endpoint that answers only the set of linked UUIDs is proposed to the cloud owner
  (`docs/HANDOFF.md`); the companion discards the rest meanwhile. The request discloses to Rustic
  Cloud which UUIDs a player sees (as the worn-cosmetics call already does, ADR 0071).
- Being shown as a Rustic user is part of using the product (no opt-out switch is required by the
  product decision); it applies only to accounts the player linked themselves. The privacy policy
  should say that a linked Minecraft profile is shown as a Rustic user to other Rustic players (to the
  legal owner: `docs/LEGAL.md`).

### The picture

- The official mark (white R with the orange stroke), cropped out of the title menu's wordmark
  (`mod/assets/rustic-wordmark.png`, the polygons of `assets/brand/rustic-mark.svg`), resampled smooth
  (area average) to the **exact screen size of the game's text** at the current GUI scale: within the
  cap height (7 GUI pixels), its last GUI column left empty (a 1 GUI pixel gap before the name), a
  45 % black outline one screen pixel wide so it reads on bright skies and dark rows alike; the list
  and chat add the font's own shadow.
- The hooks' sprites are 64x64 and drawn whole into 8 GUI pixels with nearest sampling. The render
  (8 x GUI scale pixels square) is laid into the 64x64 sprite so that each texel holds the render
  pixel under its centre: the game's sampling then gives the render back pixel for pixel (tested for
  every side 8 to 64). It goes up again when the GUI scale changes (the `hud` hook gives the scale;
  the `frame` player hook puts it). Sprite id `SPRITE_BASE | 0xFFFF_FF00` (Rustic's prefix).

## Consequences

- On a server where nobody uses Rustic, nothing changes but the lookups; Horizon's marks show as
  before. With one Rustic user, the list's mark column appears (Horizon's own rule: once anyone has
  a mark or a picture).
- The badge is client-side only: nothing is sent to the server, no server or plugin can hide it, and
  players on other clients don't see it.
- `Host::client_profiles` resolves the listed players' skins through the client's cache, as the
  player list does when shown (skins of listed players may be downloaded a little earlier).
- Each Horizon bump is a patch check (U14) and a QA pass, as for U13.

## Verification (2026-11-12)

- Companion workspace: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  clean; `cargo test -p rusticmc-companion-core` 44 passed (new: batching, one request in flight,
  the 2 s gap, TTLs, backoff, eviction, who gets the badge, the lookup's request and parsing, the
  logo's box at GUI scales 1 to 8, the sprite giving back the render pixel for pixel, the mark cut
  out of the wordmark, the badge's outline and free gap column).
- Rustic-MC (patch U14): `cargo test -p horizon-hooks --lib`, `cargo test -p horizon-client-render
  --lib -- tab_list` (new test: a picture takes the mark's place, names stay aligned, the mark comes
  back without pictures), clippy of the three patched crates clean, `patches.py check` 12 patches OK.
- In game (`rustic-client` fast build, installed into the dev build folder): a scratch Rustic Server
  (offline, port 25607) with teams `[Admin]` and `[VIP]` as prefixes, three `horizon-bots` standing
  by the player, a made-up offline session, and a local fixture on 127.0.0.1 standing in for Rustic
  Cloud (`RUSTIC_CLOUD_API`, the lookup and the loopback session) and Horizon's cosmetics service
  (`HORIZON_COSMETICS_API`, its `horizon` list). The fixture says RusticDev (the player) and
  RusticBot0 use Rustic and also play Horizon, HorizonBot0 plays Horizon only, PlainBot0 neither.
  Seen: the player list shows the Rustic logo before `[Admin] RusticDev` and `[VIP] RusticBot0`,
  Horizon's mark before `[VIP] HorizonBot0`, nothing before `PlainBot0`; the name tags the same;
  the player's chat line starts with the logo before `<[Admin] RusticDev>`. Crisp at GUI scales 2,
  3 and 4 (the sprite re-rendered on change). Lookups batched as designed (one per 2 s at most).
  With the fixture down: no badge anywhere (and no hook failure logged).
- Not checked in game: system-message chat (by design no badge), a server above 100 players (more
  than one batch at once), a real Rustic Cloud.

## Alternatives

- Horizon's `Mark` (static rectangles) from `player_mark`: pixel art, and answering it first would
  hide Horizon's mark for everyone, Rustic user or not.
- Answering `name_decoration`: replaces Horizon's whole decoration (its name tag style and lift).
- Both badges side by side: rejected by the user decision (one badge, Rustic's, for Rustic users).
- Drawing over the player list from the `hud` hook: the trait mod cannot know where the rows are.
- Changing names (the server's list names, a client-side rename): would alter what the server set
  (ranks), the opposite of the requirement.
- `/v1/cosmetics/worn` as the source: misses every Rustic user who wears nothing.
