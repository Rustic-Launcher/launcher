import { describe, expect, it } from 'vitest';
import type { JavaLogLine, JavaProfile } from '@/ipc/types/java';
import {
  errorKey,
  filterLog,
  formatJvmArgs,
  formatMemory,
  formatPlayTime,
  launchAction,
  memoryMax,
  OFFICIAL_ICONS,
  parseJvmArgs,
  pickProfile,
  profileVersionLabel,
} from './logic';

const p = (over: Partial<JavaProfile>): JavaProfile => ({
  id: 'a',
  name: 'A',
  game_version: '1.21.1',
  loader: 'vanilla',
  loader_version: null,
  version_id: null,
  memory_min_mb: 512,
  memory_max_mb: 4096,
  resolution: null,
  fullscreen: false,
  java_path: null,
  jvm_args: [],
  game_dir: null,
  created_at: 100,
  last_played_at: null,
  play_time: 0,
  game_dir_path: '/x',
  installed: false,
  running: false,
  ...over,
});

describe('java launchpad logic', () => {
  it('labels versions with their loader', () => {
    expect(profileVersionLabel(p({}))).toBe('Vanilla 1.21.1');
    expect(profileVersionLabel(p({ loader: 'fabric', loader_version: '0.20.1' }))).toBe(
      'Fabric 0.20.1 · 1.21.1',
    );
    expect(profileVersionLabel(p({ loader: 'neoforge' }))).toBe('NeoForge · 1.21.1');
  });

  it('picks the remembered, else the last played profile', () => {
    const list = [
      p({ id: 'old', created_at: 1 }),
      p({ id: 'played', created_at: 1, last_played_at: 500 }),
      p({ id: 'new', created_at: 300 }),
    ];
    expect(pickProfile(list, 'new')?.id).toBe('new');
    expect(pickProfile(list, 'gone')?.id).toBe('played');
    expect(pickProfile([], null)).toBeUndefined();
  });

  it('chooses the launch button action', () => {
    expect(launchAction(p({}), undefined)).toBe('install_launch');
    expect(launchAction(p({ installed: true }), undefined)).toBe('launch');
    expect(
      launchAction(p({ installed: true }), {
        profile_id: 'a',
        state: 'installing',
        offline: false,
      }),
    ).toBe('cancel');
    expect(
      launchAction(p({ installed: true }), { profile_id: 'a', state: 'running', offline: false }),
    ).toBe('stop');
    expect(
      launchAction(p({ installed: true }), { profile_id: 'a', state: 'crashed', offline: false }),
    ).toBe('launch');
  });

  it('maps failure reasons to their own texts', () => {
    expect(errorKey('auth.not_owned')).toBe('error.notOwned');
    expect(errorKey('auth.no_account')).toBe('error.noAccount');
    expect(errorKey('java.no_runtime')).toBe('error.noJava');
    expect(errorKey('something.else')).toBeNull();
    expect(errorKey(undefined)).toBeNull();
  });

  it('formats memory, play time and the slider bound', () => {
    expect(formatMemory(4096)).toBe('4 GB');
    expect(formatMemory(1536)).toBe('1.5 GB');
    expect(formatPlayTime(7200)).toEqual({ h: 2 });
    expect(formatPlayTime(600)).toEqual({ min: 10 });
    expect(memoryMax(16 * 1024 ** 3)).toBe(14_336);
    expect(memoryMax(2 * 1024 ** 3)).toBe(2048);
    expect(memoryMax(256 * 1024 ** 3)).toBe(32_768);
  });

  it('reads JVM arguments with quotes', () => {
    expect(parseJvmArgs(' -XX:+UseG1GC  "-Dname=a b" -Dx=1 ')).toEqual([
      '-XX:+UseG1GC',
      '-Dname=a b',
      '-Dx=1',
    ]);
    expect(formatJvmArgs(['-Dname=a b', '-Xss2M'])).toBe('"-Dname=a b" -Xss2M');
  });

  it('filters the log', () => {
    const l = (level: JavaLogLine['level'], text: string): JavaLogLine => ({
      profile_id: 'a',
      level,
      text,
      thread: null,
      time: 0,
    });
    const lines = [l('info', 'Setting user'), l('warn', 'Missing sound'), l('error', 'Boom')];
    expect(filterLog(lines, 'all', '')).toHaveLength(3);
    expect(filterLog(lines, 'warn', '').map((x) => x.text)).toEqual(['Missing sound', 'Boom']);
    expect(filterLog(lines, 'error', '')).toHaveLength(1);
    expect(filterLog(lines, 'all', 'sound')).toHaveLength(1);
  });

  it('loads the official icons from minecraft.net only', () => {
    for (const url of Object.values(OFFICIAL_ICONS)) {
      expect(url.startsWith('https://www.minecraft.net/')).toBe(true);
    }
  });
});
