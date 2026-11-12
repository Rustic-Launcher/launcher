# 0007. Cosmetics format: a superset of Horizon's format 1; our own service

- Status: accepted (spec, 2026-11-10). The content owner details the schema (ADRs 0060+).

## Context

Horizon's format (`horizon/crates/horizon-cosmetics`, MIT; `docs/cosmetics.md:20-170`): one JSON per item,
`format: 1`, slots `head, back, torso, shoulder, name_tag, emote, companion, trail, aura`, bone + attach, base64
64×64 PNG, box-UV parts tree, swing/bob/spin animations, FX blocks, emote keyframes, budgets (512 vertices, 16 parts,
24 px reach), unknown fields refused. Horizon's service (`cosmetics.playhorizon.dev`) and Paddle shop are Horizon's
business; the service code is not in the repository.

## Decision

- A Rustic item = `{ "format": "rustic/1", "horizon": <a valid Horizon format-1 item>, "rustic": { extensions } }`.
  The `horizon` member is a complete Horizon item: conversion **to** Horizon is lossless for everything Horizon
  supports; extensions (larger textures, colour variants, series labels, localised names) are dropped with a
  warning. Conversion **from** Horizon wraps an item unchanged.
- Validation reuses `horizon-cosmetics`' parser for the `horizon` member (crate at the pinned commit) plus our own
  extension checks and budgets.
- Rendering in game through `player_layers` / `name_decoration` in the Rustic edition (ADR 0006); in the launcher
  through ADR 0003.
- Rules identical to Horizon's and ours: purely visual, **no capes or cape-like items**, no random draws or loot boxes,
  no fake scarcity or countdowns (LEGAL §6).
- **Own service**: catalogue, ownership, equipped and `/v1/worn` on Rustic Cloud, keyed by Minecraft UUID proven
  with Mojang `join`/`hasJoined` (ARCHITECTURE §3.2). We never call Horizon's service.

## Consequences

Rustic items are visible to Rustic-edition players only. If Horizon's cosmetics stay enabled in the edition, both
answer `name_decoration` (first answer wins): the companion registers first and returns `None` when the player has
no Rustic name-tag item, so Horizon's style still shows (RISKS R-09).

## Alternatives

Reusing Horizon's service (no third-party API, no revenue model for us, ties our store to their Paddle account);
a new incompatible format (loses the Blockbench "Modded Entity" workflow creators already use).
