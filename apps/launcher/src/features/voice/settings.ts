/**
 * Voice settings (ADR 0176), per device, in the webview's localStorage (shared by the launcher's
 * windows, which are one origin). Nothing here leaves the computer.
 */
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { DEFAULT_PTT_KEY } from './keys';
import type { TalkMode } from './vad';

export interface VoiceSettings {
  /** `null`: the system default device. */
  inputDeviceId: string | null;
  outputDeviceId: string | null;
  mode: TalkMode;
  /** Voice activity sensitivity, 0-100. */
  sensitivity: number;
  /** A `KeyboardEvent.code`, or `Mouse4`/`Mouse5`. */
  pttKey: string;
  noiseSuppression: boolean;
  echoCancellation: boolean;
  autoGainControl: boolean;
  /** The master output volume, 0-2. */
  outputVolume: number;
  /** Per speaker (user id): 0-2, 1 = 100 %. */
  volumes: Record<string, number>;
  /** Speakers muted on this computer only. */
  localMutes: Record<string, boolean>;
  /** A Rustic Server's proximity voice: offer to join, join by itself, or never. */
  serverVoice: 'ask' | 'auto' | 'off';
  /** Draw the in-game overlay (who is in the room, who speaks). */
  overlay: boolean;
}

interface Actions {
  set: (patch: Partial<VoiceSettings>) => void;
  setVolume: (userId: string, volume: number) => void;
  toggleLocalMute: (userId: string) => void;
}

export const VOICE_DEFAULTS: VoiceSettings = {
  inputDeviceId: null,
  outputDeviceId: null,
  mode: 'vad',
  sensitivity: 55,
  pttKey: DEFAULT_PTT_KEY,
  noiseSuppression: true,
  echoCancellation: true,
  autoGainControl: true,
  outputVolume: 1,
  volumes: {},
  localMutes: {},
  serverVoice: 'ask',
  overlay: true,
};

const clampVolume = (v: number) => Math.min(2, Math.max(0, Number.isFinite(v) ? v : 1));

export const useVoiceSettings = create<VoiceSettings & Actions>()(
  persist(
    (set) => ({
      ...VOICE_DEFAULTS,
      set: (patch) => set(patch),
      setVolume: (userId, volume) =>
        set((s) => ({ volumes: { ...s.volumes, [userId]: clampVolume(volume) } })),
      toggleLocalMute: (userId) =>
        set((s) => {
          const localMutes = { ...s.localMutes };
          if (localMutes[userId]) delete localMutes[userId];
          else localMutes[userId] = true;
          return { localMutes };
        }),
    }),
    {
      name: 'rustic.voice',
      version: 1,
      storage: createJSONStorage(() => window.localStorage),
      partialize: ({ set: _s, setVolume: _v, toggleLocalMute: _m, ...rest }) => rest,
    },
  ),
);

/** Another window changed the settings: read them again. */
export function bindVoiceSettingsSync(): () => void {
  const onStorage = (e: StorageEvent) => {
    if (e.key === 'rustic.voice') void useVoiceSettings.persist.rehydrate();
  };
  window.addEventListener('storage', onStorage);
  return () => window.removeEventListener('storage', onStorage);
}

/** The microphone constraints the settings ask for. */
export function micConstraints(s: VoiceSettings): MediaTrackConstraints {
  return {
    deviceId: s.inputDeviceId ? { exact: s.inputDeviceId } : undefined,
    noiseSuppression: s.noiseSuppression,
    echoCancellation: s.echoCancellation,
    autoGainControl: s.autoGainControl,
    channelCount: 1,
  };
}
