/** Create-server wizard state, validation and request building (pure, unit-tested). */
import type { VpsConnectionRequest } from '@/ipc/types/hosting';
import type { InstanceIconPreset } from '@/ipc/types/instances';
import type { CreateServerRequest, ServerEngine, WorldSetup } from '@/ipc/types/servers';

/** Three steps (UX_SPEC 4.4, wireframes 06-08): Basics, Where to host, Access. */
export const WIZARD_STEPS = ['basics', 'hosting', 'access'] as const;
export type WizardStep = (typeof WIZARD_STEPS)[number];

export type HostingChoice = 'local' | 'cloud' | 'vps';
export type WorldKind = 'new' | 'import' | 'from_instance';

export interface VpsForm {
  host: string;
  port: string;
  username: string;
  /** generate: the launcher creates a key pair; key: existing key file; password: used once, never stored. */
  authKind: 'generate' | 'key' | 'password';
  keyPath: string;
  passphrase: string;
  password: string;
  label: string;
}

export interface WizardState {
  name: string;
  preset: InstanceIconPreset;
  color: string;
  hosting: HostingChoice;
  vps: VpsForm;
  /** Trusted connection chosen for the install (set after fingerprint confirmation). */
  vpsConnectionId: string | null;
  /** Result of the resource check for that connection. */
  vpsReady: boolean;
  /** RAM of the VPS, bounds the RAM slider. */
  vpsMemoryMb: number | null;
  engine: ServerEngine;
  horizonVersion: string;
  memoryMb: number;
  maxPlayers: string;
  eulaAccepted: boolean;
  starterPlugins: string[];
  startWithLauncher: boolean;
  /** Consent to open the game port in the VPS firewall (ufw/firewalld). */
  vpsOpenFirewall: boolean;
  access: { relay: boolean; invite: boolean };
  /** Open the domain setup (server Access tab) once created. */
  domainAfter: boolean;
  world: {
    kind: WorldKind;
    seed: string;
    levelType: 'default' | 'flat' | 'large_biomes' | 'amplified';
    structures: boolean;
    gamemode: 'survival' | 'creative' | 'adventure';
    difficulty: 'peaceful' | 'easy' | 'normal' | 'hard';
    hardcore: boolean;
    importPath: string;
    instanceId: string;
    worldId: string;
  };
}

export const initialWizardState = (): WizardState => ({
  name: '',
  preset: 'grass',
  color: '#4E9F5B',
  hosting: 'local',
  vps: {
    host: '',
    port: '22',
    username: 'root',
    authKind: 'generate',
    keyPath: '',
    passphrase: '',
    password: '',
    label: '',
  },
  vpsConnectionId: null,
  vpsReady: false,
  vpsMemoryMb: null,
  engine: 'rustic',
  horizonVersion: '',
  memoryMb: 4096,
  maxPlayers: '20',
  eulaAccepted: false,
  starterPlugins: [],
  startWithLauncher: false,
  vpsOpenFirewall: false,
  access: { relay: true, invite: true },
  domainAfter: false,
  world: {
    kind: 'new',
    seed: '',
    levelType: 'default',
    structures: true,
    gamemode: 'survival',
    difficulty: 'normal',
    hardcore: false,
    importPath: '',
    instanceId: '',
    worldId: '',
  },
});

/** i18n keys (namespace `servers`, under `wizard.errors`) per invalid field. */
export type FieldErrors = Partial<Record<string, string>>;

const HOSTNAME_RE = /^(?=.{1,253}$)(?!-)[a-z0-9-]{1,63}(?<!-)(\.(?!-)[a-z0-9-]{1,63}(?<!-))*$/i;
const IPV4_RE = /^(25[0-5]|2[0-4]\d|1?\d?\d)(\.(25[0-5]|2[0-4]\d|1?\d?\d)){3}$/;
const IPV6_RE = /^[0-9a-f:]+$/i;

export function isValidHost(host: string): boolean {
  const h = host.trim();
  if (!h) return false;
  if (IPV4_RE.test(h)) return true;
  if (h.includes(':')) return IPV6_RE.test(h) && h.split(':').length >= 3;
  if (/^\d+(\.\d+)*$/.test(h)) return false; // numeric but not a valid IPv4
  return HOSTNAME_RE.test(h);
}

export function isValidPort(port: string): boolean {
  if (!/^\d{1,5}$/.test(port.trim())) return false;
  const n = Number(port);
  return n >= 1 && n <= 65535;
}

export function validateName(name: string): string | undefined {
  const n = name.trim();
  if (!n) return 'wizard.errors.nameRequired';
  if (n.length > 40) return 'wizard.errors.nameTooLong';
  return undefined;
}

export function validateVpsForm(f: VpsForm): FieldErrors {
  const e: FieldErrors = {};
  if (!isValidHost(f.host)) e.host = 'wizard.errors.host';
  if (!isValidPort(f.port)) e.port = 'wizard.errors.port';
  if (!/^[a-z_][a-z0-9_-]{0,31}$/i.test(f.username.trim())) e.username = 'wizard.errors.username';
  if ((f.authKind === 'key' || f.authKind === 'generate') && !f.keyPath.trim()) {
    e.keyPath = f.authKind === 'generate' ? 'wizard.errors.generateKey' : 'wizard.errors.keyPath';
  }
  if (f.authKind === 'password' && !f.password) e.password = 'wizard.errors.password';
  return e;
}

export function validateStep(step: WizardStep, s: WizardState): FieldErrors {
  const e: FieldErrors = {};
  switch (step) {
    case 'basics': {
      const n = validateName(s.name);
      if (n) e.name = n;
      if (!s.horizonVersion) e.version = 'wizard.errors.version';
      const p = Number(s.maxPlayers);
      if (!/^\d+$/.test(s.maxPlayers.trim()) || p < 1 || p > 1000)
        e.maxPlayers = 'wizard.errors.maxPlayers';
      if (s.world.kind === 'import' && !s.world.importPath.trim())
        e.importPath = 'wizard.errors.importPath';
      if (s.world.kind === 'from_instance' && (!s.world.instanceId || !s.world.worldId))
        e.world = 'wizard.errors.world';
      if (s.world.kind === 'new' && s.world.seed.length > 64) e.seed = 'wizard.errors.seed';
      if (!s.eulaAccepted) e.eula = 'wizard.errors.eula';
      break;
    }
    case 'hosting':
      if (s.hosting === 'cloud') e.hosting = 'wizard.errors.cloudUnavailable';
      if (s.hosting === 'vps' && (!s.vpsConnectionId || !s.vpsReady))
        e.hosting = 'wizard.errors.vpsNotReady';
      if (s.memoryMb < 1024) e.memory = 'wizard.errors.memory';
      break;
    case 'access':
      // LAN is always on; everything else is optional. Creating validates every step.
      for (const st of WIZARD_STEPS.slice(0, -1)) Object.assign(e, validateStep(st, s));
      break;
  }
  return e;
}

export function canProceed(step: WizardStep, s: WizardState): boolean {
  return Object.keys(validateStep(step, s)).length === 0;
}

export function buildWorld(s: WizardState): WorldSetup {
  const w = s.world;
  if (w.kind === 'import') return { kind: 'import', path: w.importPath.trim() };
  if (w.kind === 'from_instance')
    return { kind: 'from_instance', instance_id: w.instanceId, world_id: w.worldId };
  return {
    kind: 'new',
    seed: w.seed.trim() || null,
    level_type: w.levelType,
    generate_structures: w.structures,
    gamemode: w.gamemode,
    difficulty: w.hardcore ? 'hard' : w.difficulty,
    hardcore: w.hardcore,
  };
}

export function buildCreateRequest(s: WizardState): CreateServerRequest {
  return {
    name: s.name.trim(),
    icon: { kind: 'preset', preset: s.preset, color: s.color },
    engine: s.engine,
    horizon_version: s.horizonVersion,
    hosting:
      s.hosting === 'vps' && s.vpsConnectionId
        ? { mode: 'vps', connection_id: s.vpsConnectionId }
        : { mode: 'local' },
    world: buildWorld(s),
    memory_mb: s.memoryMb,
    max_players: Number(s.maxPlayers),
    eula_accepted: s.eulaAccepted,
    starter_plugins: s.engine === 'rustic' ? s.starterPlugins : [],
    // A VPS has a public IP: no relay hostname needed (domains use A/AAAA records).
    access: { relay: s.hosting === 'local' && s.access.relay, invite: s.access.invite },
    start_with_launcher: s.hosting === 'local' && s.startWithLauncher,
  };
}

export function buildVpsRequest(f: VpsForm): VpsConnectionRequest {
  return {
    host: f.host.trim(),
    port: Number(f.port),
    username: f.username.trim(),
    auth:
      f.authKind !== 'password'
        ? {
            kind: 'key',
            key_path: f.keyPath.trim(),
            ...(f.passphrase ? { passphrase: f.passphrase } : {}),
          }
        : { kind: 'password', password: f.password },
  };
}

/** RAM slider bounds: local leaves 2 GB to the OS, VPS leaves 1 GB. */
export function memoryBounds(
  hosting: HostingChoice,
  systemMb: number | undefined,
  vpsMb: number | null,
): { min: number; max: number } {
  const min = 1024;
  const total = hosting === 'vps' ? (vpsMb ?? 4096) - 1024 : (systemMb ?? 8192) - 2048;
  const max = Math.max(2048, Math.min(32768, Math.floor(total / 512) * 512));
  return { min, max };
}

/** A random numeric seed as the game would generate (signed 64-bit range, as text). */
export function randomSeed(rand: () => number = Math.random): string {
  const hi = Math.floor(rand() * 2 ** 31) * (rand() < 0.5 ? -1 : 1);
  const lo = Math.floor(rand() * 2 ** 32);
  return (BigInt(hi) * 2n ** 32n + BigInt(lo)).toString();
}

/**
 * Whether creating failed because no `rustic-server` program was found for the server's build
 * (reason `server.no_rustic_server`, launcher ADR 0152): the toast then points to Settings.
 */
export function isNoRusticServer(e: unknown): boolean {
  return (
    typeof e === 'object' &&
    e !== null &&
    (e as { details?: { reason?: unknown } }).details?.reason === 'server.no_rustic_server'
  );
}
