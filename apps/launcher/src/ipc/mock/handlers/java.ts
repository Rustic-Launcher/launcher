/**
 * Mock: the Java Edition launcher and the Bedrock panel (ADR 0172). Versions and loader builds
 * are a short sample in Mojang's and the loaders' shapes; installs and launches are simulated
 * (`java-state`, `java-log`). Scenarios: `?bedrock=missing` (Bedrock not installed),
 * `?bedrock=unsupported` (not Windows), `?java=not-owned` (the account doesn't own the game).
 */
import type {
  BedrockStatus,
  JavaGameVersion,
  JavaLoader,
  JavaLoaderVersion,
  JavaLogLine,
  JavaModFile,
  JavaModHit,
  JavaProfile,
  JavaRunState,
} from '../../types/java';
import {
  emit,
  fail,
  latency,
  persistGet,
  persistSet,
  simulate,
  uid,
  type MockHandlers,
} from '../runtime';
import { OFFICIAL_ICONS } from '@/features/java/logic';
import { db } from './core';

const DAY = 86_400;
const now = () => Math.floor(Date.now() / 1000);

const VERSIONS: JavaGameVersion[] = [
  {
    id: '26.4-snapshot-8',
    kind: 'snapshot',
    release_time: '2026-11-10T14:20:05+00:00',
    installed: false,
  },
  { id: '26.3', kind: 'release', release_time: '2026-10-14T09:00:00+00:00', installed: true },
  { id: '26.2', kind: 'release', release_time: '2026-07-08T09:00:00+00:00', installed: false },
  { id: '26.1.2', kind: 'release', release_time: '2026-04-21T09:00:00+00:00', installed: false },
  { id: '1.21.11', kind: 'release', release_time: '2025-12-09T09:00:00+00:00', installed: false },
  { id: '1.21.1', kind: 'release', release_time: '2024-08-08T12:24:45+00:00', installed: true },
  { id: '1.20.1', kind: 'release', release_time: '2023-06-12T13:25:51+00:00', installed: false },
  { id: '1.12.2', kind: 'release', release_time: '2017-09-18T08:39:46+00:00', installed: false },
  { id: '1.8.9', kind: 'release', release_time: '2015-12-03T09:24:39+00:00', installed: false },
  { id: 'b1.7.3', kind: 'old_beta', release_time: '2011-07-07T22:00:00+00:00', installed: false },
  { id: 'a1.2.6', kind: 'old_alpha', release_time: '2010-12-02T22:00:00+00:00', installed: false },
];

const LOADERS: Record<Exclude<JavaLoader, 'vanilla'>, (game: string) => JavaLoaderVersion[]> = {
  fabric: () => [
    { version: '0.20.1', stable: true },
    { version: '0.20.0', stable: true },
    { version: '0.19.2', stable: true },
  ],
  quilt: (g) => (g.startsWith('26.') ? [] : [{ version: '0.29.2', stable: true }]),
  forge: (g) =>
    g === '1.20.1'
      ? [
          { version: '47.4.29', stable: false },
          { version: '47.4.10', stable: true },
        ]
      : g === '26.3'
        ? [{ version: '66.1.8', stable: true }]
        : [],
  neoforge: (g) =>
    g === '1.21.1'
      ? [
          { version: '21.1.266', stable: true },
          { version: '21.1.265', stable: true },
        ]
      : g === '26.3'
        ? [
            { version: '26.3.0.103', stable: true },
            { version: '26.3.0.102', stable: true },
          ]
        : [],
};

function profile(over: Partial<JavaProfile> & Pick<JavaProfile, 'id' | 'name'>): JavaProfile {
  return {
    game_version: '26.3',
    loader: 'vanilla',
    loader_version: null,
    version_id: null,
    memory_min_mb: 512,
    memory_max_mb: 4096,
    resolution: null,
    fullscreen: false,
    java_path: null,
    jvm_args: [],
    game_dir: null,
    created_at: now() - 20 * DAY,
    last_played_at: null,
    play_time: 0,
    game_dir_path: `C:\\Users\\player\\AppData\\Roaming\\Rustic\\Rustic Launcher\\data\\java\\profiles\\${over.id}`,
    installed: false,
    running: false,
    ...over,
  };
}

const initialProfiles: JavaProfile[] = [
  profile({
    id: 'java-fabric',
    name: 'Fabric 1.21.1',
    game_version: '1.21.1',
    loader: 'fabric',
    loader_version: '0.20.1',
    version_id: 'fabric-loader-0.20.1-1.21.1',
    memory_max_mb: 6144,
    last_played_at: now() - 3 * 3600,
    play_time: 14 * 3600,
    installed: true,
  }),
  profile({
    id: 'java-vanilla',
    name: 'Vanilla 26.3',
    version_id: '26.3',
    installed: true,
    last_played_at: now() - 4 * DAY,
    play_time: 5 * 3600,
  }),
  profile({
    id: 'java-neo',
    name: 'NeoForge tech',
    game_version: '1.21.1',
    loader: 'neoforge',
    memory_max_mb: 8192,
  }),
];

const MODS: JavaModHit[] = [
  [
    'P7dR8mSH',
    'fabric-api',
    'Fabric API',
    'Lightweight and modular API providing common hooks and intercompatibility measures.',
    'modmuss50',
    160_000_000,
  ],
  [
    'AANobbMI',
    'sodium',
    'Sodium',
    'The fastest and most compatible rendering optimization mod for Minecraft.',
    'jellysquid3',
    95_000_000,
  ],
  [
    'gvQqBUqZ',
    'lithium',
    'Lithium',
    'No-compromises game logic optimization mod.',
    'jellysquid3',
    60_000_000,
  ],
  [
    'YL57xq9U',
    'iris',
    'Iris Shaders',
    'A modern shader pack loader for Minecraft intended to be compatible with existing OptiFine shader packs.',
    'coderbot',
    70_000_000,
  ],
  [
    'mOgUt4GM',
    'modmenu',
    'Mod Menu',
    'Adds a mod menu to view the list of mods you have installed.',
    'Prospector',
    80_000_000,
  ],
  [
    'NNAgCjsB',
    'entityculling',
    'Entity Culling',
    'Using async path-tracing to hide block entities and entities that are not visible.',
    'tr7zw',
    40_000_000,
  ],
].map(([project_id, slug, name, summary, author, downloads]) => ({
  project_id: project_id as string,
  slug: slug as string,
  name: name as string,
  summary: summary as string,
  author: author as string,
  icon_url: null,
  downloads: downloads as number,
  flagged: false,
  installed: false,
}));

const store = {
  profiles: persistGet<JavaProfile[]>('java.profiles', initialProfiles),
  mods: persistGet<Record<string, JavaModFile[]>>('java.mods', {
    'java-fabric': [
      {
        file_name: 'fabric-api-0.116.7+1.21.1.jar',
        enabled: true,
        size: 2_310_000,
        project_id: 'P7dR8mSH',
      },
      {
        file_name: 'sodium-fabric-0.6.13+mc1.21.1.jar',
        enabled: true,
        size: 1_120_000,
        project_id: 'AANobbMI',
      },
      { file_name: 'modmenu-11.0.3.jar', enabled: false, size: 820_000, project_id: 'mOgUt4GM' },
    ],
  }),
  states: new Map<string, JavaRunState>(),
  logs: new Map<string, JavaLogLine[]>(),
  cancels: new Map<string, () => void>(),
};

function save() {
  persistSet('java.profiles', store.profiles);
  persistSet('java.mods', store.mods);
}

function find(id: string): JavaProfile {
  const p = store.profiles.find((x) => x.id === id);
  if (!p) fail('not_found', `No Java profile ${id}.`, { reason: 'java.unknown_profile' });
  return p;
}

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

function changed() {
  emit('java-profiles-changed', clone(store.profiles));
}

function idle(id: string): JavaRunState {
  return { profile_id: id, state: 'idle', offline: false };
}

function setState(s: JavaRunState) {
  store.states.set(s.profile_id, s);
  emit('java-state', { ...s });
}

function log(
  id: string,
  text: string,
  level: JavaLogLine['level'] = 'info',
  thread = 'Render thread',
) {
  const line: JavaLogLine = { profile_id: id, level, thread, text, time: now() };
  const list = store.logs.get(id) ?? [];
  list.push(line);
  if (list.length > 2000) list.shift();
  store.logs.set(id, list);
  emit('java-log', line);
}

function flag(name: string): string | null {
  try {
    return (
      new URL(window.location.href).searchParams.get(name) ?? persistGet<string | null>(name, null)
    );
  } catch {
    return null;
  }
}

const GAME_LINES: [string, JavaLogLine['level'], string][] = [
  ['Setting user: {{name}}', 'info', 'Render thread'],
  ['Backend library: LWJGL version 3.3.3+5', 'info', 'Render thread'],
  ['Reloading ResourceManager: vanilla', 'info', 'Render thread'],
  ['Missing sound for event: minecraft:item.goat_horn.play', 'warn', 'Render thread'],
  ['OpenAL initialized on device OpenAL Soft', 'info', 'Render thread'],
  ['Sound engine started', 'info', 'Render thread'],
  ['Created: 1024x512x4 minecraft:textures/atlas/blocks.png-atlas', 'info', 'Render thread'],
];

function startGame(p: JavaProfile, offline: boolean) {
  const started = now();
  p.running = true;
  p.last_played_at = started;
  setState({ profile_id: p.id, state: 'running', started_at: started, offline });
  const name = db.accounts.find((a) => a.active)?.username ?? 'Player';
  GAME_LINES.forEach(([text, level, thread], i) =>
    setTimeout(
      () => {
        if (store.states.get(p.id)?.state === 'running')
          log(p.id, text.replace('{{name}}', name), level, thread);
      },
      200 * (i + 1),
    ),
  );
  save();
  changed();
}

const ALL_STEPS = ['metadata', 'java', 'loader', 'libraries', 'assets'] as const;

function install(p: JavaProfile, then: () => void) {
  const task = uid('task');
  setState({
    profile_id: p.id,
    state: 'installing',
    step: 'metadata',
    progress: 0,
    task_id: task,
    offline: false,
  });
  log(p.id, `Preparing ${p.name} (${p.loader} ${p.game_version})`, 'info', 'Rustic');
  const steps = ALL_STEPS.filter((s) => s !== 'loader' || p.loader !== 'vanilla');
  const cancel = simulate(
    p.installed ? 400 : 4000,
    (f) => {
      const step = steps[Math.min(steps.length - 1, Math.floor(f * steps.length))];
      setState({
        profile_id: p.id,
        state: 'installing',
        step,
        progress: f,
        task_id: task,
        offline: false,
      });
    },
    () => {
      store.cancels.delete(p.id);
      p.installed = true;
      if (p.loader !== 'vanilla') {
        p.loader_version ??= LOADERS[p.loader](p.game_version)[0]?.version ?? null;
        p.version_id =
          p.loader === 'fabric'
            ? `fabric-loader-${p.loader_version}-${p.game_version}`
            : p.loader === 'quilt'
              ? `quilt-loader-${p.loader_version}-${p.game_version}`
              : p.loader === 'forge'
                ? `${p.game_version}-forge-${p.loader_version}`
                : `neoforge-${p.loader_version}`;
      } else {
        p.version_id = p.game_version;
      }
      save();
      then();
    },
  );
  store.cancels.set(p.id, () => {
    cancel();
    store.cancels.delete(p.id);
    setState({ ...idle(p.id), message: 'download cancelled', reason: 'java.cancelled' });
  });
}

function bedrock(): BedrockStatus {
  const f = flag('bedrock');
  if (f === 'unsupported') return { supported: false, installed: false, version: null };
  if (f === 'missing') return { supported: true, installed: false, version: null };
  return { supported: true, installed: true, version: '1.26.6301.0' };
}

/** The owner gate of launches and installs (`?java=not-owned`: the account doesn't own the game). */
function ownerGate() {
  const account = db.accounts.find((a) => a.active && a.kind === 'microsoft');
  if (!account)
    fail(
      'ownership_required',
      'Sign in with a Microsoft account that owns Minecraft: Java Edition to play.',
      {
        reason: 'auth.no_account',
      },
    );
  if (flag('java') === 'not-owned' || account.ownership?.owns_game === false)
    fail('ownership_required', "This account doesn't own Minecraft: Java Edition.", {
      reason: 'auth.not_owned',
    });
}

export const javaHandlers: MockHandlers = {
  java_versions_list: async ({ snapshots, old }) => {
    await latency();
    return VERSIONS.filter(
      (v) =>
        v.kind === 'release' ||
        (v.kind === 'snapshot' && snapshots) ||
        ((v.kind === 'old_beta' || v.kind === 'old_alpha') && old),
    );
  },
  java_loader_versions: async ({ loader, game_version }) => {
    await latency();
    return loader === 'vanilla' ? [] : LOADERS[loader](game_version);
  },
  java_profiles_list: async () => {
    await latency(0.5);
    return clone(store.profiles);
  },
  java_profile_create: async (n) => {
    await latency();
    if (!n.name.trim())
      fail('invalid_argument', 'a name of 1 to 64 characters', { reason: 'java.invalid' });
    const p = profile({
      id: uid('java'),
      name: n.name.trim(),
      game_version: n.game_version,
      loader: n.loader,
      loader_version: n.loader === 'vanilla' ? null : (n.loader_version ?? null),
      memory_max_mb: n.memory_max_mb ?? db.settings.game.default_memory_mb,
      created_at: now(),
    });
    store.profiles.unshift(p);
    save();
    changed();
    return clone(p);
  },
  java_profile_update: async ({ id, patch }) => {
    await latency();
    const p = find(id);
    const reinstall =
      (patch.game_version != null && patch.game_version !== p.game_version) ||
      (patch.loader != null && patch.loader !== p.loader) ||
      (patch.loader_version !== undefined && patch.loader_version !== p.loader_version);
    Object.assign(p, patch);
    if (p.loader === 'vanilla') p.loader_version = null;
    if (reinstall) {
      p.version_id = null;
      p.installed = false;
    }
    p.memory_min_mb = Math.min(p.memory_min_mb, p.memory_max_mb);
    save();
    changed();
    return clone(p);
  },
  java_profile_delete: async ({ id }) => {
    await latency();
    const p = find(id);
    if (p.running)
      fail('conflict', 'The profile is already running.', { reason: 'java.already_running' });
    store.profiles = store.profiles.filter((x) => x.id !== id);
    delete store.mods[id];
    save();
    changed();
  },
  java_profile_install: async ({ id }) => {
    await latency();
    const p = find(id);
    ownerGate();
    install(p, () => {
      setState(idle(p.id));
      changed();
    });
    return { task_id: store.states.get(id)?.task_id ?? uid('task') };
  },
  java_profile_launch: async ({ id }) => {
    await latency();
    const p = find(id);
    ownerGate();
    if (p.running || store.cancels.has(id))
      fail('conflict', 'This profile is already being prepared.', { reason: 'java.busy' });
    store.logs.set(id, []);
    install(p, () => {
      setState({ profile_id: id, state: 'launching', progress: 1, offline: false });
      setTimeout(() => startGame(p, false), 300);
    });
    return clone(store.states.get(id) ?? idle(id));
  },
  java_profile_stop: async ({ id }) => {
    await latency(0.3);
    const p = find(id);
    const cancel = store.cancels.get(id);
    if (cancel) {
      cancel();
      return clone(store.states.get(id) ?? idle(id));
    }
    if (!p.running) fail('conflict', 'The profile is not running.', { reason: 'java.not_running' });
    p.running = false;
    p.play_time += Math.max(60, now() - (p.last_played_at ?? now()));
    log(id, 'The game stopped (exit code 0).', 'info', 'Rustic');
    setState({ ...idle(id), exit_code: 0 });
    save();
    changed();
    return clone(store.states.get(id) ?? idle(id));
  },
  java_run_states: async () => [...store.states.values()].map((s) => ({ ...s })),
  java_profile_log: async ({ id }) => clone(store.logs.get(id) ?? []),
  java_profile_open_folder: async ({ id }) => {
    await latency(0.3);
    find(id);
  },
  java_mods_search: async ({ profile_id, query, offset }) => {
    await latency();
    const p = find(profile_id);
    if (p.loader === 'vanilla')
      fail('invalid_argument', 'Pick a mod loader for this profile first.', {
        reason: 'java.vanilla_mods',
      });
    const installed = new Set((store.mods[profile_id] ?? []).map((m) => m.project_id));
    const q = (query ?? '').toLowerCase();
    const hits = MODS.filter(
      (m) => !q || m.name.toLowerCase().includes(q) || m.summary.toLowerCase().includes(q),
    )
      .slice(offset ?? 0)
      .map((m) => ({ ...m, installed: installed.has(m.project_id) }));
    return { hits, total: hits.length };
  },
  java_mods_list: async ({ profile_id }) => {
    await latency(0.5);
    find(profile_id);
    return clone(store.mods[profile_id] ?? []);
  },
  java_mod_install: async ({ profile_id, project_id }) => {
    await latency(2);
    const p = find(profile_id);
    const hit = MODS.find((m) => m.project_id === project_id);
    if (!hit) fail('not_found', 'No such project', { reason: 'content' });
    const file: JavaModFile = {
      file_name: `${hit.slug}-${p.loader}-${p.game_version}.jar`,
      enabled: true,
      size: 1_000_000,
      project_id,
    };
    const list = (store.mods[profile_id] ??= []).filter((m) => m.project_id !== project_id);
    list.push(file);
    store.mods[profile_id] = list;
    save();
    return { ...file };
  },
  java_mod_set_enabled: async ({ profile_id, file_name, enabled }) => {
    await latency(0.3);
    const m = (store.mods[profile_id] ?? []).find((x) => x.file_name === file_name);
    if (!m) fail('not_found', 'no such mod');
    m.enabled = enabled;
    save();
    return { ...m };
  },
  java_mod_remove: async ({ profile_id, file_name }) => {
    await latency(0.3);
    store.mods[profile_id] = (store.mods[profile_id] ?? []).filter(
      (x) => x.file_name !== file_name,
    );
    save();
  },
  bedrock_status: async () => {
    await latency();
    return bedrock();
  },
  bedrock_launch: async () => {
    await latency();
    const s = bedrock();
    if (!s.supported)
      fail('unavailable', 'Bedrock Edition: Windows only', { reason: 'bedrock.failed' });
    if (!s.installed)
      fail('unavailable', 'Bedrock Edition: not installed', { reason: 'bedrock.failed' });
  },
  edition_icon: async ({ edition }) => OFFICIAL_ICONS[edition],
  bedrock_open_store: async () => {
    await latency(0.3);
    if (!bedrock().supported) fail('unavailable', 'Windows only');
  },
};
