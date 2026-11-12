/** Webview-local UI preferences (theme, palette, language, panels), persisted in localStorage. */
import { applyTheme, DEFAULT_PALETTE, type PaletteId, type ThemePreference } from '@rustic/ui';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { Language } from '@/i18n';
import type { Currency } from '@/ipc/types/dns';

export interface PrefsState {
  theme: ThemePreference;
  palette: PaletteId;
  language: Language | null;
  activeInstanceId: string | null;
  currency: Currency;
  /** Content page: "Compatible with this instance" filter (on by default). */
  compatibleOnly: boolean;
  setTheme: (t: ThemePreference) => void;
  setPalette: (p: PaletteId) => void;
  setLanguage: (l: Language) => void;
  setActiveInstance: (id: string | null) => void;
  setCurrency: (c: Currency) => void;
  setCompatibleOnly: (v: boolean) => void;
}

export const usePrefs = create<PrefsState>()(
  persist(
    (set) => ({
      theme: 'dark',
      palette: DEFAULT_PALETTE,
      language: null,
      activeInstanceId: null,
      currency: 'EUR',
      compatibleOnly: true,
      setTheme: (theme) => set({ theme }),
      setPalette: (palette) => set({ palette }),
      setLanguage: (language) => set({ language }),
      setActiveInstance: (activeInstanceId) => set({ activeInstanceId }),
      setCurrency: (currency) => set({ currency }),
      setCompatibleOnly: (compatibleOnly) => set({ compatibleOnly }),
    }),
    {
      name: 'rustic.prefs',
      version: 1,
      storage: createJSONStorage(() => window.localStorage),
      partialize: ({ theme, palette, language, activeInstanceId, currency, compatibleOnly }) => ({
        theme,
        palette,
        language,
        activeInstanceId,
        currency,
        compatibleOnly,
      }),
    },
  ),
);

/** Applies the current theme now and whenever it (or the OS scheme) changes. */
export function bindTheme(): () => void {
  const apply = () => {
    const { palette, theme } = usePrefs.getState();
    applyTheme(palette, theme);
  };
  apply();
  const unsub = usePrefs.subscribe((s, prev) => {
    if (s.palette !== prev.palette || s.theme !== prev.theme) apply();
  });
  const mq = window.matchMedia?.('(prefers-color-scheme: light)');
  const onMq = () => usePrefs.getState().theme === 'system' && apply();
  mq?.addEventListener('change', onMq);
  return () => {
    unsub();
    mq?.removeEventListener('change', onMq);
  };
}
