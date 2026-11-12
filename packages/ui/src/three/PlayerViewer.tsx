/**
 * 3D player preview (ADR 0023): react-three-fiber, no drei. The player is an ORIGINAL blocky
 * figure built from boxes in code (no Mojang/Lunar model or texture). Cosmetics are attached by slot
 * as simple procedural shapes until real assets come from the cosmetics pipeline. With `skin`, the
 * same boxes wear the player's own skin (their profile's texture, loaded at run time, never
 * bundled); the coloured figure shows while it loads or when it cannot.
 */
import { Canvas, useFrame } from '@react-three/fiber';
import { useEffect, useMemo, useRef, useState, type PointerEvent as RPointerEvent } from 'react';
import type { Group, Mesh, Texture } from 'three';
import { cn } from '../lib/cn';
import { useAnimationsPaused } from '../lib/usePaused';
import { usePrefersReducedMotion } from '../lib/useReducedMotion';
import { loadSkinTexture, skinBoxGeometry, skinParts, type SkinPart } from './skin';

export type PlayerAnimation = 'idle' | 'walk' | 'wave' | 'dance' | 'cheer' | 'bow' | 'none';

export type CosmeticSlot =
  'head' | 'back' | 'top' | 'shoulder' | 'companion' | 'trail' | 'aura' | 'emote' | 'nametag';

export interface CosmeticVisual {
  slot: CosmeticSlot;
  /** Procedural shape id from the catalogue (e.g. "crown", "lantern", "fox"). */
  shape: string;
  color: string;
  accent?: string;
}

export interface PlayerColors {
  skin: string;
  hair: string;
  shirt: string;
  pants: string;
  shoes: string;
}

/**
 * Rustic's own default look (rust hoodie, charcoal trousers, light sneakers), deliberately unlike
 * the game's default characters (no teal shirt + blue trousers, no green shirt + orange hair).
 */
export const DEFAULT_PLAYER_COLORS: PlayerColors = {
  skin: '#B57F5B',
  hair: '#1C1917',
  shirt: '#C2561C',
  pants: '#36322E',
  shoes: '#E6E0D4',
};

export interface PlayerViewerProps {
  colors?: PlayerColors;
  cosmetics?: CosmeticVisual[];
  animation?: PlayerAnimation;
  /** Accessible description of what is shown. */
  label: string;
  className?: string;
  /** Initial yaw in radians. */
  yaw?: number;
  autoRotate?: boolean;
  /** Name shown above the head when a nametag cosmetic is equipped. */
  name?: string;
  /** Change this value to reset rotation and zoom ("Reset view"). */
  resetKey?: number;
  /** Allow wheel / +- zoom (default true). */
  zoomable?: boolean;
  /** The player's skin texture (a 64×64 or 64×32 PNG) and arm model. */
  skin?: { url: string; slim: boolean } | null;
  /** Lights: neutral studio (default), or warm sun from the upper left with a green bounce,
   * to stand in the Launchpad's forest backdrop. */
  ambience?: 'studio' | 'forest';
}

interface LoadedSkin {
  texture: Texture;
  /** 64, or 32 for a legacy skin (no outer layer but the hat, left limbs = right ones). */
  height: number;
  slim: boolean;
}

/** The skin texture once loaded; null while loading, on failure, or without a skin. */
function useSkin(skin: PlayerViewerProps['skin']): LoadedSkin | null {
  const [loaded, setLoaded] = useState<LoadedSkin | null>(null);
  const url = skin?.url;
  const slim = !!skin?.slim;
  useEffect(() => {
    if (!url) return;
    let live = true;
    let texture: Texture | null = null;
    loadSkinTexture(url)
      .then((t) => {
        texture = t;
        const img = t.image as { height: number };
        if (live) setLoaded({ texture: t, height: img.height, slim });
        else t.dispose();
      })
      .catch(() => undefined);
    return () => {
      live = false;
      texture?.dispose();
      setLoaded(null);
    };
  }, [url, slim]);
  return url ? loaded : null;
}

/** One skinned box; `outer` is the second layer (grown, alpha-tested, casts no shadow). */
function SkinBox({
  part,
  skin,
  position,
  inflate = 0,
}: {
  part: SkinPart;
  skin: LoadedSkin;
  position: [number, number, number];
  inflate?: number;
}) {
  const geometry = useMemo(
    () => skinBoxGeometry(part, U, inflate, skin.height),
    [part, inflate, skin.height],
  );
  useEffect(() => () => geometry.dispose(), [geometry]);
  const outer = inflate > 0;
  return (
    <mesh geometry={geometry} position={position} castShadow={!outer}>
      <meshStandardMaterial
        map={skin.texture}
        roughness={0.9}
        metalness={0}
        transparent={outer}
        alphaTest={outer ? 0.5 : 0}
        depthWrite={!outer}
      />
    </mesh>
  );
}

/** A limb or the torso: its base box and, on 64×64 skins, the outer layer around it. */
function SkinLimb({
  base,
  outer,
  skin,
  position,
}: {
  base: SkinPart;
  outer: SkinPart | null;
  skin: LoadedSkin;
  position: [number, number, number];
}) {
  return (
    <>
      <SkinBox part={base} skin={skin} position={position} />
      {outer && <SkinBox part={outer} skin={skin} position={position} inflate={0.25} />}
    </>
  );
}

const U = 1 / 16; // one "pixel" unit

function Box({
  size,
  position,
  color,
  emissive,
}: {
  size: [number, number, number];
  position: [number, number, number];
  color: string;
  emissive?: boolean;
}) {
  return (
    <mesh position={position} castShadow>
      <boxGeometry args={size} />
      <meshStandardMaterial
        color={color}
        roughness={0.85}
        metalness={0.05}
        emissive={emissive ? color : '#000000'}
        emissiveIntensity={emissive ? 0.6 : 0}
      />
    </mesh>
  );
}

function HeadCosmetic({ c }: { c: CosmeticVisual }) {
  const a = c.accent ?? c.color;
  switch (c.shape) {
    case 'crown':
      return (
        <group position={[0, 8.5 * U, 0]}>
          <Box size={[9 * U, 2 * U, 9 * U]} position={[0, 0, 0]} color={c.color} />
          {[-3, 0, 3].map((x) => (
            <Box
              key={x}
              size={[2 * U, 3 * U, 2 * U]}
              position={[x * U, 2 * U, 3.6 * U]}
              color={a}
              emissive
            />
          ))}
        </group>
      );
    case 'beanie':
      return (
        <group position={[0, 7 * U, 0]}>
          <Box size={[8.6 * U, 3 * U, 8.6 * U]} position={[0, 0, 0]} color={c.color} />
          <Box size={[2 * U, 2 * U, 2 * U]} position={[0, 2.5 * U, 0]} color={a} />
        </group>
      );
    case 'horns':
      return (
        <group position={[0, 8 * U, 0]}>
          <Box size={[1.5 * U, 4 * U, 1.5 * U]} position={[-3 * U, 1.5 * U, 0]} color={c.color} />
          <Box size={[1.5 * U, 4 * U, 1.5 * U]} position={[3 * U, 1.5 * U, 0]} color={c.color} />
        </group>
      );
    case 'cap':
      return (
        <group position={[0, 8 * U, 0]}>
          <Box size={[8.6 * U, 1.6 * U, 8.6 * U]} position={[0, 0, 0]} color={c.color} />
          <Box size={[8 * U, 0.5 * U, 3.5 * U]} position={[0, -0.6 * U, 5.5 * U]} color={a} />
        </group>
      );
    case 'antlers':
      return (
        <group position={[0, 8 * U, 0]}>
          {[-1, 1].map((s) => (
            <group key={s} position={[s * 3 * U, 0, 0]}>
              <Box size={[1 * U, 4 * U, 1 * U]} position={[0, 2 * U, 0]} color={c.color} />
              <Box
                size={[2.5 * U, 1 * U, 1 * U]}
                position={[s * 1.2 * U, 3.5 * U, 0]}
                color={c.color}
              />
              <Box size={[1 * U, 2 * U, 1 * U]} position={[s * 2.2 * U, 4.5 * U, 0]} color={a} />
            </group>
          ))}
        </group>
      );
    case 'flower':
      return (
        <group position={[2.5 * U, 8.5 * U, 2 * U]}>
          {[
            [1, 0],
            [-1, 0],
            [0, 1],
            [0, -1],
          ].map(([x, z]) => (
            <Box
              key={`${x}${z}`}
              size={[1.4 * U, 0.6 * U, 1.4 * U]}
              position={[(x ?? 0) * 1.2 * U, 0, (z ?? 0) * 1.2 * U]}
              color={c.color}
            />
          ))}
          <Box size={[1.2 * U, 0.9 * U, 1.2 * U]} position={[0, 0.2 * U, 0]} color={a} emissive />
        </group>
      );
    default: // halo
      return (
        <mesh position={[0, 10 * U, 0]} rotation={[Math.PI / 2, 0, 0]}>
          <torusGeometry args={[3.5 * U, 0.5 * U, 8, 24]} />
          <meshStandardMaterial color={c.color} emissive={c.color} emissiveIntensity={0.8} />
        </mesh>
      );
  }
}

function BackCosmetic({ c }: { c: CosmeticVisual }) {
  // Back items are compact objects (backpack, lantern, quiver, wings stub); never cape-like.
  const a = c.accent ?? c.color;
  if (c.shape === 'wings') {
    return (
      <group position={[0, 18 * U, -3 * U]}>
        <Box size={[1 * U, 7 * U, 5 * U]} position={[-4 * U, 0, -2 * U]} color={c.color} />
        <Box size={[1 * U, 7 * U, 5 * U]} position={[4 * U, 0, -2 * U]} color={c.color} />
      </group>
    );
  }
  if (c.shape === 'lantern') {
    return (
      <group position={[2.5 * U, 15 * U, -3 * U]}>
        <Box size={[0.4 * U, 3 * U, 0.4 * U]} position={[0, 2.5 * U, 0]} color="#3a3330" />
        <Box size={[3 * U, 0.6 * U, 3 * U]} position={[0, 1 * U, 0]} color={c.color} />
        <Box size={[2.4 * U, 3 * U, 2.4 * U]} position={[0, -0.8 * U, 0]} color={a} emissive />
        <Box size={[3 * U, 0.6 * U, 3 * U]} position={[0, -2.6 * U, 0]} color={c.color} />
      </group>
    );
  }
  if (c.shape === 'quiver') {
    return (
      <group position={[0, 18 * U, -3 * U]} rotation={[0, 0, 0.45]}>
        <Box size={[2.6 * U, 10 * U, 2.6 * U]} position={[0, 0, 0]} color={c.color} />
        {[-0.6, 0, 0.6].map((x) => (
          <Box key={x} size={[0.4 * U, 2.5 * U, 0.4 * U]} position={[x * U, 6 * U, 0]} color={a} />
        ))}
      </group>
    );
  }
  return (
    <group position={[0, 18 * U, -3.5 * U]}>
      <Box size={[6 * U, 7 * U, 3 * U]} position={[0, 0, 0]} color={c.color} />
      <Box size={[4 * U, 2 * U, 0.5 * U]} position={[0, -1 * U, -1.6 * U]} color={a} />
    </group>
  );
}

function TopCosmetic({ c }: { c: CosmeticVisual }) {
  return (
    <group position={[0, 18 * U, 0]}>
      <Box size={[8.6 * U, 12.4 * U, 4.6 * U]} position={[0, 0, 0]} color={c.color} />
      <Box
        size={[8.8 * U, 1.5 * U, 4.8 * U]}
        position={[0, -3 * U, 0]}
        color={c.accent ?? c.color}
      />
    </group>
  );
}

function Creature({ c, scale = 1 }: { c: CosmeticVisual; scale?: number }) {
  const a = c.accent ?? '#1a1a1a';
  if (c.shape === 'slime') {
    return (
      <group scale={scale}>
        <mesh position={[0, 1.5 * U, 0]}>
          <boxGeometry args={[4.5 * U, 4.5 * U, 4.5 * U]} />
          <meshStandardMaterial color={c.color} transparent opacity={0.75} roughness={0.3} />
        </mesh>
        <Box size={[2.5 * U, 2.5 * U, 2.5 * U]} position={[0, 1.2 * U, 0]} color={c.color} />
        <Box size={[0.7 * U, 0.7 * U, 0.2 * U]} position={[-0.9 * U, 2.2 * U, 2.3 * U]} color={a} />
        <Box size={[0.7 * U, 0.7 * U, 0.2 * U]} position={[0.9 * U, 2.2 * U, 2.3 * U]} color={a} />
      </group>
    );
  }
  if (c.shape === 'bird') {
    return (
      <group scale={scale}>
        <Box size={[3 * U, 3 * U, 3.5 * U]} position={[0, 0, 0]} color={c.color} />
        <Box size={[2.6 * U, 2.6 * U, 2.6 * U]} position={[0, 2.4 * U, 1 * U]} color={c.color} />
        <Box
          size={[1 * U, 0.8 * U, 1.4 * U]}
          position={[0, 2.2 * U, 2.8 * U]}
          color={c.accent ?? '#E8B84A'}
        />
        <Box
          size={[0.5 * U, 2.2 * U, 3 * U]}
          position={[-1.75 * U, 0.2 * U, -0.2 * U]}
          color={c.accent ?? c.color}
        />
        <Box
          size={[0.5 * U, 2.2 * U, 3 * U]}
          position={[1.75 * U, 0.2 * U, -0.2 * U]}
          color={c.accent ?? c.color}
        />
        <Box
          size={[0.5 * U, 0.5 * U, 0.2 * U]}
          position={[-0.7 * U, 2.8 * U, 2.35 * U]}
          color="#1a1a1a"
        />
        <Box
          size={[0.5 * U, 0.5 * U, 0.2 * U]}
          position={[0.7 * U, 2.8 * U, 2.35 * U]}
          color="#1a1a1a"
        />
      </group>
    );
  }
  return (
    <group scale={scale}>
      <Box size={[4 * U, 3 * U, 5 * U]} position={[0, 0, 0]} color={c.color} />
      <Box size={[3 * U, 3 * U, 3 * U]} position={[0, 2 * U, 2.5 * U]} color={c.color} />
      <Box size={[0.6 * U, 0.6 * U, 0.2 * U]} position={[-0.8 * U, 2.5 * U, 4.05 * U]} color={a} />
      <Box size={[0.6 * U, 0.6 * U, 0.2 * U]} position={[0.8 * U, 2.5 * U, 4.05 * U]} color={a} />
      {c.shape === 'fox' && (
        <>
          <Box size={[1 * U, 1.5 * U, 0.6 * U]} position={[-1 * U, 4 * U, 2 * U]} color={c.color} />
          <Box size={[1 * U, 1.5 * U, 0.6 * U]} position={[1 * U, 4 * U, 2 * U]} color={c.color} />
          <Box size={[1.5 * U, 1.5 * U, 4 * U]} position={[0, 0.5 * U, -4 * U]} color={c.color} />
        </>
      )}
      {c.shape === 'cat' && (
        <>
          <Box
            size={[0.9 * U, 1.2 * U, 0.6 * U]}
            position={[-1 * U, 3.9 * U, 2.2 * U]}
            color={c.color}
          />
          <Box
            size={[0.9 * U, 1.2 * U, 0.6 * U]}
            position={[1 * U, 3.9 * U, 2.2 * U]}
            color={c.color}
          />
          <Box
            size={[0.8 * U, 0.8 * U, 4.5 * U]}
            position={[0, 1.6 * U, -4 * U]}
            color={c.accent ?? c.color}
          />
        </>
      )}
    </group>
  );
}

function Floating({
  children,
  offset = 0,
  speed = 1.6,
  amp = 0.06,
}: {
  children: React.ReactNode;
  offset?: number;
  speed?: number;
  amp?: number;
}) {
  const ref = useRef<Group>(null);
  const reduced = usePrefersReducedMotion();
  useFrame(({ clock }) => {
    if (!ref.current || reduced) return;
    ref.current.position.y = Math.sin(clock.elapsedTime * speed + offset) * amp;
    ref.current.rotation.y = Math.sin(clock.elapsedTime * 0.5 + offset) * 0.4;
  });
  return <group ref={ref}>{children}</group>;
}

function Aura({ c }: { c: CosmeticVisual }) {
  const ref = useRef<Group>(null);
  const reduced = usePrefersReducedMotion();
  useFrame((_, dt) => {
    if (ref.current && !reduced) ref.current.rotation.y += dt * 0.8;
  });
  const parts = useMemo(() => Array.from({ length: 10 }, (_, i) => i), []);
  return (
    <group ref={ref}>
      {parts.map((i) => {
        const ang = (i / parts.length) * Math.PI * 2;
        return (
          <mesh
            key={i}
            position={[Math.cos(ang) * 0.55, 0.2 + (i % 3) * 0.55, Math.sin(ang) * 0.55]}
          >
            <boxGeometry args={[0.05, 0.05, 0.05]} />
            <meshStandardMaterial color={c.color} emissive={c.color} emissiveIntensity={1.2} />
          </mesh>
        );
      })}
    </group>
  );
}

function Trail({ c }: { c: CosmeticVisual }) {
  return (
    <group position={[0, 0.02, -0.35]}>
      {[0, 1, 2, 3, 4].map((i) => (
        <mesh
          key={i}
          position={[i % 2 ? 0.08 : -0.08, 0, -i * 0.16]}
          rotation={[-Math.PI / 2, 0, 0]}
        >
          <planeGeometry args={[0.12 - i * 0.015, 0.12 - i * 0.015]} />
          <meshStandardMaterial
            color={c.color}
            emissive={c.color}
            emissiveIntensity={0.8}
            transparent
            opacity={0.9 - i * 0.16}
          />
        </mesh>
      ))}
    </group>
  );
}

function Figure({
  colors,
  cosmetics,
  animation,
  skin,
}: {
  colors: PlayerColors;
  cosmetics: CosmeticVisual[];
  animation: PlayerAnimation;
  skin: LoadedSkin | null;
}) {
  const leftArm = useRef<Group>(null);
  const rightArm = useRef<Group>(null);
  const leftLeg = useRef<Group>(null);
  const rightLeg = useRef<Group>(null);
  const body = useRef<Group>(null);
  const head = useRef<Mesh>(null);
  const reduced = usePrefersReducedMotion();

  useFrame(({ clock }) => {
    const t = clock.elapsedTime;
    const la = leftArm.current;
    const ra = rightArm.current;
    const ll = leftLeg.current;
    const rl = rightLeg.current;
    const b = body.current;
    if (!la || !ra || !ll || !rl || !b) return;
    if (reduced || animation === 'none') {
      la.rotation.set(0, 0, 0);
      ra.rotation.set(0, 0, 0);
      ll.rotation.set(0, 0, 0);
      rl.rotation.set(0, 0, 0);
      b.position.y = 0;
      return;
    }
    const s = Math.sin(t * 6);
    switch (animation) {
      case 'walk':
        la.rotation.set(s * 0.6, 0, 0);
        ra.rotation.set(-s * 0.6, 0, 0);
        ll.rotation.set(-s * 0.6, 0, 0);
        rl.rotation.set(s * 0.6, 0, 0);
        b.position.y = Math.abs(Math.cos(t * 6)) * 0.02;
        break;
      case 'wave':
        la.rotation.set(0, 0, 0);
        ra.rotation.set(0, 0, 2.6 + Math.sin(t * 8) * 0.35);
        ll.rotation.set(0, 0, 0);
        rl.rotation.set(0, 0, 0);
        b.position.y = 0;
        break;
      case 'dance':
        la.rotation.set(0, 0, -2.2 + Math.sin(t * 5) * 0.5);
        ra.rotation.set(0, 0, 2.2 + Math.sin(t * 5 + Math.PI) * 0.5);
        ll.rotation.set(Math.sin(t * 5) * 0.3, 0, 0);
        rl.rotation.set(-Math.sin(t * 5) * 0.3, 0, 0);
        b.position.y = Math.abs(Math.sin(t * 5)) * 0.06;
        break;
      case 'cheer': {
        const up = Math.sin(t * 7) * 0.25;
        la.rotation.set(0, 0, -2.9 - up);
        ra.rotation.set(0, 0, 2.9 + up);
        ll.rotation.set(0, 0, 0);
        rl.rotation.set(0, 0, 0);
        b.position.y = Math.max(0, Math.sin(t * 7)) * 0.08;
        break;
      }
      case 'bow': {
        const k = (Math.sin(t * 2) + 1) / 2;
        la.rotation.set(-0.4 * k, 0, 0);
        ra.rotation.set(-0.4 * k, 0, 0);
        ll.rotation.set(0, 0, 0);
        rl.rotation.set(0, 0, 0);
        b.position.y = -0.02 * k;
        break;
      }
      default: {
        const breathe = Math.sin(t * 1.6) * 0.04;
        la.rotation.set(0, 0, -0.05 - breathe);
        ra.rotation.set(0, 0, 0.05 + breathe);
        ll.rotation.set(0, 0, 0);
        rl.rotation.set(0, 0, 0);
        b.position.y = 0;
      }
    }
  });

  const by = (slot: CosmeticSlot) => cosmetics.find((c) => c.slot === slot);
  const hat = by('head');
  const back = by('back');
  const top = by('top');
  const shoulder = by('shoulder');
  const companion = by('companion');
  const aura = by('aura');
  const trail = by('trail');

  // Skin layout: the model faces +Z, so the player's right limbs are the −X groups below.
  const parts = useMemo(() => (skin ? skinParts(skin.slim) : null), [skin]);
  const legacy = skin?.height === 32;
  const armX = skin?.slim ? 0.5 * U : 0;

  // Proportions (pixels): head 8, body 8x12x4, arms/legs 4x12x4. Feet at y=0.
  if (skin && parts)
    return (
      <group>
        <group ref={body}>
          <group ref={leftLeg} position={[-2 * U, 12 * U, 0]}>
            <SkinLimb
              base={parts.rightLeg.base}
              outer={legacy ? null : parts.rightLeg.outer}
              skin={skin}
              position={[0, -6 * U, 0]}
            />
          </group>
          <group ref={rightLeg} position={[2 * U, 12 * U, 0]}>
            <SkinLimb
              base={legacy ? parts.rightLeg.base : parts.leftLeg.base}
              outer={legacy ? null : parts.leftLeg.outer}
              skin={skin}
              position={[0, -6 * U, 0]}
            />
          </group>
          <SkinLimb
            base={parts.body.base}
            outer={legacy ? null : parts.body.outer}
            skin={skin}
            position={[0, 18 * U, 0]}
          />
          {top && <TopCosmetic c={top} />}
          <group ref={leftArm} position={[-6 * U, 23.5 * U, 0]}>
            <SkinLimb
              base={parts.rightArm.base}
              outer={legacy ? null : parts.rightArm.outer}
              skin={skin}
              position={[armX, -5.5 * U, 0]}
            />
          </group>
          <group ref={rightArm} position={[6 * U, 23.5 * U, 0]}>
            <SkinLimb
              base={legacy ? parts.rightArm.base : parts.leftArm.base}
              outer={legacy ? null : parts.leftArm.outer}
              skin={skin}
              position={[-armX, -5.5 * U, 0]}
            />
          </group>
          <group position={[0, 24 * U, 0]}>
            <SkinBox part={parts.head.base} skin={skin} position={[0, 4 * U, 0]} />
            <SkinBox part={parts.head.outer} skin={skin} position={[0, 4 * U, 0]} inflate={0.5} />
            {hat && <HeadCosmetic c={hat} />}
          </group>
          {back && <BackCosmetic c={back} />}
          {shoulder && (
            <group position={[-7 * U, 25.5 * U, 0]}>
              <Floating amp={0.01} speed={3}>
                <Creature c={shoulder} scale={0.7} />
              </Floating>
            </group>
          )}
        </group>
        {companion && (
          <group position={[0.75, 0.9, 0.2]}>
            <Floating>
              <Creature c={companion} />
            </Floating>
          </group>
        )}
        {aura && <Aura c={aura} />}
        {trail && <Trail c={trail} />}
      </group>
    );
  return (
    <group>
      <group ref={body}>
        {/* legs */}
        <group ref={leftLeg} position={[-2 * U, 12 * U, 0]}>
          <Box size={[4 * U, 10 * U, 4 * U]} position={[0, -5 * U, 0]} color={colors.pants} />
          <Box size={[4.1 * U, 2 * U, 4.1 * U]} position={[0, -11 * U, 0]} color={colors.shoes} />
        </group>
        <group ref={rightLeg} position={[2 * U, 12 * U, 0]}>
          <Box size={[4 * U, 10 * U, 4 * U]} position={[0, -5 * U, 0]} color={colors.pants} />
          <Box size={[4.1 * U, 2 * U, 4.1 * U]} position={[0, -11 * U, 0]} color={colors.shoes} />
        </group>
        {/* torso */}
        <Box size={[8 * U, 12 * U, 4 * U]} position={[0, 18 * U, 0]} color={colors.shirt} />
        {top && <TopCosmetic c={top} />}
        {/* arms: pivot at the shoulder */}
        <group ref={leftArm} position={[-6 * U, 23.5 * U, 0]}>
          <Box
            size={[4 * U, 9 * U, 4 * U]}
            position={[0, -4 * U, 0]}
            color={top ? top.color : colors.shirt}
          />
          <Box size={[4 * U, 3 * U, 4 * U]} position={[0, -10 * U, 0]} color={colors.skin} />
        </group>
        <group ref={rightArm} position={[6 * U, 23.5 * U, 0]}>
          <Box
            size={[4 * U, 9 * U, 4 * U]}
            position={[0, -4 * U, 0]}
            color={top ? top.color : colors.shirt}
          />
          <Box size={[4 * U, 3 * U, 4 * U]} position={[0, -10 * U, 0]} color={colors.skin} />
        </group>
        {/* head */}
        <group position={[0, 24 * U, 0]}>
          <mesh ref={head} position={[0, 4 * U, 0]} castShadow>
            <boxGeometry args={[8 * U, 8 * U, 8 * U]} />
            <meshStandardMaterial color={colors.skin} roughness={0.85} />
          </mesh>
          <Box size={[8.2 * U, 2.5 * U, 8.2 * U]} position={[0, 7 * U, 0]} color={colors.hair} />
          <Box size={[8.2 * U, 4 * U, 1 * U]} position={[0, 5 * U, -3.7 * U]} color={colors.hair} />
          {/* eyes */}
          <Box
            size={[1.4 * U, 1.2 * U, 0.2 * U]}
            position={[-1.8 * U, 4.2 * U, 4.05 * U]}
            color="#F4F1EC"
          />
          <Box
            size={[1.4 * U, 1.2 * U, 0.2 * U]}
            position={[1.8 * U, 4.2 * U, 4.05 * U]}
            color="#F4F1EC"
          />
          <Box
            size={[0.7 * U, 1.2 * U, 0.25 * U]}
            position={[-1.5 * U, 4.2 * U, 4.1 * U]}
            color="#2A3B5C"
          />
          <Box
            size={[0.7 * U, 1.2 * U, 0.25 * U]}
            position={[2.1 * U, 4.2 * U, 4.1 * U]}
            color="#2A3B5C"
          />
          <Box size={[3 * U, 0.6 * U, 0.2 * U]} position={[0, 2 * U, 4.05 * U]} color="#8E5E45" />
          {hat && <HeadCosmetic c={hat} />}
        </group>
        {back && <BackCosmetic c={back} />}
        {shoulder && (
          <group position={[-7 * U, 25.5 * U, 0]}>
            <Floating amp={0.01} speed={3}>
              <Creature c={shoulder} scale={0.7} />
            </Floating>
          </group>
        )}
      </group>
      {companion && (
        <group position={[0.75, 0.9, 0.2]}>
          <Floating>
            <Creature c={companion} />
          </Floating>
        </group>
      )}
      {aura && <Aura c={aura} />}
      {trail && <Trail c={trail} />}
    </group>
  );
}

function Rig({
  yawRef,
  autoRotate,
  children,
}: {
  yawRef: React.MutableRefObject<number>;
  autoRotate: boolean;
  children: React.ReactNode;
}) {
  const ref = useRef<Group>(null);
  const reduced = usePrefersReducedMotion();
  useFrame((_, dt) => {
    if (autoRotate && !reduced) yawRef.current += dt * 0.35;
    if (ref.current) ref.current.rotation.y += (yawRef.current - ref.current.rotation.y) * 0.18;
  });
  return <group ref={ref}>{children}</group>;
}

function Zoom({ zoomRef }: { zoomRef: React.MutableRefObject<number> }) {
  useFrame(({ camera }) => {
    const targetZ = 5.3 / zoomRef.current;
    const targetY = 1.2 + (zoomRef.current - 1) * 0.2;
    camera.position.z += (targetZ - camera.position.z) * 0.15;
    camera.position.y += (targetY - camera.position.y) * 0.15;
    camera.lookAt(0, 1.08 + (zoomRef.current - 1) * 0.35, 0);
  });
  return null;
}

/**
 * Interactive viewer: drag to rotate, arrow keys rotate when focused.
 */
export function PlayerViewer({
  colors = DEFAULT_PLAYER_COLORS,
  cosmetics = [],
  animation = 'idle',
  label,
  className,
  yaw = -0.5,
  autoRotate = false,
  resetKey,
  zoomable = true,
  skin,
  ambience = 'studio',
}: PlayerViewerProps) {
  const paused = useAnimationsPaused();
  const loadedSkin = useSkin(skin);
  const yawRef = useRef(yaw);
  const zoomRef = useRef(1);
  useEffect(() => {
    yawRef.current = yaw;
    zoomRef.current = 1;
  }, [resetKey, yaw]);
  const zoomBy = (f: number) => {
    zoomRef.current = Math.min(1.8, Math.max(0.7, zoomRef.current * f));
  };
  const drag = useRef<{ x: number } | null>(null);
  const [grabbing, setGrabbing] = useState(false);

  const onDown = (e: RPointerEvent<HTMLDivElement>) => {
    drag.current = { x: e.clientX };
    setGrabbing(true);
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onMove = (e: RPointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    yawRef.current += (e.clientX - drag.current.x) * 0.012;
    drag.current = { x: e.clientX };
  };
  const onUp = () => {
    drag.current = null;
    setGrabbing(false);
  };

  return (
    <div
      role="img"
      aria-label={label}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'ArrowLeft') yawRef.current -= 0.3;
        if (e.key === 'ArrowRight') yawRef.current += 0.3;
        if (zoomable && (e.key === '+' || e.key === '=')) zoomBy(1.12);
        if (zoomable && (e.key === '-' || e.key === '_')) zoomBy(1 / 1.12);
      }}
      onWheel={(e) => {
        if (zoomable) zoomBy(e.deltaY < 0 ? 1.08 : 1 / 1.08);
      }}
      onPointerDown={onDown}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
      className={cn(
        'rx-focus-ring relative size-full touch-none rounded-2xl outline-none',
        grabbing ? 'cursor-grabbing' : 'cursor-grab',
        className,
      )}
    >
      <Canvas
        shadows
        // Rests while a game runs or the window is hidden (html[data-paused], useAnimationsPaused).
        frameloop={paused ? 'never' : 'always'}
        dpr={[1, 2]}
        camera={{ position: [0, 1.2, 5.3], fov: 32 }}
        gl={{ antialias: true, alpha: true, preserveDrawingBuffer: true }}
        onCreated={({ camera }) => camera.lookAt(0, 1.08, 0)}
      >
        {ambience === 'forest' ? (
          <>
            <hemisphereLight args={['#cfe3c0', '#3d4a2a', 0.55]} />
            <directionalLight position={[-3, 5, 2.5]} intensity={1.9} color="#ffe2b0" castShadow />
            <directionalLight position={[3, 1.5, 1]} intensity={0.45} color="#9fd67a" />
          </>
        ) : (
          <>
            <ambientLight intensity={0.55} />
            <directionalLight position={[2.5, 4, 3]} intensity={1.6} castShadow />
            <directionalLight position={[-3, 2, -2]} intensity={0.5} color="#ffd9c2" />
          </>
        )}
        <Zoom zoomRef={zoomRef} />
        <Rig yawRef={yawRef} autoRotate={autoRotate}>
          <Figure colors={colors} cosmetics={cosmetics} animation={animation} skin={loadedSkin} />
        </Rig>
        <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -0.001, 0]} receiveShadow>
          <circleGeometry args={[0.9, 48]} />
          <shadowMaterial opacity={0.35} />
        </mesh>
      </Canvas>
    </div>
  );
}
