import { describe, expect, it, vi } from 'vitest';
import { friendErrorKey, validName } from './addFriend';

async function mocks() {
  vi.resetModules();
  window.localStorage.clear();
  window.localStorage.setItem('rustic.mock.latency', '0');
  const { socialMock } = await import('@/cloud/mock/social');
  const { accountMock } = await import('@/cloud/mock/account');
  return { social: socialMock, account: accountMock };
}

const codeOf = async (p: Promise<unknown>) => {
  try {
    await p;
    return null;
  } catch (e) {
    return (e as { code?: string }).code ?? 'no-code';
  }
};

describe('add friend helpers', () => {
  it('validates names like the API', () => {
    expect(validName('rustic', 'lea_builds')).toBe(true);
    expect(validName('rustic', 'ab')).toBe(false);
    expect(validName('rustic', 'a'.repeat(21))).toBe(false);
    expect(validName('rustic', 'bad name')).toBe(false);
    expect(validName('minecraft', 'a'.repeat(16))).toBe(true);
    expect(validName('minecraft', 'a'.repeat(17))).toBe(false);
  });

  it('maps every API error code to a message key', () => {
    expect(friendErrorKey('not_found', 'rustic')).toBe('notFoundRustic');
    expect(friendErrorKey('not_found', 'minecraft')).toBe('notFoundMinecraft');
    expect(friendErrorKey('invalid_input', 'minecraft')).toBe('invalidMinecraft');
    for (const c of [
      'cannot_add_friend',
      'friend_requests_disabled',
      'cannot_add_self',
      'already_friends',
      'request_pending',
      'incoming_request_exists',
      'rate_limited',
    ]) {
      expect(friendErrorKey(c, 'rustic')).toBe(c);
    }
    expect(friendErrorKey(null, 'rustic')).toBe('generic');
    expect(friendErrorKey('internal', 'rustic')).toBe('generic');
  });
});

describe('social mock: lookup, requests, suggestions', { timeout: 30_000 }, () => {
  it('looks up by Rustic or Minecraft name and hides a minor’s Minecraft name', async () => {
    const { social } = await mocks();
    const nina = await social.lookupUser!('rustic', 'ninasky');
    expect(nina).toMatchObject({ minecraft_name: 'Nina_Sky', relation: 'none' });
    const rayan = await social.lookupUser!('minecraft', 'raYanTheBold');
    expect(rayan.user.username).toBe('RayanPvP');
    expect((await social.lookupUser!('rustic', 'zoe_crafts')).minecraft_name).toBeNull();
    expect((await social.lookupUser!('minecraft', 'ZoeCrafts')).minecraft_name).toBe('ZoeCrafts');
    const hugo = await social.lookupUser!('rustic', 'HugoMines');
    expect(hugo).toMatchObject({ relation: 'incoming_request', request_id: 'fr_1' });
    expect((await social.lookupUser!('rustic', 'lea_builds')).relation).toBe('friends');
    expect((await social.lookupUser!('rustic', 'chloe_art')).relation).toBe('outgoing_request');
  });

  it('answers the API error codes', async () => {
    const { social } = await mocks();
    expect(await codeOf(social.lookupUser!('rustic', 'nobody_here'))).toBe('not_found');
    expect(await codeOf(social.lookupUser!('rustic', 'a b'))).toBe('invalid_input');
    expect(await codeOf(social.lookupUser!('rustic', 'samcrafte'))).toBe('cannot_add_self');
    // Blocked and parental controls look the same.
    expect(await codeOf(social.lookupUser!('rustic', 'kev_grief'))).toBe('cannot_add_friend');
    expect(await codeOf(social.lookupUser!('rustic', 'milo_jr'))).toBe('cannot_add_friend');
    expect(await codeOf(social.sendFriendRequest!({ username: 'lea_builds' }))).toBe(
      'already_friends',
    );
    expect(await codeOf(social.sendFriendRequest!({ user_id: 'u_chloe' }))).toBe('request_pending');
    expect(await codeOf(social.sendFriendRequest!({ minecraft_name: 'Hugo_Mines' }))).toBe(
      'incoming_request_exists',
    );
    for (let i = 0; i < 30; i++) await codeOf(social.lookupUser!('rustic', 'nobody_here'));
    expect(await codeOf(social.lookupUser!('rustic', 'NinaSky'))).toBe('rate_limited');
  });

  it('drops requested players from suggestions and honours the privacy setting', async () => {
    const { social, account } = await mocks();
    const before = await social.friendSuggestions!();
    expect(before.discoverable).toBe(true);
    expect(before.played_with.map((s) => s.user.username)).toContain('alex_rs');
    expect(before.crossed.map((s) => s.user.username)).toContain('NinaSky');
    await social.sendFriendRequest!({ user_id: 'u_alex' });
    const after = await social.friendSuggestions!();
    expect(after.played_with.map((s) => s.user.username)).not.toContain('alex_rs');

    const me = await account.setDiscoverable!(false);
    expect(me.discoverable).toBe(false);
    expect(await social.friendSuggestions!()).toEqual({
      discoverable: false,
      played_with: [],
      crossed: [],
    });
  });
});
