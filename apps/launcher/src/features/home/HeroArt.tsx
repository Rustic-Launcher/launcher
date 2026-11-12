import { useMemo } from 'react';

/** Deterministic pseudo-random generator (mulberry32) so the art is stable between renders. */
function rng(seed: number) {
  let a = seed;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function Cube({
  x,
  y,
  s,
  top,
  left,
  right,
}: {
  x: number;
  y: number;
  s: number;
  top: string;
  left: string;
  right: string;
}) {
  const h = s / 2;
  return (
    <g transform={`translate(${x} ${y})`}>
      <polygon points={`0,${-h} ${s},0 0,${h} ${-s},0`} fill={top} />
      <polygon points={`${-s},0 0,${h} 0,${h + s} ${-s},${s}`} fill={left} />
      <polygon points={`${s},0 0,${h} 0,${h + s} ${s},${s}`} fill={right} />
    </g>
  );
}

/**
 * Original Launchpad hero illustration: a night sky of pixel stars, a floating voxel island and
 * drifting cubes in the accent colour. Pure SVG, drawn in code (no third-party artwork).
 */
export function HeroArt() {
  const stars = useMemo(() => {
    const r = rng(7);
    return Array.from({ length: 70 }, () => ({
      x: r() * 1000,
      y: r() * 420,
      s: r() < 0.15 ? 4 : 2,
      o: 0.25 + r() * 0.6,
    }));
  }, []);
  return (
    <svg
      aria-hidden
      viewBox="0 0 1000 420"
      preserveAspectRatio="xMidYMid slice"
      className="absolute inset-0 size-full"
    >
      <defs>
        <radialGradient id="hero-glow" cx="70%" cy="40%" r="60%">
          <stop offset="0" stopColor="var(--rl-accent)" stopOpacity="0.16" />
          <stop offset="1" stopColor="var(--rl-accent)" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="hero-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#11131a" />
          <stop offset="1" stopColor="#1a1d27" />
        </linearGradient>
      </defs>
      <rect width="1000" height="420" fill="url(#hero-sky)" />
      <rect width="1000" height="420" fill="url(#hero-glow)" />
      {stars.map((s, i) => (
        <rect key={i} x={s.x} y={s.y} width={s.s} height={s.s} fill="#ffffff" opacity={s.o} />
      ))}
      {/* floating voxel island, right */}
      <g transform="translate(820 250)">
        <Cube x={0} y={0} s={34} top="#4d6b4f" left="#3a2b22" right="#2c211b" />
        <Cube x={34} y={17} s={34} top="#58795a" left="#3a2b22" right="#2c211b" />
        <Cube x={-34} y={17} s={34} top="#4d6b4f" left="#3a2b22" right="#2c211b" />
        <Cube x={0} y={34} s={34} top="#58795a" left="#3a2b22" right="#2c211b" />
        <Cube
          x={0}
          y={-30}
          s={16}
          top="var(--rl-accent)"
          left="var(--rl-accent-active)"
          right="var(--rl-accent-hover)"
        />
      </g>
      <Cube
        x={700}
        y={90}
        s={14}
        top="var(--rl-accent-hover)"
        left="var(--rl-accent-active)"
        right="var(--rl-accent)"
      />
      <Cube x={930} y={120} s={10} top="#6b7088" left="#3b3f4f" right="#4b5063" />
      <Cube x={140} y={70} s={9} top="#6b7088" left="#3b3f4f" right="#4b5063" />
      <Cube
        x={380}
        y={350}
        s={12}
        top="var(--rl-accent)"
        left="var(--rl-accent-active)"
        right="var(--rl-accent-hover)"
      />
    </svg>
  );
}
