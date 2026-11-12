# 0066. Fair-play filter: blocked, flagged, clean

- Status: accepted (content, 2026-11-10). Implements FAIR_PLAY.md in the launcher.

## Decision

`rusticmc_content::fairplay::FairPlayPolicy`: a deny list (ids, slugs) and terms matched on whole words (or a compound
word containing a term of 5+ letters) in name, slug, categories and tags **block** a project: never promoted, never
installed through Rustic (the installer refuses). A term only in the summary, or a single-player-only feature
(freecam, fullbright), **flags** it: not promoted, installable with a notice. A term right after "anti", "no",
"detect"... (or glued to "anti") is a protection, not a match. `promotable()` is the only source of featured lists.
The cloud's catalogue keeps the authoritative deny list; the launcher applies its copy offline too.

## Consequences

False positives are possible (an honest mod named "Dupe..."): moderators allow-list it in the catalogue, and flagged
items stay installable.
