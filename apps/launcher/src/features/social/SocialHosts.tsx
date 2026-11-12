import { AnimatePresence, motion } from 'motion/react';
import { Dialog as D } from 'radix-ui';
import { Users } from 'lucide-react';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { RusticMark } from '@/layout/Logo';
import { WindowControls } from '@/layout/WindowControls';
import { useFriends } from '@/queries/social';
import { useUi, type SocialTab } from '@/stores/ui';
import { AddFriendHost } from './AddFriendDialog';
import { SocialView } from './SocialView';

function SocialHeader({ onClose, drag }: { onClose?: () => void; drag?: boolean }) {
  const { t } = useTranslation();
  const { data: friends } = useFriends();
  const online = friends?.filter((f) => f.presence !== 'offline').length ?? 0;
  return (
    <header
      className="flex h-11 shrink-0 items-center gap-2 px-3"
      data-tauri-drag-region={drag ? true : undefined}
    >
      <RusticMark size={18} />
      <span className="text-sm font-semibold" data-tauri-drag-region={drag ? true : undefined}>
        {t('social.title')}
      </span>
      <span className="flex-1" data-tauri-drag-region={drag ? true : undefined} />
      <span
        className="bg-surface-2 text-fg-muted flex h-7 items-center gap-1.5 rounded-md px-2 text-xs"
        aria-label={t('friends.onlineCount', { count: online })}
      >
        <Users className="size-3.5" aria-hidden />
        {online}
      </span>
      <WindowControls onClose={onClose} />
    </header>
  );
}

/** Browser build: the Social view as a floating, window-like overlay inside the app. */
export function SocialOverlay() {
  const { t } = useTranslation();
  const open = useUi((s) => s.socialOpen);
  const setSocial = useUi((s) => s.setSocial);
  const close = () => setSocial({ socialOpen: false });
  return (
    <D.Root open={open} onOpenChange={(o) => setSocial({ socialOpen: o })}>
      <AnimatePresence>
        {open && (
          <D.Portal forceMount>
            <D.Overlay asChild forceMount>
              <motion.div
                className="bg-overlay fixed inset-0 z-[55]"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
              />
            </D.Overlay>
            <D.Content asChild forceMount>
              <motion.div
                initial={{ opacity: 0, scale: 0.98, y: 10 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.98 }}
                transition={{ duration: 0.2, ease: [0.2, 0, 0, 1] }}
                data-testid="social-overlay"
                className="bg-bg-elevated border-border-default fixed top-1/2 left-1/2 z-[56] flex h-[min(680px,86vh)] w-[min(1080px,90vw)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-2xl border shadow-lg outline-none"
              >
                <D.Title className="sr-only">{t('social.title')}</D.Title>
                <D.Description className="sr-only">{t('social.description')}</D.Description>
                <SocialHeader onClose={close} />
                <div className="min-h-0 flex-1 px-4 pb-4">
                  <SocialView />
                </div>
              </motion.div>
            </D.Content>
          </D.Portal>
        )}
      </AnimatePresence>
    </D.Root>
  );
}

/** Desktop build: content of the detachable "social" window (route `/social`). */
export function SocialWindowPage({ tab, withUser }: { tab?: SocialTab; withUser?: string }) {
  const setSocial = useUi((s) => s.setSocial);
  useEffect(() => {
    setSocial({ socialTab: tab ?? 'friends', socialChatWith: withUser ?? null });
  }, [tab, withUser, setSocial]);
  return (
    <div className="bg-bg-elevated text-fg flex h-screen flex-col">
      <SocialHeader drag />
      <div className="min-h-0 flex-1 px-4 pb-4">
        <SocialView />
      </div>
      <AddFriendHost />
    </div>
  );
}
