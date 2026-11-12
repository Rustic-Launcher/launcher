import { describe, expect, it } from 'vitest';
import { initialInstances } from '@/ipc/mock/data/core';
import { serversDb } from '@/ipc/mock/data/servers';
import {
  eligibleServers,
  formatTarget,
  hueOf,
  isInstalledOn,
  parseTarget,
  resolveTarget,
  targetKindsFor,
  toInstallTarget,
} from './target';

const servers = serversDb.servers;

describe('content targets', () => {
  it('parses and formats', () => {
    expect(parseTarget('instance:inst_pvp')).toEqual({ kind: 'instance', id: 'inst_pvp' });
    expect(parseTarget('server:srv_1')).toEqual({ kind: 'server', id: 'srv_1' });
    expect(parseTarget('world:x')).toBeNull();
    expect(parseTarget('instance:')).toBeNull();
    expect(parseTarget(undefined)).toBeNull();
    expect(formatTarget({ kind: 'server', id: 'a' })).toBe('server:a');
  });

  it('knows where each kind goes', () => {
    expect(targetKindsFor('mod')).toEqual(['instance']);
    expect(targetKindsFor('plugin')).toEqual(['server']);
    expect(targetKindsFor('modpack')).toEqual([]);
  });

  it('only offers Rustic Server servers for plugins', () => {
    const list = eligibleServers('plugin', servers);
    expect(list.length).toBeGreaterThan(0);
    expect(list.every((s) => s.engine === 'rustic')).toBe(true);
    expect(eligibleServers('mod', servers)).toEqual([]);
  });

  it('resolves the URL target, else the active instance, else a server', () => {
    expect(
      resolveTarget(
        'mod',
        { kind: 'instance', id: 'inst_pvp' },
        initialInstances,
        servers,
        'inst_survival',
      ),
    ).toEqual({ kind: 'instance', id: 'inst_pvp' });
    expect(
      resolveTarget(
        'mod',
        { kind: 'server', id: 'srv_copains' },
        initialInstances,
        servers,
        'inst_survival',
      ),
    ).toEqual({ kind: 'instance', id: 'inst_survival' });
    expect(resolveTarget('plugin', null, initialInstances, servers, 'inst_survival')?.kind).toBe(
      'server',
    );
    expect(
      resolveTarget(
        'plugin',
        { kind: 'server', id: 'srv_creative' },
        initialInstances,
        servers,
        undefined,
      ),
    ).not.toEqual({ kind: 'server', id: 'srv_creative' });
    expect(resolveTarget('plugin', null, initialInstances, [], undefined)).toBeNull();
    expect(resolveTarget('modpack', null, initialInstances, servers, 'inst_survival')).toBeNull();
  });

  it('maps to IPC install targets', () => {
    expect(toInstallTarget({ kind: 'instance', id: 'i' }, 'datapack', 'w')).toEqual({
      kind: 'world',
      instance_id: 'i',
      world_id: 'w',
    });
    expect(toInstallTarget({ kind: 'server', id: 's' }, 'datapack')).toEqual({
      kind: 'server_world',
      server_id: 's',
    });
    expect(toInstallTarget({ kind: 'server', id: 's' }, 'plugin')).toEqual({
      kind: 'server',
      server_id: 's',
    });
    expect(toInstallTarget({ kind: 'instance', id: 'i' }, 'mod')).toEqual({
      kind: 'instance',
      instance_id: 'i',
    });
  });

  it('checks installation and hashes ids stably', () => {
    expect(isInstalledOn({ installed_in: ['a'] }, { kind: 'instance', id: 'a' })).toBe(true);
    expect(isInstalledOn({ installed_in: ['a'] }, null)).toBe(false);
    expect(hueOf('p_minimap')).toBe(hueOf('p_minimap'));
    expect(hueOf('x')).toBeGreaterThanOrEqual(0);
  });
});
