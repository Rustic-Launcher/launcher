/**
 * Mock Rustic marketplace (ADR 0155): creators' projects as the desktop side lists them
 * (`rustic:<id>`, a banner from Rustic's CDN, verified, priced, native). Names, people and texts
 * are invented; the banners are generated SVG art (no third-party image).
 */

/** Original banner art: voxel hills, a sun and a motif under a hue gradient sky. */
export function bannerArt(
  hue: number,
  motif: 'islands' | 'lanterns' | 'paths' | 'arena' | 'village' | 'sun',
): string {
  const h2 = (hue + 28) % 360;
  const h3 = (hue + 50) % 360;
  const hills = Array.from({ length: 24 }, (_, i) => 22 + (((hue + 11) * (i + 3)) % 34));
  const bars = hills
    .map(
      (h, i) =>
        `<rect x="${i * 10}" y="${135 - h}" width="10.5" height="${h}" fill="hsl(${h3} 38% 14%)" fill-opacity="${(0.6 + (i % 3) * 0.12).toFixed(2)}"/>` +
        `<rect x="${i * 10}" y="${135 - h}" width="10.5" height="2.5" fill="white" fill-opacity="0.14"/>`,
    )
    .join('');
  const motifs: Record<typeof motif, string> = {
    islands: [40, 110, 175]
      .map(
        (x, i) =>
          `<g transform="translate(${x} ${38 + i * 9})"><rect width="34" height="8" fill="hsl(110 45% 42%)"/><rect y="8" width="34" height="6" fill="hsl(28 45% 30%)"/><path d="M6 14h22l-8 12h-6z" fill="hsl(28 40% 22%)"/></g>`,
      )
      .join(''),
    lanterns: [50, 95, 140, 185]
      .map(
        (x, i) =>
          `<g transform="translate(${x} ${30 + (i % 2) * 14})"><line x1="6" y1="-30" x2="6" y2="0" stroke="white" stroke-opacity=".35"/><rect width="12" height="16" rx="2" fill="hsl(40 95% 62%)"/><circle cx="6" cy="8" r="12" fill="hsl(40 95% 62%)" fill-opacity=".18"/></g>`,
      )
      .join(''),
    paths: `<path d="M0 120 C 60 95, 120 130, 240 88" stroke="hsl(45 80% 70%)" stroke-width="6" fill="none" stroke-dasharray="10 6"/>`,
    arena: `<g transform="translate(120 70)"><ellipse rx="70" ry="22" fill="none" stroke="white" stroke-opacity=".4" stroke-width="3"/><path d="M-12 -20 l12 -18 l12 18z" fill="hsl(0 70% 60%)"/></g>`,
    village: [60, 120, 170]
      .map(
        (x, i) =>
          `<g transform="translate(${x} ${74 - i * 4})"><rect width="26" height="20" fill="hsl(30 35% 70%)"/><path d="M-3 0 L13 -14 L29 0z" fill="hsl(8 55% 45%)"/><rect x="10" y="9" width="6" height="11" fill="hsl(28 40% 25%)"/></g>`,
      )
      .join(''),
    sun: `<circle cx="120" cy="64" r="26" fill="hsl(40 100% 70%)" fill-opacity=".9"/><circle cx="120" cy="64" r="44" fill="hsl(40 100% 70%)" fill-opacity=".15"/>`,
  };
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 135" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="g" x1="0" y1="0" x2="0.3" y2="1"><stop offset="0" stop-color="hsl(${hue} 70% 62%)"/><stop offset=".55" stop-color="hsl(${h2} 55% 38%)"/><stop offset="1" stop-color="hsl(${h3} 45% 16%)"/></linearGradient></defs><rect width="240" height="135" fill="url(#g)"/><circle cx="200" cy="26" r="11" fill="white" fill-opacity=".3"/>${motifs[motif]}${bars}</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** The signed-in mock user's own projects (the cloud mock's "My creations"). */
export const MY_PROJECT_IDS = ['cozy_village', 'sam_tools'] as const;
