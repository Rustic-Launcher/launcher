import { RusticAuthForm } from '../RusticAuthForm';
import { Avatar, Badge, Button, MicrosoftLogo, Skeleton, toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Check, ExternalLink, Info, Lock, LogOut, RefreshCw, UserPlus } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { Account } from '@/ipc/types/auth';
import { formatDate, formatRelative } from '@/lib/format';
import { qk, useAccounts, useRusticSession } from '@/queries/core';
import { SettingsCard } from '../components';
import { MicrosoftLogin } from '../MicrosoftLogin';
import { OFFLINE_GRACE_DAYS, WEB_BASE } from '../legal';
import { ConnectionsCard } from './ConnectionsCard';
import { AccountAvatar } from '@/components/AccountAvatar';

function useAccountActions() {
  const qc = useQueryClient();
  const set = (list: Account[]) => qc.setQueryData(qk.accounts, list);
  return {
    setActive: useMutation({
      mutationFn: (id: string) => invoke('auth_set_active', { account_id: id }),
      onSuccess: set,
    }),
    logout: useMutation({
      mutationFn: (id: string) => invoke('auth_logout', { account_id: id }),
      onSuccess: set,
    }),
    check: useMutation({
      mutationFn: (id: string) => invoke('auth_ownership_check', { account_id: id }),
      onSuccess: () => void qc.invalidateQueries({ queryKey: qk.accounts }),
    }),
  };
}

export function AccountsSection() {
  const { data: accounts, isLoading, isError, refetch } = useAccounts();
  if (isLoading) return <Skeleton className="h-64 rounded-2xl" />;
  if (isError || !accounts) return <QueryError onRetry={() => void refetch()} />;
  const verified = (a: Account) => !!a.ownership?.owns_game && a.ownership.status === 'valid';
  // The account that plays offline: the active one if it owns the game, else another owner.
  const owner = accounts.find((a) => a.active && verified(a)) ?? accounts.find(verified);
  return (
    <div className="flex flex-col gap-4">
      <MicrosoftAccounts accounts={accounts} />
      <RusticAccount />
      <OfflinePlay owner={owner} />
      <ConnectionsCard />
    </div>
  );
}

function OwnershipBadge({ a }: { a: Account }) {
  const { t } = useTranslation('settings');
  const o = a.ownership;
  if (!o) return null;
  if (o.owns_game && o.status === 'valid')
    return (
      <Badge variant="success" icon={<Check />}>
        {t('accounts.ownership.valid')}
      </Badge>
    );
  if (o.status === 'expired')
    return <Badge variant="warning">{t('accounts.ownership.expired')}</Badge>;
  if (o.status === 'never_checked')
    return <Badge variant="neutral">{t('accounts.ownership.never')}</Badge>;
  return <Badge variant="danger">{t('accounts.ownership.failed')}</Badge>;
}

function MicrosoftAccounts({ accounts }: { accounts: Account[] }) {
  const { t, i18n } = useTranslation('settings');
  const { setActive, logout, check } = useAccountActions();
  const [adding, setAdding] = useState(accounts.length === 0);
  return (
    <SettingsCard
      testId="settings-microsoft"
      title={t('accounts.microsoft.title')}
      description={t('accounts.microsoft.description')}
      action={
        !adding && (
          <Button size="sm" leftIcon={<UserPlus />} onClick={() => setAdding(true)}>
            {t('accounts.microsoft.add')}
          </Button>
        )
      }
    >
      <ul className="flex flex-col gap-2">
        {accounts.map((a) => (
          <li
            key={a.id}
            className="border-border bg-surface-2/50 flex flex-wrap items-center gap-3 rounded-xl border p-3"
          >
            <AccountAvatar account={a} size="md" />
            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <span className="flex flex-wrap items-center gap-2 text-sm font-semibold">
                {a.username}
                {a.active && <Badge variant="accent">{t('accounts.active')}</Badge>}
                <OwnershipBadge a={a} />
              </span>
              <span className="text-fg-subtle text-xs">
                {a.ownership?.checked_at
                  ? t('accounts.ownership.lastChecked', {
                      when: formatRelative(a.ownership.checked_at, i18n.language),
                    })
                  : t('accounts.ownership.never')}
                {a.ownership?.next_check_at &&
                  ` · ${t('accounts.ownership.nextCheck', { date: formatDate(a.ownership.next_check_at, i18n.language) })}`}
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<RefreshCw />}
                loading={check.isPending && check.variables === a.id}
                onClick={() =>
                  check.mutate(a.id, {
                    onSuccess: () =>
                      toast({ tone: 'success', title: t('accounts.ownership.checkedToast') }),
                    onError: () =>
                      toast({ tone: 'error', title: t('accounts.ownership.checkFailed') }),
                  })
                }
              >
                {t('accounts.ownership.checkNow')}
              </Button>
              {!a.active && (
                <Button size="sm" onClick={() => setActive.mutate(a.id)}>
                  {t('accounts.useThis')}
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<LogOut />}
                onClick={() =>
                  logout.mutate(a.id, {
                    onSuccess: () =>
                      toast({ tone: 'info', title: t('accounts.signedOut', { name: a.username }) }),
                  })
                }
              >
                {t('accounts.signOut')}
              </Button>
            </div>
          </li>
        ))}
      </ul>
      {adding && (
        <div className="mt-3">
          <MicrosoftLogin
            onSuccess={(a) => {
              toast({
                tone: a.ownership?.owns_game ? 'success' : 'warning',
                title: t('accounts.microsoft.signedInAs', { name: a.username }),
                description: a.ownership?.owns_game
                  ? t('accounts.microsoft.ownershipOk')
                  : t('accounts.microsoft.ownershipMissing'),
              });
              setAdding(false);
            }}
            onCancel={accounts.length > 0 ? () => setAdding(false) : undefined}
          />
        </div>
      )}
      <p className="text-fg-subtle mt-3 flex items-start gap-2 text-xs">
        <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        {t('accounts.microsoft.keychain')}
      </p>
    </SettingsCard>
  );
}

function RusticAccount() {
  const { t } = useTranslation('settings');
  const qc = useQueryClient();
  const { data: session, isLoading } = useRusticSession();
  const signOut = useMutation({
    mutationFn: () => invoke('auth_rustic_logout'),
    onSuccess: (s) => qc.setQueryData(qk.rustic, s),
  });
  return (
    <SettingsCard
      title={t('accounts.rustic.title')}
      description={t('accounts.rustic.description')}
      testId="settings-rustic"
    >
      {isLoading ? (
        <Skeleton className="h-14" />
      ) : session?.signed_in ? (
        <div className="flex flex-col gap-3">
          <div className="border-border bg-surface-2/50 flex flex-wrap items-center gap-3 rounded-xl border p-3">
            <Avatar name={session.username ?? '?'} size="md" shape="circle" />
            <div className="flex min-w-0 flex-1 flex-col">
              <span className="text-sm font-semibold">{session.username}</span>
              <span className="text-fg-subtle text-xs">{session.email}</span>
            </div>
            <Button
              size="sm"
              variant="ghost"
              leftIcon={<LogOut />}
              loading={signOut.isPending}
              onClick={() => signOut.mutate()}
            >
              {t('accounts.signOut')}
            </Button>
          </div>
          <LinkedSignIns />
        </div>
      ) : (
        <div className="border-border flex flex-col gap-3 rounded-xl border border-dashed p-4">
          <span className="text-fg-muted text-sm">{t('accounts.rustic.signedOut')}</span>
          <RusticAuthForm />
        </div>
      )}
      <p className="text-fg-subtle mt-3 flex items-start gap-2 text-xs">
        <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        {t('accounts.rustic.licenceNote')}
      </p>
    </SettingsCard>
  );
}

/** Sign-in methods of the Rustic account: Microsoft link (on the website) and unlink (API). */
function LinkedSignIns() {
  const { t } = useTranslation('settings');
  const qc = useQueryClient();
  const key = [...qk.rustic, 'identities'] as const;
  const { data, isLoading, isError } = useQuery({
    queryKey: key,
    queryFn: async () => (await cloud()).identities(),
  });
  const unlink = useMutation({
    mutationFn: async () => (await cloud()).unlinkIdentity('microsoft'),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: key });
      toast({ tone: 'success', title: t('accounts.linked.unlinked') });
    },
    onError: (e) => {
      const last =
        e.message === 'last_sign_in_method' ||
        (e as { code?: string }).code === 'last_sign_in_method';
      toast({
        tone: 'error',
        title: last ? t('accounts.linked.lastMethod') : t('accounts.linked.unlinkFailed'),
      });
    },
  });
  if (isLoading) return <Skeleton className="h-12" />;
  if (isError || !data) return null;
  const ms = data.identities.find((i) => i.provider === 'microsoft');
  return (
    <div
      className="border-border flex flex-wrap items-center gap-3 rounded-xl border p-3"
      data-testid="rustic-linked-microsoft"
    >
      <MicrosoftLogo size={18} />
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-sm font-semibold">{t('accounts.linked.microsoft')}</span>
        <span className="text-fg-subtle truncate text-xs">
          {ms ? (ms.email ?? t('accounts.linked.linked')) : t('accounts.linked.notLinked')}
        </span>
      </div>
      {ms ? (
        <Button
          size="sm"
          variant="ghost"
          loading={unlink.isPending}
          onClick={() => unlink.mutate()}
          data-testid="rustic-unlink-microsoft"
        >
          {t('accounts.linked.unlink')}
        </Button>
      ) : (
        <Button
          size="sm"
          leftIcon={<ExternalLink />}
          onClick={() => void invoke('app_open_url', { url: `${WEB_BASE}/account/linked` })}
          data-testid="rustic-link-microsoft"
        >
          {t('accounts.linked.link')}
        </Button>
      )}
    </div>
  );
}

/**
 * Offline play (LAN, no internet): the owner's own account, under its official name and UUID,
 * for a while after the last successful check (ADR 0146). There are no free-name profiles.
 */
function OfflinePlay({ owner }: { owner: Account | undefined }) {
  const { t, i18n } = useTranslation('settings');
  return (
    <SettingsCard
      testId="settings-offline"
      title={t('accounts.offline.title')}
      description={t('accounts.offline.description', { days: OFFLINE_GRACE_DAYS })}
    >
      {owner ? (
        <div
          className="border-border bg-surface-2/50 flex items-center gap-3 rounded-xl border p-3"
          data-testid="offline-identity"
        >
          <AccountAvatar account={owner} size="sm" />
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-sm font-medium">{owner.username}</span>
            {owner.ownership?.next_check_at && (
              <span className="text-fg-subtle text-xs">
                {t('accounts.offline.until', {
                  date: formatDate(owner.ownership.next_check_at, i18n.language),
                })}
              </span>
            )}
          </div>
        </div>
      ) : (
        <div
          className="border-warning/30 bg-warning/10 flex items-start gap-3 rounded-xl border p-3"
          role="note"
        >
          <Lock className="text-warning-text mt-0.5 size-4 shrink-0" aria-hidden />
          <p className="text-fg-muted text-sm">{t('accounts.offline.locked')}</p>
        </div>
      )}
    </SettingsCard>
  );
}
