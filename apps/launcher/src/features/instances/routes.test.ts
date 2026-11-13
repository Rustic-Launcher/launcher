import { describe, expect, it } from 'vitest';
import { instancesRoute, validateInstancesSearch } from './routes';

describe('instances route search', () => {
  it('opens the new-profile sheet from ?new=1 parsed as a number or a string', () => {
    const validate = instancesRoute.options.validateSearch as (s: Record<string, unknown>) => {
      new?: string;
    };
    expect(validate({ new: 1 }).new).toBe('1');
    expect(validate({ new: '1' }).new).toBe('1');
    expect(validate({ new: 'import' }).new).toBe('import');
    expect(validate({}).new).toBeUndefined();
  });

  it('keeps known sorts and filters and drops the rest', () => {
    expect(
      validateInstancesSearch({
        q: 'pvp',
        sort: 'version',
        source: 'modpack',
        version: 'v0.2.15',
        edition: 'rustic',
        native: 'true',
      }),
    ).toEqual({
      new: undefined,
      selected: undefined,
      q: 'pvp',
      sort: 'version',
      source: 'modpack',
      version: 'v0.2.15',
      edition: 'rustic',
      native: true,
    });
    const bad = validateInstancesSearch({
      q: '',
      sort: 'size',
      source: 'all',
      version: 'v0.2.15<script>',
      edition: 'forge',
      native: 'yes',
    });
    expect(bad).toEqual({
      new: undefined,
      selected: undefined,
      q: undefined,
      sort: undefined,
      source: undefined,
      version: undefined,
      edition: undefined,
      native: undefined,
    });
    expect(validateInstancesSearch({ native: true }).native).toBe(true);
    expect(validateInstancesSearch({ version: 'nightly-2026-11-08-a84f637' }).version).toBe(
      'nightly-2026-11-08-a84f637',
    );
  });
});
