/**
 * Native mods (ADR 0157), as `cmd/native.rs` answers: per-instance trust notice and build, Rust
 * missing until installed (so the whole flow shows in the browser), a simulated build task.
 */
import type {
  NativeBuildInfo,
  NativeMod,
  NativeOverview,
  NativeStatus,
  NativeToolchain,
} from '../../types/native';
import { startDownload } from '../downloads';
import { emit, fail, latency, type MockHandlers } from '../runtime';
import { db, findInstance } from './core';

const GIB = 1024 ** 3;

export const nativeDb = {
  rust: 'missing' as 'missing' | 'installing' | 'ready',
  consent: new Map<string, number>(),
  /** The Horizon Essentials pack ships one native mod, not built yet (the Profiles page shows it). */
  mods: new Map<string, NativeMod[]>([
    ['inst_essentials', [{ id: 'smooth-zoom', version: '1.2.0', enabled: true }]],
  ]),
  builds: new Map<string, { key: string; info: NativeBuildInfo }>(),
  tasks: new Map<string, string>(),
  cacheBytes: 0,
  limitGib: 30,
};

const keyOf = (mods: NativeMod[]) =>
  mods
    .filter((m) => m.enabled)
    .map((m) => `${m.id}@${m.version}`)
    .sort()
    .join(',');

function toolchain(): NativeToolchain {
  return nativeDb.rust === 'ready'
    ? {
        state: 'ready',
        kind: 'managed',
        version: '1.99.1',
        reason: null,
        installable: false,
        prerequisites: [],
      }
    : {
        state: 'missing',
        kind: null,
        version: '1.99.1',
        reason: 'Rust is not installed',
        installable: true,
        prerequisites: [],
      };
}

/** A native mod installed into an instance (content_install). */
export function addNativeMod(instanceId: string, id: string, version: string): void {
  const list = (nativeDb.mods.get(instanceId) ?? []).filter((m) => m.id !== id);
  list.push({ id, version, enabled: true });
  nativeDb.mods.set(instanceId, list);
}

export function hasNativeConsent(instanceId: string): boolean {
  return nativeDb.consent.has(instanceId);
}

function status(instanceId: string): NativeStatus {
  findInstance(instanceId);
  const mods = nativeDb.mods.get(instanceId) ?? [];
  const on = mods.filter((m) => m.enabled);
  const build = nativeDb.builds.get(instanceId);
  const state =
    on.length === 0
      ? 'not_needed'
      : !build
        ? 'not_built'
        : build.key === keyOf(mods)
          ? 'up_to_date'
          : 'needs_rebuild';
  return {
    instance_id: instanceId,
    state,
    supported: true,
    mods: structuredClone(mods),
    consent: nativeDb.consent.has(instanceId),
    consent_at: nativeDb.consent.get(instanceId) ?? null,
    build: build ? structuredClone(build.info) : null,
    task_id: nativeDb.tasks.get(instanceId) ?? null,
    toolchain: toolchain(),
  };
}

const PHASES: [number, string][] = [
  [0.02, 'Rust 1.99.1 (Managed)'],
  [0.08, "Preparing the Rustic edition's source"],
  [0.12, 'horizon-mixins ready'],
  [0.16, 'Applying the mods'],
  [0.5, 'Compiling horizon-client-render, naga (212/404)'],
  [0.9, 'Compiling horizon-client (402/404)'],
  [0.96, 'Installing the built client'],
];

function phaseAt(p: number): string {
  let label = PHASES[0]![1];
  for (const [at, text] of PHASES) if (p >= at) label = text;
  return label;
}

function cacheInfo() {
  return {
    usage: {
      cache_bytes: nativeDb.cacheBytes,
      toolchain_bytes: nativeDb.rust === 'ready' ? 1.7 * GIB : 0,
      crates_bytes: nativeDb.cacheBytes ? 0.66 * GIB : 0,
      source_bytes: nativeDb.cacheBytes ? 0.3 * GIB : 0,
      limit_bytes: nativeDb.limitGib * GIB,
      path: 'C:/Users/Sam/AppData/Roaming/Rustic/Rustic Launcher/data/native',
    },
    limit_gib: nativeDb.limitGib,
    toolchain: toolchain(),
    building: nativeDb.tasks.size > 0,
  };
}

export const nativeHandlers: MockHandlers = {
  native_status: async ({ instance_id }) => {
    await latency(0.5);
    return status(instance_id);
  },
  native_overview: async () => {
    await latency(0.5);
    return db.instances.flatMap((i): NativeOverview[] => {
      const s = status(i.id);
      if (s.mods.length === 0) return [];
      return [
        {
          instance_id: i.id,
          state: s.state,
          mods: s.mods.length,
          enabled: s.mods.filter((m) => m.enabled).length,
          supported: s.supported,
          building: s.task_id !== null,
        },
      ];
    });
  },
  native_consent: async ({ instance_id, accept }) => {
    await latency(0.5);
    findInstance(instance_id);
    if (accept) nativeDb.consent.set(instance_id, Math.floor(Date.now() / 1000));
    else nativeDb.consent.delete(instance_id);
    return status(instance_id);
  },
  native_toolchain_install: async () => {
    await latency();
    nativeDb.rust = 'installing';
    const task_id = startDownload({
      kind: 'native-toolchain',
      label: 'Rust 1.99.1',
      total_bytes: 1000,
      duration_ms: 2500,
      detail: (p) => (p < 0.5 ? 'Installing Rust 1.99.1 (rustup)' : 'Downloading MinGW-w64'),
      onDone: () => {
        nativeDb.rust = 'ready';
        emit('native-changed', {});
      },
      onCancel: () => {
        nativeDb.rust = 'missing';
      },
    });
    return { task_id };
  },
  native_build: async ({ instance_id }) => {
    await latency();
    const running = nativeDb.tasks.get(instance_id);
    if (running) return { task_id: running };
    if (!nativeDb.consent.has(instance_id)) {
      fail('unauthorized', 'native mods run unsandboxed: accept the trust notice first', {
        reason: 'native.consent_required',
      });
    }
    if (nativeDb.rust !== 'ready') {
      fail('unavailable', 'Rust 1.99.1 is needed to build native mods: Rust is not installed', {
        reason: 'native.toolchain_missing',
      });
    }
    const mods = (nativeDb.mods.get(instance_id) ?? []).filter((m) => m.enabled);
    if (mods.length === 0) {
      fail('not_found', 'no native mod is turned on in this instance', {
        reason: 'native.no_mods',
      });
    }
    const first = nativeDb.cacheBytes === 0;
    const started = Date.now();
    const task_id: string = startDownload({
      kind: 'native-build',
      label: `Native mods: ${findInstance(instance_id).name}`,
      instance_id,
      total_bytes: 1000,
      duration_ms: first ? 6000 : 2500,
      detail: phaseAt,
      onProgress: (p) =>
        emit('native-build-progress', {
          instance_id,
          task_id,
          phase: p < 0.16 ? 'mixins' : p < 0.96 ? 'compile' : 'install',
          fraction: p,
          message: phaseAt(p),
          units: p >= 0.16 && p < 0.96 ? [Math.round(404 * p), 404] : null,
        }),
      onDone: () => {
        nativeDb.tasks.delete(instance_id);
        nativeDb.cacheBytes = Math.max(nativeDb.cacheBytes, 2.4 * GIB);
        nativeDb.builds.set(instance_id, {
          key: keyOf(mods),
          info: {
            built_at: Math.floor(Date.now() / 1000),
            seconds: (Date.now() - started) / 1000 + (first ? 820 : 50),
            compiled: first ? 358 : 2,
            mods: structuredClone(mods),
            left_out: [],
            client_bytes: 106_789_888,
            base_build: 'rustic-edition-2026.11.12',
            rust_version: '1.99.1',
          },
        });
        emit('native-changed', { instance_id });
      },
      onCancel: () => {
        nativeDb.tasks.delete(instance_id);
      },
    });
    nativeDb.tasks.set(instance_id, task_id);
    return { task_id };
  },
  native_cache_info: async () => {
    await latency(0.5);
    return cacheInfo();
  },
  native_cache_clear: async () => {
    await latency();
    if (nativeDb.tasks.size > 0)
      fail('conflict', 'a native build is already running', {
        reason: 'native.busy',
      });
    const freed = nativeDb.cacheBytes;
    nativeDb.cacheBytes = 0;
    return freed;
  },
  native_cache_set_limit: async ({ cache_limit_gib }) => {
    await latency(0.5);
    if (cache_limit_gib > 4096) fail('invalid_argument', 'The cache limit is at most 4096 GiB.');
    nativeDb.limitGib = cache_limit_gib;
    return cacheInfo();
  },
};
