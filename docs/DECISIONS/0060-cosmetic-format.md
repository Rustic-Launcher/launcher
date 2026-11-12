# 0060. Rustic cosmetic format: documented JSON (Blockbench boxes) plus glTF meshes, convertible to Horizon's

- Status: accepted (content, 2026-11-10). Refines spec ADR 0007.
- Owners affected: content, frontend (TS types from the JSON Schema), cloud (catalogue stores bundles)

## Context

ADR 0007 proposed `{ "format": "rustic/1", "horizon": <Horizon item>, "rustic": {...} }` and validation through
`horizon-cosmetics`' parser. Two facts change that: the brief asks for named attachment points on the skeleton
(back, shoulders, name tag...) and glTF meshes, neither of which a Horizon item can hold; and the launcher workspace
cannot link `horizon-cosmetics` (Horizon is a separate, read-only checkout with its own `[patch]`).

## Decision

- One file, `cosmetic.json`, format 1 (`launcher/docs/COSMETICS_FORMAT.md`), whose geometry, motion and effect blocks
  have **exactly the JSON shapes of Horizon's** (parts, box-UV cubes, `animations` as `loops`, emote, companion,
  trail, aura) plus Rustic metadata (version, name, authors), categories (`top`, `shoulder_companion`, `nametag`...),
  attachment points with base offsets, textures by path or embedded, and `mesh` models from glTF 2.0.
- `rusticmc_cosmetics_format::horizon` converts both ways: Horizon -> Rustic -> Horizon is lossless (tested on
  Horizon's documented example), Rustic -> Horizon reports what it drops; meshes are "not representable".
- Validation reimplements Horizon's rules (same limits) plus ours; re-checked at each Horizon bump.
- JSON Schema generated with `schemars`, committed (`schema/cosmetic.schema.json`), kept current by a test.

## Consequences

A wrapper would have duplicated every field and still not carried meshes. The price: our validator must follow
Horizon's when it changes (RISKS). Box cosmetics stay usable by Horizon itself through the CDN's `horizon/` copy.

## Alternatives

ADR 0007's wrapper (no meshes, double bookkeeping); glTF only (loses the box-UV workflow and Horizon conversion).

## Crates

| Crate | Version | Licence | Why |
|---|---|---|---|
| `serde`, `serde_json` | 1 | MIT OR Apache-2.0 | the file |
| `schemars` | 1.2 | MIT | JSON Schema for the TS side |
| `png` | 0.18 | MIT OR Apache-2.0 | textures (Horizon's version) |
| `base64` | 0.22 | MIT OR Apache-2.0 | embedded textures |
| `gltf` (feature `gltf`: `utils` + `names`, no `import`) | 1.4 | MIT OR Apache-2.0 | glTF 2.0 meshes; off in the game client |
| `thiserror` | 2 | MIT OR Apache-2.0 | errors |
