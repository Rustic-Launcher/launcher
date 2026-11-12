import type { IsoDate } from './app';

export type ContentKind = 'mod' | 'modpack' | 'resourcepack' | 'shader' | 'datapack' | 'plugin';
export type ContentSource = 'horizon' | 'modrinth' | 'rustic';

export type IncompatibilityReason =
  | 'horizon_version'
  | 'java_only'
  | 'side'
  | 'conflict'
  | 'unsupported_format'
  | 'needs_rustic_server'
  | 'not_fair_play';

export type Compatibility =
  | {
      status: 'compatible';
      /** The project ships a build made for the target's Horizon version. */
      builds_on_horizon: boolean;
    }
  | { status: 'incompatible'; reason: IncompatibilityReason; detail: string }
  | { status: 'unknown' };

/** Horizon has two shader formats; the UI shows which and its real support status. */
export type ShaderFormat = 'horizon' | 'shaders_dir';

export interface ContentSummary {
  id: string;
  slug: string;
  kind: ContentKind;
  source: ContentSource;
  name: string;
  summary: string;
  author: string;
  icon_url: string | null;
  downloads: number;
  follows: number;
  updated_at: IsoDate;
  categories: string[];
  compatibility: Compatibility;
  /** Instance or server ids where this project is installed. */
  installed_in: string[];
  featured: boolean;
  shader_format?: ShaderFormat;
}

export interface ContentVersion {
  id: string;
  version: string;
  channel: 'release' | 'beta' | 'alpha';
  horizon_versions: string[];
  published_at: IsoDate;
  downloads: number;
  size_bytes: number;
  compatibility: Compatibility;
}

export interface ContentDependency {
  project_id: string;
  name: string;
  relation: 'required' | 'optional' | 'incompatible';
  installed: boolean;
}

export interface ContentDetail extends ContentSummary {
  description_md: string;
  license: string;
  side: 'client' | 'server' | 'both';
  gallery: { url: string | null; title: string; hue: number }[];
  versions: ContentVersion[];
  dependencies: ContentDependency[];
  changelog: { version: string; published_at: IsoDate; notes: string }[];
  links: { source?: string; issues?: string; website?: string };
}

export interface ContentSearchQuery {
  query: string;
  kind: ContentKind;
  sources: ContentSource[];
  /** Compatibility is computed against this instance (or server for plugins). */
  target?: InstallTarget;
  compatible_only: boolean;
  category?: string;
  sort: 'relevance' | 'downloads' | 'updated' | 'newest';
  offset: number;
  limit: number;
}

export interface ContentSearchResult {
  hits: ContentSummary[];
  total: number;
  offset: number;
  limit: number;
  /** Whether a next page has results (sources page differently, so `total` alone can't tell). */
  has_more?: boolean;
}

export type InstallTarget =
  | { kind: 'instance'; instance_id: string }
  | { kind: 'server'; server_id: string }
  | { kind: 'world'; instance_id: string; world_id: string }
  | { kind: 'server_world'; server_id: string };

export interface InstallRequest {
  project_id: string;
  version_id?: string;
  target: InstallTarget;
  with_dependencies: boolean;
}

export interface InstallResult {
  task_id: string;
  installed: { project_id: string; name: string; version: string }[];
}

export interface ContentCategory {
  id: string;
  kind: ContentKind;
}
