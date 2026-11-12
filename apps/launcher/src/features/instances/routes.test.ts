import { describe, expect, it } from 'vitest';
import { instancesRoute } from './routes';

describe('instances route search', () => {
  it('opens the new-profile sheet from ?new=1 parsed as a number or a string', () => {
    const validate = instancesRoute.options.validateSearch as (s: Record<string, unknown>) => {
      new?: string;
    };
    expect(validate({ new: 1 }).new).toBe('1');
    expect(validate({ new: '1' }).new).toBe('1');
    expect(validate({}).new).toBeUndefined();
  });
});
