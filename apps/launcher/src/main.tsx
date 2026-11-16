import '@/styles.css';
import './i18n';
import { DocumentDirection, Toaster, TooltipProvider } from '@rustic/ui';
import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { MotionConfig } from 'motion/react';
import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { useTranslation } from 'react-i18next';
import { changeLanguage, currentLanguage, i18nReady } from './i18n';
import { bindCoreEvents, queryClient } from './queries/core';
import { bindVoice } from './features/voice/bridge';
import { bindRealtime } from './queries/social';
import { router } from './router';
import { bindActivity } from './stores/activity';
import { bindTheme, usePrefs } from './stores/prefs';
import { bindUpdates } from './stores/updates';

// Theme before first paint.
bindTheme();
usePrefs.subscribe((s, prev) => {
  if (s.language && s.language !== prev.language) void changeLanguage(s.language);
});

function App() {
  const { t } = useTranslation();
  useEffect(() => {
    const offs = [bindActivity(), bindUpdates(), bindCoreEvents(queryClient)];
    let offRealtime: (() => void) | undefined;
    let offVoice: (() => void) | undefined;
    void bindRealtime(queryClient).then((off) => (offRealtime = off));
    let unmounted = false;
    // One voice engine per window, even when the effect runs twice (StrictMode in development).
    void bindVoice(queryClient).then((off) => (unmounted ? off() : (offVoice = off)));
    return () => {
      unmounted = true;
      offs.forEach((o) => o());
      offRealtime?.();
      offVoice?.();
    };
  }, []);
  return (
    <MotionConfig reducedMotion="user">
      <DocumentDirection>
        <TooltipProvider delayDuration={250}>
          <QueryClientProvider client={queryClient}>
            <RouterProvider router={router} />
            <Toaster closeLabel={t('actions.close')} regionLabel={t('a11y.notifications')} />
          </QueryClientProvider>
        </TooltipProvider>
      </DocumentDirection>
    </MotionConfig>
  );
}

// The starting language's bundle first (it is its own chunk), then the app.
void i18nReady.then(() => {
  const lang = usePrefs.getState().language;
  const ready = lang && lang !== currentLanguage() ? changeLanguage(lang) : Promise.resolve();
  return ready.finally(() =>
    createRoot(document.getElementById('root')!).render(
      <StrictMode>
        <App />
      </StrictMode>,
    ),
  );
});
