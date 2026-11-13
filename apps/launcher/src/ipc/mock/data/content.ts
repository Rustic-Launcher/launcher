/**
 * Mock content catalogue (Horizon registry, Modrinth, Rustic) and the pure rules the mock uses to
 * compute compatibility and search results. Names, authors and texts are invented; the shader
 * statuses follow horizon/docs/shaders.md and docs/HORIZON_FACTS.md A2 (Horizon .hshader packs: shader API
 * 0.1, experimental; Iris/OptiFine `shaders/` packs from the Horizon registry; Modrinth shader packs: not supported).
 */
import { bannerArt } from './marketplace';
import type {
  Compatibility,
  ContentDetail,
  ContentKind,
  ContentSearchQuery,
  ContentSource,
  ContentSummary,
  ContentVersion,
  ShaderFormat,
} from '../../types/content';

/** Horizon versions known by the mock, oldest first. */
export const HORIZON_LINE = ['v0.2.13', 'v0.2.14', 'v0.2.15'] as const;
export const LATEST_RELEASE = 'v0.2.15';

export type CategoryId =
  | 'performance'
  | 'utility'
  | 'ui'
  | 'pvp'
  | 'adventure'
  | 'decoration'
  | 'storage'
  | 'worldgen'
  | 'library'
  | 'tech'
  | 'cozy'
  | 'multiplayer'
  | 'realistic'
  | 'simple'
  | 'lightweight'
  | 'cinematic'
  | 'qol'
  | 'admin'
  | 'chat'
  | 'minigames'
  | 'protection'
  | 'integration';

export interface CatalogProject {
  id: string;
  kind: ContentKind;
  source: ContentSource;
  name: string;
  author: string;
  summary: string;
  hue: number;
  downloads: number;
  follows: number;
  updated_days: number;
  created_days: number;
  categories: CategoryId[];
  featured?: boolean;
  /** Horizon versions the latest release has builds for (mods, modpacks). */
  horizon: string[];
  version: string;
  side: 'client' | 'server' | 'both';
  license: string;
  java_only?: boolean;
  /** Gives an unfair advantage: never promoted, hidden by default (fair play policy). */
  fair_play_violation?: boolean;
  unsupported_format?: string;
  conflicts?: string[];
  deps?: { project_id: string; relation: 'required' | 'optional' | 'incompatible' }[];
  shader_format?: ShaderFormat;
  size_mb: number;
  /** Plugins: declared capabilities, shown in the description. */
  network?: string[];
  // Rustic marketplace (ADR 0155)
  /** A Rustic admin verified it. */
  verified?: boolean;
  /** Price in cents (EUR); paid projects are shown, never sold (creator sales are off). */
  price_cents?: number;
  /** A Horizon native mod (Rust compiled into the game on the player's PC). */
  native?: boolean;
  /** Banner art (the first screenshot). */
  banner?: string;
  /** Uploaded by its creator on the website. */
  marketplace?: boolean;
}

const ALL = [...HORIZON_LINE];

type P = Omit<CatalogProject, 'follows' | 'created_days' | 'license' | 'side' | 'size_mb'> &
  Partial<Pick<CatalogProject, 'follows' | 'created_days' | 'license' | 'side' | 'size_mb'>>;

function p(x: P): CatalogProject {
  return {
    follows: Math.round(x.downloads / 37),
    created_days: x.updated_days + 120 + (x.hue % 200),
    license: 'MIT',
    side: 'client',
    size_mb: 2,
    ...x,
  };
}

export const catalog: CatalogProject[] = [
  // ---------------- mods ----------------
  p({
    id: 'p_sodiumlike',
    kind: 'mod',
    source: 'horizon',
    name: 'Lumen Renderer',
    author: 'lumen-team',
    summary: 'Faster chunk rendering and steadier frame times on every GPU.',
    hue: 28,
    downloads: 1_840_000,
    updated_days: 3,
    categories: ['performance'],
    featured: true,
    horizon: ['v0.2.14', 'v0.2.15'],
    version: '2.2.0',
    license: 'Apache-2.0',
    size_mb: 3.4,
  }),
  p({
    id: 'p_minimap',
    kind: 'mod',
    source: 'horizon',
    name: 'Atlas Minimap',
    author: 'cartographe',
    summary: 'Minimap, waypoints and a full-screen world map, shareable with your party.',
    hue: 150,
    downloads: 1_210_000,
    updated_days: 6,
    categories: ['utility', 'adventure'],
    featured: true,
    horizon: ALL,
    version: '1.8.3',
  }),
  p({
    id: 'p_jei',
    kind: 'mod',
    source: 'horizon',
    name: 'Recipe Lens',
    author: 'lens-dev',
    summary: 'Look up any recipe and its uses in one keystroke.',
    hue: 205,
    downloads: 2_400_000,
    updated_days: 12,
    categories: ['utility', 'qol'],
    horizon: ALL,
    version: '3.0.1',
  }),
  p({
    id: 'p_waystones',
    kind: 'mod',
    source: 'modrinth',
    name: 'Waystones Plus',
    author: 'Wayfarer',
    summary: 'Activate waystones across the world and travel between them.',
    hue: 265,
    downloads: 640_000,
    updated_days: 20,
    categories: ['adventure'],
    horizon: ['v0.2.14', 'v0.2.15'],
    version: '0.9.2',
    side: 'both',
    deps: [{ project_id: 'p_corelib', relation: 'required' }],
  }),
  p({
    id: 'p_torches',
    kind: 'mod',
    source: 'horizon',
    name: 'Better Torches',
    author: 'flamme',
    summary: 'Torches that burn out, wall lanterns and soft flicker.',
    hue: 38,
    downloads: 210_000,
    updated_days: 48,
    categories: ['decoration'],
    horizon: ['v0.2.13', 'v0.2.14'],
    version: '1.1.0',
  }),
  p({
    id: 'p_zoom',
    kind: 'mod',
    source: 'horizon',
    name: 'Smooth Zoom',
    author: 'optique',
    summary: 'A smooth, scroll-adjustable zoom key.',
    hue: 190,
    downloads: 980_000,
    updated_days: 9,
    categories: ['utility', 'qol'],
    horizon: ALL,
    version: '1.0.4',
  }),
  p({
    id: 'p_old',
    kind: 'mod',
    source: 'modrinth',
    name: 'Legacy Chunk Tools',
    author: 'oldschool',
    summary: 'Chunk analysis tools for Java Edition servers.',
    hue: 0,
    downloads: 95_000,
    updated_days: 400,
    categories: ['utility'],
    horizon: [],
    version: '0.3.0',
    java_only: true,
  }),
  p({
    id: 'p_photon',
    kind: 'mod',
    source: 'modrinth',
    name: 'Photon Renderer',
    author: 'photonics',
    summary: 'An alternative renderer with aggressive culling.',
    hue: 55,
    downloads: 330_000,
    updated_days: 15,
    categories: ['performance'],
    horizon: ['v0.2.15'],
    version: '0.6.1',
    conflicts: ['p_sodiumlike'],
    deps: [{ project_id: 'p_sodiumlike', relation: 'incompatible' }],
  }),
  p({
    id: 'p_shulker',
    kind: 'mod',
    source: 'horizon',
    name: 'Shulker Peek',
    author: 'boîte-à-outils',
    summary: 'Aperçu du contenu des boîtes de Shulker au survol.',
    hue: 300,
    downloads: 420_000,
    updated_days: 30,
    categories: ['qol', 'storage'],
    horizon: ALL,
    version: '1.4.0',
  }),
  p({
    id: 'p_hud',
    kind: 'mod',
    source: 'rustic',
    name: 'Clean HUD',
    author: 'Rustic',
    summary: 'A tidy, movable HUD: armour, potions, coordinates, clock.',
    hue: 22,
    downloads: 760_000,
    updated_days: 2,
    categories: ['ui'],
    featured: true,
    horizon: ALL,
    version: '1.2.0',
  }),
  p({
    id: 'p_keystrokes',
    kind: 'mod',
    source: 'rustic',
    name: 'Keystrokes Overlay',
    author: 'Rustic',
    summary: 'Show WASD, mouse buttons and CPS on screen for recordings.',
    hue: 12,
    downloads: 520_000,
    updated_days: 5,
    categories: ['ui', 'pvp'],
    horizon: ALL,
    version: '1.0.2',
  }),
  p({
    id: 'p_fps',
    kind: 'mod',
    source: 'rustic',
    name: 'FPS Display',
    author: 'Rustic',
    summary: 'Frame rate, frame time graph and ping in a corner.',
    hue: 18,
    downloads: 690_000,
    updated_days: 5,
    categories: ['ui', 'performance'],
    horizon: ALL,
    version: '1.1.0',
  }),
  p({
    id: 'p_freelook',
    kind: 'mod',
    source: 'horizon',
    name: 'Freelook',
    author: 'regard',
    summary: 'Look around without turning your character.',
    hue: 175,
    downloads: 300_000,
    updated_days: 4,
    categories: ['utility'],
    horizon: ['v0.2.15'],
    version: '0.4.0',
  }),
  p({
    id: 'p_xray',
    kind: 'mod',
    source: 'modrinth',
    name: 'Ore Vision X',
    author: 'unknown',
    summary: 'See ores through walls.',
    hue: 330,
    downloads: 51_000,
    updated_days: 90,
    categories: ['utility'],
    horizon: ALL,
    version: '1.0.0',
    fair_play_violation: true,
  }),
  p({
    id: 'p_create',
    kind: 'mod',
    source: 'modrinth',
    name: 'Cogwork Machines',
    author: 'gearsmith',
    summary: 'Rotational power, belts and contraptions.',
    hue: 32,
    downloads: 3_100_000,
    updated_days: 11,
    categories: ['tech'],
    horizon: [],
    version: '6.0.4',
    java_only: true,
    side: 'both',
    size_mb: 21,
  }),
  p({
    id: 'p_biomes',
    kind: 'mod',
    source: 'horizon',
    name: 'Wildlands Biomes',
    author: 'terraform',
    summary: 'Forty new biomes and smoother terrain transitions.',
    hue: 110,
    downloads: 870_000,
    updated_days: 8,
    categories: ['worldgen', 'adventure'],
    horizon: ['v0.2.14', 'v0.2.15'],
    version: '2.0.0',
    side: 'both',
    size_mb: 9,
  }),
  p({
    id: 'p_storage',
    kind: 'mod',
    source: 'horizon',
    name: 'Crate & Barrel Storage',
    author: 'rangement',
    summary: 'Stackable crates and barrels with item displays.',
    hue: 45,
    downloads: 260_000,
    updated_days: 14,
    categories: ['storage', 'decoration'],
    horizon: ['v0.2.15'],
    version: '1.0.1',
    side: 'both',
    deps: [{ project_id: 'p_corelib', relation: 'required' }],
  }),
  p({
    id: 'p_dynlights',
    kind: 'mod',
    source: 'horizon',
    name: 'Handheld Lights',
    author: 'lueur',
    summary: 'Torches and glowstone light the world while you hold them.',
    hue: 50,
    downloads: 1_020_000,
    updated_days: 7,
    categories: ['decoration', 'qol'],
    horizon: ALL,
    version: '1.6.0',
    deps: [
      { project_id: 'p_corelib', relation: 'required' },
      { project_id: 'p_sodiumlike', relation: 'optional' },
    ],
  }),
  p({
    id: 'p_corelib',
    kind: 'mod',
    source: 'horizon',
    name: 'Horizon Core Lib',
    author: 'horizon-community',
    summary: 'Shared code used by many Horizon mods.',
    hue: 220,
    downloads: 2_900_000,
    updated_days: 10,
    categories: ['library'],
    horizon: ALL,
    version: '4.1.0',
    side: 'both',
    size_mb: 1.1,
  }),
  // ---------------- modpacks ----------------
  p({
    id: 'pack_essentials',
    kind: 'modpack',
    source: 'horizon',
    name: 'Horizon Essentials',
    author: 'horizon-community',
    summary: 'Performance, map, recipes and quality of life: vanilla, but better.',
    hue: 255,
    downloads: 410_000,
    updated_days: 4,
    categories: ['qol', 'performance'],
    featured: true,
    horizon: ['v0.2.15'],
    version: '1.4.0',
    size_mb: 180,
  }),
  p({
    id: 'pack_cobble',
    kind: 'modpack',
    source: 'modrinth',
    name: 'Cobblestone Odyssey',
    author: 'odyssey-team',
    summary: 'An adventure pack with dungeons, new biomes and quests.',
    hue: 95,
    downloads: 220_000,
    updated_days: 9,
    categories: ['adventure'],
    horizon: ['v0.2.14', 'v0.2.15'],
    version: '2.0.1',
    size_mb: 240,
  }),
  p({
    id: 'pack_pvp',
    kind: 'modpack',
    source: 'rustic',
    name: 'Rustic PvP Pack',
    author: 'Rustic',
    summary: 'HUD, keystrokes, zoom and a crisp 16x pack, tuned for duels.',
    hue: 8,
    downloads: 150_000,
    updated_days: 2,
    categories: ['pvp'],
    horizon: ['v0.2.15'],
    version: '1.0.0',
    size_mb: 40,
  }),
  p({
    id: 'pack_tech',
    kind: 'modpack',
    source: 'modrinth',
    name: 'Industrial Dawn',
    author: 'gearsmith',
    summary: 'Factories, power networks and automation.',
    hue: 35,
    downloads: 510_000,
    updated_days: 21,
    categories: ['tech'],
    horizon: [],
    version: '3.2.0',
    java_only: true,
    size_mb: 420,
  }),
  p({
    id: 'pack_cozy',
    kind: 'modpack',
    source: 'horizon',
    name: 'Village douillet',
    author: 'cocon',
    summary: 'Cuisine, déco et petits animaux pour une partie tranquille.',
    hue: 340,
    downloads: 88_000,
    updated_days: 35,
    categories: ['cozy', 'decoration'],
    horizon: ['v0.2.14'],
    version: '0.8.0',
    size_mb: 120,
  }),
  // ---------------- resource packs ----------------
  p({
    id: 'p_rp_faithful',
    kind: 'resourcepack',
    source: 'modrinth',
    name: 'Clarity 32x',
    author: 'pixel-atelier',
    summary: 'Sharper 32x textures that keep the original look.',
    hue: 120,
    downloads: 1_500_000,
    updated_days: 18,
    categories: ['simple'],
    featured: true,
    horizon: ALL,
    version: '5.2',
    size_mb: 28,
  }),
  p({
    id: 'p_rp_ui',
    kind: 'resourcepack',
    source: 'rustic',
    name: 'Soft UI',
    author: 'Rustic',
    summary: 'Rounded, low-contrast menus that match the launcher.',
    hue: 24,
    downloads: 240_000,
    updated_days: 6,
    categories: ['ui'],
    horizon: ALL,
    version: '1.3',
    size_mb: 3,
  }),
  p({
    id: 'p_rp_dark',
    kind: 'resourcepack',
    source: 'rustic',
    name: 'Nuit Profonde UI',
    author: 'Rustic',
    summary: 'Interface sombre pour les longues sessions de nuit.',
    hue: 250,
    downloads: 310_000,
    updated_days: 3,
    categories: ['ui'],
    horizon: ALL,
    version: '2.0',
    size_mb: 2,
  }),
  p({
    id: 'p_rp_pvp',
    kind: 'resourcepack',
    source: 'modrinth',
    name: 'Crisp PvP 16x',
    author: 'duelist',
    summary: 'Short swords, clean particles and a readable hotbar.',
    hue: 0,
    downloads: 700_000,
    updated_days: 40,
    categories: ['pvp', 'simple'],
    horizon: ALL,
    version: '3.1',
    size_mb: 6,
  }),
  p({
    id: 'p_rp_ctm',
    kind: 'resourcepack',
    source: 'horizon',
    name: 'Connected Glass',
    author: 'verrier',
    summary: 'Seamless glass and bookshelves (connected textures, read natively by Horizon).',
    hue: 185,
    downloads: 450_000,
    updated_days: 26,
    categories: ['realistic'],
    horizon: ALL,
    version: '1.7',
    size_mb: 4,
  }),
  // ---------------- shaders ----------------
  p({
    id: 'p_sh_golden',
    kind: 'shader',
    source: 'horizon',
    name: 'Golden Hour',
    author: 'Horizon',
    summary: 'A warm sky and two post passes. Horizon’s example pack.',
    hue: 36,
    downloads: 380_000,
    updated_days: 12,
    categories: ['lightweight', 'cinematic'],
    featured: true,
    horizon: ALL,
    version: '1.0.0',
    shader_format: 'horizon',
    size_mb: 0.4,
  }),
  p({
    id: 'p_sh_nocturne',
    kind: 'shader',
    source: 'horizon',
    name: 'Nocturne',
    author: 'astre',
    summary: 'Moonlit nights, starfields and a soft bloom.',
    hue: 240,
    downloads: 120_000,
    updated_days: 5,
    categories: ['cinematic'],
    horizon: ALL,
    version: '0.3.0',
    shader_format: 'horizon',
    size_mb: 0.6,
  }),
  p({
    id: 'p_sh_vivid',
    kind: 'shader',
    source: 'horizon',
    name: 'Vivid Lite',
    author: 'lumiere',
    summary: 'Light shadows and water for mid-range GPUs.',
    hue: 160,
    downloads: 2_200_000,
    updated_days: 30,
    categories: ['lightweight'],
    horizon: ALL,
    version: '4.5',
    shader_format: 'shaders_dir',
    size_mb: 1.2,
  }),
  p({
    id: 'p_sh_ultra',
    kind: 'shader',
    source: 'modrinth',
    name: 'Photoreal Ultra',
    author: 'raytrace-fan',
    summary: 'Volumetric clouds, reflections and long shadows.',
    hue: 200,
    downloads: 1_100_000,
    updated_days: 22,
    categories: ['realistic', 'cinematic'],
    horizon: ALL,
    version: '2.1',
    shader_format: 'shaders_dir',
    size_mb: 3.5,
  }),
  p({
    id: 'p_sh_legacy',
    kind: 'shader',
    source: 'modrinth',
    name: 'Retro GLSL Pack',
    author: 'oldschool',
    summary: 'A classic pack from the early shader days.',
    hue: 280,
    downloads: 60_000,
    updated_days: 900,
    categories: ['lightweight'],
    horizon: ALL,
    version: '1.2',
    shader_format: 'shaders_dir',
  }),
  // ---------------- datapacks ----------------
  p({
    id: 'p_dp_trees',
    kind: 'datapack',
    source: 'horizon',
    name: 'Bigger Trees',
    author: 'forestier',
    summary: 'Taller oaks, fallen logs and mossy roots.',
    hue: 105,
    downloads: 190_000,
    updated_days: 16,
    categories: ['worldgen'],
    horizon: ALL,
    version: '1.2.0',
    side: 'server',
  }),
  p({
    id: 'p_dp_graves',
    kind: 'datapack',
    source: 'rustic',
    name: 'Graves',
    author: 'Rustic',
    summary: 'Your items wait in a grave where you fell.',
    hue: 270,
    downloads: 330_000,
    updated_days: 8,
    categories: ['qol'],
    horizon: ALL,
    version: '2.3.0',
    side: 'server',
  }),
  p({
    id: 'p_dp_coords',
    kind: 'datapack',
    source: 'modrinth',
    name: 'Coordinates HUD',
    author: 'boussole',
    summary: 'Coordinates and direction in the action bar, toggled per player.',
    hue: 60,
    downloads: 410_000,
    updated_days: 50,
    categories: ['utility'],
    horizon: ALL,
    version: '1.0.5',
    side: 'server',
  }),
  p({
    id: 'p_dp_tweaks',
    kind: 'datapack',
    source: 'modrinth',
    name: 'Small Tweaks Bundle',
    author: 'tweakers',
    summary: 'Anti-creeper-grief, more mob heads, silence mobs.',
    hue: 140,
    downloads: 600_000,
    updated_days: 27,
    categories: ['qol'],
    horizon: ALL,
    version: '26.3.1',
    side: 'server',
  }),
  // ---------------- plugins (.rplugin, Rustic Server) ----------------
  p({
    id: 'pl_essentials',
    kind: 'plugin',
    source: 'rustic',
    name: 'Essentials Lite',
    author: 'Rustic',
    summary: 'Homes, spawn, warps, /tpa and kits.',
    hue: 20,
    downloads: 96_000,
    updated_days: 3,
    categories: ['admin'],
    featured: true,
    horizon: ALL,
    version: '0.4.0',
    side: 'server',
    size_mb: 0.8,
  }),
  p({
    id: 'pl_perms',
    kind: 'plugin',
    source: 'rustic',
    name: 'Perms',
    author: 'Rustic',
    summary: 'Groups, inheritance and per-world permissions.',
    hue: 210,
    downloads: 88_000,
    updated_days: 3,
    categories: ['admin'],
    horizon: ALL,
    version: '0.3.2',
    side: 'server',
    size_mb: 0.5,
  }),
  p({
    id: 'pl_chat',
    kind: 'plugin',
    source: 'rustic',
    name: 'Chat Format',
    author: 'Rustic',
    summary: 'Prefixes, colours and mentions in chat.',
    hue: 180,
    downloads: 71_000,
    updated_days: 6,
    categories: ['chat'],
    horizon: ALL,
    version: '0.2.1',
    side: 'server',
    size_mb: 0.3,
    deps: [{ project_id: 'pl_perms', relation: 'optional' }],
  }),
  p({
    id: 'pl_menus',
    kind: 'plugin',
    source: 'rustic',
    name: 'Menus',
    author: 'Rustic',
    summary: 'Clickable chest menus defined in a config file.',
    hue: 300,
    downloads: 40_000,
    updated_days: 10,
    categories: ['admin'],
    horizon: ALL,
    version: '0.2.0',
    side: 'server',
    size_mb: 0.4,
  }),
  p({
    id: 'pl_minigame',
    kind: 'plugin',
    source: 'rustic',
    name: 'Arena Mini-Game',
    author: 'Rustic',
    summary: 'Queue, countdown, arenas and a scoreboard.',
    hue: 0,
    downloads: 33_000,
    updated_days: 12,
    categories: ['minigames'],
    horizon: ALL,
    version: '0.1.4',
    side: 'server',
    size_mb: 0.9,
    deps: [{ project_id: 'pl_menus', relation: 'required' }],
  }),
  p({
    id: 'pl_claims',
    kind: 'plugin',
    source: 'rustic',
    name: 'Land Claims',
    author: 'gardien',
    summary: 'Players protect their builds with a golden shovel.',
    hue: 48,
    downloads: 52_000,
    updated_days: 19,
    categories: ['protection'],
    horizon: ALL,
    version: '1.0.0',
    side: 'server',
    size_mb: 1.1,
  }),
  p({
    id: 'pl_discord',
    kind: 'plugin',
    source: 'rustic',
    name: 'Discord Bridge',
    author: 'passerelle',
    summary: 'Relay chat and join messages to a Discord channel.',
    hue: 235,
    downloads: 61_000,
    updated_days: 7,
    categories: ['integration', 'chat'],
    horizon: ALL,
    version: '0.5.0',
    side: 'server',
    size_mb: 0.7,
    network: ['discord.com'],
  }),
  // ---------------- Rustic marketplace (creators, ADR 0155) ----------------
  p({
    id: 'rustic:sky_islands',
    kind: 'mod',
    source: 'rustic',
    name: 'Sky Islands',
    author: 'Léa',
    summary: 'Floating islands with their own biomes, waterfalls and a sky village to find.',
    hue: 200,
    downloads: 48_200,
    updated_days: 1,
    created_days: 3,
    categories: ['worldgen', 'adventure'],
    horizon: ALL,
    version: '1.2.0',
    verified: true,
    marketplace: true,
    banner: bannerArt(200, 'islands'),
    size_mb: 1.6,
  }),
  p({
    id: 'rustic:fast_paths',
    kind: 'mod',
    source: 'rustic',
    name: 'Fast Paths',
    author: 'MaxRedstone',
    summary: 'Gravel and dirt paths that speed you up a little. A native mod: built on your PC.',
    hue: 96,
    downloads: 12_400,
    updated_days: 4,
    created_days: 9,
    categories: ['qol'],
    horizon: ALL,
    version: '0.3.0',
    native: true,
    marketplace: true,
    banner: bannerArt(96, 'paths'),
    size_mb: 0.3,
  }),
  p({
    id: 'rustic:cozy_lanterns',
    kind: 'resourcepack',
    source: 'rustic',
    name: 'Cozy Lanterns',
    author: 'atelier-nox',
    summary: 'Hand-painted lanterns, candles and campfires with a warm glow.',
    hue: 32,
    downloads: 9_800,
    updated_days: 2,
    created_days: 5,
    categories: ['cozy', 'decoration'],
    horizon: ALL,
    version: '2.0',
    price_cents: 299,
    marketplace: true,
    banner: bannerArt(32, 'lanterns'),
    size_mb: 6,
  }),
  p({
    id: 'rustic:arena_kit',
    kind: 'datapack',
    source: 'rustic',
    name: 'Arena Kit',
    author: 'noah.pvp',
    summary: 'Duels, kits and a scoreboard for small PvP arenas, in one data pack.',
    hue: 352,
    downloads: 21_000,
    updated_days: 6,
    created_days: 20,
    categories: ['pvp', 'minigames'],
    horizon: ALL,
    version: '1.4.0',
    verified: true,
    marketplace: true,
    banner: bannerArt(352, 'arena'),
    size_mb: 0.2,
  }),
  p({
    id: 'rustic:golden_hour_plus',
    kind: 'shader',
    source: 'rustic',
    name: 'Golden Hour+',
    author: 'Léa',
    summary: 'Long shadows and a soft bloom at sunset, tuned for Horizon’s WGSL pipeline.',
    hue: 40,
    downloads: 31_500,
    updated_days: 3,
    created_days: 14,
    categories: ['cinematic', 'lightweight'],
    horizon: ALL,
    version: '1.1.0',
    shader_format: 'horizon',
    verified: true,
    marketplace: true,
    banner: bannerArt(40, 'sun'),
    size_mb: 0.5,
  }),
  p({
    id: 'rustic:cozy_village',
    kind: 'modpack',
    source: 'rustic',
    name: 'Cozy Village',
    author: 'Sam',
    summary: 'A calm survival pack: farming, lanterns, sky islands and gentle mobs.',
    hue: 18,
    downloads: 3_100,
    updated_days: 2,
    created_days: 8,
    categories: ['cozy', 'multiplayer'],
    horizon: ALL,
    version: '1.0.0',
    marketplace: true,
    banner: bannerArt(18, 'village'),
    size_mb: 40,
  }),
];

export function findProject(id: string): CatalogProject | undefined {
  return catalog.find((x) => x.id === id);
}

export interface CompatContext {
  /** Kind of install target, or null when no target is selected. */
  target: 'instance' | 'server' | 'world' | 'server_world' | null;
  horizonVersion?: string;
  serverEngine?: 'horizon' | 'rustic';
  /** Project ids already installed on the target (conflict detection). */
  installed: ReadonlySet<string>;
}

function versionsText(list: string[]): string {
  return list.map((v) => v.replace(/^v/, '')).join(', ');
}

/** Compatibility of a project (or of one of its versions) with an install target. */
export function computeCompatibility(
  project: CatalogProject,
  ctx: CompatContext,
  versionHorizon: string[] = project.horizon,
): Compatibility {
  if (project.fair_play_violation) {
    return {
      status: 'incompatible',
      reason: 'not_fair_play',
      detail: 'Gives an unfair advantage; not allowed by the Rustic fair play policy.',
    };
  }
  const toServer = ctx.target === 'server' || ctx.target === 'server_world';
  if (project.kind === 'plugin') {
    if (toServer && ctx.serverEngine === 'rustic')
      return { status: 'compatible', builds_on_horizon: true };
    return {
      status: 'incompatible',
      reason: 'needs_rustic_server',
      detail: 'Plugins (.rplugin) run on servers using the Rustic Server engine.',
    };
  }
  if (project.native) {
    // As the desktop side (ADR 0157): a native mod fits an instance and is compiled on the PC.
    if (ctx.target === 'instance' || ctx.target === null) {
      return {
        status: 'compatible',
        builds_on_horizon: false,
        needs_build: true,
        detail:
          "needs hook points 1, your Horizon has 415; native code: Rustic Launcher compiles it into this instance's own client on your computer, after you accept the trust notice",
      };
    }
    return {
      status: 'incompatible',
      reason: 'unsupported_format',
      detail: 'A native mod goes into an instance.',
    };
  }
  if (project.java_only) {
    return {
      status: 'incompatible',
      reason: 'java_only',
      detail: 'Only published for Java Edition loaders; no Horizon build.',
    };
  }
  if (project.kind === 'shader' && project.source === 'modrinth') {
    // HORIZON_FACTS A2: the launcher installs Horizon-registry shader packs only.
    return {
      status: 'incompatible',
      reason: 'unsupported_format',
      detail: 'Modrinth shader packs are not installable yet; packs from the Horizon registry are.',
    };
  }
  if (project.unsupported_format) {
    return {
      status: 'incompatible',
      reason: 'unsupported_format',
      detail: project.unsupported_format,
    };
  }
  const conflict = project.conflicts?.find((c) => ctx.installed.has(c));
  if (conflict) {
    return {
      status: 'incompatible',
      reason: 'conflict',
      detail: `Conflicts with ${findProject(conflict)?.name ?? conflict}.`,
    };
  }
  if (project.kind === 'modpack') {
    // A modpack becomes a new instance: it must build on the current Horizon release.
    return versionHorizon.includes(LATEST_RELEASE)
      ? { status: 'compatible', builds_on_horizon: true }
      : {
          status: 'incompatible',
          reason: 'horizon_version',
          detail: `Builds exist for Horizon ${versionsText(versionHorizon)}.`,
        };
  }
  if (ctx.target === null) return { status: 'unknown' };
  if (project.kind === 'mod') {
    if (toServer && project.side === 'client') {
      return { status: 'incompatible', reason: 'side', detail: 'Client-side only.' };
    }
    if (!ctx.horizonVersion) return { status: 'unknown' };
    return versionHorizon.includes(ctx.horizonVersion)
      ? { status: 'compatible', builds_on_horizon: true }
      : {
          status: 'incompatible',
          reason: 'horizon_version',
          detail: `Builds exist for Horizon ${versionsText(versionHorizon)}; this target runs ${ctx.horizonVersion.replace(/^v/, '')}.`,
        };
  }
  if ((project.kind === 'resourcepack' || project.kind === 'shader') && toServer) {
    return { status: 'incompatible', reason: 'side', detail: 'Client-side content.' };
  }
  // Resource packs, shaders and datapacks are format-level compatible (no per-version build).
  return { status: 'compatible', builds_on_horizon: false };
}

/** Lower-case, accent-free text for matching ("Boîte" ~ "boite"). */
export function fold(s: string): string {
  return s
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase();
}

export function matchesQuery(project: CatalogProject, query: string): boolean {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const hay = fold(
    [project.name, project.summary, project.author, ...project.categories].join(' '),
  );
  return words.every((w) => hay.includes(w));
}

function relevance(project: CatalogProject, query: string): number {
  const q = fold(query.trim());
  let score = Math.log10(project.downloads + 10);
  if (project.featured) score += 1;
  if (q) {
    const name = fold(project.name);
    if (name === q) score += 20;
    else if (name.startsWith(q)) score += 10;
    else if (name.includes(q)) score += 5;
  }
  return score;
}

/**
 * Filters and sorts the catalogue. `compat` gives each project's compatibility with the target.
 * Fair-play violations are hidden whenever `compatible_only` is on and never ranked first.
 */
export function searchCatalog(
  list: CatalogProject[],
  q: Pick<
    ContentSearchQuery,
    | 'query'
    | 'kind'
    | 'sources'
    | 'compatible_only'
    | 'category'
    | 'sort'
    | 'offset'
    | 'limit'
    | 'price'
    | 'verified'
  >,
  compat: (p: CatalogProject) => Compatibility,
): { hits: { project: CatalogProject; compatibility: Compatibility }[]; total: number } {
  const rows = list
    .filter((x) => x.kind === q.kind)
    .filter((x) => q.sources.length === 0 || q.sources.includes(x.source))
    .filter((x) => !q.category || x.categories.includes(q.category as CategoryId))
    .filter((x) => !q.verified || !!x.verified)
    .filter(
      (x) =>
        !q.price || (q.price === 'paid' ? (x.price_cents ?? 0) > 0 : (x.price_cents ?? 0) === 0),
    )
    .filter((x) => matchesQuery(x, q.query))
    .map((project) => ({ project, compatibility: compat(project) }))
    .filter((r) => !q.compatible_only || r.compatibility.status !== 'incompatible');
  const key = (r: { project: CatalogProject }) => {
    switch (q.sort) {
      case 'downloads':
        return r.project.downloads;
      case 'updated':
        return -r.project.updated_days;
      case 'newest':
        return -r.project.created_days;
      default:
        return relevance(r.project, q.query);
    }
  };
  rows.sort((a, b) => {
    const fa = a.project.fair_play_violation ? 1 : 0;
    const fb = b.project.fair_play_violation ? 1 : 0;
    return fa - fb || key(b) - key(a) || a.project.name.localeCompare(b.project.name);
  });
  return { hits: rows.slice(q.offset, q.offset + q.limit), total: rows.length };
}

const NOTES = [
  'Fixes a crash when joining a server with a full inventory.',
  'Performance: fewer allocations per frame.',
  'Ajout de la traduction française.',
  'New config screen and better defaults.',
  'Compatibility with the latest Horizon release.',
  'Correction d’un bug d’affichage en plein écran.',
];

/** The `back`-th previous version: decrements the last component, borrowing (1.6.0 → 1.5.9). */
export function bump(version: string, back: number): string {
  const parts = version.split('.').map((n) => Number.parseInt(n, 10) || 0);
  for (let step = 0; step < back; step++) {
    let i = parts.length - 1;
    while (i > 0 && parts[i] === 0) {
      parts[i] = 9;
      i--;
    }
    parts[i] = Math.max(0, (parts[i] ?? 0) - 1);
  }
  return parts.join('.');
}

/** Older Horizon versions supported by an older release (shift each by `back` steps). */
function shiftedHorizon(list: string[], back: number): string[] {
  const out = new Set<string>();
  for (const v of list) {
    const i = HORIZON_LINE.indexOf(v as (typeof HORIZON_LINE)[number]);
    const j = i - back;
    if (i >= 0 && j >= 0) out.add(HORIZON_LINE[j]!);
  }
  return [...out];
}

/** Version history (newest first) with their Horizon builds. */
export function projectVersions(
  project: CatalogProject,
  compat: (horizon: string[]) => ContentVersion['compatibility'],
): ContentVersion[] {
  const count = 4;
  return Array.from({ length: count }, (_, i) => {
    const versionAgnostic = project.kind !== 'mod' && project.kind !== 'modpack';
    const horizon =
      versionAgnostic || i === 0 ? project.horizon : shiftedHorizon(project.horizon, i);
    return {
      id: `${project.id}@${bump(project.version, i)}`,
      version: bump(project.version, i),
      channel: i === 1 && project.downloads % 3 === 0 ? 'beta' : 'release',
      horizon_versions: horizon,
      published_at: new Date(
        Date.now() - (project.updated_days + i * 23) * 86_400_000,
      ).toISOString(),
      downloads: Math.round(project.downloads / (2 + i * 3)),
      size_bytes: Math.round(project.size_mb * 1024 ** 2 * (1 - i * 0.04)),
      compatibility: compat(horizon),
    };
  });
}

const KIND_FEATURES: Record<ContentKind, string[]> = {
  mod: [
    'Configurable in game (Options > Mods)',
    'Works in single player and on servers',
    'Lightweight: no measurable FPS cost',
  ],
  modpack: [
    'One click: a ready-to-play instance',
    'Updates keep your worlds and settings',
    'Curated for stability',
  ],
  resourcepack: [
    'Covers blocks, items and UI',
    'Stack it with other packs and pick the order',
    'No mod needed',
  ],
  shader: [
    'Adjustable quality presets',
    'Hot reload while you tweak',
    'Toggle it in Options > Video Settings > Shaders',
  ],
  datapack: [
    'Installs into one world',
    'Survives updates of your instance',
    'Remove it any time from the world folder',
  ],
  plugin: [
    'Sandboxed WebAssembly: a faulty plugin is disabled, the server keeps running',
    'Configurable in config.toml',
    'Hot reload with /rustic reload',
  ],
};

export function projectDetail(
  project: CatalogProject,
  summary: ContentSummary,
  versions: ContentVersion[],
): ContentDetail {
  const features = KIND_FEATURES[project.kind];
  const caps =
    project.kind === 'plugin'
      ? `\n\n## Capabilities\n\n- Network: ${project.network?.length ? project.network.join(', ') : 'none'}\n- Files: data folder only`
      : '';
  return {
    ...summary,
    description_md: `${project.summary}\n\n## Features\n\n${features.map((f) => `- ${f}`).join('\n')}${caps}\n\n## Credits\n\nMade by ${project.author}. Published under ${project.license}.`,
    license: project.license,
    side: project.side,
    gallery: [0, 1, 2].map((i) => ({
      url: project.banner && i === 0 ? project.banner : null,
      title: [project.name, 'Settings', 'In game'][i]!,
      hue: (project.hue + i * 40) % 360,
    })),
    versions,
    dependencies: [],
    changelog: versions.map((v, i) => ({
      version: v.version,
      published_at: v.published_at,
      notes: NOTES[(project.hue + i) % NOTES.length]!,
    })),
    links: {
      source: `https://example.org/${project.author}/${project.id}`,
      issues: `https://example.org/${project.author}/${project.id}/issues`,
      website: project.source === 'rustic' ? 'https://rustic.example.invalid' : undefined,
    },
    marketplace: !!project.marketplace,
  };
}

export const CATEGORIES_BY_KIND: Record<ContentKind, CategoryId[]> = (() => {
  const out = {} as Record<ContentKind, CategoryId[]>;
  for (const x of catalog) {
    const set = new Set(out[x.kind] ?? []);
    x.categories.forEach((c) => set.add(c));
    out[x.kind] = [...set].sort();
  }
  return out;
})();
