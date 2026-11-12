import {
  Avatar,
  Button,
  Dialog,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownSeparator,
  DropdownTrigger,
  IconButton,
  SearchInput,
  Skeleton,
  toast,
} from '@rustic/ui';
import {
  Check,
  Crown,
  LogIn,
  MessageSquare,
  MoreHorizontal,
  PartyPopper,
  Star,
  UserMinus,
  UserPlus,
  Users,
  X,
} from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Friend } from '@/cloud/types';
import { cloud } from '@/cloud/client';
import { usePlay } from '@/features/play/usePlay';
import { formatRelative } from '@/lib/format';
import { useActiveInstance } from '@/queries/core';
import {
  sk,
  sortFriends,
  useFriendRequestActions,
  useFriendRequests,
  useFriends,
  useInviteToParty,
  useParty,
} from '@/queries/social';
import { useUi } from '@/stores/ui';
import { useQueryClient } from '@tanstack/react-query';

/** Friends tab of the Social window: search, add, party, requests, online/offline lists. */
export function FriendsTab({ onMessage }: { onMessage: (friend: Friend) => void }) {
  const { t } = useTranslation();
  const { data: friends, isLoading, isError, refetch } = useFriends();
  const { data: requests } = useFriendRequests();
  const [query, setQuery] = useState('');
  const addOpen = useUi((s) => s.addFriendOpen);
  const setAddOpen = useUi((s) => s.setAddFriendOpen);
  const focus = useUi((s) => s.friendsFocus);
  const setFocus = useUi((s) => s.setFriendsFocus);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (focus === 'search') searchRef.current?.focus();
    if (focus) setFocus(null);
  }, [focus, setFocus]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = sortFriends(friends ?? []);
    return q
      ? list.filter(
          (f) => f.display_name.toLowerCase().includes(q) || f.username.toLowerCase().includes(q),
        )
      : list;
  }, [friends, query]);
  const online = filtered.filter((f) => f.presence !== 'offline');
  const offline = filtered.filter((f) => f.presence === 'offline');
  const incoming = requests?.filter((r) => r.direction === 'incoming') ?? [];
  const outgoing = requests?.filter((r) => r.direction === 'outgoing') ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col" aria-label={t('friends.title')}>
      <div className="flex flex-col gap-2 pb-3">
        <div className="flex items-center gap-2">
          <SearchInput
            ref={searchRef}
            value={query}
            onValueChange={setQuery}
            placeholder={t('friends.search')}
            clearLabel={t('actions.clear')}
            aria-label={t('friends.search')}
            className="flex-1"
          />
          <IconButton
            variant="secondary"
            label={t('friends.add')}
            icon={<UserPlus />}
            onClick={() => setAddOpen(true)}
            aria-haspopup="dialog"
            aria-expanded={addOpen}
            data-testid="add-friend"
          />
        </div>
      </div>
      <div className="-mx-2 min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        <PartyCard />
        {incoming.length + outgoing.length > 0 && (
          <Group title={t('friends.requests', { count: incoming.length + outgoing.length })}>
            {incoming.map((r) => (
              <RequestRow
                key={r.id}
                id={r.id}
                name={r.user.display_name}
                username={r.user.username}
                incoming
              />
            ))}
            {outgoing.map((r) => (
              <RequestRow
                key={r.id}
                id={r.id}
                name={r.user.display_name}
                username={r.user.username}
              />
            ))}
          </Group>
        )}
        {isLoading && (
          <div className="flex flex-col gap-2 px-2 pt-2">
            {Array.from({ length: 6 }, (_, i) => (
              <Skeleton key={i} className="h-11" />
            ))}
          </div>
        )}
        {isError && (
          <div className="text-fg-muted px-3 py-6 text-center text-sm">
            {t('friends.error')}{' '}
            <Button variant="link" size="sm" onClick={() => void refetch()}>
              {t('actions.retry')}
            </Button>
          </div>
        )}
        {friends && friends.length === 0 && (
          <div className="text-fg-muted flex flex-col items-center gap-2 px-3 py-8 text-center text-sm">
            {t('friends.empty')}
            <Button
              size="sm"
              variant="soft"
              leftIcon={<UserPlus />}
              onClick={() => setAddOpen(true)}
            >
              {t('friends.add')}
            </Button>
          </div>
        )}
        {online.length > 0 && (
          <Group title={t('friends.online', { count: online.length })}>
            {online.map((f) => (
              <FriendRow key={f.id} friend={f} onMessage={onMessage} />
            ))}
          </Group>
        )}
        {offline.length > 0 && (
          <Group title={t('friends.offline', { count: offline.length })}>
            {offline.map((f) => (
              <FriendRow key={f.id} friend={f} onMessage={onMessage} />
            ))}
          </Group>
        )}
      </div>
    </div>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mt-3">
      <h3 className="text-2xs text-fg-subtle px-2 pb-1 font-semibold tracking-wider uppercase">
        {title}
      </h3>
      <ul className="flex flex-col">{children}</ul>
    </section>
  );
}

function RequestRow({
  id,
  name,
  username,
  incoming,
}: {
  id: string;
  name: string;
  username: string;
  incoming?: boolean;
}) {
  const { t } = useTranslation();
  const { respond, cancel } = useFriendRequestActions();
  return (
    <li className="flex items-center gap-2.5 rounded-lg px-2 py-1.5">
      <Avatar name={name} size="sm" />
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-sm font-medium">{name}</span>
        <span className="text-fg-subtle truncate text-xs">
          {incoming ? t('friends.incoming') : t('friends.outgoing')} · @{username}
        </span>
      </div>
      {incoming ? (
        <>
          <IconButton
            size="sm"
            variant="primary"
            label={t('friends.accept')}
            icon={<Check />}
            onClick={() => respond.mutate({ id, accept: true })}
          />
          <IconButton
            size="sm"
            label={t('friends.decline')}
            icon={<X />}
            onClick={() => respond.mutate({ id, accept: false })}
          />
        </>
      ) : (
        <IconButton
          size="sm"
          label={t('friends.cancelRequest')}
          icon={<X />}
          onClick={() => cancel.mutate(id)}
        />
      )}
    </li>
  );
}

export function PartyCard() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { data: party, isLoading } = useParty();
  if (isLoading) return <Skeleton className="mx-2 h-20" />;
  if (!party) return null;
  return (
    <section
      className="rx-surface mx-2 rounded-xl p-3"
      aria-label={t('party.title')}
      data-testid="party-card"
    >
      <div className="mb-2 flex items-center justify-between">
        <h3 className="flex items-center gap-1.5 text-xs font-semibold">
          <PartyPopper className="text-accent-text size-3.5" />
          {t('party.title')}
          <span className="text-fg-subtle font-normal">
            {party.members.length}/{party.max_size}
          </span>
        </h3>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2"
          onClick={() =>
            void cloud()
              .then((c) => c.leaveParty())
              .then(() => qc.setQueryData(sk.party, null))
          }
        >
          {t('party.leave')}
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-1.5">
        {party.members.map((m) => (
          <span key={m.id} className="relative" title={m.display_name}>
            <Avatar
              name={m.display_name}
              size="sm"
              presence={m.presence}
              presenceLabel={t(`presence.${m.presence}`)}
            />
            {m.id === party.leader_id && (
              <Crown
                className="text-warning absolute -top-1.5 left-1/2 size-3 -translate-x-1/2"
                aria-label={t('party.leader')}
              />
            )}
          </span>
        ))}
        {party.pending_invites.map((u) => (
          <span
            key={u.id}
            className="opacity-50"
            title={t('party.pending', { name: u.display_name })}
          >
            <Avatar name={u.display_name} size="sm" />
          </span>
        ))}
      </div>
    </section>
  );
}

export type LooseT = (key: string, options?: Record<string, string | undefined>) => string;

export function activityText(f: Friend, t: LooseT, lang: string): string {
  if (f.presence === 'offline') {
    if (!f.last_seen_at) return t('presence.offline');
    const when = formatRelative(f.last_seen_at, lang);
    return t('friends.lastSeen', { when });
  }
  const a = f.activity;
  if (!a) return t(`presence.${f.presence}`);
  switch (a.kind) {
    case 'server':
      return t('friends.activity.server', { server: a.server_name });
    case 'singleplayer':
      return t('friends.activity.singleplayer');
    case 'hosting':
      return t('friends.activity.hosting', { server: a.server_name });
    default:
      return t('friends.activity.launcher');
  }
}

function FriendRow({ friend, onMessage }: { friend: Friend; onMessage: (friend: Friend) => void }) {
  const { t, i18n } = useTranslation();
  const { instance } = useActiveInstance();
  const play = usePlay(instance);
  const invite = useInviteToParty();
  const { remove, favorite } = useFriendRequestActions();
  const [confirmRemove, setConfirmRemove] = useState(false);
  const joinable = friend.activity?.joinable && friend.activity.server_address;

  return (
    <li
      className="group hover:bg-surface-2 flex items-center gap-2.5 rounded-lg px-2 py-1.5"
      data-testid={`friend-${friend.username}`}
    >
      <Avatar
        name={friend.display_name}
        size="sm"
        presence={friend.presence}
        presenceLabel={t(`presence.${friend.presence}`)}
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="flex items-center gap-1 truncate text-sm font-medium">
          {friend.display_name}
          {friend.favorite && (
            <Star className="fill-warning text-warning size-3" aria-label={t('friends.favorite')} />
          )}
        </span>
        <span
          className={`truncate text-xs ${friend.presence === 'in-game' ? 'text-accent-text' : 'text-fg-subtle'}`}
        >
          {activityText(friend, t as unknown as LooseT, i18n.language)}
        </span>
      </div>
      {joinable && (
        <IconButton
          size="sm"
          label={t('friends.joinAria', { name: friend.display_name })}
          icon={<LogIn />}
          tooltipSide="left"
          className="bg-accent-soft text-accent-text hover:bg-accent/25 hover:text-accent-text"
          onClick={() => {
            void play.play({
              join: {
                address: friend.activity!.server_address!,
                name: friend.activity!.server_name,
              },
            });
            toast({
              tone: 'info',
              title: t('friends.joining', { server: friend.activity!.server_name }),
            });
          }}
        />
      )}
      <Dropdown>
        <DropdownTrigger asChild>
          <IconButton
            size="sm"
            tooltip={false}
            label={t('friends.more', { name: friend.display_name })}
            icon={<MoreHorizontal />}
            className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
          />
        </DropdownTrigger>
        <DropdownContent>
          <DropdownItem icon={<MessageSquare />} onSelect={() => onMessage(friend)}>
            {t('social.message')}
          </DropdownItem>
          <DropdownItem
            icon={<Users />}
            disabled={friend.presence === 'offline'}
            onSelect={() =>
              invite.mutate(friend.id, {
                onSuccess: () =>
                  toast({
                    tone: 'success',
                    title: t('party.invited', { name: friend.display_name }),
                  }),
              })
            }
          >
            {t('party.invite')}
          </DropdownItem>
          <DropdownItem
            icon={<Star />}
            onSelect={() => favorite.mutate({ id: friend.id, favorite: !friend.favorite })}
          >
            {friend.favorite ? t('friends.unfavorite') : t('friends.makeFavorite')}
          </DropdownItem>
          <DropdownSeparator />
          <DropdownItem icon={<UserMinus />} destructive onSelect={() => setConfirmRemove(true)}>
            {t('friends.remove')}
          </DropdownItem>
        </DropdownContent>
      </Dropdown>
      <Dialog
        open={confirmRemove}
        onOpenChange={setConfirmRemove}
        tone="destructive"
        icon={<UserMinus />}
        title={t('friends.removeTitle', { name: friend.display_name })}
        description={t('friends.removeBody')}
        closeLabel={t('actions.close')}
        footer={
          <>
            <Button onClick={() => setConfirmRemove(false)}>{t('actions.cancel')}</Button>
            <Button
              variant="danger"
              loading={remove.isPending}
              onClick={() => remove.mutate(friend.id, { onSuccess: () => setConfirmRemove(false) })}
            >
              {t('friends.remove')}
            </Button>
          </>
        }
      />
    </li>
  );
}
