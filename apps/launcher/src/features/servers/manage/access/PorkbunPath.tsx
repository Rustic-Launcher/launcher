import { Badge, Button, Field, Input, Select, Skeleton, toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, Unplug, Wand2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { DomainBinding } from '@/ipc/types/dns';
import type { Server } from '@/ipc/types/servers';
import { fqdnOf, normalizeSubdomain } from '../dnsLogic';
import { mk, usePorkbun } from '../queries';
import { PorkbunConnect } from './PorkbunConnect';
import { RecordsTable } from './RecordsTable';
import { VerificationPanel } from './VerificationPanel';

/** Connected-account header with a disconnect action. */
export function PorkbunAccount() {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const { data } = usePorkbun();
  const disconnect = useMutation({
    mutationFn: () => invoke('dns_disconnect', { provider: 'porkbun' }),
    onSuccess: (c) => qc.setQueryData(mk.porkbun, c),
  });
  return (
    <div className="border-border bg-surface-2/60 flex flex-wrap items-center gap-3 rounded-xl border px-4 py-2.5">
      <CheckCircle2 className="text-success-text size-4" aria-hidden />
      <span className="text-fg text-sm">{t('domains.porkbun.connected')}</span>
      <code className="text-fg-subtle font-mono text-xs">{data?.account_label}</code>
      <Button
        variant="ghost"
        size="sm"
        leftIcon={<Unplug />}
        className="ml-auto"
        loading={disconnect.isPending}
        onClick={() => disconnect.mutate()}
      >
        {t('domains.porkbun.disconnect')}
      </Button>
    </div>
  );
}

/** Applies records automatically on the user's Porkbun domain, then shows the live status. */
export function useConfigureDomain(serverId: string) {
  const qc = useQueryClient();
  const { t } = useTranslation('servers');
  return useMutation({
    mutationFn: (v: { domain: string; subdomain: string | null }) =>
      invoke('dns_configure_server', {
        server_id: serverId,
        domain: v.domain,
        subdomain: v.subdomain,
      }),
    onSuccess: (b) => qc.setQueryData<DomainBinding | null>(mk.domain(serverId), b),
    onError: (e) =>
      toast({ tone: 'error', title: t('domains.configureFailed'), description: e.message }),
  });
}

export function PorkbunPath({ server, onDone }: { server: Server; onDone: () => void }) {
  const { t } = useTranslation('servers');
  const { data: conn, isLoading } = usePorkbun();
  const [domain, setDomain] = useState<string>('');
  const [sub, setSub] = useState('play');
  const domains = useQuery({
    queryKey: mk.ownedDomains,
    queryFn: () => invoke('dns_list_domains', { provider: 'porkbun' }),
    enabled: !!conn?.connected,
  });
  const subdomain = normalizeSubdomain(sub);
  const valid = !!domain && subdomain !== null;
  const preview = useQuery({
    queryKey: ['dns', 'required', server.id, domain, subdomain ?? ''],
    queryFn: () =>
      invoke('dns_required_records', {
        server_id: server.id,
        domain,
        subdomain: subdomain ?? null,
      }),
    enabled: valid,
  });
  const configure = useConfigureDomain(server.id);
  const applied = configure.data;

  if (isLoading) return <Skeleton className="h-48 rounded-xl" />;

  return (
    <div className="flex flex-col gap-4" data-testid="porkbun-path">
      <h4 className="text-base font-semibold">{t('domains.paths.porkbun.title')}</h4>
      {!conn?.connected ? (
        <PorkbunConnect />
      ) : applied ? (
        <>
          <PorkbunAccount />
          <VerificationPanel
            target={{ server_id: server.id, domain: applied.domain, subdomain: applied.subdomain }}
            records={applied.records}
          />
          <Button variant="secondary" className="w-fit" onClick={onDone}>
            {t('manage.done')}
          </Button>
        </>
      ) : (
        <>
          <PorkbunAccount />
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
            <Field
              label={t('domains.porkbun.domain')}
              hint={
                domains.data?.some((d) => !d.api_access)
                  ? t('domains.porkbun.apiAccessHint')
                  : undefined
              }
            >
              {(p) =>
                domains.isLoading ? (
                  <Skeleton className="h-9" />
                ) : (
                  <Select
                    id={p.id}
                    value={domain || undefined}
                    onValueChange={setDomain}
                    placeholder={t('domains.porkbun.pickDomain')}
                    options={(domains.data ?? []).map((d) => ({
                      value: d.domain,
                      label: d.domain,
                      disabled: !d.api_access,
                      description: d.api_access ? undefined : t('domains.porkbun.apiAccessOff'),
                    }))}
                  />
                )
              }
            </Field>
            <Field
              label={t('domains.subdomain')}
              hint={subdomain === undefined ? t('domains.apexHint') : t('domains.subdomainHint')}
              error={subdomain === null ? t('domains.invalidSubdomain') : undefined}
            >
              {(p) => (
                <Input
                  {...p}
                  value={sub}
                  onChange={(e) => setSub(e.target.value)}
                  placeholder="play"
                  className="font-mono"
                />
              )}
            </Field>
          </div>
          {valid && (
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-2">
                <h5 className="text-sm font-semibold">{t('domains.porkbun.preview')}</h5>
                <Badge variant="info">{t('domains.porkbun.dryRun')}</Badge>
              </div>
              <p className="text-fg-muted text-xs">
                {t('domains.porkbun.previewBody', { fqdn: fqdnOf(domain, subdomain) })}
              </p>
              {preview.isLoading && <Skeleton className="h-32 rounded-xl" />}
              {preview.data && <RecordsTable records={preview.data.records} compact />}
            </div>
          )}
          <Button
            variant="primary"
            leftIcon={<Wand2 />}
            className="w-fit"
            disabled={!valid || !preview.data}
            loading={configure.isPending}
            onClick={() => configure.mutate({ domain, subdomain: subdomain ?? null })}
            data-testid="porkbun-apply"
          >
            {t('domains.porkbun.apply')}
          </Button>
        </>
      )}
    </div>
  );
}
