import { CommandPalette, Kbd, type CommandGroup } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  Gamepad2,
  Globe,
  Moon,
  Play,
  Plus,
  Puzzle,
  Server,
  Settings,
  Sun,
  UserPlus,
  UserRound,
} from 'lucide-react';
import { useDeferredValue, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import { SETTINGS_SECTIONS } from '@/features/settings/routes';
import { useInstances } from '@/queries/core';
import { useFeaturedServers, useFriends } from '@/queries/social';
import { openSocial } from '@/features/social/openSocial';
import { usePrefs } from '@/stores/prefs';
import { useUi } from '@/stores/ui';
import { MAIN_NAV, SETTINGS_NAV } from './nav';

/** Ctrl+K: global search over pages, content, friends, servers and settings. */
export function CommandMenu() {
  const { t } = useTranslation();
  const open = useUi((s) => s.commandOpen);
  const setOpen = useUi((s) => s.setCommandOpen);
  const setAddFriendOpen = useUi((s) => s.setAddFriendOpen);
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const deferred = useDeferredValue(query.trim());
  const { data: instances } = useInstances();
  const { data: friends } = useFriends();
  const { data: featured } = useFeaturedServers();
  const theme = usePrefs((s) => s.theme);
  const setTheme = usePrefs((s) => s.setTheme);

  const servers = useQuery({
    queryKey: ['servers'],
    queryFn: () => invoke('servers_list'),
    enabled: open,
    retry: false,
  });
  const content = useQuery({
    queryKey: ['palette-content', deferred],
    queryFn: () =>
      invoke('content_search', {
        query: deferred,
        kind: 'mod',
        sources: ['horizon', 'modrinth', 'rustic'],
        compatible_only: false,
        sort: 'relevance',
        offset: 0,
        limit: 6,
      }),
    enabled: open && deferred.length >= 2,
    retry: false,
  });

  const groups = useMemo<CommandGroup[]>(() => {
    const go = (to: string, search?: Record<string, string>) => () =>
      void navigate({ to, search: search as never });
    return [
      {
        id: 'pages',
        heading: t('search.groups.pages'),
        items: [...MAIN_NAV, SETTINGS_NAV].map((n) => ({
          id: n.id,
          label: t(`nav.${n.id}`),
          icon: <n.icon />,
          hint: n.shortcut,
          onSelect: n.href ? () => void invoke('app_open_url', { url: n.href! }) : go(n.to ?? '/'),
        })),
      },
      {
        id: 'actions',
        heading: t('search.groups.actions'),
        items: [
          {
            id: 'new-instance',
            label: t('search.actions.newInstance'),
            icon: <Plus />,
            onSelect: go('/instances'),
          },
          {
            id: 'new-server',
            label: t('search.actions.newServer'),
            icon: <Server />,
            onSelect: go('/servers/new'),
          },
          {
            id: 'add-friend',
            label: t('search.actions.addFriend'),
            icon: <UserPlus />,
            onSelect: () => setAddFriendOpen(true),
          },
          {
            id: 'theme',
            label:
              theme === 'light' ? t('search.actions.darkTheme') : t('search.actions.lightTheme'),
            icon: theme === 'light' ? <Moon /> : <Sun />,
            onSelect: () => setTheme(theme === 'light' ? 'dark' : 'light'),
          },
        ],
      },
      {
        id: 'instances',
        heading: t('search.groups.instances'),
        items: (instances ?? []).map((i) => ({
          id: i.id,
          label: i.name,
          icon: <Gamepad2 />,
          hint: t('search.open'),
          onSelect: go(`/instances/${i.id}`),
        })),
      },
      {
        id: 'content',
        heading: t('search.groups.content'),
        items: (content.data?.hits ?? []).map((h) => ({
          id: h.id,
          label: h.name,
          keywords: [deferred, h.author],
          icon: <Puzzle />,
          hint: h.author,
          onSelect: go(`/content/${h.id}`),
        })),
      },
      {
        id: 'friends',
        heading: t('search.groups.friends'),
        items: (friends ?? []).map((f) => ({
          id: f.id,
          label: f.display_name,
          keywords: [f.username],
          icon: <UserRound />,
          hint: t(`presence.${f.presence}`),
          onSelect: () => {
            void openSocial({ tab: 'chats', chatWith: f.id });
          },
        })),
      },
      {
        id: 'servers',
        heading: t('search.groups.servers'),
        items: [
          ...(servers.data ?? []).map((s) => ({
            id: s.id,
            label: s.name,
            icon: <Server />,
            hint: t('search.myServer'),
            onSelect: go(`/servers/${s.id}`),
          })),
          ...(featured ?? []).map((s) => ({
            id: s.id,
            label: s.name,
            keywords: [s.address],
            icon: <Globe />,
            hint: s.address,
            onSelect: go('/servers', { tab: 'featured' }),
          })),
        ],
      },
      {
        id: 'settings',
        heading: t('search.groups.settings'),
        items: SETTINGS_SECTIONS.map((s) => ({
          id: s,
          label: t(`search.settings.${s}`),
          icon: <Settings />,
          onSelect: go('/settings', { section: s }),
        })),
      },
      {
        id: 'play',
        heading: t('search.groups.play'),
        items: (instances ?? []).map((i) => ({
          id: `play-${i.id}`,
          label: t('search.playInstance', { name: i.name }),
          icon: <Play />,
          onSelect: () => void invoke('instance_launch', { instance_id: i.id }),
        })),
      },
    ];
  }, [
    t,
    navigate,
    instances,
    friends,
    featured,
    servers.data,
    content.data,
    deferred,
    theme,
    setTheme,
    setAddFriendOpen,
  ]);

  return (
    <CommandPalette
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (!o) setQuery('');
      }}
      groups={groups}
      query={query}
      onQueryChange={setQuery}
      title={t('search.title')}
      placeholder={t('search.placeholder')}
      emptyText={t('search.empty')}
      loading={content.isFetching}
      loadingText={t('search.loading')}
      footer={
        <>
          <span className="flex items-center gap-1.5">
            <Kbd>↑↓</Kbd> {t('search.hintNavigate')}
          </span>
          <span className="flex items-center gap-1.5">
            <Kbd>Enter</Kbd> {t('search.hintOpen')}
          </span>
          <span className="flex items-center gap-1.5">
            <Kbd>Esc</Kbd> {t('search.hintClose')}
          </span>
        </>
      }
    />
  );
}
