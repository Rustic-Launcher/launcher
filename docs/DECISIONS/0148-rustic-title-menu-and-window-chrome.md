# 0148. The Rustic edition's main menu and window title bar

Date: 2026-11-12 · Status: accepted (user decisions of 2026-11-12) · Builds on ADRs 0006, 0070

## Context

The Rustic edition (`rustic-client`, ADR 0070) showed Horizon's vanilla-looking title screen in a
window titled "Horizon" with Horizon's icon and the system's light title bar. The user asked for:

1. the **native** Windows title bar kept (snap, resize, double-click, several screens) but in Rustic's
   colours, with "Rustic" as its text and Rustic's icon;
2. a **Lunar-like main menu** (user's design updates of 2026-11-12): smooth and modern, no game
   look (no vanilla button textures, nine-slices or pixel font): the Rustic logo; Singleplayer and
   Multiplayer (about 400x40 px at 1280x720, 8 px apart), then a secondary action and the **Store**
   (in the accent, about 195x40 each); a fixed account box top left (head and name only); a store
   news card bottom right with the products' pictures, names, prices and carousel bars; small
   square icon buttons bottom centre (about 36x36); the version bottom left; the "Not affiliated
   with Mojang or Microsoft" line kept; the vanilla panorama darkened behind; the game's language;
   keyboard usable; every window size and GUI scale (854x480 included).

Horizon's mod API could draw over the title (`screen_render_post`, id `title`) and cancel its input,
but could neither hide the title's own widgets nor open what its buttons open.

## Decision

- **Horizon patch U13** (Rustic-MC `patches/0011-rustic-title_widgets_hidden-and-title_open-helpers.patch`,
  Rustic-MC ADR 0011): two generic client helpers, `title_widgets_hidden(hidden)` and
  `title_open(button) -> bool` (`singleplayer`, `multiplayer`, `realms`, `friends`, `options`,
  `language`, `accessibility`, `credits`, `quit`: the button's own press). The edition is therefore
  built from Rustic's patched Horizon, not from a bare Horizon commit (CI sketch updated).
- **The menu** (`client-mod/rustic-companion`): `core/src/title.rs` lays it out in the screen's own
  pixels from a 1280x720 design scaled by the window (`unit`: the smaller of width/1280 and
  height/720, kept within 0.6-2) and hands GUI pixels to the game (pure, tested at 854x480 to
  2560x1440 at the GUI scales the game picks: no overlap, every button reachable, the reference sizes
  at 1280x720, the legal line always there, pictures and text on whole screen pixels);
  `mod/src/title_menu.rs` hides the vanilla widgets, draws the layout in `screen_render_post`, and
  handles `screen_mouse_button_pressed_pre` and `screen_key_pressed_pre` (Tab/Shift+Tab/arrows walk
  the buttons, Enter/Space press; Escape is left to the title, which ignores it as vanilla).
  - **Everything smooth is an image of the companion's own**, uploaded into the HUD atlas slot a
    native mod may fill (`Host::mod_image`, API 355, 1024x1024) and drawn with `Host::sprite` one image
    pixel to one screen pixel (so the atlas's filtering never matters): the logo and icons resampled
    to the exact screen size, and **every line of text set in Inter** (SIL OFL 1.1:
    `mod/assets/Inter-Medium.ttf`, `Inter-SemiBold.ttf`, static Latin instances made from the
    launcher's own Inter by `mod/assets/make_fonts.py`, licence in `mod/assets/Inter-LICENSE.txt`)
    with `ab_glyph`, anti-aliased, at the screen's pixel size. The images of a frame are packed into
    one picture (shelves, tallest first) and uploaded once when the set changes (a hover adds a
    tooltip, the card turns), cached by key; nothing is drawn in the game's font. So no Horizon patch
    was needed for smooth text: the existing helper already uploads a mod's RGBA.
  - Boxes: #121212 at 80 % with a one-screen-pixel lighter border, radius 8 (design pixels), round
    corners drawn as fans of thin `quad` triangles at the screen's own pixels (no overlap, so the
    translucency is even); hover and keyboard focus turn the border orange (#FF6A00). The Store box is
    orange-tinted.
  - Icons: Rustic's own flat shapes (person, people, tower, bag, gear, globe, accessibility, power,
    and the cosmetic slots' crown, sparkle, paw, smile, tag, shirt), drawn smooth at 4x by
    `mod/assets/make_assets.py` into `rustic-icons.png`, resampled and tinted at run time.
  - Top left: a fixed account box (not a button): the player's face (their skin's, from the account
    profile as the game asks for it, else the default skin's for their UUID) and name. Accounts live
    in Rustic Launcher (the game receives one session at launch), so no switch is offered in game.
  - Centre: the wordmark (56 px high), Singleplayer and Multiplayer (400x40), Realms and **Store**
    (196x40 each).
  - Bottom centre: Options, Language, Accessibility, Quit (36x36, 10 apart; the name of the hovered or
    focused one above the row). Join a Friend stays reachable from Multiplayer (Horizon's own screen).
  - Bottom right: the **store news** card, the newest products of Rustic Cloud's public catalogue
    (`GET <api>/v1/store/catalog?currency=EUR`, asked off the render thread every 10 minutes while
    the title shows, at most 4 MB read): picture, name and price (the API's own display string),
    turning every 5 s, a bar per product under it (the shown one orange); hidden while the catalogue
    is empty or unreachable, and when the window has no room for it. The catalogue has **no picture
    URL** (products are 3D cosmetics, previewed in 3D on the website), so the card's picture is the
    website's own product art: a gradient of the item's hue (`hueOf`, the launcher's hash) with its
    slot's icon (`SLOT_ICON`), rendered in the companion. A press opens
    `https://rustic-launcher.com/store/<slug>` (slugs checked to letters, digits, `-`, `_`); the Store
    box opens `https://rustic-launcher.com/store`. Links open with the `open` crate on a thread of
    their own (ShellExecute on Windows), fixed https URLs only.
  - Bottom: "Rustic · Horizon 26.3" left (dropped when the window is too narrow), the legal line
    right (a press opens the credits, as vanilla's).
  - Labels: the game's own translation keys for its buttons (`menu.singleplayer`...), so they follow
    the game's language ("Minecraft Realms" shows as "Realms"); Rustic's own texts (Store, Store
    news) in French for `fr_*` and English otherwise.
  - The companion's endpoints now default to the production hosts (`https://api.rustic-launcher.com`,
    `https://cdn.rustic-launcher.com`); `RUSTIC_CLOUD_API`/`RUSTIC_CDN` may also be plain http on
    this computer (127.0.0.1, localhost, ::1) for a local cloud.
  - The upkeep runs in the `frame` hook (before each frame is drawn, so the title's widgets are
    hidden from the first frame); the picture goes up again on `client_resources_reloaded` and every
    5 s while the title shows, because a new renderer starts with a blank slot.
  - `title_menu: false` in `config/rustic-companion.json` brings the game's own title back.
- **The logo** is `mod/assets/rustic-wordmark.png`: the official mark's polygons from
  `assets/brand/rustic-mark.svg` beside "Rustic" in Inter SemiBold (`mod/assets/make_assets.py`).
- **The title bar** (`edition/src/chrome.rs`, Windows only): a thread finds this process's visible
  top-level winit windows (class `Window Class`) and sets `DWMWA_USE_IMMERSIVE_DARK_MODE`, then
  `DWMWA_CAPTION_COLOR` #0D0D0C, `DWMWA_TEXT_COLOR` #F3F1EE, `DWMWA_BORDER_COLOR` #262421 (Windows 11;
  Windows 10 takes dark mode only) and the text "Rustic · Horizon 26.3" (`WM_SETTEXT` with a 1 s
  timeout), again whenever the text was changed or every 10 s. No handle is exposed by the game, hence
  the search. Its Win32 calls are the only `unsafe` code of the workspace, in one module
  (`chrome::win32`, `#[allow(unsafe_code)]`, a `// SAFETY:` comment per block; the crate root is
  `#![deny(unsafe_code)]` instead of `forbid` so that one module can allow it). Elsewhere: no-op.
- **The icon**: `main` gives the game Rustic's (`horizon_client::window::set_icon_png`, before
  Horizon's own: the first one given is kept), the window and the task bar show it; on Windows the
  executable also embeds the launcher's `icon.ico` as resource 1 (build.rs, `winresource`).
- **Launcher**: `InstalledBuild::client()` runs `rustic-client` in a build whose source is
  `rustic-edition` and that holds it (the HANDOFF item "launch rustic-client as the instance's
  game"), Horizon's `horizon-client` otherwise.

### Crates (all already in Horizon's `Cargo.lock`, so nothing new is downloaded)

| crate | licence | where | why |
|---|---|---|---|
| `windows-sys` 0.61 | MIT OR Apache-2.0 | edition, Windows only | `DwmSetWindowAttribute`, `EnumWindows`, `SendMessageTimeoutW` |
| `winresource` 0.1 (build) | MIT | edition | the executable's icon resource |
| `png` 0.18 | MIT OR Apache-2.0 | mod | decoding the wordmark, the icons and the player's skin |
| `open` 5 | MIT | mod | opening the website's store in the system's browser |
| `ab_glyph` 0.2 | Apache-2.0 | mod | setting the menu's text in Inter, anti-aliased |
| `horizon-core` (path) | MIT | mod | the game version in the footer and the window title |

Font: Inter (SIL OFL 1.1, The Inter Project Authors, no Reserved Font Name): two static instances
are embedded in the companion, with the licence beside them (`mod/assets/Inter-LICENSE.txt`); the
wordmark PNG is also set in it. The bundle's notices must list it (ADR 0006).

## Consequences

- The title looks like Rustic from the first frame the game runs its mods. The **startup menu** (the
  interactive title the loading window shows while the game's resources finish loading) runs no
  trait mod: it shows the vanilla title until the game takes over.
- Every other title screen (world list, multiplayer list, Realms, options) stays Horizon's.
- `horizon-ctl ui` / `click-button` no longer see the main title's buttons (they are hidden); the
  Rustic menu's buttons are not widgets of the game. Agents drive the menu with `click-at`/keys.
- The face comes from the account's profile (`GET /minecraft/profile`, as the game itself asks) and
  the texture server Horizon accepts; offline, the default skin's.

## Alternatives

- A borderless window with a drawn title bar: loses the system's snap, resize and double-click
  behaviour, which the user wanted kept.
- Drawing the logo and icons as merged `fill` rectangles (no mod image): pixel art, only crisp at one
  pixel per GUI pixel, blocky at GUI scale 3 and more; the user asked for a smooth look.
- Using the game's own GUI sprites for icons and buttons: Mojang's art, and the "game look" the user
  did not want.
- A new Horizon helper to upload named mod textures: not needed, the existing `mod_image` slot
  already takes a mod's RGBA, and drawing at one image pixel per screen pixel makes filtering
  irrelevant. Its limit is one 1024x1024 slot per game: the menu stays within it up to the 1440p
  sizes (`unit` capped at 2; larger windows keep those sizes), and a set that does not fit is logged
  and left out rather than cut.
- The game's own font (`Host::text`): the pixel look the user did not want.
- An in-game account switch: the game gets one session at launch from the launcher, which holds the
  accounts (and their tokens, in the OS keychain); switching means relaunching from the launcher.
