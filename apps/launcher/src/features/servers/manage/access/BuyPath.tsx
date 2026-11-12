import { Badge, Button, Checkbox, Dialog, Field, Input, Skeleton, toast } from '@rustic/ui';
import { useMutation, useQuery } from '@tanstack/react-query';
import { Search, ShoppingCart, Store } from 'lucide-react';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import type { DomainAvailability, Money } from '@/ipc/types/dns';
import type { Server } from '@/ipc/types/servers';
import { formatMoney } from '@/lib/format';
import { fqdnOf, normalizeSubdomain } from '../dnsLogic';
import { usePorkbun } from '../queries';
import { PorkbunConnect } from './PorkbunConnect';
import { PorkbunAccount, useConfigureDomain } from './PorkbunPath';
import { VerificationPanel } from './VerificationPanel';

/** UX_SPEC 4.4.3: orders above $100 are not placed from the launcher. */
export const MAX_IN_APP_PRICE: Money = { amount_minor: 10_000, currency: 'USD' };

export function purchasable(d: DomainAvailability): boolean {
  return d.available && !!d.price && d.price.amount_minor <= MAX_IN_APP_PRICE.amount_minor;
}

/** Buy a domain at Porkbun on the user's own account, then configure it automatically. */
export function BuyPath({ server, onDone }: { server: Server; onDone: () => void }) {
  const { t, i18n } = useTranslation('servers');
  const lang = i18n.language;
  const { data: conn, isLoading } = usePorkbun();
  const [query, setQuery] = useState('');
  const [submitted, setSubmitted] = useState('');
  const [sub, setSub] = useState('play');
  const [selected, setSelected] = useState<DomainAvailability | null>(null);
  const [agreeTerms, setAgreeTerms] = useState(false);
  const [agreeOrder, setAgreeOrder] = useState(false);
  const termsId = useId();
  const orderId = useId();
  const subdomain = normalizeSubdomain(sub);

  const results = useQuery({
    queryKey: ['dns', 'check', submitted],
    queryFn: () => invoke('dns_check_domain', { query: submitted }),
    enabled: submitted.length >= 2,
    staleTime: 60_000,
  });
  const configure = useConfigureDomain(server.id);
  const buy = useMutation({
    mutationFn: (d: DomainAvailability) =>
      invoke('dns_buy_domain', {
        domain: d.domain,
        years: 1,
        expected_price: d.price!,
        agree_registrar_terms: true,
        authorize_order: true,
      }),
    onSuccess: (r) => {
      setSelected(null);
      toast({
        tone: 'success',
        title: t('domains.buy.bought', { domain: r.domain }),
        description: t('domains.buy.boughtBody', { price: formatMoney(r.charged, lang) }),
      });
      configure.mutate({ domain: r.domain, subdomain: subdomain ?? null });
    },
    onError: (e) => {
      const reason =
        e instanceof IpcFailure ? (e.details?.reason as string | undefined) : undefined;
      if (reason === 'price_changed') void results.refetch();
      toast({
        tone: 'error',
        title: t('domains.buy.failed'),
        description: reason
          ? t(`domains.buy.errors.${reason as 'price_changed'}`, { defaultValue: e.message })
          : e.message,
      });
    },
  });

  if (isLoading) return <Skeleton className="h-48 rounded-xl" />;

  const applied = configure.data;

  return (
    <div className="flex flex-col gap-4" data-testid="buy-path">
      <h4 className="text-base font-semibold">{t('domains.paths.buy.title')}</h4>
      <p className="text-fg-muted text-sm">{t('domains.buy.intro')}</p>
      {!conn?.connected ? (
        <>
          <p className="border-info/30 bg-info/10 text-info-text rounded-xl border px-4 py-2.5 text-sm">
            {t('domains.buy.needsAccount')}
          </p>
          <PorkbunConnect />
        </>
      ) : applied ? (
        <>
          <VerificationPanel
            target={{ server_id: server.id, domain: applied.domain, subdomain: applied.subdomain }}
            records={applied.records}
          />
          <Button variant="secondary" className="w-fit" onClick={onDone}>
            {t('manage.done')}
          </Button>
        </>
      ) : configure.isPending ? (
        <div className="flex flex-col gap-2" aria-busy="true">
          <p className="text-fg-muted text-sm">{t('domains.buy.configuring')}</p>
          <Skeleton className="h-32 rounded-xl" />
        </div>
      ) : (
        <>
          <PorkbunAccount />
          <form
            className="flex flex-wrap items-end gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              setSubmitted(query.trim().toLowerCase());
            }}
            role="search"
          >
            <Field label={t('domains.buy.search')} className="min-w-64 flex-1">
              {(p) => (
                <Input
                  {...p}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder={t('domains.buy.searchPlaceholder')}
                  leftIcon={<Search />}
                  data-testid="domain-search"
                />
              )}
            </Field>
            <Field
              label={t('domains.subdomain')}
              className="w-40"
              error={subdomain === null ? t('domains.invalidSubdomain') : undefined}
            >
              {(p) => (
                <Input
                  {...p}
                  value={sub}
                  onChange={(e) => setSub(e.target.value)}
                  className="font-mono"
                />
              )}
            </Field>
            <Button
              type="submit"
              variant="primary"
              disabled={query.trim().length < 2}
              loading={results.isFetching}
              data-testid="domain-search-submit"
            >
              {t('domains.buy.searchButton')}
            </Button>
          </form>
          {results.isLoading && <Skeleton className="h-40 rounded-xl" />}
          {results.data && results.data.length === 0 && (
            <p className="text-fg-muted text-sm">{t('domains.buy.noResults')}</p>
          )}
          {results.data && results.data.length > 0 && (
            <ul
              className="divide-border border-border flex flex-col divide-y overflow-hidden rounded-xl border"
              data-testid="domain-results"
            >
              {results.data.map((d) => {
                const tooPricey = d.available && !!d.price && !purchasable(d);
                return (
                  <li
                    key={d.domain}
                    className="flex flex-wrap items-center gap-3 px-4 py-3"
                    data-testid={`domain-${d.domain}`}
                  >
                    <span className="text-fg min-w-0 flex-1 font-mono text-sm font-medium">
                      {d.domain}
                    </span>
                    {d.premium && d.available && (
                      <Badge variant="warning">{t('domains.buy.premium')}</Badge>
                    )}
                    {d.available && d.price ? (
                      <span className="text-fg-muted text-sm tabular-nums">
                        {t('domains.buy.price', {
                          price: formatMoney(d.price, lang),
                          renewal: d.renewal_price ? formatMoney(d.renewal_price, lang) : '–',
                        })}
                      </span>
                    ) : (
                      <Badge variant="neutral">{t('domains.buy.taken')}</Badge>
                    )}
                    {d.available &&
                      (tooPricey ? (
                        <span className="text-fg-subtle text-xs">{t('domains.buy.overLimit')}</span>
                      ) : (
                        <Button
                          size="sm"
                          variant="soft"
                          disabled={subdomain === null}
                          onClick={() => {
                            setAgreeTerms(false);
                            setAgreeOrder(false);
                            setSelected(d);
                          }}
                          data-testid={`select-${d.domain}`}
                        >
                          {t('domains.buy.select')}
                        </Button>
                      ))}
                  </li>
                );
              })}
            </ul>
          )}
          <p className="text-fg-subtle text-xs">{t('domains.buy.pricesNote')}</p>
        </>
      )}
      <Dialog
        open={selected != null}
        onOpenChange={(o) => !o && !buy.isPending && setSelected(null)}
        tone="purchase"
        icon={<Store />}
        size="lg"
        title={t('domains.buy.confirmTitle')}
        description={t('domains.buy.seller')}
        closeLabel={t('manage.close')}
        footer={
          <>
            <Button onClick={() => setSelected(null)} disabled={buy.isPending}>
              {t('manage.cancel')}
            </Button>
            <Button
              variant="primary"
              leftIcon={<ShoppingCart />}
              disabled={!agreeTerms || !agreeOrder}
              loading={buy.isPending}
              onClick={() => selected && buy.mutate(selected)}
              data-testid="buy-confirm"
            >
              {t('domains.buy.buyButton', {
                price: selected?.price ? formatMoney(selected.price, lang) : '',
              })}
            </Button>
          </>
        }
      >
        {selected && (
          <div className="flex flex-col gap-4">
            <dl className="border-border bg-surface-2/60 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 rounded-xl border p-4 text-sm">
              <dt className="text-fg-subtle">{t('domains.buy.domain')}</dt>
              <dd className="text-fg font-mono font-medium">{selected.domain}</dd>
              <dt className="text-fg-subtle">{t('domains.buy.address')}</dt>
              <dd className="text-fg font-mono">{fqdnOf(selected.domain, subdomain)}</dd>
              <dt className="text-fg-subtle">{t('domains.buy.registrar')}</dt>
              <dd className="text-fg">{t('domains.buy.registrarValue')}</dd>
              <dt className="text-fg-subtle">{t('domains.buy.firstYear')}</dt>
              <dd className="text-fg font-semibold tabular-nums" data-testid="buy-price">
                {selected.price && formatMoney(selected.price, lang)} ({selected.price?.currency})
              </dd>
              <dt className="text-fg-subtle">{t('domains.buy.renewal')}</dt>
              <dd className="text-fg tabular-nums">
                {selected.renewal_price && formatMoney(selected.renewal_price, lang)}
              </dd>
            </dl>
            <p className="text-fg-muted text-sm">
              {t('domains.buy.payment', {
                price: selected.price ? formatMoney(selected.price, lang) : '',
                renewal: selected.renewal_price ? formatMoney(selected.renewal_price, lang) : '',
              })}
            </p>
            <div className="flex items-start gap-2.5">
              <Checkbox
                id={termsId}
                checked={agreeTerms}
                onCheckedChange={(v) => setAgreeTerms(v === true)}
                data-testid="agree-terms"
              />
              <label htmlFor={termsId} className="text-fg text-sm">
                {t('domains.buy.agreeTerms')}{' '}
                <button
                  type="button"
                  className="rx-focus-ring text-accent-text rounded font-medium hover:underline"
                  onClick={() => void invoke('app_open_url', { url: 'https://porkbun.com/legal' })}
                >
                  {t('domains.buy.termsLink')}
                </button>
              </label>
            </div>
            <div className="flex items-start gap-2.5">
              <Checkbox
                id={orderId}
                checked={agreeOrder}
                onCheckedChange={(v) => setAgreeOrder(v === true)}
                data-testid="agree-order"
              />
              <label htmlFor={orderId} className="text-fg text-sm">
                {t('domains.buy.agreeOrder')}
              </label>
            </div>
            <p className="text-fg-subtle text-xs">{t('domains.buy.refunds')}</p>
          </div>
        )}
      </Dialog>
    </div>
  );
}
