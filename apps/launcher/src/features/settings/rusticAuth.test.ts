import { describe, expect, it } from 'vitest';
import { ageOn, validateSignup, type SignupForm } from './rusticAuth';

const ok: SignupForm = {
  email: 'sam@example.com',
  password: 'correct-horse-1',
  username: 'SamCrafte',
  date_of_birth: '2000-05-01',
  country: 'FR',
  accept_privacy: true,
};
const now = new Date('2026-11-10T12:00:00Z');

describe('Rustic sign-up validation', () => {
  it('accepts a valid form', () => {
    expect(validateSignup(ok, now)).toEqual([]);
  });
  it('flags each problem', () => {
    expect(validateSignup({ ...ok, email: 'nope' }, now)).toEqual(['email']);
    expect(validateSignup({ ...ok, password: 'short' }, now)).toEqual(['password']);
    expect(validateSignup({ ...ok, username: 'a b' }, now)).toEqual(['username']);
    expect(validateSignup({ ...ok, date_of_birth: '' }, now)).toEqual(['dateOfBirth']);
    expect(validateSignup({ ...ok, date_of_birth: '2015-01-01' }, now)).toEqual(['tooYoung']);
    expect(validateSignup({ ...ok, accept_privacy: false }, now)).toEqual(['privacy']);
  });
  it('computes age at a date', () => {
    expect(ageOn('2013-11-11', now)).toBe(12);
    expect(ageOn('2013-11-10', now)).toBe(13);
    expect(ageOn('x', now)).toBeNull();
  });
});
