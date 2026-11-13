/** Native mods compiled on the player's PC (ADR 0157, `cmd/native.rs`). */

/** `not_needed`: no native mod turned on (the prebuilt client plays). */
export type NativeBuildState = 'not_needed' | 'not_built' | 'needs_rebuild' | 'up_to_date';

export interface NativeMod {
  id: string;
  version: string;
  enabled: boolean;
}

export interface NativeLeftOut {
  id: string;
  version: string;
  /** `other_version` (doesn't apply to or compile on this Horizon) or `unsupported`. */
  why: string;
  detail: string;
}

export interface NativeBuildInfo {
  /** Unix seconds. */
  built_at: number;
  seconds: number;
  /** Crates compiled (0: nothing changed). */
  compiled: number;
  mods: NativeMod[];
  left_out: NativeLeftOut[];
  client_bytes: number;
  base_build: string;
  rust_version: string;
}

export interface NativeToolchain {
  state: 'ready' | 'missing';
  /** Where the Rust found comes from, when ready. */
  kind: 'managed' | 'rustup' | 'system' | null;
  version: string;
  reason: string | null;
  /** The launcher can install its own Rust (after the player agreed). */
  installable: boolean;
  /** What the player installs first (Linux and macOS system packages). */
  prerequisites: string[];
}

export interface NativeStatus {
  instance_id: string;
  state: NativeBuildState;
  /** False when the instance plays a build native mods can't go into (a plain Horizon build). */
  supported: boolean;
  mods: NativeMod[];
  /** The player accepted the trust notice for this instance. */
  consent: boolean;
  consent_at: number | null;
  build: NativeBuildInfo | null;
  /** The running build's task. */
  task_id: string | null;
  toolchain: NativeToolchain;
}

/** One instance's native mods at a glance (`native_overview`, ADR 0159). */
export interface NativeOverview {
  instance_id: string;
  state: NativeBuildState;
  /** Native mods installed. */
  mods: number;
  /** Of which turned on. */
  enabled: number;
  /** False when the instance plays a plain Horizon build. */
  supported: boolean;
  /** A build of this instance is running. */
  building: boolean;
}

export interface NativeCacheUsage {
  cache_bytes: number;
  toolchain_bytes: number;
  crates_bytes: number;
  source_bytes: number;
  limit_bytes: number;
  path: string;
}

export interface NativeCacheInfo {
  usage: NativeCacheUsage;
  /** 0: never cleared automatically. */
  limit_gib: number;
  toolchain: NativeToolchain;
  building: boolean;
}

export type NativePhase =
  'toolchain' | 'source' | 'tool' | 'mixins' | 'compile' | 'install' | 'cleanup' | 'done';

export interface NativeBuildProgress {
  instance_id: string;
  task_id: string;
  phase: NativePhase;
  /** 0..1 over the whole build. */
  fraction: number;
  message: string;
  /** While compiling: crates done and in all. */
  units: [number, number] | null;
}
