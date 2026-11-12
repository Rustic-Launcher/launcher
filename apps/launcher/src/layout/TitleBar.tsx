import {
  Avatar,
  Badge,
  Button,
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownSeparator,
  DropdownTrigger,
  formatBytes,
  IconButton,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Progress,
  Skeleton,
} from '@rustic/ui';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate, useRouter, useRouterState } from '@tanstack/react-router';
import {
  Bell,
  BookOpen,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ExternalLink,
  LogOut,
  Plus,
  Search,
  Settings,
  Square,
  UserRound,
  Users,
  X,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import { cloud } from '@/cloud/client';
import type { AppNotification } from '@/cloud/types';
import { InstanceIcon } from '@/components/InstanceIcon';
import { LEGAL_URLS, siteUrlFor } from '@/features/settings/legal';
import { openSocial } from '@/features/social/openSocial';
import { invoke, isNative } from '@/ipc';
import { formatPercent, formatRelative, formatSpeed } from '@/lib/format';
import { qk, useAccounts, useInstances, useRusticSession } from '@/queries/core';
import { sk, useNotifications, useSocialUnread } from '@/queries/social';
import { summarizeDownloads, useActivity } from '@/stores/activity';
import { useUi } from '@/stores/ui';
import { RusticMark, RusticWordmark } from './Logo';
import { WindowControls } from './WindowControls';
import { AccountAvatar } from '@/components/AccountAvatar';

/**
 * Custom title bar (Tauri `decorations: false`): product switcher, history, breadcrumb, then
 * search, notifications, social, account pill, running-instances pill and window controls.
 * Empty areas are drag regions (`data-tauri-drag-region`).
 */
export function TitleBar() {
  const { t } = useTranslation();
  const router = useRouter();
  const setCommandOpen = useUi((s) => s.setCommandOpen);
  const unread = useSocialUnread();
  const drag = isNative() ? true : undefined;

  return (
    <header
      data-tauri-drag-region={drag}
      className="z-20 col-span-2 row-start-1 flex items-center gap-2 pr-2 pl-3 select-none"
    >
      <ProductSwitcher />
      <div className="ml-1 flex items-center">
        <IconButton
          size="sm"
          label={t('actions.back')}
          icon={<ChevronLeft />}
          onClick={() => router.history.back()}
        />
        <IconButton
          size="sm"
          label={t('actions.forward')}
          icon={<ChevronRight />}
          onClick={() => router.history.forward()}
        />
      </div>
      <Breadcrumb />
      <div className="h-full flex-1" data-tauri-drag-region={drag} />
      <div className="flex items-center gap-1.5">
        <IconButton
          size="sm"
          variant="secondary"
          label={t('search.title')}
          shortcut="Ctrl K"
          icon={<Search />}
          onClick={() => setCommandOpen(true)}
          data-testid="global-search"
        />
        <NotificationsButton />
        <IconButton
          size="sm"
          variant="secondary"
          label={t('social.open')}
          icon={<Users />}
          badge={unread || undefined}
          onClick={() => void openSocial()}
          data-testid="open-social"
        />
        <AccountPill />
        <RunningPill />
      </div>
      <WindowControls className="ml-1" />
    </header>
  );
}

function ProductSwitcher() {
  const { t } = useTranslation();
  return (
    <Dropdown>
      <DropdownTrigger asChild>
        <button
          type="button"
          className="rx-focus-ring hover:bg-surface-2 flex h-8 items-center gap-2 rounded-md px-1.5 text-sm font-semibold"
          aria-label={t('product.switcher')}
          data-testid="product-switcher"
        >
          <RusticWordmark height={17} title={t('product.name')} />
          <ChevronDown className="text-fg-subtle size-3.5" />
        </button>
      </DropdownTrigger>
      <DropdownContent align="start" className="w-64">
        <DropdownLabel>{t('product.label')}</DropdownLabel>
        <DropdownItem icon={<RusticMark size={16} />}>
          <span className="flex items-center justify-between gap-2">
            {t('product.launcher')}
            <Check className="text-accent-text size-4" />
          </span>
        </DropdownItem>
        <DropdownItem
          icon={<BookOpen />}
          onSelect={() => void invoke('app_open_url', { url: LEGAL_URLS.serverDocs })}
        >
          <span className="flex items-center justify-between gap-2">
            {t('product.serverDocs')}
            <ExternalLink className="size-3.5" />
          </span>
        </DropdownItem>
      </DropdownContent>
    </Dropdown>
  );
}

/** Route → breadcrumb segments ("Versions / Horizon", "Explore / Mods", …). */
function Breadcrumb() {
  const { t } = useTranslation();
  const loc = useRouterState({ select: (s) => s.location });
  const { data: instances } = useInstances();
  const servers = useQuery({
    queryKey: ['servers'],
    queryFn: () => invoke('servers_list'),
    retry: false,
  });
  const parts = loc.pathname.split('/').filter(Boolean);
  const search = loc.search as Record<string, string | undefined>;
  const crumbs: string[] = [];
  switch (parts[0]) {
    case undefined:
      crumbs.push(t('nav.home'));
      break;
    case 'instances':
      crumbs.push(t('nav.instances'));
      crumbs.push(
        parts[1] ? (instances?.find((i) => i.id === parts[1])?.name ?? '…') : t('crumbs.horizon'),
      );
      break;
    case 'content':
      crumbs.push(t('nav.content'));
      crumbs.push(
        parts[1]
          ? t('crumbs.details')
          : t(`crumbs.kinds.${search.kind ?? 'mod'}` as 'crumbs.kinds.mod'),
      );
      break;
    case 'servers':
      crumbs.push(t('nav.servers'));
      if (parts[1] === 'new') crumbs.push(t('crumbs.create'));
      else if (parts[1]) crumbs.push(servers.data?.find((s) => s.id === parts[1])?.name ?? '…');
      else
        crumbs.push(t(`crumbs.servers.${search.tab ?? 'featured'}` as 'crumbs.servers.featured'));
      break;
    case 'locker':
      crumbs.push(t('nav.locker'));
      break;
    case 'settings':
      crumbs.push(t('nav.settings'));
      if (search.section)
        crumbs.push(t(`search.settings.${search.section}` as 'search.settings.game'));
      break;
  }
  return (
    <nav aria-label={t('crumbs.label')} className="min-w-0" data-testid="breadcrumb">
      <ol className="text-fg-muted flex items-center gap-1.5 text-sm">
        {crumbs.map((c, i) => (
          <li key={i} className="flex min-w-0 items-center gap-1.5">
            {i > 0 && (
              <span aria-hidden className="text-fg-subtle">
                /
              </span>
            )}
            <span
              className={cn('truncate', i === crumbs.length - 1 && 'text-fg')}
              aria-current={i === crumbs.length - 1 ? 'page' : undefined}
            >
              {c}
            </span>
          </li>
        ))}
      </ol>
    </nav>
  );
}

function NotificationsButton() {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const { data, isLoading } = useNotifications();
  const unread = data?.filter((n) => !n.read) ?? [];
  // Store notifications point at the website's store (the launcher has no store page).
  const openNotification = (href: string) => {
    const site = siteUrlFor(href);
    if (site) void invoke('app_open_url', { url: site });
    else void navigate({ to: href });
  };

  const markAll = async () => {
    await (await cloud()).markNotificationsRead(unread.map((n) => n.id));
    qc.setQueryData<AppNotification[]>(sk.notifications, (old) =>
      old?.map((n) => ({ ...n, read: true })),
    );
  };

  return (
    <Popover>
      <PopoverTrigger asChild>
        <IconButton
          size="sm"
          variant="secondary"
          label={t('notifications.title')}
          icon={<Bell />}
          badge={unread.length || undefined}
        />
      </PopoverTrigger>
      <PopoverContent className="w-96">
        <div className="border-border flex items-center justify-between border-b px-4 py-3">
          <h2 className="text-sm font-semibold">{t('notifications.title')}</h2>
          {unread.length > 0 && (
            <Button variant="link" size="sm" onClick={() => void markAll()}>
              {t('notifications.markAllRead')}
            </Button>
          )}
        </div>
        <ul className="max-h-96 overflow-y-auto p-1.5">
          {isLoading &&
            Array.from({ length: 3 }, (_, i) => (
              <li key={i} className="p-2">
                <Skeleton className="h-10" />
              </li>
            ))}
          {data?.length === 0 && (
            <li className="text-fg-muted px-3 py-8 text-center text-sm">
              {t('notifications.empty')}
            </li>
          )}
          {data?.map((n) => (
            <li key={n.id}>
              <button
                type="button"
                onClick={() => n.href && openNotification(n.href)}
                className="rx-focus-ring hover:bg-surface-3 flex w-full items-start gap-3 rounded-lg px-3 py-2.5 text-left"
              >
                <span
                  className={cn(
                    'mt-1.5 size-2 shrink-0 rounded-full',
                    n.read ? 'bg-transparent' : 'bg-accent',
                  )}
                />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="text-fg truncate text-sm font-medium">{n.title}</span>
                  <span className="text-fg-muted text-xs">
                    {t(`notifications.kinds.${n.body as 'friend_request'}`, {
                      defaultValue: n.body,
                    })}
                  </span>
                </span>
                <span className="text-fg-subtle text-2xs shrink-0">
                  {formatRelative(n.created_at, i18n.language)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}

function AccountPill() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const { data: accounts, isLoading } = useAccounts();
  const { data: rustic } = useRusticSession();
  const active = accounts?.find((a) => a.active);

  if (isLoading) return <Skeleton className="h-8 w-32 rounded-lg" />;

  return (
    <Dropdown>
      <DropdownTrigger asChild>
        <button
          type="button"
          data-testid="account-switcher"
          aria-label={t('account.switcher')}
          className="rx-focus-ring border-accent/40 bg-accent-soft hover:bg-accent/20 flex h-8 items-center gap-2 rounded-lg border py-1 pr-2 pl-1"
        >
          {active ? <AccountAvatar account={active} size="xs" /> : <Avatar name="?" size="xs" />}
          <span className="max-w-32 truncate text-sm font-semibold">
            {active?.username ?? t('account.none')}
          </span>
          <ChevronDown className="text-fg-muted size-3.5" />
        </button>
      </DropdownTrigger>
      <DropdownContent className="w-72">
        <DropdownLabel>{t('account.rustic')}</DropdownLabel>
        {rustic?.signed_in ? (
          <div className="flex items-center gap-2.5 px-2.5 py-2">
            <Avatar name={rustic.username ?? ''} size="sm" />
            <div className="flex min-w-0 flex-col">
              <span className="truncate text-sm font-medium">{rustic.username}</span>
              <span className="text-fg-subtle truncate text-xs">{rustic.email}</span>
            </div>
          </div>
        ) : (
          <DropdownItem
            icon={<UserRound />}
            onSelect={() => void navigate({ to: '/settings', search: { section: 'accounts' } })}
          >
            {t('account.signInRustic')}
          </DropdownItem>
        )}
        <DropdownSeparator />
        <DropdownLabel>{t('account.gameProfiles')}</DropdownLabel>
        {accounts?.map((a) => (
          <DropdownItem
            key={a.id}
            icon={<AccountAvatar account={a} size="xs" />}
            onSelect={() => {
              void invoke('auth_set_active', { account_id: a.id }).then((list) =>
                qc.setQueryData(qk.accounts, list),
              );
            }}
          >
            <span className="flex items-center gap-2">
              {a.username}
              <Badge variant={a.kind === 'microsoft' ? 'info' : 'neutral'}>
                {t(`account.kind.${a.kind}`)}
              </Badge>
              {a.active && <Check className="text-accent-text ml-auto size-4" />}
            </span>
          </DropdownItem>
        ))}
        <DropdownItem
          icon={<Plus />}
          onSelect={() => void navigate({ to: '/settings', search: { section: 'accounts' } })}
        >
          {t('account.add')}
        </DropdownItem>
        <DropdownSeparator />
        <DropdownItem
          icon={<Settings />}
          onSelect={() => void navigate({ to: '/settings', search: { section: 'accounts' } })}
        >
          {t('account.manage')}
        </DropdownItem>
        {rustic?.signed_in && (
          <DropdownItem
            icon={<LogOut />}
            destructive
            onSelect={() =>
              void invoke('auth_rustic_logout').then((s) => qc.setQueryData(qk.rustic, s))
            }
          >
            {t('account.signOutRustic')}
          </DropdownItem>
        )}
      </DropdownContent>
    </Dropdown>
  );
}

/** "N instances running" pill: running games and background downloads. */
function RunningPill() {
  const { t, i18n } = useTranslation();
  const { data: instances } = useInstances();
  const launches = useActivity(useShallow((s) => Object.values(s.launches)));
  const tasks = useActivity(useShallow((s) => Object.values(s.downloads)));
  const running = launches.filter((l) => l.phase === 'running');
  const sum = summarizeDownloads(tasks);
  const busy = sum.count > 0;

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          data-testid="running-pill"
          className="rx-focus-ring bg-surface-2 border-border hover:bg-surface-3 relative flex h-8 items-center gap-2 overflow-hidden rounded-lg border px-2.5 text-xs font-medium"
        >
          <span
            className={cn(
              'size-1.5 rounded-full',
              running.length
                ? 'bg-success'
                : busy
                  ? 'bg-accent animate-pulse-soft'
                  : 'bg-fg-subtle',
            )}
          />
          {busy && running.length === 0
            ? t('running.downloading', {
                pct: sum.progress != null ? formatPercent(sum.progress, i18n.language) : '…',
              })
            : t('running.count', { count: running.length })}
          {busy && (
            <span className="absolute inset-x-0 bottom-0" data-testid="global-progress">
              <Progress
                value={sum.progress}
                size="xs"
                className="rounded-none bg-transparent"
                label={t('downloads.global')}
              />
            </span>
          )}
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-96 p-4">
        <h2 className="mb-2 text-sm font-semibold">{t('running.title')}</h2>
        {running.length === 0 && <p className="text-fg-muted text-sm">{t('running.none')}</p>}
        <ul className="flex flex-col gap-1">
          {running.map((l) => {
            const inst = instances?.find((i) => i.id === l.instance_id);
            return (
              <li key={l.instance_id} className="flex items-center gap-3 rounded-lg py-1.5">
                {inst && <InstanceIcon icon={inst.icon} size="sm" />}
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-sm font-medium">
                    {inst?.name ?? l.instance_id}
                  </span>
                  {l.started_at && (
                    <span className="text-fg-subtle text-xs">
                      {formatRelative(l.started_at, i18n.language)}
                    </span>
                  )}
                </span>
                <Button
                  size="sm"
                  leftIcon={<Square />}
                  onClick={() => void invoke('instance_stop', { instance_id: l.instance_id })}
                >
                  {t('play.stop')}
                </Button>
              </li>
            );
          })}
        </ul>
        {tasks.length > 0 && (
          <>
            <h3 className="mt-4 mb-2 text-sm font-semibold">{t('downloads.title')}</h3>
            <ul className="flex flex-col gap-3">
              {tasks.map((d) => {
                const p = d.total_bytes ? d.received_bytes / d.total_bytes : null;
                return (
                  <li key={d.id} className="flex flex-col gap-1.5">
                    <div className="flex items-center justify-between gap-2 text-sm">
                      <span className="truncate font-medium">
                        {t(`downloads.kind.${d.kind}`)} ·{' '}
                        {d.label === 'prepare' ? t('downloads.kind.prepare') : d.label}
                      </span>
                      <IconButton
                        size="sm"
                        label={t('actions.cancel')}
                        icon={<X />}
                        onClick={() => void invoke('download_cancel', { id: d.id })}
                      />
                    </div>
                    <Progress value={p} label={d.label} size="xs" />
                    <div className="text-fg-subtle text-2xs flex justify-between tabular-nums">
                      <span>
                        {d.total_bytes
                          ? `${formatBytes(d.received_bytes, i18n.language)} / ${formatBytes(d.total_bytes, i18n.language)}`
                          : t('downloads.working')}
                      </span>
                      <span>
                        {p != null && formatPercent(p, i18n.language)}{' '}
                        {d.speed_bps > 0 && `· ${formatSpeed(d.speed_bps, i18n.language)}`}
                      </span>
                    </div>
                  </li>
                );
              })}
            </ul>
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}
