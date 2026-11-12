# 0061. Cosmetic budgets and rules: Horizon's budgets, no capes, nothing random

- Status: accepted (content, 2026-11-10)

## Context

Cosmetics are drawn for every player in view on every frame through Horizon's skin-atlas path (64x64 slots), and
must never become a gameplay advantage, a cape, or a gamble (launcher/AGENTS.md, LEGAL section 6).

## Decision

- Budgets (`validate::budget`): 512 box vertices (Horizon's), 512 mesh triangles, 16 parts, power-of-two textures at
  most 64x64, PNG at most 64 KiB, bundle at most 256 KiB, reach 24 px sampled over every loop and the emote;
  companions, trails and auras with Horizon's own limits.
- No capes: refused by name (`cape`, `cloak`, `mantle`, `elytra`) and by shape (a flat back panel hanging down).
  There is no cape category.
- Deterministic: loops and emotes are functions of the model's age; trail particles are spawned by distance walked
  and varied by their index (a hash), never by a random generator. The format has no field that could carry
  randomness or gameplay data (unknown fields are refused).
- Every problem is reported at once, with a code and a place, for the creator UI.

## Consequences

A box cosmetic valid here is valid in Horizon. The cape heuristic may refuse an honest design: a reviewer reshapes
or renames it; it never lets a cape through silently.

## Alternatives

Larger budgets for meshes (would need an atlas path the game's hooks do not offer).
