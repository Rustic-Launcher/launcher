/** Mock: app, settings, downloads, preparation, accounts, versions, instances, launch. */
import { takeMockDeepLink } from '../deepLink';
import type { Account, RusticSession } from '../../types/auth';
import type { RusticServerStatus, Settings } from '../../types/app';
import type { Instance, InstalledContent, LaunchStatus } from '../../types/instances';
import {
  defaultSettings,
  horizonVersions,
  initialAccounts,
  initialInstanceContent,
  initialInstances,
  initialRusticSession,
  initialWorlds,
  javaRuntimes,
} from '../data/core';
import { cancelDownload, cancelDownloadsFor, listDownloads, startDownload } from '../downloads';
import {
  emit,
  fail,
  latency,
  nowIso,
  persistGet,
  persistSet,
  simulate,
  sleep,
  uid,
  type MockHandlers,
} from '../runtime';

const clone = <T>(v: T): T => structuredClone(v);

export const db = {
  settings: (() => {
    const s = clone(defaultSettings);
    const saved = persistGet<Partial<Settings> | null>('settings', null);
    if (saved) Object.assign(s, saved);
    if (persistGet('onboarded', false)) s.onboarding_completed = true;
    return s;
  })(),
  // `rustic.mock.fresh`: a first launch on a new machine (no account, game files not prepared).
  // `rustic.mock.skin` ({ url, slim }): the first account's skin, to see the textured model.
  accounts: persistGet('fresh', false)
    ? []
    : clone(initialAccounts).map((a, i) =>
        i === 0 ? { ...a, skin: persistGet<Account['skin']>('skin', null) } : a,
      ),
  rustic: clone(initialRusticSession),
  // `rustic.mock.no-instances`: no profile yet (the Profiles page's empty state).
  instances: persistGet('no-instances', false) ? [] : clone(initialInstances),
  content: clone(initialInstanceContent),
  worlds: clone(initialWorlds),
  launches: new Map<string, LaunchStatus>(),
  launchCancels: new Map<string, () => void>(),
  prepared: !persistGet('fresh', false),
  // The Settings override of `rustic-server`; `rustic.mock.no-rustic-server`: none found at all
  // (creating a Rustic Server then fails with `server.no_rustic_server`).
  rusticServer: {
    configured: persistGet<string | null>('rustic-server', null),
    automatic: !persistGet('no-rustic-server', false),
  },
};

const RUSTIC_SERVER_VERSION = 'Rustic 0.1.0 (rustic:api 0.1) on Horizon (Minecraft 26.3)';

/** What `rustic_server_status` answers (launcher ADR 0152's lookup, mocked). */
export function rusticServerStatus(): RusticServerStatus {
  const build = horizonVersions[0]?.id ?? null;
  const { configured, automatic } = db.rusticServer;
  if (configured)
    return {
      path: configured,
      origin: 'configured',
      configured,
      build,
      version: RUSTIC_SERVER_VERSION,
    };
  if (automatic)
    return {
      path: `C:/Users/Sam/AppData/Roaming/Rustic/Rustic Launcher/data/builds/${build}/rustic-server.exe`,
      origin: 'build',
      configured: null,
      build,
      version: RUSTIC_SERVER_VERSION,
    };
  return { path: null, origin: null, configured: null, build, version: null };
}

export function findInstance(id: string): Instance {
  return db.instances.find((i) => i.id === id) ?? fail('not_found', `instance ${id}`);
}

function instancesChanged() {
  emit('instances-changed', clone(db.instances));
}

function accountsChanged() {
  emit('accounts-changed', clone(db.accounts));
}

function versionLabel(id: string): string {
  return horizonVersions.find((v) => v.id === id)?.label ?? id;
}

function setLaunch(s: LaunchStatus) {
  db.launches.set(s.instance_id, s);
  emit('launch-state', { ...s });
}

export const coreHandlers: MockHandlers = {
  // ---- app ----
  app_info: () => ({
    version: '0.1.0',
    channel: 'beta',
    os: 'windows',
    arch: 'x86_64',
    native: false,
    data_dir: db.settings.game.game_dir,
  }),
  system_info: () => ({
    total_memory_mb: 32768,
    available_memory_mb: 19800,
    cpu_model: 'AMD Ryzen 7 7800X3D',
    cpu_threads: 16,
  }),
  settings_get: async () => {
    await latency(0.5);
    return clone(db.settings);
  },
  settings_set: async (patch) => {
    await latency(0.3);
    const s = db.settings;
    if (patch.game) Object.assign(s.game, patch.game);
    if (patch.launcher) Object.assign(s.launcher, patch.launcher);
    if (patch.privacy) Object.assign(s.privacy, patch.privacy);
    if (patch.onboarding_completed !== undefined) {
      s.onboarding_completed = patch.onboarding_completed;
      persistSet('onboarded', patch.onboarding_completed);
    }
    persistSet('settings', { game: s.game, launcher: s.launcher, privacy: s.privacy });
    return clone(s);
  },
  java_runtimes_list: () => clone(javaRuntimes),
  rustic_server_status: async () => {
    await latency(0.3);
    return rusticServerStatus();
  },
  rustic_server_set: async ({ path }) => {
    await latency(0.4);
    if (path && !/(^|[\\/])rustic-server(\.exe)?$/i.test(path))
      fail('invalid_argument', `${path} is not a rustic-server program`, {
        reason: 'server.not_rustic_server',
      });
    db.rusticServer.configured = path || null;
    persistSet('rustic-server', db.rusticServer.configured);
    return rusticServerStatus();
  },
  app_open_path: async () => {
    await latency(0.2);
  },
  app_open_url: ({ url }) => {
    window.open(url, '_blank', 'noopener');
  },
  app_take_deep_link: () => takeMockDeepLink(),
  // The browser preview has no network to Mojang: players keep their initials.
  player_skin_url: () => null,
  app_pick_path: async ({ kind, title }) => {
    await latency(0.5);
    if (kind === 'folder') return 'C:/Users/Sam/Documents/Rustic';
    return title?.includes('rustic-server')
      ? 'C:/Users/Sam/Rustic/rustic-server.exe'
      : 'C:/Users/Sam/Downloads/pack.mrpack';
  },
  app_check_update: async () => {
    await latency(1);
    return { available: false, version: null, notes: null };
  },
  app_install_update: async () => {
    await latency(1);
  },
  downloads_list: () => listDownloads(),
  download_cancel: ({ id }) => cancelDownload(id),
  game_detect_install: async () => {
    await latency(3);
    return { found: true, path: 'C:/Users/Sam/AppData/Roaming/.minecraft', versions: ['26.3'] };
  },
  game_prepare_status: () => ({
    state: db.prepared ? 'ready' : 'missing',
    game_version: '26.3',
    prepared_at: db.prepared ? nowIso() : null,
    source_path: 'C:/Users/Sam/AppData/Roaming/.minecraft',
  }),
  game_prepare: () => {
    const steps = ['detect', 'download', 'verify', 'compile_exporters', 'export', 'index'] as const;
    const task_id = startDownload({
      kind: 'prepare',
      label: 'prepare',
      total_bytes: null,
      duration_ms: 5000,
      onProgress: (p) => {
        const step = steps[Math.min(steps.length - 1, Math.floor(p * steps.length))] ?? 'index';
        emit('prepare-progress', { step, progress: p });
      },
      onDone: () => {
        db.prepared = true;
        emit('prepare-progress', { step: 'done', progress: 1 });
      },
    });
    return { task_id };
  },

  // ---- auth ----
  auth_accounts_list: async () => {
    await latency(0.4);
    return clone(db.accounts);
  },
  auth_microsoft_login_start: async () => {
    await latency();
    return {
      flow_id: uid('flow'),
      verification_uri: 'https://www.microsoft.com/link',
      user_code: 'RX7-K2M',
      expires_in_s: 900,
    };
  },
  auth_microsoft_browser_login_start: async () => {
    await latency();
    return {
      flow_id: uid('flow'),
      authorize_url: 'https://login.live.com/oauth20_authorize.srf?mock=1',
    };
  },
  auth_microsoft_login_poll: async () => {
    await sleep(1200);
    const existing = db.accounts.find((a) => a.kind === 'microsoft');
    if (existing) return { status: 'success', account: clone(existing) };
    const acc: Account = {
      id: uid('acc'),
      kind: 'microsoft',
      username: 'SamCrafte',
      uuid: '7d3f9b2e-41c2-4f7a-9a51-2b8f0c6e1d44',
      active: true,
      ownership: {
        owns_game: true,
        status: 'valid',
        checked_at: nowIso(),
        next_check_at: new Date(Date.now() + 30 * 86_400_000).toISOString(),
      },
    };
    db.accounts.forEach((a) => (a.active = false));
    db.accounts.push(acc);
    accountsChanged();
    return { status: 'success', account: clone(acc) };
  },
  auth_ownership_check: async ({ account_id }) => {
    await latency(4);
    const a = db.accounts.find((x) => x.id === account_id) ?? fail('not_found', account_id);
    if (a.ownership) {
      a.ownership.checked_at = nowIso();
      a.ownership.status = 'valid';
      a.ownership.next_check_at = new Date(Date.now() + 30 * 86_400_000).toISOString();
    }
    accountsChanged();
    return clone(a);
  },
  auth_set_active: async ({ account_id }) => {
    await latency(0.3);
    db.accounts.forEach((a) => (a.active = a.id === account_id));
    accountsChanged();
    return clone(db.accounts);
  },
  auth_logout: async ({ account_id }) => {
    await latency();
    db.accounts = db.accounts.filter((a) => a.id !== account_id);
    if (!db.accounts.some((a) => a.active) && db.accounts[0]) db.accounts[0].active = true;
    accountsChanged();
    return clone(db.accounts);
  },
  auth_rustic_access_token: () => {
    if (!db.rustic.signed_in) fail('unauthorized', 'Not signed in to Rustic');
    return {
      access_token: 'mock-access-token',
      expires_at: new Date(Date.now() + 15 * 60_000).toISOString(),
    };
  },
  auth_rustic_session: async () => {
    await latency(0.3);
    return clone(db.rustic);
  },
  auth_rustic_login: async () => {
    await sleep(1200);
    db.rustic = clone(initialRusticSession);
    return clone(db.rustic);
  },
  auth_rustic_signup: async (req) => {
    await latency();
    if (!req.accept_privacy) fail('invalid_argument', 'The privacy policy must be accepted.');
    if (req.password.length < 10)
      fail('invalid_argument', 'Password must be at least 10 characters.');
    return { email_verification_required: true, guardian_required: false };
  },
  auth_rustic_login_password: async ({ email, password }) => {
    await latency();
    if (!email.includes('@') || password.length < 10)
      fail('invalid_argument', 'Wrong email or password.');
    db.rustic = {
      signed_in: true,
      user_id: 'usr_sam',
      username: email.split('@')[0] ?? 'player',
      email,
    };
    return { mfa_required: false, session: clone(db.rustic) };
  },
  auth_rustic_login_mfa: async () => {
    await latency();
    return clone(db.rustic);
  },
  auth_rustic_resend_verification: async () => {
    await latency(0.5);
  },
  auth_rustic_logout: async () => {
    await latency();
    db.rustic = { signed_in: false, user_id: null, username: null, email: null };
    return clone(db.rustic) as RusticSession;
  },

  // ---- versions & instances ----
  horizon_versions_list: async () => {
    await latency(0.6);
    return clone(horizonVersions);
  },
  instances_list: async () => {
    await latency();
    return clone(db.instances);
  },
  instance_get: async ({ id }) => {
    await latency(0.5);
    return clone(findInstance(id));
  },
  instance_create: async (req) => {
    await latency(2);
    const inst: Instance = {
      id: uid('inst'),
      name: req.name,
      icon: req.icon,
      horizon_version: req.horizon_version,
      horizon_version_label: versionLabel(req.horizon_version),
      state: 'ready',
      update_available: null,
      mod_count: 0,
      resource_pack_count: 0,
      shader_count: 0,
      datapack_world_count: 0,
      playtime_s: 0,
      last_played_at: null,
      last_server: null,
      created_at: nowIso(),
      memory_mb: req.memory_mb ?? null,
      game_args: '',
      resolution: null,
      folder: `${db.settings.game.instances_dir}/${req.name.toLowerCase().replace(/\W+/g, '-')}`,
      source: null,
      pinned: false,
    };
    db.instances.push(inst);
    db.content[inst.id] = [];
    db.worlds[inst.id] = [];
    instancesChanged();
    return clone(inst);
  },
  instance_import: async (req) => {
    await latency(2);
    const name = req.name ?? (req.kind === 'modpack' ? 'Cobblestone Odyssey' : 'Imported instance');
    const inst: Instance = {
      ...clone(initialInstances[2]!),
      id: uid('inst'),
      name,
      state: 'installing',
      playtime_s: 0,
      last_played_at: null,
      last_server: null,
      created_at: nowIso(),
      pinned: false,
      source:
        req.kind === 'modpack'
          ? { kind: 'modpack', project_id: req.project_id, name, version: '2.0.1' }
          : null,
    };
    db.instances.push(inst);
    db.content[inst.id] = [];
    db.worlds[inst.id] = [];
    instancesChanged();
    const task_id = startDownload({
      kind: 'modpack',
      label: name,
      instance_id: inst.id,
      total_bytes: 180 * 1024 ** 2,
      duration_ms: 6000,
      onDone: () => {
        inst.state = 'ready';
        instancesChanged();
      },
    });
    return { instance: clone(inst), task_id };
  },
  instance_update: async ({ id, patch }) => {
    await latency(0.5);
    const inst = findInstance(id);
    Object.assign(inst, patch);
    instancesChanged();
    return clone(inst);
  },
  instance_delete: async ({ id }) => {
    await latency();
    db.instances = db.instances.filter((i) => i.id !== id);
    instancesChanged();
  },
  instance_duplicate: async ({ id }) => {
    await latency();
    const src = findInstance(id);
    const copy: Instance = {
      ...clone(src),
      id: uid('inst'),
      name: `${src.name} (2)`,
      playtime_s: 0,
      pinned: false,
      created_at: nowIso(),
    };
    db.instances.push(copy);
    db.content[copy.id] = clone(db.content[id] ?? []);
    db.worlds[copy.id] = clone(db.worlds[id] ?? []);
    instancesChanged();
    return copy;
  },
  instance_set_version: async ({ id, horizon_version }) => {
    await latency();
    const inst = findInstance(id);
    inst.state = 'installing';
    instancesChanged();
    const task_id = startDownload({
      kind: 'game',
      label: versionLabel(horizon_version),
      instance_id: id,
      total_bytes: 410 * 1024 ** 2,
      duration_ms: 5000,
      onDone: () => {
        inst.horizon_version = horizon_version;
        inst.horizon_version_label = versionLabel(horizon_version);
        inst.state = 'ready';
        inst.update_available = null;
        instancesChanged();
      },
    });
    return { task_id };
  },
  instance_open_folder: async () => {
    await latency(0.2);
  },
  instance_worlds_list: async ({ instance_id }) => {
    await latency(0.6);
    return clone(db.worlds[instance_id] ?? []);
  },
  instance_content_list: async ({ instance_id, kind }) => {
    await latency(0.7);
    const list = db.content[instance_id] ?? [];
    return clone(kind ? list.filter((c) => c.kind === kind) : list);
  },
  instance_content_set_enabled: async ({ instance_id, content_id, enabled }) => {
    await latency(0.3);
    const c =
      (db.content[instance_id] ?? []).find((x) => x.id === content_id) ??
      fail('not_found', content_id);
    c.enabled = enabled;
    return clone(c);
  },
  instance_content_remove: async ({ instance_id, content_id }) => {
    await latency(0.4);
    db.content[instance_id] = (db.content[instance_id] ?? []).filter((c) => c.id !== content_id);
    const inst = findInstance(instance_id);
    inst.mod_count = (db.content[instance_id] ?? []).filter((c) => c.kind === 'mod').length;
    instancesChanged();
  },
  instance_content_update: async ({ instance_id, content_id }) => {
    await latency();
    const list = (db.content[instance_id] ?? []).filter(
      (c) => c.update_available && (!content_id || c.id === content_id),
    );
    const task_id = startDownload({
      kind: 'content',
      label: list.map((c) => c.name).join(', ') || 'content',
      instance_id,
      total_bytes: 6 * 1024 ** 2 * Math.max(1, list.length),
      duration_ms: 2000,
      onDone: () => {
        for (const c of list) {
          c.version = c.update_available ?? c.version;
          c.update_available = null;
        }
      },
    });
    return { task_id };
  },
  instance_resource_packs_reorder: async ({ instance_id, ordered_ids }) => {
    await latency(0.3);
    const list = db.content[instance_id] ?? [];
    ordered_ids.forEach((id, i) => {
      const c = list.find((x) => x.id === id);
      if (c) c.order = i;
    });
    return clone(
      list.filter((c) => c.kind === 'resourcepack').sort((a, b) => (a.order ?? 0) - (b.order ?? 0)),
    );
  },

  // ---- launch ----
  instance_launch: async ({ instance_id, options }) => {
    const inst = findInstance(instance_id);
    if (db.launches.get(instance_id)?.phase === 'running')
      return clone(db.launches.get(instance_id)!);
    if (!db.accounts.some((a) => a.active)) fail('unauthorized', 'No active game account');
    const needsUpdate = inst.state === 'needs_update' && inst.update_available;
    setLaunch({ instance_id, phase: 'checking', progress: null });
    const run = async () => {
      await sleep(500);
      if (db.launches.get(instance_id)?.phase !== 'checking') return;
      const finish = async () => {
        setLaunch({ instance_id, phase: 'launching', progress: null });
        await sleep(900);
        if (db.launches.get(instance_id)?.phase !== 'launching') return;
        inst.state = 'running';
        inst.last_played_at = nowIso();
        if (options?.join)
          inst.last_server = {
            name: options.join.name ?? options.join.address,
            address: options.join.address,
          };
        instancesChanged();
        setLaunch({ instance_id, phase: 'running', progress: null, started_at: nowIso() });
        emit('game-log', {
          instance_id,
          level: 'info',
          line: '[Horizon] Rustic edition client started',
        });
      };
      if (needsUpdate) {
        const target = inst.update_available!;
        let taskId = '';
        taskId = startDownload({
          kind: 'game',
          label: versionLabel(target),
          instance_id,
          total_bytes: 412 * 1024 ** 2,
          duration_ms: 4500,
          onProgress: (p) => setLaunch({ instance_id, phase: 'downloading', progress: p }),
          onDone: () => {
            inst.horizon_version = target;
            inst.horizon_version_label = versionLabel(target);
            inst.update_available = null;
            inst.state = 'ready';
            instancesChanged();
            void finish();
          },
        });
        db.launchCancels.set(instance_id, () => cancelDownload(taskId));
      } else {
        await finish();
      }
    };
    void run();
    return clone(db.launches.get(instance_id)!);
  },
  instance_launch_cancel: async ({ instance_id }) => {
    db.launchCancels.get(instance_id)?.();
    db.launchCancels.delete(instance_id);
    cancelDownloadsFor(instance_id);
    const inst = findInstance(instance_id);
    if (inst.update_available) inst.state = 'needs_update';
    const s: LaunchStatus = { instance_id, phase: 'idle', progress: null };
    setLaunch(s);
    return s;
  },
  instance_stop: async ({ instance_id }) => {
    const inst = findInstance(instance_id);
    setLaunch({ instance_id, phase: 'stopping', progress: null });
    await sleep(600);
    inst.state = inst.update_available ? 'needs_update' : 'ready';
    inst.playtime_s += 42 * 60;
    instancesChanged();
    const s: LaunchStatus = { instance_id, phase: 'idle', progress: null };
    setLaunch(s);
    emit('game-exited', { instance_id, code: 0, crashed: false, played_s: 42 * 60 });
    return s;
  },
  launch_status: () => [...db.launches.values()].map((s) => ({ ...s })),
};

/** Helper for other mock areas: add installed content to an instance. */
export function addInstanceContent(instanceId: string, item: InstalledContent): void {
  const list = (db.content[instanceId] ??= []);
  list.push(item);
  const inst = db.instances.find((i) => i.id === instanceId);
  if (inst) {
    inst.mod_count = list.filter((c) => c.kind === 'mod').length;
    inst.resource_pack_count = list.filter((c) => c.kind === 'resourcepack').length;
    inst.shader_count = list.filter((c) => c.kind === 'shader').length;
    instancesChanged();
  }
}

// keep `simulate` referenced for areas that import it from here
export { simulate };
