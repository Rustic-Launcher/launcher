import { describe, expect, it } from 'vitest';
import type { Account } from '@/ipc/types/auth';
import { idleRefusal, ownerVerified, refusalKey, statusTone } from './usePlay';

describe('idleRefusal', () => {
  it('surfaces why a launch was refused', () => {
    expect(
      idleRefusal({ phase: 'idle', message: 'Sign in with Microsoft', reason: 'auth.no_account' }),
    ).toEqual({ message: 'Sign in with Microsoft', reason: 'auth.no_account' });
    expect(idleRefusal({ phase: 'idle' })).toBeNull();
    expect(idleRefusal({ phase: 'running', message: 'x' })).toBeNull();
    expect(idleRefusal(undefined)).toBeNull();
  });

  it('drops an account refusal once a verified owner is active', () => {
    // Regression: "Sign in with a Microsoft account..." stayed under the button after signing in.
    const refused = { phase: 'idle' as const, message: 'Sign in', reason: 'auth.no_account' };
    expect(idleRefusal(refused, true)).toBeNull();
    // Any other refusal still applies.
    const unprepared = { phase: 'idle' as const, message: 'x', reason: 'prepare.failed' };
    expect(idleRefusal(unprepared, true)?.reason).toBe('prepare.failed');
  });
});

describe('refusalKey', () => {
  it('words known reasons in the player language, others keep the backend message', () => {
    expect(refusalKey('auth.no_account')).toBe('play.refusals.auth_no_account');
    expect(refusalKey('prepare.needed')).toBe('play.refusals.prepare_needed');
    expect(refusalKey('launch.process')).toBeNull();
    expect(refusalKey(undefined)).toBeNull();
  });
});

describe('ownerVerified', () => {
  const owner: Account = {
    id: 'ms',
    kind: 'microsoft',
    username: 'Owner',
    uuid: 'u1',
    active: true,
    ownership: { owns_game: true, status: 'valid', checked_at: null, next_check_at: null },
  };
  it('is true for an active Microsoft account that owns the game', () => {
    expect(ownerVerified([owner])).toBe(true);
    expect(ownerVerified([{ ...owner, active: false }])).toBe(false);
    expect(
      ownerVerified([{ ...owner, ownership: { ...owner.ownership!, status: 'expired' } }]),
    ).toBe(false);
    expect(ownerVerified(undefined)).toBe(false);
  });
});

describe('statusTone', () => {
  it('never shows a refusal with the green ready dot', () => {
    // Regression (QA): "Sign in with a Microsoft account..." had the green "ready" dot.
    expect(statusTone('play', true)).toBe('warning');
    expect(statusTone('play', false)).toBe('success');
    expect(statusTone('running', false)).toBe('accent');
  });
});
