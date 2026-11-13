/** Server directory queries (ADR 0158) and the "Play" flow: add to the list, record, join. */
import { toast } from '@rustic/ui';
import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import type { ServerListing, ServerListingQuery } from '@/cloud/types';
import { usePlay } from '@/features/play/usePlay';
import { WEB_BASE } from '@/features/settings/legal';
import { useActiveInstance } from '@/queries/core';
import { useMultiplayer, useMultiplayerActions } from '../list/queries';

export const listingKeys = {
  list: (q: ServerListingQuery) => ['server-listings', q] as const,
  one: (slug: string) => ['server-listing', slug] as const,
};

/** The website's publishing form (signed-in players). */
export const PUBLISH_SERVER_URL = `${WEB_BASE}/servers/publish`;
export const listingWebUrl = (slug: string) => `${WEB_BASE}/servers/${encodeURIComponent(slug)}`;

/** One page of listings (a Discover row, the hero). */
export const useListings = (q: ServerListingQuery, enabled = true) =>
  useQuery({
    queryKey: listingKeys.list(q),
    queryFn: async () => (await cloud()).serverListings(q),
    staleTime: 60_000,
    enabled,
  });

/** Browse: pages of `limit` with "Load more". */
export function useListingPages(q: Omit<ServerListingQuery, 'offset'>, limit = 24) {
  return useInfiniteQuery({
    queryKey: ['server-listings', 'pages', q, limit],
    initialPageParam: 0,
    queryFn: async ({ pageParam }) =>
      (await cloud()).serverListings({ ...q, offset: pageParam, limit }),
    getNextPageParam: (last, pages) => (last.has_more ? pages.length * limit : undefined),
    placeholderData: keepPreviousData,
    staleTime: 60_000,
  });
}

export const useListing = (slug: string) =>
  useQuery({
    queryKey: listingKeys.one(slug),
    queryFn: async () => (await cloud()).serverListing(slug),
    retry: (n, e) => (e as { status?: number }).status !== 404 && n < 2,
  });

/** The hero's picks: staff picks first, then verified servers, deduplicated, at most `max`. */
export function heroListings(
  featured: ServerListing[],
  verified: ServerListing[],
  max = 6,
): ServerListing[] {
  const seen = new Set<string>();
  return [...featured, ...verified]
    .filter((l) => !seen.has(l.id) && !!seen.add(l.id))
    .slice(0, max);
}

/** Lowercase address without the default port, as the multiplayer list compares them. */
export function sameAddress(a: string, b: string): boolean {
  const n = (s: string) =>
    s
      .trim()
      .toLowerCase()
      .replace(/\.$/, '')
      .replace(/:25565$/, '');
  return n(a) === n(b);
}

/**
 * "Play" on a listed server: adds it to the multiplayer list when missing, counts the press for
 * the trending ranking (anonymous, best effort), then launches the active instance and joins.
 */
export function useJoinListing() {
  const { t } = useTranslation('servers');
  const { instance } = useActiveInstance();
  const play = usePlay(instance);
  const { data: saved } = useMultiplayer();
  const { add } = useMultiplayerActions();
  const inList = (l: Pick<ServerListing, 'address'>) =>
    !!saved?.some((e) => sameAddress(e.address, l.address));
  const join = async (l: ServerListing) => {
    if (!instance) return;
    if (!inList(l)) {
      try {
        await add.mutateAsync({ name: l.name, address: l.address });
      } catch {
        /* the join still works without the list entry */
      }
    }
    void cloud()
      .then((c) => c.recordServerPlay(l.id))
      .catch(() => undefined);
    toast({ tone: 'info', title: t('multiplayer.joining', { name: l.name }) });
    void play.play({ join: { address: l.address, name: l.name } });
  };
  return { join, inList, instance, adding: add };
}
