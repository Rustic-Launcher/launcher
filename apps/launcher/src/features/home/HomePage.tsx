import {
  Avatar,
  Badge,
  cn,
  Dropdown,
  DropdownCheckboxItem,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownTrigger,
  IconButton,
  PremiumCard,
  Skeleton,
  toast,
  Tooltip,
  AccentText,
} from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { ChevronDown, ExternalLink, LogIn, Settings2, Users } from 'lucide-react';
import { lazy, Suspense, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import type { NewsItem } from '@/cloud/types';
import { LaunchButton } from '@/features/play/LaunchButton';
import { usePlay } from '@/features/play/usePlay';
import { activityText, type LooseT } from '@/features/social/FriendsTab';
import { openSocial } from '@/features/social/openSocial';
import { invoke } from '@/ipc';
import { formatCompact, formatRelative } from '@/lib/format';
import { qk, useAccounts, useActiveInstance } from '@/queries/core';
import { sortFriends, useFeaturedServers, useFriends, useNews } from '@/queries/social';
import { HeroVideo } from './HeroVideo';
import { coverGradient } from './logic';
import { AccountAvatar } from '@/components/AccountAvatar';

const HeroPlayer = lazy(() => import('./HeroPlayer'));

/** Launchpad (step 1b): greeting, launch hero, latest news; friends column on the right. */
export function HomePage() {
  return (
    <div className="rx-page-enter grid min-h-full grid-cols-[minmax(0,1fr)_minmax(300px,380px)] gap-4 p-4">
      <div className="flex min-w-0 flex-col gap-5">
        <Greeting />
        <LaunchHero />
        <NewsGrid />
      </div>
      <LaunchpadAside />
    </div>
  );
}

function Greeting() {
  const { t, i18n } = useTranslation('home');
  const qc = useQueryClient();
  const { data: accounts } = useAccounts();
  const { data: featured } = useFeaturedServers();
  const active = accounts?.find((a) => a.active);
  const measured = featured?.filter((s) => s.players_online != null);
  const online = measured?.length
    ? measured.reduce((n, s) => n + (s.players_online ?? 0), 0)
    : null;
  return (
    <div className="flex items-end justify-between gap-3 px-1">
      <div className="flex flex-col gap-1.5">
        {accounts && !active ? (
          <h1 className="text-lg font-semibold">{t('hero.greetingAnon')}</h1>
        ) : (
          <h1 className="flex items-center gap-2 text-lg font-medium">
            <span className="text-fg-muted">
              <AccentText text={t('launchpad.welcome')} />
            </span>
            <Dropdown>
              <DropdownTrigger asChild>
                <button
                  type="button"
                  className="rx-focus-ring hover:bg-surface-2 flex items-center gap-1.5 rounded-md px-1 font-bold"
                >
                  {active && <AccountAvatar account={active} size="xs" />}
                  {active?.username ?? '…'}
                  <ChevronDown className="text-fg-subtle size-4" />
                </button>
              </DropdownTrigger>
              <DropdownContent align="start" className="w-64">
                <DropdownLabel>{t('launchpad.playAs')}</DropdownLabel>
                {accounts?.map((a) => (
                  <DropdownItem
                    key={a.id}
                    icon={<AccountAvatar account={a} size="xs" />}
                    onSelect={() =>
                      void invoke('auth_set_active', { account_id: a.id }).then((l) =>
                        qc.setQueryData(qk.accounts, l),
                      )
                    }
                  >
                    {a.username}
                  </DropdownItem>
                ))}
              </DropdownContent>
            </Dropdown>
          </h1>
        )}
      </div>
      {online != null && (
        <span className="text-fg-muted flex items-center gap-2 text-sm" data-testid="online-pill">
          <span className="bg-success size-2 rounded-full" />
          <span className="text-fg font-semibold tabular-nums">
            {formatCompact(online, i18n.language)}
          </span>
          {t('launchpad.online')}
        </span>
      )}
    </div>
  );
}

function LaunchHero() {
  const { t, i18n } = useTranslation('home');
  const { t: tc } = useTranslation();
  const { instance, isLoading } = useActiveInstance();
  const play = usePlay(instance);
  return (
    <section
      aria-label={t('launchpad.heroLabel')}
      className="border-border-default relative isolate flex aspect-[16/7] min-h-72 flex-col items-center justify-center overflow-hidden rounded-2xl border"
    >
      <HeroVideo running={play.state === 'running'} />
      <div className="absolute inset-y-0 left-0 w-[34%] max-w-80">
        <Suspense fallback={<Skeleton className="m-6 h-4/5 rounded-xl" />}>
          <HeroPlayer />
        </Suspense>
      </div>
      <div className="relative flex flex-col items-center gap-3 px-4">
        {isLoading ? (
          <Skeleton className="h-[72px] w-80 rounded-xl" />
        ) : (
          <LaunchButton instance={instance} play={play} />
        )}
        {instance && (
          <div className="flex flex-col items-center gap-1.5 text-center">
            <p className="text-sm text-white/75" data-testid="hero-continue">
              {instance.last_played_at
                ? t('hero.continue', {
                    name: instance.name,
                    when: formatRelative(instance.last_played_at, i18n.language),
                  })
                : t('hero.continueNew', { name: instance.name })}
            </p>
            <div className="flex flex-wrap items-center justify-center gap-2 text-xs">
              <span className="bg-surface/70 text-fg flex items-center gap-1.5 rounded-full px-2.5 py-1 backdrop-blur">
                <span
                  className={cn(
                    'size-1.5 rounded-full',
                    play.tone === 'success' && 'bg-success',
                    play.tone === 'warning' && 'bg-warning',
                    play.tone === 'accent' && 'bg-accent',
                  )}
                />
                <span data-testid="play-status">{play.status}</span>
              </span>
              {play.state === 'update' && instance.update_available && (
                <Badge variant="warning" className="bg-black/55 text-[#EDBC5C]">
                  {tc('play.newVersion', { version: instance.update_available.replace(/^v/, '') })}
                </Badge>
              )}
              {instance.last_server && play.state !== 'running' && play.state !== 'progress' && (
                <button
                  type="button"
                  data-testid="hero-resume-server"
                  onClick={() =>
                    void play.play({
                      join: {
                        address: instance.last_server!.address,
                        name: instance.last_server!.name,
                      },
                    })
                  }
                  className="rx-focus-ring bg-surface/70 text-accent-text hover:bg-surface-2 flex items-center gap-1.5 rounded-full px-2.5 py-1 font-medium backdrop-blur"
                >
                  <LogIn className="size-3.5" />
                  {t('hero.resumeServer', { server: instance.last_server.name })}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
      {instance && (
        <Tooltip content={t('launchpad.instanceSettings')}>
          <Link
            to="/instances/$instanceId"
            params={{ instanceId: instance.id }}
            search={{ tab: 'settings' }}
            aria-label={t('launchpad.instanceSettings')}
            className="rx-focus-ring bg-surface/80 text-fg-muted hover:text-fg absolute right-3 bottom-3 grid size-9 place-items-center rounded-lg backdrop-blur"
          >
            <Settings2 className="size-4.5" />
          </Link>
        </Tooltip>
      )}
      <Link
        to="/locker"
        className="rx-focus-ring bg-surface/70 text-fg-muted hover:text-fg absolute bottom-3 left-3 rounded-lg px-2.5 py-1.5 text-xs font-medium backdrop-blur"
      >
        {t('hero.openLocker')}
      </Link>
    </section>
  );
}

const READ_KEY = 'rustic.news.read';
function readSet(): Set<string> {
  try {
    return new Set(JSON.parse(window.localStorage.getItem(READ_KEY) ?? '[]') as string[]);
  } catch {
    return new Set();
  }
}

function NewsGrid() {
  const { t, i18n } = useTranslation('home');
  const { data, isLoading, isError, refetch } = useNews();
  const [read, setRead] = useState(readSet);
  const [sources, setSources] = useState<Record<NewsItem['source'], boolean>>({
    horizon: true,
    rustic: true,
  });
  const list = useMemo(() => (data ?? []).filter((n) => sources[n.source]), [data, sources]);
  const unread = list.filter((n) => !read.has(n.id)).length;

  const open = (n: NewsItem) => {
    const next = new Set(read).add(n.id);
    setRead(next);
    try {
      window.localStorage.setItem(READ_KEY, JSON.stringify([...next]));
    } catch {
      /* ignore */
    }
    void invoke('app_open_url', { url: n.url });
  };

  return (
    <section aria-labelledby="news-title" className="flex flex-col gap-3">
      <div className="flex items-center justify-between px-1">
        <h2 id="news-title" className="text-base font-semibold">
          {t('launchpad.news')}{' '}
          <span className="text-fg-subtle font-normal">
            {t('launchpad.unread', { count: unread })}
          </span>
        </h2>
        <Dropdown>
          <DropdownTrigger asChild>
            <button
              type="button"
              className="rx-focus-ring text-fg-muted hover:text-fg flex items-center gap-1 rounded text-sm"
            >
              {t('launchpad.explore')}
              <ChevronDown className="size-3.5" />
            </button>
          </DropdownTrigger>
          <DropdownContent>
            {(['horizon', 'rustic'] as const).map((s) => (
              <DropdownCheckboxItem
                key={s}
                checked={sources[s]}
                onCheckedChange={(v) => setSources((o) => ({ ...o, [s]: !!v }))}
              >
                {t(`news.source.${s}`)}
              </DropdownCheckboxItem>
            ))}
          </DropdownContent>
        </Dropdown>
      </div>
      {isError && <QueryError onRetry={() => void refetch()} />}
      {data && !list.length && (
        <p className="text-fg-muted px-1 text-sm" data-testid="news-empty">
          {t('launchpad.noNews')}
        </p>
      )}
      <ul className="grid grid-cols-1 gap-4 lg:grid-cols-2" data-testid="news-grid">
        {isLoading &&
          Array.from({ length: 4 }, (_, i) => (
            <li key={i}>
              <Skeleton className="aspect-[16/9] rounded-xl" />
            </li>
          ))}
        {list.map((n) => (
          <li key={n.id} className="relative">
            {!read.has(n.id) && (
              <span
                className="bg-accent ring-surface absolute -top-1 -left-1 z-10 size-3 rounded-full ring-4"
                role="img"
                aria-label={t('launchpad.unreadDot')}
              />
            )}
            <button
              type="button"
              onClick={() => open(n)}
              data-testid={`news-${n.id}`}
              className="rx-focus-ring group border-border relative flex aspect-[16/9] w-full flex-col justify-end overflow-hidden rounded-xl border p-5 text-left text-white"
              style={{ background: coverGradient(n.hue) }}
            >
              <NewsArt seed={n.hue} />
              <span
                aria-hidden
                className="absolute inset-0 bg-gradient-to-t from-black/70 via-black/15 to-transparent"
              />
              <span className="relative flex flex-col gap-1.5">
                <span className="flex flex-wrap items-center gap-2 text-xs">
                  <span className="bg-accent text-accent-fg rounded-full px-2 py-0.5 font-semibold">
                    {t(`news.source.${n.source}`)}
                  </span>
                  <span className="text-white/80">{t(`news.kind.${n.kind}`)}</span>
                  {n.version && <span className="text-white/70 tabular-nums">v{n.version}</span>}
                  <span className="text-white/70">
                    · {formatRelative(n.published_at, i18n.language)}
                  </span>
                </span>
                <span className="text-xl font-bold tracking-tight">{n.title}</span>
                <span className="line-clamp-2 text-sm text-white/85">{n.summary}</span>
              </span>
              <ExternalLink className="absolute top-4 right-4 size-4 text-white/70 opacity-0 transition-opacity group-hover:opacity-100" />
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Voxel pattern for news covers (original art drawn in code). */
function NewsArt({ seed }: { seed: number }) {
  const cells = Array.from({ length: 24 }, (_, i) => {
    const v = (seed * 31 + i * 17) % 100;
    return { x: (i % 6) * 34, y: Math.floor(i / 6) * 34, o: v > 70 ? 0.18 : v > 40 ? 0.08 : 0 };
  });
  return (
    <svg aria-hidden viewBox="0 0 204 136" className="absolute top-4 right-4 h-3/4 w-auto">
      {cells.map((c, i) => (
        <rect
          key={i}
          x={c.x}
          y={c.y}
          width="30"
          height="30"
          rx="4"
          fill="white"
          fillOpacity={c.o}
        />
      ))}
    </svg>
  );
}

function LaunchpadAside() {
  const { t } = useTranslation('home');
  const { t: tc, i18n } = useTranslation();
  const { data: featured, isLoading: fLoading } = useFeaturedServers();
  const { data: friends, isLoading } = useFriends();
  const { instance } = useActiveInstance();
  const play = usePlay(instance);
  const sorted = sortFriends(friends ?? []);
  const online = sorted.filter((f) => f.presence !== 'offline');
  const playing = online.filter((f) => f.activity && f.activity.kind !== 'launcher');

  const join = (address: string, name: string) => {
    toast({ tone: 'info', title: tc('friends.joining', { server: name }) });
    void play.play({ join: { address, name } });
  };

  return (
    <aside className="flex min-w-0 flex-col gap-4" aria-label={t('launchpad.aside')}>
      {(fLoading || !!featured?.length) && (
        <PremiumCard
          variant="compact"
          bodyClassName="flex items-center justify-around gap-2 px-3 py-2.5"
          data-testid="featured-strip"
        >
          {fLoading &&
            Array.from({ length: 4 }, (_, i) => (
              <Skeleton key={i} className="size-10 rounded-lg" />
            ))}
          {featured?.map((s) => (
            <Tooltip key={s.id} content={t('launchpad.joinServer', { name: s.name })}>
              <button
                type="button"
                data-testid={`join-${s.id}`}
                aria-label={t('launchpad.joinServer', { name: s.name })}
                onClick={() => join(s.address, s.name)}
                className="rx-focus-ring grid size-10 place-items-center rounded-lg text-sm font-black text-white shadow-sm transition-transform hover:-translate-y-0.5"
                style={{ background: coverGradient(s.hue) }}
              >
                <span data-testid={`featured-${s.id}`}>
                  <span aria-hidden>{s.name.slice(0, 1)}</span>
                  <span className="sr-only">{s.name}</span>
                </span>
              </button>
            </Tooltip>
          ))}
        </PremiumCard>
      )}

      <section aria-labelledby="aside-friends" className="flex min-h-0 flex-1 flex-col gap-3 px-1">
        {/* Stays at the top of the visible panel while the page scrolls. */}
        <div
          className="bg-surface/90 sticky top-0 z-10 -mx-1 flex items-center justify-between px-1 py-2 backdrop-blur"
          data-testid="aside-friends-header"
        >
          <h2 id="aside-friends" className="text-base font-semibold">
            {tc('friends.title')}{' '}
            <span className="text-fg-subtle font-normal">
              {t('launchpad.friendsOnline', { count: online.length })}
            </span>
          </h2>
          <button
            type="button"
            data-testid="open-social-link"
            onClick={() => void openSocial({ tab: 'friends' })}
            className="rx-focus-ring text-fg-muted hover:text-fg flex items-center gap-1 rounded text-sm"
          >
            {t('launchpad.openSocial')}
            <ExternalLink className="size-3.5" />
          </button>
        </div>
        <div className="flex flex-wrap gap-2">
          {isLoading &&
            Array.from({ length: 8 }, (_, i) => <Skeleton key={i} className="size-9 rounded-lg" />)}
          {online.map((f) => (
            <Tooltip key={f.id} content={`${f.display_name} · ${tc(`presence.${f.presence}`)}`}>
              <button
                type="button"
                className="rx-focus-ring rounded-[30%]"
                aria-label={f.display_name}
                onClick={() => void openSocial({ tab: 'chats', chatWith: f.id })}
              >
                <Avatar
                  name={f.display_name}
                  size="sm"
                  presence={f.presence}
                  presenceLabel={tc(`presence.${f.presence}`)}
                />
              </button>
            </Tooltip>
          ))}
        </div>
        {playing.length > 0 && (
          <ul className="flex flex-col gap-1" aria-label={t('friends.title')}>
            {playing.map((f) => (
              <li
                key={f.id}
                className="hover:bg-surface-2 flex items-center gap-3 rounded-lg px-2 py-2"
              >
                <Avatar
                  name={f.display_name}
                  size="sm"
                  presence={f.presence}
                  presenceLabel={tc(`presence.${f.presence}`)}
                />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-sm font-medium">{f.display_name}</span>
                  <span className="text-fg-subtle truncate text-xs">
                    {activityText(f, tc as unknown as LooseT, i18n.language)}
                  </span>
                </span>
                {f.activity?.joinable && f.activity.server_address && (
                  <IconButton
                    size="sm"
                    label={tc('friends.joinAria', { name: f.display_name })}
                    icon={<LogIn />}
                    className="bg-accent-soft text-accent-text hover:bg-accent/25 hover:text-accent-text"
                    onClick={() =>
                      join(
                        f.activity!.server_address!,
                        f.activity!.server_name ?? f.activity!.server_address!,
                      )
                    }
                  />
                )}
              </li>
            ))}
          </ul>
        )}
        {playing.length > 0 ? (
          <div className="text-fg-subtle mt-auto flex flex-col items-center gap-2 py-6 text-center text-xs">
            <Users className="size-8 opacity-40" aria-hidden />
            {t('launchpad.socialHint')}
          </div>
        ) : (
          // Nothing going on: the hint stays in the middle of the visible panel while the page
          // scrolls (sticky against the scroll area's half height, held inside the aside).
          <div className="relative flex-1">
            <div
              className="text-fg-subtle sticky top-[calc(50%-3rem)] flex flex-col items-center gap-2 py-6 text-center text-xs"
              data-testid="aside-empty"
            >
              <Users className="size-8 opacity-40" aria-hidden />
              {friends && online.length === 0 && (
                <p className="text-fg-muted text-sm">{t('launchpad.noFriendsOnline')}</p>
              )}
              <p className="max-w-60">{t('launchpad.socialHint')}</p>
            </div>
          </div>
        )}
      </section>
    </aside>
  );
}
