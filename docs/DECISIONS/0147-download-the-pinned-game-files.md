# 0147. Download the pinned game files from Mojang when no install has them whole

Date: 2026-11-12 · Status: accepted · Implements the "downloading Mojang files" part of ADR 0005

## Context

`horizon-prepare` reads the player's own launcher folder and downloads nothing: the pinned version
JSON, client jar, libraries, asset index and every asset object must already be there. The official
launcher writes the client jar and the sounds only when that version is played once. A player who
never played 26.3 in it (only its version JSON listed, as on the product owner's computer) got
"Missing …\versions\26.3\26.3.jar … play 26.3 once in the launcher you play Minecraft Java Edition
with", and could not play from Rustic at all. ADR 0005 already planned for Rustic to download Mojang
files for a signed-in owner, like Horizon's own launcher (`horizon/launcher/src/mojang.rs`).

## Decision

- `rusticmc_instances::game_files` (rewritten from the idea of Horizon's `mojang.rs`, MIT): the build's
  `research-lock.json` pins the version, the version JSON's sha1 (and the republished ones Horizon
  accepts) and the client jar's sha1. `is_complete(dir)` checks a folder has every file by size (only
  the version JSON is hashed), cheap enough for every preparation.
- A preparation (Settings, onboarding, or a launch that finds the files unprepared) uses the player's
  folder when it is complete. Otherwise it uses Rustic's own folder (`<data>/minecraft`, the official
  launcher's layout) and, when that is not complete either, downloads into it first: the version JSON
  from piston-meta by its pinned sha1, then the client jar (the pin wins over the version file), this
  system's libraries, the asset index and its objects from Mojang's hosts, each through the download
  manager (sha1 and size checked, retries, resume, cancel with the task). Files the player's folder
  holds are hard-linked (copied across volumes) first and re-checked; only the rest is fetched.
- Only for a verified owner: the download takes the `VerifiedOwner` proof that the preparation already
  requires for the server jar. Nothing of Mojang's is in the repository or the release.
- Progress: a `download` step in `prepare-progress`, and the first half of the task's bar (the launch
  status shows it under "Preparing the game files").

## Consequences

- First launch works on a computer whose launcher never played the pinned version, or without any
  launcher installed; the onboarding "Prepare" button no longer needs a found install.
- About 600 MB for 26.3, mostly linked from the player's folder when it has other versions (23 s on the
  product owner's computer, then 104 s of `horizon-prepare`). Assets are not kept in the shared
  download cache, to avoid storing them twice.
- An ignored test (`real_mojang_download`) checks the real hosts and files by hand.

## Alternatives

- Writing the missing files into the player's `.minecraft`: touches another application's folder.
- Asking the player to start the version in the official launcher: what the error said; a dead end
  for players who don't use it.
