import '@/styles.css';
import './i18n';
import { Toaster, TooltipProvider } from '@rustic/ui';
import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { MotionConfig } from 'motion/react';
import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { useTranslation } from 'react-i18next';
import i18n from './i18n';
import { bindCoreEvents, queryClient } from './queries/core';
import { bindRealtime } from './queries/social';
import { router } from './router';
import { bindActivity } from './stores/activity';
import { bindTheme, usePrefs } from './stores/prefs';

// Theme before first paint.
bindTheme();
const lang = usePrefs.getState().language;
if (lang && lang !== i18n.language) void i18n.changeLanguage(lang);
usePrefs.subscribe((s, prev) => {
  if (s.language && s.language !== prev.language) void i18n.changeLanguage(s.language);
});

function App() {
  const { t } = useTranslation();
  useEffect(() => {
    const offs = [bindActivity(), bindCoreEvents(queryClient)];
    let offRealtime: (() => void) | undefined;
    void bindRealtime(queryClient).then((off) => (offRealtime = off));
    return () => {
      offs.forEach((o) => o());
      offRealtime?.();
    };
  }, []);
  return (
    <MotionConfig reducedMotion="user">
      <TooltipProvider delayDuration={250}>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
          <Toaster closeLabel={t('actions.close')} regionLabel={t('a11y.notifications')} />
        </QueryClientProvider>
      </TooltipProvider>
    </MotionConfig>
  );
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
