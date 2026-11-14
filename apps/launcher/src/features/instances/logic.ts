/** Pure helpers for the instances area (unit-tested). */
import type { Instance } from '@/ipc/types/instances';
import type { NativeOverview } from '@/ipc/types/native';

/** Profiles page orders (ADR 0159); pinned profiles always come first. */
export const PROFILE_SORTS = ['recent', 'name', 'created', 'version', 'playtime'] as const;
export type ProfileSort = (typeof PROFILE_SORTS)[number];

/** Where a profile comes from: made in the launcher, or installed from a modpack. */
export type ProfileSource = 'custom' | 'modpack';

export function filterInstances(list: Instance[], query: string): Instance[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  return list.filter(
    (i) =>
      i.name.toLowerCase().includes(q) ||
      i.horizon_version.toLowerCase().includes(q) ||
      (i.source?.name.toLowerCase().includes(q) ?? false),
  );
}

/** Newest build first: `v0.2.15` before `v0.2.9`; releases before nightlies. */
export function compareBuilds(a: string, b: string): number {
  return b.localeCompare(a, 'en', { numeric: true });
}

/** Pinned first, then by the chosen order (name breaks ties). */
export function sortInstances(list: Instance[], sort: ProfileSort): Instance[] {
  const by = (a: Instance, b: Instance): number => {
    switch (sort) {
      case 'name':
        return 0;
      case 'playtime':
        return b.playtime_s - a.playtime_s;
      case 'created':
        return b.created_at.localeCompare(a.created_at);
      case 'version':
        return compareBuilds(a.horizon_version, b.horizon_version);
      default:
        return (b.last_played_at ?? '').localeCompare(a.last_played_at ?? '');
    }
  };
  return [...list].sort(
    (a, b) =>
      Number(b.pinned) - Number(a.pinned) ||
      by(a, b) ||
      a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
  );
}

export interface ProfileFilters {
  q?: string;
  source?: ProfileSource;
  version?: string;
  edition?: 'rustic' | 'horizon';
  native?: boolean;
}

/** What the filters need to know beyond the instance itself. */
export interface ProfileFacts {
  /** Whether a build is a Rustic edition (the versions list). */
  isRustic: (versionId: string) => boolean;
  /** Ids of the instances with at least one native mod (`native_overview`). */
  native: ReadonlySet<string>;
}

export function filterProfiles(
  list: Instance[],
  f: ProfileFilters,
  facts: ProfileFacts,
): Instance[] {
  return filterInstances(list, f.q ?? '').filter(
    (i) =>
      (!f.source || (f.source === 'modpack') === !!i.source) &&
      (!f.version || i.horizon_version === f.version) &&
      (!f.edition || (f.edition === 'rustic') === facts.isRustic(i.horizon_version)) &&
      (!f.native || facts.native.has(i.id)),
  );
}

/** How many filters (not the search text or the source) differ from the defaults. */
export function activeFilterCount(f: ProfileFilters): number {
  return (f.version ? 1 : 0) + (f.edition ? 1 : 0) + (f.native ? 1 : 0);
}

/** The builds the profiles use, newest first (the version filter's chips). */
export function buildsInUse(list: Instance[]): string[] {
  return [...new Set(list.map((i) => i.horizon_version))].sort(compareBuilds);
}

/**
 * "Jump back in": the most recently played profiles, only when there are enough profiles for
 * a shortcut row to help (`min` profiles, at least two of them played).
 */
export function recentlyPlayed(list: Instance[], max = 3, min = 4): Instance[] {
  const played = list
    .filter((i) => i.last_played_at)
    .sort((a, b) => (b.last_played_at ?? '').localeCompare(a.last_played_at ?? ''));
  return list.length >= min && played.length >= 2 ? played.slice(0, max) : [];
}

/** What a profile card says about its native mods (ADR 0159). */
export type NativeLineKey =
  'building' | 'unsupported' | 'off' | 'not_built' | 'needs_rebuild' | 'up_to_date';

/**
 * The native-mods line of a profile from `native_overview` and its build task: `null` without
 * native mods. A running build wins; a plain Horizon build can't take native mods at all.
 */
export function nativeLineKey(
  overview: Pick<NativeOverview, 'state' | 'enabled' | 'supported' | 'building'> | undefined,
  buildRunning: boolean,
): NativeLineKey | null {
  if (buildRunning || overview?.building) return 'building';
  if (!overview) return null;
  if (!overview.supported) return 'unsupported';
  if (overview.enabled === 0 || overview.state === 'not_needed') return 'off';
  return overview.state;
}

/** The badge tone of a native line. */
export function nativeLineTone(key: NativeLineKey): 'accent' | 'success' | 'neutral' | 'warning' {
  return key === 'building'
    ? 'accent'
    : key === 'up_to_date'
      ? 'success'
      : key === 'off'
        ? 'neutral'
        : 'warning';
}

/** Total play time of a list, in seconds. */
export function totalPlaytime(list: Instance[]): number {
  return list.reduce((s, i) => s + i.playtime_s, 0);
}

/** Moves one item; returns a new array (clamped indices). */
export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
  const next = [...list];
  if (from < 0 || from >= next.length) return next;
  const target = Math.max(0, Math.min(next.length - 1, to));
  const [item] = next.splice(from, 1);
  next.splice(target, 0, item as T);
  return next;
}

export function sameOrder(a: readonly { id: string }[], b: readonly { id: string }[]): boolean {
  return a.length === b.length && a.every((x, i) => x.id === b[i]?.id);
}

/** Initials for a content tile without an icon. */
export function initials(name: string): string {
  const words = name.split(/\s+/).filter(Boolean);
  return ((words[0]?.[0] ?? '') + (words[1]?.[0] ?? words[0]?.[1] ?? '')).toUpperCase();
}

/** The file kinds the New instance dialog imports (`instance_import`). */
export type ImportKind = 'folder' | 'mrpack' | 'curseforge_zip';

/** The import kind a dropped or picked path is read as, or null when it can't be imported. */
export function importKindOf(path: string): ImportKind | null {
  const base =
    path
      .replace(/[\\/]+$/, '')
      .split(/[\\/]/)
      .pop() ?? '';
  const lower = base.toLowerCase();
  if (lower.endsWith('.mrpack')) return 'mrpack';
  if (lower.endsWith('.zip')) return 'curseforge_zip';
  // No file extension (a version number like `5.4.1` is not one): an instance folder, which the
  // desktop side checks.
  if (base && !/\.[a-z][a-z0-9]{0,7}$/i.test(base)) return 'folder';
  return null;
}
