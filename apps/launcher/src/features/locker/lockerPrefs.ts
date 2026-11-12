/** Cosmetics visibility switches (Locker). Synced to the game by the Rustic edition client. */
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface LockerPrefs {
  showMine: boolean;
  hideOthers: boolean;
  hideTrailsAuras: boolean;
  set: (patch: Partial<Pick<LockerPrefs, 'showMine' | 'hideOthers' | 'hideTrailsAuras'>>) => void;
}

export const useLockerPrefs = create<LockerPrefs>()(
  persist(
    (set) => ({
      showMine: true,
      hideOthers: false,
      hideTrailsAuras: false,
      set: (patch) => set(patch),
    }),
    {
      name: 'rustic.locker',
      storage: createJSONStorage(() => window.localStorage),
      partialize: ({ showMine, hideOthers, hideTrailsAuras }) => ({
        showMine,
        hideOthers,
        hideTrailsAuras,
      }),
    },
  ),
);
