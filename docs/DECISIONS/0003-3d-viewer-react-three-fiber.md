# 0003. 3D player and cosmetics viewer: react-three-fiber + three, our own player model

- Status: accepted (spec, 2026-11-10)

## Context

The Locker and Store show the player's skin in 3D wearing cosmetics: box-UV parts on bones, animations (swing, bob,
spin), emotes, companions, trails and auras (Horizon format-1 concepts, `horizon/docs/cosmetics.md:20-170`).
`skinview3d` 3.4.2 (MIT, last publish 2026-05-03) renders skins, capes and elytra well, but its model is built around
those parts, and we never sell or show capes as products (LEGAL §6).

## Decision

`three` 0.187.0 (MIT, 2026-10-30) + `@react-three/fiber` 9.8.2 (MIT, 2026-10-21) + `@react-three/drei` 10.7.9 (MIT,
2026-09-25; OrbitControls and helpers only). We write our own player model (64×64 skin layout, classic and slim arms,
outer layer) and a cosmetics mesh builder from `cosmetics-format` JSON (geometry ideas ported from Horizon's
`horizon-cosmetics` mesher, MIT, attributed). `frameloop="demand"` outside animations; static thumbnails from the
cdn-pipeline in grids.

## Consequences

One renderer for Locker, Store and profile cards. The default stand-in skin is **our own original art**, never
Steve/Alex (Mojang assets); Horizon does the same (`stand_in`, `horizon/docs/cosmetics.md`, Shop pictures).

## Alternatives

skinview3d (quick start, cape-centric, harder to extend for bones and FX); Babylon.js (heavier).
