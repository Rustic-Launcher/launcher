import { Fragment } from 'react';
import { Badge, cn } from '@rustic/ui';
import { useTranslation } from 'react-i18next';
import type { DnsRecord, DnsVerification } from '@/ipc/types/dns';
import { hostForRegistrar, type Registrar } from '../dnsLogic';
import { CopyButton } from '../shared';
import { RecordStatus } from './RecordStatus';

/**
 * The exact records to add, with a copy button per field. With a verification result, a live
 * status column (missing / wrong value: found vs expected / propagating n of 3 / OK).
 */
export function RecordsTable({
  records,
  verification,
  registrar = 'generic',
  compact,
  caption,
}: {
  records: DnsRecord[];
  verification?: DnsVerification;
  registrar?: Registrar;
  compact?: boolean;
  caption?: string;
}) {
  const { t } = useTranslation('servers');
  return (
    <div className="border-border overflow-x-auto rounded-xl border">
      <table className="w-full min-w-[640px] text-left text-sm" data-testid="records-table">
        {caption && <caption className="sr-only">{caption}</caption>}
        <thead className="bg-surface-2/70 text-2xs text-fg-subtle font-semibold tracking-wider uppercase">
          <tr>
            <th scope="col" className="px-3 py-2">
              {t('domains.records.type')}
            </th>
            <th scope="col" className="px-3 py-2">
              {t('domains.records.name')}
            </th>
            <th scope="col" className="px-3 py-2">
              {t('domains.records.value')}
            </th>
            {!compact && (
              <th scope="col" className="px-3 py-2">
                {t('domains.records.ttl')}
              </th>
            )}
            {verification && (
              <th scope="col" className="px-3 py-2">
                {t('domains.records.status')}
              </th>
            )}
          </tr>
        </thead>
        <tbody className="divide-border divide-y">
          {records.map((r) => {
            const check = verification?.records.find((x) => x.record_id === r.id);
            const host = hostForRegistrar(r, registrar);
            return (
              <tr key={r.id} className="align-top" data-testid={`record-${r.type}`}>
                <td className="px-3 py-2.5">
                  <Badge variant={r.purpose === 'ownership' ? 'info' : 'accent'}>{r.type}</Badge>
                  {r.optional && (
                    <span className="text-2xs text-fg-subtle mt-1 block">
                      {t('domains.records.optional')}
                    </span>
                  )}
                </td>
                <td className="px-3 py-2.5">
                  <div className="flex items-center gap-1">
                    <code className="text-fg font-mono text-xs [overflow-wrap:anywhere]">
                      {host ? dotBreaks(host) : t('domains.records.emptyHost')}
                    </code>
                    <CopyButton
                      value={host}
                      label={t('domains.records.copyName', { type: r.type })}
                    />
                  </div>
                  <span className="text-2xs text-fg-subtle block">{r.fqdn}</span>
                </td>
                <td className="px-3 py-2.5">
                  <div className="flex items-center gap-1">
                    <code className={cn('text-fg font-mono text-xs [overflow-wrap:anywhere]')}>
                      {dotBreaks(r.value)}
                    </code>
                    <CopyButton
                      value={r.value}
                      label={t('domains.records.copyValue', { type: r.type })}
                    />
                  </div>
                  {r.type === 'SRV' && (
                    <span className="text-2xs text-fg-subtle block">
                      {t('domains.records.srvFields', {
                        priority: r.priority ?? 0,
                        weight: r.weight ?? 0,
                        port: r.port ?? 0,
                      })}
                    </span>
                  )}
                </td>
                {!compact && (
                  <td className="text-fg-muted px-3 py-2.5 font-mono text-xs">{r.ttl}</td>
                )}
                {verification && (
                  <td className="px-3 py-2.5">
                    <RecordStatus record={r} check={check} />
                  </td>
                )}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

/** Lets long DNS names wrap after dots and spaces instead of mid-label. */
function dotBreaks(s: string): React.ReactNode[] {
  return s.split(/(?<=[. ])/).map((part, i) => (
    <Fragment key={i}>
      {part}
      <wbr />
    </Fragment>
  ));
}
