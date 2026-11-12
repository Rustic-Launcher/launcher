import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { mockedCommands } from './index';

/** Every command of the contract has a mock implementation, and the mock invents none. */
describe('IPC mock coverage', () => {
  const src = readFileSync(path.resolve(__dirname, '../commands.ts'), 'utf8');
  const body = src.slice(
    src.indexOf('export interface IpcCommands'),
    src.indexOf('export interface IpcEvents'),
  );
  const declared = [...body.matchAll(/^\s{2}([a-z0-9_]+):\s*\[/gm)].map((m) => m[1]).sort();

  it('declares commands', () => {
    expect(declared.length).toBeGreaterThan(100);
  });

  it('mocks every declared command', () => {
    const mocked = new Set(mockedCommands());
    expect(declared.filter((c) => !mocked.has(c as never))).toEqual([]);
  });

  it('mocks nothing outside the contract', () => {
    const set = new Set(declared);
    expect(mockedCommands().filter((c) => !set.has(c))).toEqual([]);
  });
});
