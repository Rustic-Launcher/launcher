/**
 * Mock: the update service (ADR 0162). The browser preview starts up to date; a scenario shows
 * the other states, from `?update=<scenario>` / localStorage `rustic.mock.update`, or live with
 * `window.rusticMockUpdate('<scenario>')`:
 *
 * - `available`: a launcher and an edition update found, automatic updates off ("Update and
 *   restart");
 * - `deferred`: found while a game runs, applied when it exits;
 * - `downloading`: the launcher update downloading (62 %);
 * - `restarting`: about to restart;
 * - `updated`: the launcher was just updated (from 0.1.0), and a launch resumes;
 * - `error`: the last check failed.
 */
import type { UpdateStatus } from '../../types/app';
import {
  emit,
  fail,
  latency,
  nowIso,
  persistGet,
  persistSet,
  simulate,
  type MockHandlers,
} from '../runtime';
import { db } from './core';

const CURRENT = '0.2.0';
const NEXT_LAUNCHER = {
  version: '0.2.1',
  notes: 'Faster start, new friends list.',
  size: 11_800_000,
  published_at: '2026-11-12T09:00:00Z',
};
const NEXT_EDITION = {
  version: 'rustic-edition-2026-11-12-4c1e2a9',
  notes: null,
  size: 412 * 1024 ** 2,
  published_at: '2026-11-12T08:00:00Z',
};

export type MockUpdateScenario =
  'none' | 'available' | 'deferred' | 'downloading' | 'restarting' | 'updated' | 'error';

function readScenario(): MockUpdateScenario {
  try {
    const q = new URL(window.location.href).searchParams.get('update');
    if (q) return q as MockUpdateScenario;
  } catch {
    /* no window */
  }
  return persistGet<MockUpdateScenario>('update', 'none');
}

function base(): UpdateStatus {
  return {
    phase: 'idle',
    current_version: CURRENT,
    channel: 'stable',
    auto: db.settings.launcher.auto_update,
    last_check: nowIso(),
    launcher: null,
    edition: null,
    edition_current: 'rustic-edition-2026-11-05-a84f637',
    working_on: null,
    progress: null,
    error: null,
    updated_from: null,
    resume_instance: null,
  };
}

export function scenarioStatus(s: MockUpdateScenario): UpdateStatus {
  const st = base();
  switch (s) {
    case 'available':
      return {
        ...st,
        phase: 'available',
        auto: false,
        launcher: NEXT_LAUNCHER,
        edition: NEXT_EDITION,
      };
    case 'deferred':
      return { ...st, phase: 'deferred', launcher: NEXT_LAUNCHER };
    case 'downloading':
      return {
        ...st,
        phase: 'downloading',
        working_on: 'launcher',
        progress: 0.62,
        launcher: NEXT_LAUNCHER,
      };
    case 'restarting':
      return {
        ...st,
        phase: 'restarting',
        working_on: 'launcher',
        progress: 1,
        launcher: NEXT_LAUNCHER,
      };
    case 'updated':
      return { ...st, updated_from: '0.1.0', resume_instance: db.instances[0]?.id ?? null };
    case 'error':
      return {
        ...st,
        phase: 'error',
        error: 'launcher: the update server did not answer in time',
      };
    default:
      return st;
  }
}

let status: UpdateStatus | null = null;
const current = (): UpdateStatus => (status ??= scenarioStatus(readScenario()));

function publish(next: UpdateStatus): UpdateStatus {
  status = next;
  emit('update-state', structuredClone(next));
  return structuredClone(next);
}

declare global {
  interface Window {
    /** Browser mock only: switch the update service to a scenario (see handlers/updates.ts). */
    rusticMockUpdate?: (scenario: MockUpdateScenario) => void;
  }
}

if (typeof window !== 'undefined') {
  window.rusticMockUpdate = (scenario) => {
    persistSet('update', scenario);
    publish(scenarioStatus(scenario));
  };
}

/** Downloads what was found, then "restarts" into the new version. */
function install(): void {
  const s = current();
  const from = s.current_version;
  const to = s.launcher?.version;
  simulate(
    2500,
    (p) =>
      publish({
        ...current(),
        phase: 'downloading',
        working_on: s.launcher ? 'launcher' : 'edition',
        progress: p,
      }),
    () => {
      if (!to) {
        publish({
          ...current(),
          phase: 'idle',
          edition: null,
          edition_current: s.edition?.version ?? s.edition_current,
          working_on: null,
          progress: null,
        });
        return;
      }
      publish({ ...current(), phase: 'restarting', progress: 1 });
      setTimeout(() => {
        publish({
          ...base(),
          current_version: to,
          updated_from: from,
          edition_current: s.edition?.version ?? s.edition_current,
        });
      }, 1200);
    },
  );
}

export const updateHandlers: MockHandlers = {
  app_update_status: () => structuredClone(current()),
  app_update_check: async () => {
    publish({ ...current(), phase: 'checking', error: null });
    await latency(4);
    const s = current();
    const found = !!(s.launcher || s.edition);
    const next = publish({
      ...s,
      phase: found ? (s.auto ? s.phase : 'available') : 'idle',
      last_check: nowIso(),
    });
    if (found && next.auto && next.phase !== 'deferred') install();
    return next;
  },
  app_update_apply: async () => {
    const s = current();
    if (!s.launcher && !s.edition) fail('not_found', 'no update available');
    await latency(1);
    install();
    return structuredClone(current());
  },
  app_take_pending_launch: () => {
    const s = current();
    if (!s.resume_instance) return null;
    const instance_id = s.resume_instance;
    publish({ ...s, resume_instance: null });
    return { instance_id, options: null, created_at: Math.floor(Date.now() / 1000) };
  },
};
