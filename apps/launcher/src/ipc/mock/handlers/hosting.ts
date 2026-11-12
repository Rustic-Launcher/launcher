/** Mock handlers for hosting: Rustic Cloud placeholder and VPS over SSH/SFTP. */
import type {
  InstallStep,
  InstallStepId,
  VpsConnection,
  VpsConnectionRequest,
} from '../../types/hosting';
import type { Server } from '../../types/servers';
import { serversDb } from '../data/servers';
import {
  daysAgo,
  emit,
  fail,
  latency,
  nowIso,
  sleep,
  SPEED,
  uid,
  type MockHandlers,
} from '../runtime';
import { serversChanged, setServerState, slug, versionText, worldName } from './servers';

let connections: VpsConnection[] = [
  {
    id: 'vps_ovh',
    label: 'OVH VPS-2',
    host: '203.0.113.84',
    port: 22,
    username: 'rustic',
    fingerprint_sha256: 'SHA256:q3V7nT0kz1mE4yLr9bWcXpA2fHsD8uJgK6oNiR5tYvM',
    added_at: daysAgo(25),
  },
];

/** Deterministic, realistic-looking SHA256 host key fingerprint for a host. */
export function fakeFingerprint(host: string): string {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  let h = 2166136261;
  let out = '';
  for (let i = 0; i < 43; i++) {
    const c = host.charCodeAt(i % Math.max(1, host.length)) || 0;
    h = Math.imul(h ^ (c + i), 16777619) >>> 0;
    out += alphabet[h % 64];
  }
  return `SHA256:${out}`;
}

function validate(req: VpsConnectionRequest) {
  if (!req.host.trim()) fail('invalid_argument', 'host');
  if (!req.username.trim()) fail('invalid_argument', 'username');
  if (!(req.port > 0 && req.port < 65536)) fail('invalid_argument', 'port');
}

const STEPS: InstallStepId[] = [
  'connect',
  'check',
  'user',
  'download',
  'engine',
  'service',
  'firewall',
  'world',
  'start',
];

const STEP_LOGS: Record<InstallStepId, string[]> = {
  connect: ['$ ssh {user}@{host} -p {port}', 'Authenticated with host key {fp}'],
  check: [
    '$ uname -m && cat /etc/os-release',
    'x86_64 · Ubuntu 24.04.1 LTS',
    'Memory 8 GiB, disk free 61 GiB, systemd 255',
  ],
  user: ['$ sudo useradd --system --home /srv/rustic rustic', 'Created service user rustic'],
  download: [
    'Uploading horizon-server (supervisor) over SFTP… 18.4 MiB',
    'Uploading rustic-server {version} over SFTP… 77.9 MiB',
    'SHA-256 verified',
  ],
  engine: [
    'Writing /srv/rustic/{slug}/server.toml (engine.path = ./rustic-server)',
    'Prepared game files transferred from this PC (your own install)',
  ],
  service: [
    'Writing /etc/systemd/system/rustic-{slug}.service',
    '$ sudo systemctl daemon-reload && sudo systemctl enable rustic-{slug}',
  ],
  firewall: ['$ sudo ufw allow 25565/tcp', 'Rule added (v4 and v6)'],
  world: ['Generating world settings (seed, level type)', 'eula and properties written'],
  start: ['$ sudo systemctl start rustic-{slug}', 'Server listening on 0.0.0.0:25565', 'Done.'],
};

export const hostingHandlers: MockHandlers = {
  hosting_providers: async () => {
    await latency(0.3);
    return [
      { id: 'local', available: true, coming_soon: false },
      { id: 'rustic_cloud', available: false, coming_soon: true },
      { id: 'vps', available: true, coming_soon: false },
    ];
  },
  hosting_waitlist_join: async ({ email }) => {
    await latency();
    if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) fail('invalid_argument', 'email');
    return { position: 1284 };
  },
  hosting_vps_test: async (req) => {
    validate(req);
    await latency(3);
    const host = req.host.trim();
    if (/unreachable|^10\.255\./.test(host)) {
      return {
        reachable: false,
        latency_ms: null,
        fingerprint: null,
        known: false,
        auth_ok: false,
        error: 'Connection timed out',
      };
    }
    const authOk =
      req.auth.kind === 'key'
        ? req.auth.key_path.trim() !== ''
        : req.auth.password !== 'wrong' && req.auth.password !== '';
    const known = connections.some((c) => c.host === host && c.port === req.port);
    return {
      reachable: true,
      latency_ms: 18 + (host.length % 30),
      fingerprint: {
        algorithm: 'ssh-ed25519',
        sha256: known
          ? connections.find((c) => c.host === host)!.fingerprint_sha256
          : fakeFingerprint(host),
      },
      known,
      auth_ok: authOk,
      error: authOk ? undefined : 'Permission denied (publickey,password)',
    };
  },
  hosting_vps_trust: async (req) => {
    validate(req);
    await latency();
    const existing = connections.find((c) => c.host === req.host.trim() && c.port === req.port);
    if (existing) return structuredClone(existing);
    const conn: VpsConnection = {
      id: uid('vps'),
      label: req.label?.trim() || req.host.trim(),
      host: req.host.trim(),
      port: req.port,
      username: req.username.trim(),
      fingerprint_sha256: req.fingerprint_sha256,
      added_at: nowIso(),
    };
    connections = [...connections, conn];
    return structuredClone(conn);
  },
  hosting_vps_generate_key: async ({ label }) => {
    await latency(1.5);
    const name = slug(label || 'vps') || 'vps';
    const body = fakeFingerprint(`${name}-${Date.now()}`).slice(7).replace(/\//g, 'x');
    const public_key = `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI${body} rustic-launcher@${name}`;
    return {
      key_path: `<launcher>/keys/${name}_ed25519`,
      public_key,
      authorize_command: `mkdir -p ~/.ssh && echo "${public_key}" >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys`,
    };
  },
  hosting_vps_list: async () => {
    await latency(0.5);
    return structuredClone(connections);
  },
  hosting_vps_remove: async ({ id }) => {
    await latency(0.5);
    if (serversDb.servers.some((s) => s.hosting.mode === 'vps' && s.hosting.connection_id === id)) {
      fail('conflict', 'a server still uses this connection');
    }
    connections = connections.filter((c) => c.id !== id);
  },
  hosting_vps_check: async ({ connection_id }) => {
    await latency(4);
    const c = connections.find((x) => x.id === connection_id) ?? fail('not_found', connection_id);
    return {
      os: 'Ubuntu 24.04.1 LTS',
      arch: 'x86_64',
      cpu_cores: 4,
      memory_mb: 8192,
      disk_free_mb: 62_400,
      has_systemd: true,
      public_ip: c.host.match(/^\d+\.\d+\.\d+\.\d+$/) ? c.host : '203.0.113.17',
      requirements_met: true,
      problems: [
        {
          code: 'sudo',
          detail: 'sudo asks for a password: you will be asked for it once during the install',
        },
      ],
    };
  },
  hosting_vps_install: async ({ connection_id, server: req }) => {
    await latency();
    if (!req.eula_accepted) fail('invalid_argument', 'eula');
    const conn =
      connections.find((x) => x.id === connection_id) ?? fail('not_found', connection_id);
    if (!req.name.trim()) fail('invalid_argument', 'name');
    const s = slug(req.name);
    const server: Server = {
      id: uid('srv'),
      name: req.name.trim(),
      icon: req.icon,
      engine: req.engine,
      horizon_version: req.horizon_version,
      horizon_version_label: versionText(req.horizon_version),
      hosting: { mode: 'vps', connection_id, host: conn.host },
      state: 'installing',
      players_online: 0,
      max_players: req.max_players,
      port: 25565,
      motd:
        req.engine === 'rustic'
          ? `${req.name.trim()} · Rustic Server`
          : `${req.name.trim()} · Horizon`,
      world_name: worldName(req),
      memory_mb: req.memory_mb,
      created_at: nowIso(),
      last_started_at: null,
      share_address: null,
      plugin_count: req.engine === 'rustic' ? (req.starter_plugins?.length ?? 0) : 0,
      folder: `/srv/rustic/${s}`,
    };
    serversDb.servers.push(server);
    serversChanged();
    const task_id = uid('task');
    const steps: InstallStep[] = STEPS.map((id) => ({ id, state: 'pending' }));
    const fill = (line: string) =>
      line
        .replaceAll('{user}', conn.username)
        .replaceAll('{host}', conn.host)
        .replaceAll('{port}', String(conn.port))
        .replaceAll('{fp}', conn.fingerprint_sha256)
        .replaceAll('{slug}', s)
        .replaceAll('{version}', versionText(req.horizon_version));
    const send = (log?: string, done = false) =>
      emit('hosting-install', {
        task_id,
        server_id: server.id,
        steps: structuredClone(steps),
        log,
        done,
      });
    void (async () => {
      await sleep(300);
      for (const st of steps) {
        if (req.engine === 'horizon' && st.id === 'engine') {
          st.state = 'skipped';
          st.detail = 'vanilla Horizon';
          send('Vanilla Horizon: no Rustic engine to configure');
          continue;
        }
        st.state = 'running';
        send();
        for (const line of STEP_LOGS[st.id]) {
          await sleep(260 / Math.max(0.1, SPEED));
          send(fill(line));
        }
        st.state = 'done';
        send();
      }
      server.last_started_at = nowIso();
      server.share_address = `${conn.host}:${server.port}`;
      setServerState(server, 'running');
      send(undefined, true);
    })();
    return { server: structuredClone(server), task_id };
  },
};
