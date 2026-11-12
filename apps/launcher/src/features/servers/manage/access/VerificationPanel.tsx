import { Button, Progress } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { motion } from 'motion/react';
import { PartyPopper, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { DnsRecord, DomainTarget } from '@/ipc/types/dns';
import { requiredOk, summarizeVerification, type Registrar } from '../dnsLogic';
import { useNow } from '../queries';
import { CopyField } from '../shared';
import { RecordsTable } from './RecordsTable';

export const VERIFY_INTERVAL_MS = 10_000;

/** Live DNS check: polls every 10 s, per-record status, final "Domain connected" state. */
export function VerificationPanel({
  target,
  records,
  registrar,
}: {
  target: DomainTarget;
  records: DnsRecord[];
  registrar?: Registrar;
}) {
  const { t } = useTranslation('servers');
  const now = useNow(1000);
  const q = useQuery({
    queryKey: ['dns', 'verify', target.server_id, target.domain, target.subdomain ?? ''],
    queryFn: () => invoke('dns_verify', target),
    refetchInterval: (query) => (query.state.data?.all_ok ? false : VERIFY_INTERVAL_MS),
    refetchIntervalInBackground: true,
    staleTime: 0,
    gcTime: 0,
  });
  const v = q.data;
  const required = records.filter((r) => !r.optional);
  const summary = summarizeVerification(
    v && { ...v, records: v.records.filter((x) => required.some((r) => r.id === x.record_id)) },
    required.length,
  );
  const connected = summary.connected || requiredOk(records, v);
  const ago = v ? Math.max(0, Math.round((now - Date.parse(v.checked_at)) / 1000)) : null;

  return (
    <div className="flex flex-col gap-3" data-testid="verification-panel">
      {connected ? (
        <motion.div
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.25 }}
          role="status"
          className="border-success/30 bg-success/10 flex flex-col gap-3 rounded-xl border p-4"
          data-testid="domain-connected"
        >
          <span className="text-success-text flex items-center gap-2 text-base font-semibold">
            <PartyPopper className="size-5" aria-hidden />
            {t('domains.connected')}
          </span>
          <p className="text-fg-muted text-sm">{t('domains.connectedBody')}</p>
          <CopyField value={v?.fqdn ?? ''} label={t('domains.copyAddress')} className="max-w-md" />
        </motion.div>
      ) : (
        <div className="border-border bg-surface-2/60 flex flex-wrap items-center justify-between gap-3 rounded-xl border px-4 py-3">
          <div className="flex min-w-0 flex-1 flex-col gap-1.5" aria-live="polite">
            <span className="text-fg text-sm font-medium">
              {v
                ? t('domains.check.progress', { ok: summary.ok, total: summary.total })
                : t('domains.check.first')}
            </span>
            <Progress
              value={v ? summary.ok / Math.max(1, summary.total) : null}
              label={t('domains.check.progressLabel')}
              size="xs"
            />
            <span className="text-fg-subtle text-xs">
              {t('domains.check.auto')}
              {ago != null && ` · ${t('domains.check.lastCheck', { seconds: ago })}`}
            </span>
          </div>
          <Button
            size="sm"
            variant="secondary"
            leftIcon={<RefreshCw className={q.isFetching ? 'animate-spin' : undefined} />}
            onClick={() => void q.refetch()}
            disabled={q.isFetching}
            data-testid="check-now"
          >
            {t('domains.check.now')}
          </Button>
        </div>
      )}
      <RecordsTable
        records={records}
        verification={v}
        registrar={registrar}
        caption={t('domains.records.caption')}
      />
      {!connected && <p className="text-fg-subtle text-xs">{t('domains.help.propagation')}</p>}
    </div>
  );
}
