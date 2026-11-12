/** Friends, presence, party, notifications (Rustic Cloud). */
import { useMutation, useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query';
import { cloud } from '@/cloud/client';
import type { ChatMessage, Friend, FriendTarget, Party } from '@/cloud/types';

export const sk = {
  me: ['me'] as const,
  friends: ['friends'] as const,
  requests: ['friend-requests'] as const,
  suggestions: ['friend-suggestions'] as const,
  party: ['party'] as const,
  notifications: ['notifications'] as const,
  news: ['news'] as const,
  featured: ['featured-servers'] as const,
  conversations: ['conversations'] as const,
  messages: (userId: string) => ['messages', userId] as const,
};

export const useConversations = () =>
  useQuery({ queryKey: sk.conversations, queryFn: async () => (await cloud()).conversations() });

export const useMessages = (userId: string | null) =>
  useQuery({
    queryKey: sk.messages(userId ?? ''),
    queryFn: async () => (await cloud()).messages(userId!),
    enabled: !!userId,
  });

/**
 * Adds a message once: the same message can come back from the send call, from the realtime
 * stream and from a refetch of the conversation.
 */
export function appendMessage(list: ChatMessage[], m: ChatMessage): ChatMessage[] {
  return list.some((x) => x.id === m.id) ? list : [...list, m];
}

export function useSendMessage(userId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: string) => (await cloud()).sendMessage(userId, body),
    onSuccess: (m) => {
      qc.setQueryData<ChatMessage[]>(sk.messages(userId), (old) => appendMessage(old ?? [], m));
      void qc.invalidateQueries({ queryKey: sk.conversations });
    },
  });
}

/** Unread chat messages + incoming friend requests: the social button badge. */
export function useSocialUnread(): number {
  const { data: convs } = useConversations();
  const { data: reqs } = useFriendRequests();
  return (
    (convs ?? []).reduce((n, c) => n + c.unread, 0) +
    (reqs ?? []).filter((r) => r.direction === 'incoming').length
  );
}

export const useMe = () => useQuery({ queryKey: sk.me, queryFn: async () => (await cloud()).me() });
export const useFriends = () =>
  useQuery({ queryKey: sk.friends, queryFn: async () => (await cloud()).friends() });
export const useFriendRequests = () =>
  useQuery({ queryKey: sk.requests, queryFn: async () => (await cloud()).friendRequests() });
/** Players met recently (ADR 0145). Refetched each time the add-friend dialog opens. */
export const useFriendSuggestions = () =>
  useQuery({
    queryKey: sk.suggestions,
    queryFn: async () => (await cloud()).friendSuggestions(),
    // Players asked meanwhile must drop out when the dialog opens again (global staleTime is 30 s).
    staleTime: 0,
    refetchOnMount: 'always',
  });

/** "Appear in suggestions of players I meet" (Settings › Privacy). */
export function useSetDiscoverable() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (on: boolean) => (await cloud()).setDiscoverable(on),
    onSuccess: (me) => {
      qc.setQueryData(sk.me, me);
      void qc.invalidateQueries({ queryKey: sk.suggestions });
    },
  });
}

export const useParty = () =>
  useQuery({ queryKey: sk.party, queryFn: async () => (await cloud()).party() });
export const useNotifications = () =>
  useQuery({ queryKey: sk.notifications, queryFn: async () => (await cloud()).notifications() });
export const useNews = () =>
  useQuery({
    queryKey: sk.news,
    queryFn: async () => (await cloud()).news(),
    staleTime: 10 * 60_000,
  });
export const useFeaturedServers = () =>
  useQuery({
    queryKey: sk.featured,
    queryFn: async () => (await cloud()).featuredServers(),
    staleTime: 5 * 60_000,
  });

export function useInviteToParty() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (userId: string) => (await cloud()).inviteToParty(userId),
    onSuccess: (p) => qc.setQueryData(sk.party, p),
  });
}

export function useFriendRequestActions() {
  const qc = useQueryClient();
  const refresh = () => {
    void qc.invalidateQueries({ queryKey: sk.requests });
    void qc.invalidateQueries({ queryKey: sk.friends });
  };
  return {
    respond: useMutation({
      mutationFn: async (v: { id: string; accept: boolean }) =>
        (await cloud()).respondFriendRequest(v.id, v.accept),
      onSuccess: refresh,
    }),
    send: useMutation({
      mutationFn: async (target: string | FriendTarget) =>
        (await cloud()).sendFriendRequest(target),
      onSuccess: refresh,
    }),
    cancel: useMutation({
      mutationFn: async (id: string) => (await cloud()).cancelFriendRequest(id),
      onSuccess: refresh,
    }),
    remove: useMutation({
      mutationFn: async (id: string) => (await cloud()).removeFriend(id),
      onSuccess: refresh,
    }),
    favorite: useMutation({
      mutationFn: async (v: { id: string; favorite: boolean }) =>
        (await cloud()).setFavoriteFriend(v.id, v.favorite),
      onSuccess: refresh,
    }),
  };
}

/** Sorts friends: in-game, online, away/busy, then offline; favourites first in each group. */
export function sortFriends(list: Friend[]): Friend[] {
  const rank = { 'in-game': 0, online: 1, away: 2, busy: 2, offline: 3 } as const;
  return [...list].sort(
    (a, b) =>
      rank[a.presence] - rank[b.presence] ||
      Number(b.favorite) - Number(a.favorite) ||
      a.display_name.localeCompare(b.display_name),
  );
}

export async function bindRealtime(qc: QueryClient): Promise<() => void> {
  const c = await cloud();
  return c.subscribe((e) => {
    if (e.type === 'presence') {
      qc.setQueryData<Friend[]>(sk.friends, (old) =>
        old?.map((f) => (f.id === e.friend.id ? e.friend : f)),
      );
    } else if (e.type === 'party') {
      qc.setQueryData<Party | null>(sk.party, e.party);
    } else if (e.type === 'friend-request') {
      void qc.invalidateQueries({ queryKey: sk.requests });
    } else if (e.type === 'friends-changed') {
      void qc.invalidateQueries({ queryKey: sk.friends });
      void qc.invalidateQueries({ queryKey: sk.requests });
    } else if (e.type === 'message') {
      const other = e.message.from_id;
      qc.setQueryData<ChatMessage[]>(sk.messages(other), (old) =>
        old ? appendMessage(old, e.message) : old,
      );
      void qc.invalidateQueries({ queryKey: sk.conversations });
    } else if (e.type === 'notification') {
      void qc.invalidateQueries({ queryKey: sk.notifications });
    }
  });
}
