import { Badge } from '@rustic/ui';
import { CheckCircle2, CircleDashed, CircleMinus, Hourglass, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { DnsRecord, DnsVerification } from '@/ipc/types/dns';

type Check = DnsVerification['records'][number];

/** Per-record live status badge with found-vs-expected detail for wrong values. */
export function RecordStatus({ record, check }: { record: DnsRecord; check: Check | undefined }) {
  const { t } = useTranslation('servers');
  if (!check) {
    return (
      <Badge variant="neutral" icon={<CircleDashed />}>
        {t('domains.check.pending')}
      </Badge>
    );
  }
  if (record.optional && check.status === 'missing') {
    return (
      <Badge variant="neutral" icon={<CircleMinus />}>
        {t('domains.check.notNeeded')}
      </Badge>
    );
  }
  switch (check.status) {
    case 'ok':
      return (
        <Badge variant="success" icon={<CheckCircle2 />} data-testid="status-ok">
          {t('domains.check.ok')}
        </Badge>
      );
    case 'propagating':
      return (
        <Badge variant="warning" icon={<Hourglass />}>
          {t('domains.check.propagating', {
            ok: check.resolvers_ok ?? 0,
            total: check.resolvers_total ?? 3,
          })}
        </Badge>
      );
    case 'wrong_value':
      return (
        <div className="flex flex-col gap-1">
          <Badge variant="danger" icon={<XCircle />}>
            {t('domains.check.wrong')}
          </Badge>
          <span className="text-2xs text-fg-muted">
            {t('domains.check.found')}{' '}
            <code className="text-danger-text font-mono">{check.found.join(', ') || '–'}</code>
          </span>
          <span className="text-2xs text-fg-muted">
            {t('domains.check.expected')}{' '}
            <code className="text-success-text font-mono">{record.value}</code>
          </span>
        </div>
      );
    default:
      return (
        <Badge variant="danger" icon={<XCircle />}>
          {t('domains.check.missing')}
        </Badge>
      );
  }
}
