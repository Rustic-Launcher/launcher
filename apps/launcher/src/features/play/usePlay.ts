/** Play button logic shared by the bottom bar, Home "Resume" and instance cards. */
import { toast, type PlayButtonState } from '@rustic/ui';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { Account } from '@/ipc/types/auth';
import type { Instance, LaunchOptions, LaunchPhase } from '@/ipc/types/instances';
import { formatPercent } from '@/lib/format';
import { useAccounts } from '@/queries/core';
import { useActivity } from '@/stores/activity';

/** Why the last launch attempt ended idle: the backend's message and its stable reason code. */
export interface Refusal {
  message: string;
  reason?: string;
}

/**
 * The refusal of the last launch attempt, if it still applies. A "sign in / own the game" refusal
 * (`auth.*`) is stale once a verified owner is active: it would otherwise stay under the button
 * until the next attempt.
 */
export function idleRefusal(
  launch: { phase: LaunchPhase; message?: string; reason?: string } | undefined,
  ownerVerified = false,
): Refusal | null {
  if (launch?.phase !== 'idle' || !launch.message) return null;
  if (ownerVerified && launch.reason?.startsWith('auth.')) return null;
  return { message: launch.message, reason: launch.reason };
}

/** Reasons the launcher words itself (`play.refusals.*`); any other shows the backend's message. */
const REFUSAL_KEYS = {
  'auth.no_account': 'play.refusals.auth_no_account',
  'native.consent_required': 'play.refusals.native_consent',
  'native.toolchain_missing': 'play.refusals.native_toolchain',
  'native.not_edition': 'play.refusals.native_not_edition',
  'native.build_failed': 'play.refusals.native_build_failed',
  'native.disk_space': 'play.refusals.native_disk',
  'auth.not_owned': 'play.refusals.auth_not_owned',
  'auth.not_approved': 'play.refusals.auth_not_approved',
  'auth.reauth_required': 'play.refusals.auth_reauth_required',
  'auth.offline_not_allowed': 'play.refusals.auth_offline_not_allowed',
  'auth.network': 'play.refusals.auth_network',
  'build.not_installed': 'play.refusals.build_not_installed',
  'prepare.needed': 'play.refusals.prepare_needed',
  'prepare.failed': 'play.refusals.prepare_failed',
  'prepare.no_java': 'play.refusals.prepare_no_java',
  'prepare.no_minecraft_dir': 'play.refusals.prepare_no_minecraft_dir',
  'launch.already_running': 'play.refusals.launch_already_running',
} as const;

/** The `play.refusals.*` key of a reason code, or null when the backend's message is shown. */
export function refusalKey(
  reason: string | undefined,
): (typeof REFUSAL_KEYS)[keyof typeof REFUSAL_KEYS] | null {
  return reason && Object.hasOwn(REFUSAL_KEYS, reason)
    ? REFUSAL_KEYS[reason as keyof typeof REFUSAL_KEYS]
    : null;
}

/** Whether the active account owns the game (its last check is valid). */
export function ownerVerified(accounts: Account[] | undefined): boolean {
  const active = accounts?.find((a) => a.active);
  return !!active?.ownership?.owns_game && active.ownership.status === 'valid';
}

/** Colour of the status dot: a refusal is a warning, never the green "ready" dot. */
export function statusTone(
  state: PlayButtonState,
  refused: boolean,
): 'success' | 'warning' | 'accent' | null {
  if (state === 'play') return refused ? 'warning' : 'success';
  if (state === 'update') return 'warning';
  if (state === 'progress' || state === 'running') return 'accent';
  return null;
}

const BUSY: LaunchPhase[] = [
  'updating',
  'checking',
  'downloading',
  'preparing',
  'building',
  'launching',
  'stopping',
];

export interface PlayModel {
  state: PlayButtonState;
  progress: number | null;
  /** Short status text for the bottom bar. */
  status: string;
  /** Colour of the status dot (see `statusTone`). */
  tone: ReturnType<typeof statusTone>;
  progressLabel: string;
  phase: LaunchPhase;
  play: (options?: LaunchOptions) => Promise<void>;
  cancel: () => Promise<void>;
  stop: () => Promise<void>;
  /** Single entry point for the PLAY button. */
  primary: () => void;
}

export function usePlay(instance: Instance | undefined): PlayModel {
  const { t, i18n } = useTranslation();
  const launch = useActivity((s) => (instance ? s.launches[instance.id] : undefined));
  const install = useActivity((s) =>
    instance
      ? Object.values(s.downloads).find(
          (d) => d.instance_id === instance.id && (d.state === 'running' || d.state === 'queued'),
        )
      : undefined,
  );
  const phase: LaunchPhase = launch?.phase ?? 'idle';

  let state: PlayButtonState = 'play';
  let progress: number | null = null;
  let status = t('play.status.ready');
  let progressLabel = '';

  if (!instance) {
    state = 'disabled';
    status = t('play.status.noInstance');
  } else if (BUSY.includes(phase)) {
    state = 'progress';
    progress =
      phase === 'downloading' ||
      phase === 'preparing' ||
      phase === 'building' ||
      phase === 'updating'
        ? (launch?.progress ?? null)
        : null;
    progressLabel =
      progress != null
        ? t(
            phase === 'preparing'
              ? 'play.progress.preparingFiles'
              : phase === 'building'
                ? 'play.progress.buildingPct'
                : 'play.progress.downloading',
            { pct: formatPercent(progress, i18n.language) },
          )
        : t(
            `play.progress.${phase as 'updating' | 'checking' | 'preparing' | 'building' | 'launching' | 'stopping'}`,
          );
    status = progressLabel;
  } else if (phase === 'running' || instance.state === 'running') {
    state = 'running';
    status = t('play.status.running');
  } else if (instance.state === 'installing' || install) {
    state = 'progress';
    const total = install?.total_bytes;
    progress = install && total ? install.received_bytes / total : null;
    const building = install?.kind === 'native-build';
    progressLabel =
      progress != null
        ? t(building ? 'play.progress.buildingPct' : 'play.progress.downloading', {
            pct: formatPercent(progress, i18n.language),
          })
        : t(building ? 'play.progress.building' : 'play.progress.preparing');
    status = progressLabel;
  } else if (instance.state === 'needs_update' && instance.update_available) {
    state = 'update';
    status = t('play.status.update');
  } else if (instance.state === 'broken') {
    state = 'update';
    status = t('play.status.broken');
  }

  // A refused launch (e.g. no Microsoft owner) stays visible next to the button, not only in a toast.
  const { data: accounts } = useAccounts();
  const refusal = idleRefusal(launch, ownerVerified(accounts));
  if (refusal && state === 'play') {
    const key = refusalKey(refusal.reason);
    status = key ? t(key) : refusal.message;
  }
  const tone = statusTone(state, !!refusal);

  const id = instance?.id;

  const play = useCallback(
    async (options?: LaunchOptions) => {
      if (!id) return;
      try {
        await invoke('instance_launch', { instance_id: id, options });
      } catch (e) {
        const reason = (e as { details?: { reason?: unknown } }).details?.reason;
        const key = refusalKey(typeof reason === 'string' ? reason : undefined);
        toast({
          tone: 'error',
          title: t('play.errors.launch'),
          // A failed preparation keeps its technical cause, the rest is worded for the player.
          description: !key
            ? (e as Error).message
            : reason === 'prepare.failed'
              ? `${t(key)} (${(e as Error).message})`
              : t(key),
        });
      }
    },
    [id, t],
  );
  const cancel = useCallback(async () => {
    if (!id) return;
    await invoke('instance_launch_cancel', { instance_id: id });
    toast({ tone: 'info', title: t('play.cancelled') });
  }, [id, t]);
  const stop = useCallback(async () => {
    if (!id) return;
    await invoke('instance_stop', { instance_id: id });
  }, [id]);

  const primary = useCallback(() => {
    if (state === 'progress') {
      if (install && !launch) void invoke('download_cancel', { id: install.id });
      else void cancel();
    } else if (state === 'running') void stop();
    else if (state === 'play' || state === 'update') void play();
  }, [state, install, launch, cancel, stop, play]);

  return { state, progress, status, tone, progressLabel, phase, play, cancel, stop, primary };
}
