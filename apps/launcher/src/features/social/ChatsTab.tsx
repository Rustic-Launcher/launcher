import { Avatar, cn, IconButton, SearchInput, Skeleton } from '@rustic/ui';
import { MessageCircle, SendHorizontal } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import type { ChatMessage, Conversation } from '@/cloud/types';
import { formatRelative } from '@/lib/format';
import { sk, useConversations, useFriends, useMessages, useSendMessage } from '@/queries/social';
import { useQueryClient } from '@tanstack/react-query';

/** Conversation list (left column of the Chats tab). */
export function ConversationList({
  selected,
  onSelect,
}: {
  selected: string | null;
  onSelect: (userId: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const { data, isLoading } = useConversations();
  const [query, setQuery] = useState('');
  const list = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (data ?? []).filter((c) => !q || c.user.display_name.toLowerCase().includes(q));
  }, [data, query]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <SearchInput
        value={query}
        onValueChange={setQuery}
        placeholder={t('social.searchChats')}
        clearLabel={t('actions.clear')}
        aria-label={t('social.searchChats')}
        className="mb-3"
      />
      <ul className="-mx-2 min-h-0 flex-1 overflow-y-auto px-2" aria-label={t('social.chats')}>
        {isLoading &&
          Array.from({ length: 6 }, (_, i) => (
            <li key={i} className="flex items-center gap-3 px-2 py-2">
              <Skeleton className="size-9 rounded-lg" />
              <div className="flex flex-1 flex-col gap-1.5">
                <Skeleton className="h-3 w-2/3" />
                <Skeleton className="h-2.5 w-1/2" />
              </div>
            </li>
          ))}
        {data && list.length === 0 && (
          <li className="text-fg-muted px-3 py-8 text-center text-sm">{t('social.noChats')}</li>
        )}
        {list.map((c) => (
          <li key={c.user.id}>
            <button
              type="button"
              onClick={() => onSelect(c.user.id)}
              aria-current={selected === c.user.id || undefined}
              data-testid={`chat-${c.user.username}`}
              className={cn(
                'rx-focus-ring flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left',
                selected === c.user.id ? 'bg-surface-3' : 'hover:bg-surface-2',
              )}
            >
              <Avatar
                name={c.user.display_name}
                size="sm"
                presence={c.user.presence}
                presenceLabel={t(`presence.${c.user.presence}`)}
              />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="flex items-center justify-between gap-2">
                  <span className="truncate text-sm font-medium">{c.user.display_name}</span>
                  {c.last_message && (
                    <span className="text-fg-subtle text-2xs shrink-0">
                      {formatRelative(c.last_message.sent_at, i18n.language)}
                    </span>
                  )}
                </span>
                <span className="flex items-center justify-between gap-2">
                  <span className="text-fg-subtle truncate text-xs">
                    {c.last_message?.body ?? t('social.sayHello')}
                  </span>
                  {c.unread > 0 && (
                    <span className="bg-accent text-accent-fg text-2xs grid min-w-4.5 place-items-center rounded-full px-1 font-bold">
                      {c.unread}
                    </span>
                  )}
                </span>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * The last message the peer sent: the open conversation is marked read again whenever it
 * changes, so a message arriving while the chat is on screen does not stay "unread".
 */
export function lastIncomingId(messages: ChatMessage[] | undefined, peerId: string | null) {
  if (!messages || !peerId) return null;
  for (let i = messages.length - 1; i >= 0; i--)
    if (messages[i]!.from_id === peerId) return messages[i]!.id;
  return null;
}

/** Right pane: the selected conversation, or the "Select a conversation" empty state. */
export function ConversationPane({ userId }: { userId: string | null }) {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const { data: friends } = useFriends();
  const { data: convs } = useConversations();
  const { data: messages, isLoading } = useMessages(userId);
  const send = useSendMessage(userId ?? '');
  const [draft, setDraft] = useState('');
  const endRef = useRef<HTMLDivElement>(null);
  const peer =
    friends?.find((f) => f.id === userId) ??
    (convs?.find((c) => c.user.id === userId)?.user as Conversation['user'] | undefined);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: 'end' });
  }, [messages?.length]);

  const lastIncoming = lastIncomingId(messages, userId);
  useEffect(() => {
    if (!userId) return;
    void cloud()
      .then((c) => c.markConversationRead(userId))
      .then(() => qc.invalidateQueries({ queryKey: sk.conversations }));
  }, [userId, lastIncoming, qc]);

  if (!userId || !peer) {
    return (
      <div
        className="flex h-full flex-col items-center justify-center gap-3 text-center"
        data-testid="chat-empty"
      >
        <span className="bg-surface-3 text-fg-muted grid size-16 place-items-center rounded-full">
          <MessageCircle className="size-7" />
        </span>
        <p className="text-lg font-semibold">{t('social.selectTitle')}</p>
        <p className="text-fg-muted max-w-64 text-sm">{t('social.selectBody')}</p>
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="chat-pane">
      <header className="border-border flex items-center gap-3 border-b px-4 py-3">
        <Avatar
          name={peer.display_name}
          size="sm"
          presence={peer.presence}
          presenceLabel={t(`presence.${peer.presence}`)}
        />
        <div className="flex flex-col">
          <span className="text-sm font-semibold">{peer.display_name}</span>
          <span className="text-fg-subtle text-xs">{t(`presence.${peer.presence}`)}</span>
        </div>
      </header>
      <div
        className="min-h-0 flex-1 overflow-y-auto px-4 py-3"
        role="log"
        aria-label={peer.display_name}
      >
        {isLoading && <Skeleton className="h-10 w-1/2" />}
        <ul className="flex flex-col gap-1.5">
          {messages?.map((m) => {
            const mine = m.from_id !== userId;
            return (
              <li key={m.id} className={cn('flex flex-col', mine ? 'items-end' : 'items-start')}>
                <span
                  className={cn(
                    'max-w-[75%] rounded-2xl px-3 py-2 text-sm',
                    mine
                      ? 'bg-accent text-accent-fg rounded-br-md'
                      : 'bg-surface-3 text-fg rounded-bl-md',
                  )}
                >
                  {m.body}
                </span>
                <span className="text-fg-subtle text-2xs mt-0.5">
                  {formatRelative(m.sent_at, i18n.language)}
                </span>
              </li>
            );
          })}
        </ul>
        <div ref={endRef} />
      </div>
      <form
        className="border-border flex items-center gap-2 border-t p-3"
        onSubmit={(e) => {
          e.preventDefault();
          const body = draft.trim();
          if (!body) return;
          send.mutate(body, { onSuccess: () => setDraft('') });
        }}
      >
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          maxLength={2000}
          placeholder={t('social.messagePlaceholder', { name: peer.display_name })}
          aria-label={t('social.messagePlaceholder', { name: peer.display_name })}
          data-testid="chat-input"
          className="bg-surface-2 border-border-strong/40 text-fg placeholder:text-fg-subtle focus-visible:border-accent h-10 flex-1 rounded-lg border px-3 text-sm outline-none"
        />
        <IconButton
          type="submit"
          variant="primary"
          label={t('social.send')}
          icon={<SendHorizontal />}
          disabled={!draft.trim()}
        />
      </form>
    </div>
  );
}
