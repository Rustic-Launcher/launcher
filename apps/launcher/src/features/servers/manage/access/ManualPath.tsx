import { Button, Field, Input, Select, Skeleton } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { BookOpen, HelpCircle } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { Server } from '@/ipc/types/servers';
import { normalizeDomain, normalizeSubdomain, REGISTRARS, type Registrar } from '../dnsLogic';
import { Collapsible } from '../shared';
import { VerificationPanel } from './VerificationPanel';

/** Use a domain from any registrar: exact records, registrar guides, live verification. */
export function ManualPath({ server, onDone }: { server: Server; onDone: () => void }) {
  const { t } = useTranslation('servers');
  const [domainInput, setDomainInput] = useState('');
  const [sub, setSub] = useState('play');
  const [registrar, setRegistrar] = useState<Registrar>('generic');
  const [submitted, setSubmitted] = useState<{ domain: string; subdomain: string | null } | null>(
    null,
  );
  const domain = normalizeDomain(domainInput);
  const subdomain = normalizeSubdomain(sub);
  const valid = domain !== null && subdomain !== null;

  const records = useQuery({
    queryKey: ['dns', 'required', server.id, submitted?.domain, submitted?.subdomain ?? ''],
    queryFn: () => invoke('dns_required_records', { server_id: server.id, ...submitted! }),
    enabled: submitted != null,
  });

  return (
    <div className="flex flex-col gap-4" data-testid="manual-path">
      <h4 className="text-base font-semibold">{t('domains.paths.manual.title')}</h4>
      <p className="text-fg-muted text-sm">{t('domains.manual.intro')}</p>
      <form
        className="grid grid-cols-1 items-end gap-3 md:grid-cols-[1fr_160px_200px_auto]"
        onSubmit={(e) => {
          e.preventDefault();
          if (valid) setSubmitted({ domain: domain!, subdomain: subdomain ?? null });
        }}
      >
        <Field
          label={t('domains.manual.domain')}
          error={domainInput && domain === null ? t('domains.invalidDomain') : undefined}
        >
          {(p) => (
            <Input
              {...p}
              value={domainInput}
              onChange={(e) => setDomainInput(e.target.value)}
              placeholder="example.net"
              className="font-mono"
              data-testid="manual-domain"
            />
          )}
        </Field>
        <Field
          label={t('domains.subdomain')}
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
        <Field label={t('domains.manual.registrar')}>
          {(p) => (
            <Select
              id={p.id}
              value={registrar}
              onValueChange={(v) => setRegistrar(v as Registrar)}
              options={REGISTRARS.map((r) => ({ value: r, label: t(`domains.guides.${r}.name`) }))}
            />
          )}
        </Field>
        <Button type="submit" variant="primary" disabled={!valid} data-testid="manual-show-records">
          {t('domains.manual.show')}
        </Button>
      </form>
      {subdomain === undefined && domain && (
        <p className="text-warning-text flex items-start gap-2 text-xs">
          <HelpCircle className="mt-0.5 size-3.5 shrink-0" aria-hidden />
          {t('domains.help.apex')}
        </p>
      )}
      {submitted && records.isLoading && <Skeleton className="h-40 rounded-xl" />}
      {submitted && records.isError && <QueryError onRetry={() => void records.refetch()} />}
      {submitted && records.data && (
        <>
          <RegistrarGuide registrar={registrar} />
          <p className="text-fg-subtle text-xs">{t('domains.help.txt')}</p>
          <VerificationPanel
            key={`${submitted.domain}|${submitted.subdomain ?? ''}`}
            target={{ server_id: server.id, ...submitted }}
            records={records.data.records}
            registrar={registrar}
          />
          <Button variant="secondary" className="w-fit" onClick={onDone}>
            {t('manage.done')}
          </Button>
        </>
      )}
      <Collapsible title={t('domains.manual.allGuides')} icon={<BookOpen />}>
        <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
          {REGISTRARS.map((r) => (
            <Collapsible key={r} title={t(`domains.guides.${r}.name`)} className="bg-surface-2/40">
              <GuideSteps registrar={r} />
            </Collapsible>
          ))}
        </div>
      </Collapsible>
    </div>
  );
}

function GuideSteps({ registrar }: { registrar: Registrar }) {
  const { t } = useTranslation('servers');
  return (
    <ol className="text-fg-muted flex list-decimal flex-col gap-1 pl-5 text-sm">
      {t(`domains.guides.${registrar}.steps`)
        .split('\n')
        .map((s) => (
          <li key={s}>{s}</li>
        ))}
    </ol>
  );
}

function RegistrarGuide({ registrar }: { registrar: Registrar }) {
  const { t } = useTranslation('servers');
  return (
    <div
      className="border-border bg-surface-2/50 rounded-xl border p-4"
      data-testid="registrar-guide"
    >
      <h5 className="mb-2 flex items-center gap-2 text-sm font-semibold">
        <BookOpen className="text-accent-text size-4" aria-hidden />
        {t('domains.manual.guideTitle', { registrar: t(`domains.guides.${registrar}.name`) })}
      </h5>
      <GuideSteps registrar={registrar} />
    </div>
  );
}
