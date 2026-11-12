import { describe, expect, it } from 'vitest';
import type { ChatMessage } from '@/cloud/types';
import { lastIncomingId } from './ChatsTab';

const msg = (id: string, from_id: string): ChatMessage => ({
  id,
  from_id,
  to_id: from_id === 'lea' ? 'sam' : 'lea',
  body: id,
  sent_at: '2026-11-10T18:00:00Z',
});

describe('lastIncomingId', () => {
  // Regression (QA): a DM received while its conversation was open stayed unread.
  it('changes when the peer sends a new message, not when I reply', () => {
    const before = [msg('m1', 'lea')];
    const received = [...before, msg('m2', 'lea')];
    const replied = [...received, msg('m3', 'sam')];
    expect(lastIncomingId(before, 'lea')).toBe('m1');
    expect(lastIncomingId(received, 'lea')).toBe('m2');
    expect(lastIncomingId(replied, 'lea')).toBe('m2');
  });
  it('is null without a conversation', () => {
    expect(lastIncomingId(undefined, 'lea')).toBeNull();
    expect(lastIncomingId([msg('m1', 'sam')], 'lea')).toBeNull();
    expect(lastIncomingId([msg('m1', 'lea')], null)).toBeNull();
  });
});
