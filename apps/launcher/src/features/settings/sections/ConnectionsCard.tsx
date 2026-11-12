/** Connections: Porkbun API keys and trusted VPS hosts (managed elsewhere, revocable here). */
import { Badge, Button, Skeleton, toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Globe, Server, Unplug } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import { formatDate } from '@/lib/format';
import { SettingRow, SettingsCard } from '../components';

const porkbunKey = ['dns', 'porkbun-status'] as const;
const vpsKey = ['hosting', 'vps-list'] as const;

export function ConnectionsCard() {
  const { t, i18n } = useTranslation('settings');
  const qc = useQueryClient();
  const porkbun = useQuery({
    queryKey: porkbunKey,
    queryFn: () => invoke('dns_porkbun_status'),
    retry: false,
  });
  const vps = useQuery({
    queryKey: vpsKey,
    queryFn: () => invoke('hosting_vps_list'),
    retry: false,
  });
  const disconnect = useMutation({
    mutationFn: () => invoke('dns_disconnect', { provider: 'porkbun' }),
    onSuccess: (s) => {
      qc.setQueryData(porkbunKey, s);
      toast({ tone: 'info', title: t('accounts.connections.porkbunRemoved') });
    },
  });
  const removeVps = useMutation({
    mutationFn: (id: string) => invoke('hosting_vps_remove', { id }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: vpsKey }),
  });

  return (
    <SettingsCard
      title={t('accounts.connections.title')}
      description={t('accounts.connections.description')}
    >
      <SettingRow
        label={
          <span className="flex items-center gap-2">
            <Globe className="text-fg-subtle size-4" aria-hidden />
            {t('accounts.connections.porkbun')}
          </span>
        }
        description={t('accounts.connections.porkbunHint')}
      >
        {porkbun.isLoading ? (
          <Skeleton className="h-6 w-28" />
        ) : porkbun.data?.connected ? (
          <>
            <Badge variant="success">
              {porkbun.data.account_label ?? t('accounts.connections.connected')}
            </Badge>
            <Button
              size="sm"
              variant="ghost"
              leftIcon={<Unplug />}
              loading={disconnect.isPending}
              onClick={() => disconnect.mutate()}
            >
              {t('accounts.connections.remove')}
            </Button>
          </>
        ) : (
          <Badge variant="neutral">{t('accounts.connections.notConnected')}</Badge>
        )}
      </SettingRow>
      <SettingRow
        label={
          <span className="flex items-center gap-2">
            <Server className="text-fg-subtle size-4" aria-hidden />
            {t('accounts.connections.vps')}
          </span>
        }
        description={t('accounts.connections.vpsHint')}
      >
        {vps.isLoading ? (
          <Skeleton className="h-6 w-28" />
        ) : (vps.data?.length ?? 0) === 0 ? (
          <Badge variant="neutral">{t('accounts.connections.noVps')}</Badge>
        ) : null}
      </SettingRow>
      {vps.data && vps.data.length > 0 && (
        <ul className="flex flex-col gap-2 pb-1">
          {vps.data.map((v) => (
            <li
              key={v.id}
              className="border-border bg-surface-2/50 flex flex-wrap items-center gap-3 rounded-xl border px-3 py-2"
            >
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-medium">
                  {v.label} · {v.username}@{v.host}:{v.port}
                </span>
                <code className="text-2xs text-fg-subtle truncate">
                  {t('accounts.connections.fingerprint', {
                    fp: v.fingerprint_sha256,
                    date: formatDate(v.added_at, i18n.language),
                  })}
                </code>
              </div>
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<Unplug />}
                loading={removeVps.isPending && removeVps.variables === v.id}
                onClick={() => removeVps.mutate(v.id)}
              >
                {t('accounts.connections.forget')}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </SettingsCard>
  );
}
