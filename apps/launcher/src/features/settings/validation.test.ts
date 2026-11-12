import { describe, expect, it } from 'vitest';
import {
  clampMemory,
  loginFailureDetail,
  loginFailureReason,
  memoryMax,
  memoryWarnAbove,
  parseAmountMinor,
  validatePin,
} from './validation';

describe('validatePin', () => {
  it('accepts 4 to 6 digits when confirmed', () => {
    expect(validatePin('2580', '2580')).toBeNull();
    expect(validatePin('491735', '491735')).toBeNull();
  });
  it('rejects bad formats', () => {
    expect(validatePin('12a4', '12a4')).toBe('format');
    expect(validatePin('123', '123')).toBe('format');
    expect(validatePin('1234567', '1234567')).toBe('format');
  });
  it('rejects weak PINs', () => {
    expect(validatePin('0000', '0000')).toBe('weak');
    expect(validatePin('1234', '1234')).toBe('weak');
    expect(validatePin('9876', '9876')).toBe('weak');
  });
  it('requires confirmation', () => {
    expect(validatePin('2580', '2581')).toBe('mismatch');
  });
});

describe('parseAmountMinor', () => {
  it('parses euro-style and dot amounts', () => {
    expect(parseAmountMinor('15')).toBe(1500);
    expect(parseAmountMinor('15,5')).toBe(1550);
    expect(parseAmountMinor('15.99')).toBe(1599);
  });
  it('treats empty as no limit and garbage as NaN', () => {
    expect(parseAmountMinor('')).toBeNull();
    expect(parseAmountMinor('abc')).toBeNaN();
    expect(parseAmountMinor('1.234')).toBeNaN();
  });
});

describe('memory helpers', () => {
  it('clamps to [1 GB, 90 % of RAM] on the step', () => {
    expect(clampMemory(300, 32768)).toBe(1024);
    expect(clampMemory(4300, 32768)).toBe(4096);
    expect(clampMemory(64000, 32768)).toBe(memoryMax(32768));
  });
  it('computes the slider bounds', () => {
    expect(memoryMax(32768)).toBe(29184);
    expect(memoryWarnAbove(16384)).toBe(9830);
  });
});

describe('Microsoft login failure reasons', () => {
  it('tells a build without Azure id apart from a network error', () => {
    expect(loginFailureReason({ code: 'unavailable', message: 'no client id' })).toBe(
      'not_configured',
    );
    expect(loginFailureReason({ code: 'network', message: 'offline' })).toBe('network');
    expect(loginFailureReason(new Error('x'))).toBe('error');
    expect(loginFailureReason(null)).toBe('error');
  });
  it('uses the auth error code carried in details.reason', () => {
    const err = (reason: string, code = 'internal') => ({
      code,
      message: 'm',
      details: { reason },
    });
    expect(loginFailureReason(err('auth.not_approved'))).toBe('not_approved');
    expect(loginFailureReason(err('auth.xbox'))).toBe('xbox');
    expect(loginFailureReason(err('auth.rate_limited', 'network'))).toBe('rate_limited');
    expect(loginFailureReason(err('auth.network', 'network'))).toBe('network');
    expect(loginFailureReason(err('auth.not_configured', 'unavailable'))).toBe('not_configured');
    // A Rustic Cloud error is `unavailable` too, but it is not a missing Azure id.
    expect(loginFailureReason(err('auth.cloud', 'unavailable'))).toBe('error');
    expect(loginFailureReason(err('auth.minecraft_refused'))).toBe('error');
  });
  it('keeps the technical message as the detail', () => {
    expect(
      loginFailureDetail({ code: 'internal', message: 'The Minecraft services refused' }),
    ).toBe('The Minecraft services refused');
    expect(loginFailureDetail({ code: 'internal', message: ' ' })).toBeUndefined();
    expect(loginFailureDetail(undefined)).toBeUndefined();
  });
});
