import { describe, expect, it } from 'vitest';
import { IpcFailure } from '@/ipc';
import {
  buildCreateRequest,
  buildVpsRequest,
  canProceed,
  initialWizardState,
  isNoRusticServer,
  isValidHost,
  isValidPort,
  memoryBounds,
  randomSeed,
  validateStep,
  validateVpsForm,
  type WizardState,
} from './model';

const valid = (): WizardState => ({
  ...initialWizardState(),
  name: 'Les Copains',
  horizonVersion: 'v0.2.15',
  eulaAccepted: true,
});

describe('host and port validation', () => {
  it.each(['203.0.113.84', 'vps.example.com', 'my-server', '2001:db8::1'])('accepts %s', (h) => {
    expect(isValidHost(h)).toBe(true);
  });
  it.each(['', '999.1.1.1', '-bad.com', 'bad_host.com', 'a..b'])('rejects %s', (h) => {
    expect(isValidHost(h)).toBe(false);
  });
  it('validates ports', () => {
    expect(isValidPort('22')).toBe(true);
    expect(isValidPort('65535')).toBe(true);
    expect(isValidPort('0')).toBe(false);
    expect(isValidPort('70000')).toBe(false);
    expect(isValidPort('ssh')).toBe(false);
  });
});

describe('validateStep', () => {
  it('requires a name and the EULA on Basics', () => {
    const e = validateStep('basics', { ...initialWizardState(), horizonVersion: 'v1' });
    expect(e).toEqual({ name: 'wizard.errors.nameRequired', eula: 'wizard.errors.eula' });
    expect(canProceed('basics', valid())).toBe(true);
  });
  it('checks players and version', () => {
    expect(validateStep('basics', { ...valid(), maxPlayers: '0' }).maxPlayers).toBeDefined();
    expect(validateStep('basics', { ...valid(), horizonVersion: '' }).version).toBeDefined();
  });
  it('needs a source for imported and copied worlds', () => {
    const s = valid();
    expect(canProceed('basics', { ...s, world: { ...s.world, kind: 'import' } })).toBe(false);
    expect(
      canProceed('basics', { ...s, world: { ...s.world, kind: 'import', importPath: 'C:/w' } }),
    ).toBe(true);
    expect(
      canProceed('basics', { ...s, world: { ...s.world, kind: 'from_instance', instanceId: 'i' } }),
    ).toBe(false);
  });
  it('refuses Rustic Cloud (coming soon) and an unverified VPS', () => {
    expect(canProceed('hosting', { ...valid(), hosting: 'cloud' })).toBe(false);
    expect(canProceed('hosting', { ...valid(), hosting: 'vps' })).toBe(false);
    expect(
      canProceed('hosting', {
        ...valid(),
        hosting: 'vps',
        vpsConnectionId: 'vps_1',
        vpsReady: true,
      }),
    ).toBe(true);
  });
  it('the last step aggregates every step', () => {
    expect(Object.keys(validateStep('access', initialWizardState()))).toEqual(
      expect.arrayContaining(['name', 'version', 'eula']),
    );
    expect(canProceed('access', valid())).toBe(true);
  });
});

describe('VPS form', () => {
  it('requires the right credential for the auth kind', () => {
    const f = {
      host: '203.0.113.84',
      port: '22',
      username: 'root',
      authKind: 'key' as 'key' | 'generate' | 'password',
      keyPath: '',
      passphrase: '',
      password: '',
      label: '',
    };
    expect(validateVpsForm(f)).toEqual({ keyPath: 'wizard.errors.keyPath' });
    expect(validateVpsForm({ ...f, authKind: 'generate' })).toEqual({
      keyPath: 'wizard.errors.generateKey',
    });
    expect(validateVpsForm({ ...f, authKind: 'password' })).toEqual({
      password: 'wizard.errors.password',
    });
    expect(validateVpsForm({ ...f, keyPath: '~/.ssh/id_ed25519' })).toEqual({});
  });
  it('builds the IPC request without leaking unused secrets', () => {
    const r = buildVpsRequest({
      host: ' h.example ',
      port: '2222',
      username: 'deploy',
      authKind: 'key',
      keyPath: 'k',
      passphrase: '',
      password: 'nope',
      label: '',
    });
    expect(r).toEqual({
      host: 'h.example',
      port: 2222,
      username: 'deploy',
      auth: { kind: 'key', key_path: 'k' },
    });
  });
});

describe('buildCreateRequest', () => {
  it('builds a local Rustic Server request with a new world', () => {
    const r = buildCreateRequest({ ...valid(), world: { ...valid().world, seed: '  42 ' } });
    expect(r.hosting).toEqual({ mode: 'local' });
    expect(r.engine).toBe('rustic');
    expect(r.max_players).toBe(20);
    expect(r.world).toMatchObject({ kind: 'new', seed: '42', level_type: 'default' });
    expect(r.eula_accepted).toBe(true);
    expect(r.access).toEqual({ relay: true, invite: true });
  });
  it('drops starter plugins for vanilla Horizon and the relay for a VPS', () => {
    const r = buildCreateRequest({
      ...valid(),
      engine: 'horizon',
      starterPlugins: ['perms'],
      hosting: 'vps',
      vpsConnectionId: 'v',
    });
    expect(r.starter_plugins).toEqual([]);
    expect(r.access.relay).toBe(false);
  });
  it('forces hard difficulty for hardcore worlds', () => {
    const s = valid();
    const r = buildCreateRequest({
      ...s,
      world: { ...s.world, hardcore: true, difficulty: 'easy' },
    });
    expect(r.world).toMatchObject({ hardcore: true, difficulty: 'hard' });
  });
  it('targets the trusted VPS connection', () => {
    expect(
      buildCreateRequest({ ...valid(), hosting: 'vps', vpsConnectionId: 'vps_1' }).hosting,
    ).toEqual({
      mode: 'vps',
      connection_id: 'vps_1',
    });
  });
});

describe('helpers', () => {
  it('bounds RAM from the machine size', () => {
    expect(memoryBounds('local', 16384, null)).toEqual({ min: 1024, max: 14336 });
    expect(memoryBounds('vps', 32768, 8192)).toEqual({ min: 1024, max: 7168 });
    expect(memoryBounds('vps', undefined, 2048).max).toBe(2048);
  });
  it('generates numeric seeds', () => {
    expect(randomSeed(() => 0.5)).toMatch(/^-?\d+$/);
  });
  it('recognises the missing rustic-server refusal by its reason', () => {
    expect(
      isNoRusticServer(
        new IpcFailure({
          code: 'unavailable',
          message: 'The rustic-server program was not found.',
          details: { reason: 'server.no_rustic_server' },
        }),
      ),
    ).toBe(true);
    expect(isNoRusticServer(new IpcFailure({ code: 'unavailable', message: 'down' }))).toBe(false);
    expect(isNoRusticServer(new Error('x'))).toBe(false);
    expect(isNoRusticServer(null)).toBe(false);
  });
});
