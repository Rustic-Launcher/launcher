import { cn } from '@rustic/ui';
import { MessagesSquare, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useConversations } from '@/queries/social';
import { useUi } from '@/stores/ui';
import { ConversationList, ConversationPane } from './ChatsTab';
import { FriendsTab } from './FriendsTab';

/**
 * The Social view: Friends / Chats tabs on the left, conversation on the right. Hosted by the
 * detachable Social window (desktop) or the in-app overlay (browser build).
 */
export function SocialView() {
  const { t } = useTranslation();
  const tab = useUi((s) => s.socialTab);
  const chatWith = useUi((s) => s.socialChatWith);
  const setSocial = useUi((s) => s.setSocial);
  const { data: convs } = useConversations();
  const unread = (convs ?? []).reduce((n, c) => n + c.unread, 0);

  const tabBtn = (id: 'friends' | 'chats', label: string, Icon: typeof Users, badge?: number) => (
    <button
      type="button"
      role="tab"
      aria-selected={tab === id}
      data-testid={`social-tab-${id}`}
      onClick={() => setSocial({ socialTab: id })}
      className={cn(
        'rx-focus-ring flex h-9 flex-1 items-center justify-center gap-2 rounded-lg text-sm font-medium transition-colors',
        tab === id ? 'bg-surface-3 text-fg' : 'text-fg-muted hover:text-fg',
      )}
    >
      <Icon className="size-4" aria-hidden />
      {label}
      {badge ? (
        <span className="bg-accent text-accent-fg text-2xs rounded-full px-1.5 font-bold">
          {badge}
        </span>
      ) : null}
    </button>
  );

  return (
    <div
      className="grid h-full min-h-0 grid-cols-[minmax(280px,320px)_1fr] gap-4"
      data-testid="social-view"
    >
      <div className="flex min-h-0 flex-col">
        <div
          role="tablist"
          aria-label={t('social.title')}
          className="bg-surface-2 mb-3 flex gap-1 rounded-xl p-1"
        >
          {tabBtn('friends', t('social.friends'), Users)}
          {tabBtn('chats', t('social.chats'), MessagesSquare, unread)}
        </div>
        <div role="tabpanel" className="min-h-0 flex-1">
          {tab === 'friends' ? (
            <FriendsTab
              onMessage={(f) => setSocial({ socialTab: 'chats', socialChatWith: f.id })}
            />
          ) : (
            <ConversationList
              selected={chatWith}
              onSelect={(id) => setSocial({ socialChatWith: id })}
            />
          )}
        </div>
      </div>
      <section className="bg-surface-2/60 border-border min-h-0 overflow-hidden rounded-xl border">
        <ConversationPane userId={chatWith} />
      </section>
    </div>
  );
}
