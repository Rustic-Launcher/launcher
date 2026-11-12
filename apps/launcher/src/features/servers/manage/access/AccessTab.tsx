import { Badge, Button, Skeleton, Switch, toast } from '@rustic/ui';
import { Cable, Globe2, KeyRound, Network, Radio, ShieldAlert, Ticket } from 'lucide-react';
import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { Server, ServerAccess } from '@/ipc/types/servers';
import { RELAY_DOMAIN } from '@/lib/relay';
import { formatRelative } from '@/lib/format';
import { useAccess, useAccessMutation } from '../queries';
import { Collapsible, CopyButton, CopyField, Panel } from '../shared';
import { DomainSection } from './DomainSection';

/** Access: LAN / public IP, optional temporary relay address, invite code, domains, UPnP. */
export function AccessTab({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const { data: access, isLoading, isError, refetch } = useAccess(server.id);

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        {Array.from({ length: 4 }, (_, i) => (
          <Skeleton key={i} className="h-40 rounded-2xl" />
        ))}
      </div>
    );
  }
  if (isError || !access) return <QueryError onRetry={() => void refetch()} />;

  return (
    <div className="flex flex-col gap-4">
      <p className="text-fg-muted max-w-3xl text-sm">{t('access.intro')}</p>
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
        <LanCard server={server} access={access} />
        <RelayCard server={server} access={access} />
        <InviteCard server={server} access={access} />
      </div>
      <DomainSection server={server} />
      {server.hosting.mode === 'local' && <UpnpSection server={server} access={access} />}
    </div>
  );
}

function LanCard({ server, access }: { server: Server; access: ServerAccess }) {
  const { t } = useTranslation('servers');
  const vps = server.hosting.mode === 'vps';
  return (
    <Panel
      title={
        <span className="flex items-center gap-2">
          <Network className="text-accent-text size-4" aria-hidden />
          {vps ? t('access.public.title') : t('access.lan.title')}
        </span>
      }
      description={vps ? t('access.public.body') : t('access.lan.body')}
    >
      <CopyField value={access.lan.address} label={t('access.lan.copy')} testId="lan-address" />
    </Panel>
  );
}

function RelayCard({ server, access }: { server: Server; access: ServerAccess }) {
  const { t } = useTranslation('servers');
  const relay = access.relay;
  const [label, setLabel] = useState(relay.label ?? '');
  const inputId = useId();
  useEffect(() => setLabel(relay.label ?? ''), [relay.label]);
  const mutate = useAccessMutation(server.id, (v: { enabled: boolean; label?: string | null }) =>
    invoke('server_relay_set', { server_id: server.id, ...v }),
  );
  const onError = (e: Error) =>
    toast({
      tone: 'error',
      title: t('access.relay.failed'),
      description:
        e.message === 'label taken'
          ? t('access.relay.taken')
          : e.message === 'label'
            ? t('access.relay.invalid')
            : e.message,
    });
  const labelValid = /^[a-z0-9](?:[a-z0-9-]{1,30}[a-z0-9])$/.test(label);

  return (
    <Panel
      testId="relay-card"
      title={
        <span className="flex items-center gap-2">
          <Radio className="text-accent-text size-4" aria-hidden />
          {t('access.relay.title')}
          <Badge variant="neutral">{t('access.optional')}</Badge>
        </span>
      }
      description={t('access.relay.body')}
      action={
        <Switch
          checked={relay.enabled}
          disabled={mutate.isPending}
          onCheckedChange={(v) => mutate.mutate({ enabled: v, label: label || null }, { onError })}
          aria-label={t('access.relay.toggle')}
          data-testid="relay-toggle"
        />
      }
    >
      {relay.enabled && relay.hostname ? (
        <div className="flex flex-col gap-2">
          <CopyField
            value={relay.hostname}
            label={t('access.relay.copy')}
            testId="relay-hostname"
          />
          <p className="text-fg-muted flex items-center gap-1.5 text-xs">
            <span
              className={
                relay.status === 'online'
                  ? 'bg-success size-2 rounded-full'
                  : 'bg-warning size-2 rounded-full'
              }
              aria-hidden
            />
            {t(`access.relay.status.${relay.status}`)}
          </p>
        </div>
      ) : (
        <p className="text-fg-subtle text-xs">{t('access.relay.off')}</p>
      )}
      <form
        className="mt-3 flex flex-col gap-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          if (labelValid) mutate.mutate({ enabled: true, label }, { onError });
        }}
      >
        <label htmlFor={inputId} className="text-fg-muted text-xs font-medium">
          {t('access.relay.label')}
        </label>
        <div className="flex items-center gap-2">
          <div className="border-border-strong/50 bg-surface-2 focus-within:border-accent flex min-w-0 flex-1 items-center rounded-md border pr-2">
            <input
              id={inputId}
              value={label}
              onChange={(e) => setLabel(e.target.value.toLowerCase())}
              placeholder="mon-serveur"
              className="text-fg h-8 min-w-0 flex-1 bg-transparent px-2 font-mono text-xs outline-none"
              aria-invalid={label !== '' && !labelValid}
              data-testid="relay-label"
            />
          </div>
          <Button
            type="submit"
            size="sm"
            variant="secondary"
            disabled={!labelValid || label === relay.label}
            loading={mutate.isPending}
          >
            {t('access.relay.apply')}
          </Button>
        </div>
        <p className="text-2xs text-fg-subtle truncate font-mono" aria-live="polite">
          {(label || 'mon-serveur') + '.' + RELAY_DOMAIN}
        </p>
      </form>
    </Panel>
  );
}

function InviteCard({ server, access }: { server: Server; access: ServerAccess }) {
  const { t, i18n } = useTranslation('servers');
  const create = useAccessMutation(server.id, () =>
    invoke('server_invite_create', { server_id: server.id }),
  );
  const revoke = useAccessMutation(server.id, () =>
    invoke('server_invite_revoke', { server_id: server.id }),
  );
  const code = access.invite.code;
  return (
    <Panel
      title={
        <span className="flex items-center gap-2">
          <Ticket className="text-accent-text size-4" aria-hidden />
          {t('access.invite.title')}
        </span>
      }
      description={t('access.invite.body')}
    >
      {code ? (
        <div className="flex flex-col gap-2">
          <div className="border-border bg-surface-2 flex items-center gap-2 rounded-md border py-1 pr-1 pl-3">
            <code
              className="text-fg flex-1 font-mono text-base font-semibold tracking-wider"
              data-testid="invite-code"
            >
              {code}
            </code>
            <CopyButton value={code} label={t('access.invite.copy')} />
          </div>
          <div className="text-fg-subtle flex items-center justify-between text-xs">
            <span>
              {access.invite.expires_at &&
                t('access.invite.expires', {
                  when: formatRelative(access.invite.expires_at, i18n.language),
                })}
            </span>
            <Button variant="link" size="sm" onClick={() => revoke.mutate(undefined)}>
              {t('access.invite.revoke')}
            </Button>
          </div>
        </div>
      ) : (
        <Button
          variant="secondary"
          leftIcon={<KeyRound />}
          loading={create.isPending}
          onClick={() => create.mutate(undefined)}
        >
          {t('access.invite.create')}
        </Button>
      )}
    </Panel>
  );
}

function UpnpSection({ server, access }: { server: Server; access: ServerAccess }) {
  const { t } = useTranslation('servers');
  const mutate = useAccessMutation(server.id, (enabled: boolean) =>
    invoke('server_upnp_set', { server_id: server.id, enabled }),
  );
  return (
    <Collapsible title={t('access.advanced')} icon={<Cable />}>
      <div className="flex flex-col gap-3">
        <div
          className="border-warning/30 bg-warning/10 text-warning-text flex items-start gap-3 rounded-xl border p-3 text-sm"
          role="note"
        >
          <ShieldAlert className="mt-0.5 size-4 shrink-0" aria-hidden />
          <p>{t('access.upnp.warning')}</p>
        </div>
        <div className="flex items-center justify-between gap-4">
          <div className="flex flex-col gap-0.5">
            <span className="text-fg text-sm font-medium">{t('access.upnp.title')}</span>
            <span className="text-fg-muted text-xs">
              {access.upnp.status === 'mapped' && access.upnp.external_address
                ? t('access.upnp.mapped', { address: access.upnp.external_address })
                : t(`access.upnp.status.${access.upnp.status}`)}
            </span>
          </div>
          <Switch
            checked={access.upnp.enabled}
            disabled={mutate.isPending}
            onCheckedChange={(v) =>
              mutate.mutate(v, {
                onError: (e) =>
                  toast({ tone: 'error', title: t('access.upnp.failed'), description: e.message }),
              })
            }
            aria-label={t('access.upnp.title')}
          />
        </div>
        {access.upnp.external_address && (
          <CopyField
            value={access.upnp.external_address}
            label={t('access.upnp.copy')}
            className="max-w-sm"
          />
        )}
        <p className="text-fg-subtle flex items-center gap-1.5 text-xs">
          <Globe2 className="size-3.5" aria-hidden />
          {t('access.upnp.prefer')}
        </p>
      </div>
    </Collapsible>
  );
}
