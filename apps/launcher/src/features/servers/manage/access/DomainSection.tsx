import { Badge, Button, cn, Dialog, Skeleton } from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Globe, PlugZap, ShoppingCart, Trash2, Wrench } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { Server } from '@/ipc/types/servers';
import { mk, useDomainBinding } from '../queries';
import { CopyField, Panel } from '../shared';
import { BuyPath } from './BuyPath';
import { ManualPath } from './ManualPath';
import { PorkbunPath } from './PorkbunPath';
import { RecordsTable } from './RecordsTable';
import { VerificationPanel } from './VerificationPanel';

export type DomainPath = 'porkbun' | 'buy' | 'manual';

/** "My own domain": three equal paths, or the current binding with its live status. */
export function DomainSection({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const { data: binding, isLoading } = useDomainBinding(server.id);
  const [path, setPath] = useState<DomainPath | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const remove = useMutation({
    mutationFn: () => invoke('dns_remove_binding', { server_id: server.id }),
    onSuccess: () => {
      qc.setQueryData(mk.domain(server.id), null);
      void qc.invalidateQueries({ queryKey: mk.server(server.id), exact: true });
      setConfirmRemove(false);
      setPath(null);
    },
  });

  const title = (
    <span className="flex items-center gap-2">
      <Globe className="text-accent-text size-4" aria-hidden />
      {t('domains.title')}
    </span>
  );

  if (isLoading) return <Skeleton className="h-56 rounded-2xl" />;

  // A domain is bound (configured through Porkbun or verified manually).
  if (binding && !path) {
    const target = { server_id: server.id, domain: binding.domain, subdomain: binding.subdomain };
    return (
      <Panel
        testId="domain-binding"
        title={title}
        description={t('domains.boundBody')}
        action={
          <Button
            variant="ghost"
            size="sm"
            leftIcon={<Trash2 />}
            onClick={() => setConfirmRemove(true)}
          >
            {t('domains.remove')}
          </Button>
        }
      >
        <div className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <CopyField
              value={binding.fqdn}
              label={t('domains.copyAddress')}
              className="w-full max-w-md"
              testId="bound-fqdn"
            />
            <Badge
              variant={
                binding.status === 'verified'
                  ? 'success'
                  : binding.status === 'error'
                    ? 'danger'
                    : 'warning'
              }
            >
              {t(`domains.status.${binding.status}`)}
            </Badge>
            <span className="text-fg-subtle text-xs">
              {t(`domains.provider.${binding.provider}`)}
            </span>
          </div>
          {binding.status === 'verified' ? (
            <RecordsTable records={binding.records} compact />
          ) : (
            <VerificationPanel target={target} records={binding.records} />
          )}
        </div>
        <Dialog
          open={confirmRemove}
          onOpenChange={setConfirmRemove}
          tone="destructive"
          icon={<Trash2 />}
          title={t('domains.removeTitle', { domain: binding.fqdn })}
          description={t('domains.removeBody')}
          closeLabel={t('manage.close')}
          footer={
            <>
              <Button onClick={() => setConfirmRemove(false)}>{t('manage.cancel')}</Button>
              <Button variant="danger" loading={remove.isPending} onClick={() => remove.mutate()}>
                {t('domains.remove')}
              </Button>
            </>
          }
        />
      </Panel>
    );
  }

  return (
    <Panel testId="domain-section" title={title} description={t('domains.intro')}>
      {path ? (
        <div className="flex flex-col gap-4">
          <Button
            variant="ghost"
            size="sm"
            leftIcon={<ArrowLeft />}
            className="w-fit"
            onClick={() => setPath(null)}
          >
            {t('domains.otherPaths')}
          </Button>
          {path === 'porkbun' && <PorkbunPath server={server} onDone={() => setPath(null)} />}
          {path === 'buy' && <BuyPath server={server} onDone={() => setPath(null)} />}
          {path === 'manual' && <ManualPath server={server} onDone={() => setPath(null)} />}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 lg:grid-cols-3" role="list">
          <PathTile
            icon={<PlugZap />}
            title={t('domains.paths.porkbun.title')}
            body={t('domains.paths.porkbun.body')}
            badge={t('domains.automatic')}
            onClick={() => setPath('porkbun')}
            testId="path-porkbun"
          />
          <PathTile
            icon={<ShoppingCart />}
            title={t('domains.paths.buy.title')}
            body={t('domains.paths.buy.body')}
            badge={t('domains.automatic')}
            onClick={() => setPath('buy')}
            testId="path-buy"
          />
          <PathTile
            icon={<Wrench />}
            title={t('domains.paths.manual.title')}
            body={t('domains.paths.manual.body')}
            badge={t('domains.anyRegistrar')}
            onClick={() => setPath('manual')}
            testId="path-manual"
          />
        </div>
      )}
    </Panel>
  );
}

function PathTile({
  icon,
  title,
  body,
  badge,
  onClick,
  testId,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  badge: string;
  onClick: () => void;
  testId: string;
}) {
  return (
    <div role="listitem">
      <button
        type="button"
        onClick={onClick}
        data-testid={testId}
        className={cn(
          'rx-focus-ring group border-border bg-surface-2/60 hover:border-accent/50 hover:bg-surface-2 flex h-full w-full flex-col gap-3 rounded-2xl border p-4 text-left transition-[border-color,background-color,transform] duration-200 hover:-translate-y-0.5',
        )}
      >
        <span className="flex items-center justify-between">
          <span className="bg-accent-soft text-accent-text grid size-10 place-items-center rounded-xl [&_svg]:size-5">
            {icon}
          </span>
          <Badge variant="neutral">{badge}</Badge>
        </span>
        <span className="text-fg text-sm font-semibold">{title}</span>
        <span className="text-fg-muted text-xs">{body}</span>
      </button>
    </div>
  );
}
