import { installSurfaceSpotlight, toast } from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { Outlet, useRouterState } from '@tanstack/react-router';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { AddFriendHost } from '@/features/social/AddFriendDialog';
import { SocialOverlay } from '@/features/social/SocialHosts';
import { invoke, listen } from '@/ipc';
import type { Instance } from '@/ipc/types/instances';
import { pickActiveInstance, qk } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import { useActivity } from '@/stores/activity';
import { useUi } from '@/stores/ui';
import { CommandMenu } from './CommandMenu';
import { Rail } from './Rail';
import { TitleBar } from './TitleBar';

/**
 * Launcher chrome (step 1b, ADR 0028; restyled in the website's language, ADR 0142): a warm grain
 * frame with the custom title bar on top, the icon rail on the left and the page in a rounded
 * content panel with a faint dot grid. Friends live on the Launchpad and in
 * the Social window; long work shows in the title bar's running pill.
 */
export function AppShell() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const mainRef = useRef<HTMLElement>(null);
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const setCommandOpen = useUi((s) => s.setCommandOpen);

  useEffect(() => {
    mainRef.current?.scrollTo({ top: 0 });
  }, [pathname]);

  // While a game runs every animation rests (CSS animations, the 3D viewers, the hero light):
  // the game gets the GPU. A hidden or minimised window rests too (useAnimationsPaused).
  const gameRunning = useActivity((s) =>
    Object.values(s.launches).some((l) => l.phase === 'running'),
  );
  useEffect(() => {
    const html = document.documentElement;
    if (gameRunning) html.setAttribute('data-paused', 'game');
    else html.removeAttribute('data-paused');
  }, [gameRunning]);

  // Premium surfaces follow the pointer (one delegated listener).
  useEffect(() => installSurfaceSpotlight(), []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setCommandOpen(true);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [setCommandOpen]);

  // rustic:// deep links forwarded by the desktop side.
  useEffect(
    () =>
      listen('deep-link', (e) => {
        if (e.action === 'auth') {
          void qc.invalidateQueries({ queryKey: qk.rustic });
          toast({
            tone: e.ok ? 'success' : 'error',
            title: e.ok ? t('deepLink.authOk') : t('deepLink.authFailed'),
            description: e.ok ? undefined : (e.error ?? undefined),
          });
          return;
        }
        const list = qc.getQueryData<Instance[]>(qk.instances) ?? [];
        const inst = pickActiveInstance(list, usePrefs.getState().activeInstanceId);
        if (!inst) return;
        toast({ tone: 'info', title: t('deepLink.joining', { target: e.target }) });
        void invoke('instance_launch', {
          instance_id: inst.id,
          options: { join: { address: e.target } },
        });
      }),
    [qc, t],
  );

  return (
    <div
      className="rx-ambient grid h-screen w-screen overflow-hidden"
      style={{
        gridTemplateColumns: 'var(--rl-rail-width) minmax(0, 1fr)',
        gridTemplateRows: 'var(--rl-topbar-height) minmax(0, 1fr)',
      }}
    >
      <a
        href="#main"
        onClick={(e) => {
          e.preventDefault();
          mainRef.current?.focus();
        }}
        className="bg-accent text-accent-fg sr-only z-[90] rounded-md px-3 py-2 focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
      >
        {t('a11y.skipToContent')}
      </a>
      <TitleBar />
      <Rail />
      <main
        id="main"
        ref={mainRef}
        tabIndex={-1}
        className="rx-panel border-border relative col-start-2 row-start-2 mr-2 mb-2 overflow-y-auto rounded-2xl border outline-none"
      >
        <Outlet />
      </main>
      <CommandMenu />
      <SocialOverlay />
      <AddFriendHost />
    </div>
  );
}
