import { beforeEach, describe, expect, it } from 'vitest';
import { createMockCloud } from '@/cloud/mock';
import { useUi } from '@/stores/ui';
import { openSocial } from './openSocial';

describe('openSocial (browser build)', () => {
  beforeEach(() =>
    useUi.setState({ socialOpen: false, socialTab: 'friends', socialChatWith: null }),
  );

  it('opens the in-app overlay on the requested tab and conversation', async () => {
    await openSocial({ tab: 'chats', chatWith: 'u_lea' });
    const s = useUi.getState();
    expect(s.socialOpen).toBe(true);
    expect(s.socialTab).toBe('chats');
    expect(s.socialChatWith).toBe('u_lea');
  });

  it('keeps the current tab when none is given', async () => {
    useUi.setState({ socialTab: 'chats' });
    await openSocial();
    expect(useUi.getState().socialTab).toBe('chats');
  });
});

describe('mock chats', () => {
  it('lists conversations newest first with unread counts, and marks them read', async () => {
    const c = createMockCloud();
    const list = await c.conversations();
    expect(list[0]?.user.id).toBe('u_lea');
    expect(list.find((x) => x.user.id === 'u_lea')?.unread).toBe(2);
    await c.markConversationRead('u_lea');
    expect((await c.conversations()).find((x) => x.user.id === 'u_lea')?.unread).toBe(0);
  });

  it('sends a message into the conversation', async () => {
    const c = createMockCloud();
    const m = await c.sendMessage('u_noah', 'salut');
    expect(m.body).toBe('salut');
    expect((await c.messages('u_noah')).at(-1)?.body).toBe('salut');
  });
});

describe('chat messages', () => {
  it('a message reaching the cache twice (send, realtime, refetch) is shown once', async () => {
    const { appendMessage } = await import('@/queries/social');
    const m = { id: 'm1', from_id: 'a', to_id: 'b', body: 'hi', sent_at: '2026-11-11T10:00:00Z' };
    const once = appendMessage([], m);
    expect(appendMessage(once, { ...m })).toBe(once);
    expect(appendMessage(once, { ...m, id: 'm2' })).toHaveLength(2);
  });
});
