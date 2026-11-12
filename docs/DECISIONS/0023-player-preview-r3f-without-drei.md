# 0023. 3D player preview: react-three-fiber without drei, procedural original model

- Status: accepted (frontend, 2026-11-10)
- Refines: ADR 0003 (spec)

## Context

The Locker and the product page need a rotatable player with equipped cosmetics and preview animations. ADR 0003
picked three + react-three-fiber + drei; skinview3d (MIT, 3.4.2) was the alternative. No Mojang or Lunar model or
texture may be shipped.

## Decision

- `three` 0.187.0 + `@react-three/fiber` 9.8.2, **without `@react-three/drei`**: we only needed orbit-style
  rotation, which is ~30 lines (pointer drag + arrow keys with easing). drei pulls `three-stdlib` and many optional
  heavy dependencies; dropping it saves disk and bundle size.
- The figure is an **original blocky character built from boxes in code** (`packages/ui/src/three/PlayerViewer.tsx`):
  proportions 8/12/4 px units, separate pivoted limbs, simple face, flat colours representing the user's skin
  until the desktop side provides the user's own skin texture.
- Cosmetics are procedural shapes per slot (head, back — compact objects only, never capes —, top, shoulder
  companion, companion, trail, aura, emote, nametag) until the cosmetics pipeline (ADR 0007) delivers meshes.
- Animations: idle, walk, wave, dance; none under reduced motion. Keyboard-focusable canvas with an accessible
  label. The viewer is in a lazily loaded chunk (Locker, Store product page only).

## Consequences

The real skin (64×64 layout, slim arms, outer layer) and the cosmetics mesh builder still have to be implemented
on top of this component, as ADR 0003 describes; the public props (`cosmetics`, `animation`, `colors`) stay.

## Alternatives

skinview3d (Minecraft-skin specific, less control over cosmetics), drei OrbitControls (heavier), a static image.
