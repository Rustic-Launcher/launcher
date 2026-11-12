import { Button, Dialog, Input, Select, Switch, toast } from '@rustic/ui';
import { useMutation } from '@tanstack/react-query';
import { Download, ExternalLink, ShieldCheck, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { PrivacySettings } from '@/ipc/types/app';
import { formatDate } from '@/lib/format';
import { useSettings } from '@/queries/core';
import { useMe, useSetDiscoverable } from '@/queries/social';
import { SettingRow, SettingsCard, SectionSkeleton, useSaveSettings } from '../components';
import { LEGAL_URLS } from '../legal';

export function PrivacySection() {
  const { t } = useTranslation('settings');
  const { data: settings, isLoading, isError, refetch } = useSettings();
  const save = useSaveSettings();
  if (isLoading) return <SectionSkeleton />;
  if (isError || !settings) return <QueryError onRetry={() => void refetch()} />;
  const p = settings.privacy;
  const patch = (v: Partial<PrivacySettings>) => save.mutate({ privacy: v });

  return (
    <div className="flex flex-col gap-4">
      <SettingsCard
        title={t('privacy.telemetry.title')}
        description={t('privacy.telemetry.description')}
        testId="settings-telemetry"
      >
        <SettingRow
          label={t('privacy.telemetry.usage')}
          description={t('privacy.telemetry.usageHint')}
          htmlFor="p-usage"
        >
          <Switch
            id="p-usage"
            checked={p.telemetry}
            onCheckedChange={(v) => patch({ telemetry: v })}
          />
        </SettingRow>
        <SettingRow
          label={t('privacy.telemetry.performance')}
          description={t('privacy.telemetry.performanceHint')}
          htmlFor="p-perf"
        >
          <Switch
            id="p-perf"
            checked={!!p.performance_reports}
            onCheckedChange={(v) => patch({ performance_reports: v })}
          />
        </SettingRow>
        <SettingRow
          label={t('privacy.telemetry.crash')}
          description={t('privacy.telemetry.crashHint')}
          htmlFor="p-crash"
        >
          <Switch
            id="p-crash"
            checked={p.crash_reports}
            onCheckedChange={(v) => patch({ crash_reports: v })}
          />
        </SettingRow>
        <Button
          variant="link"
          size="sm"
          rightIcon={<ExternalLink />}
          onClick={() => void invoke('app_open_url', { url: LEGAL_URLS.privacy })}
        >
          {t('privacy.policyLink')}
        </Button>
      </SettingsCard>

      <SettingsCard title={t('privacy.social.title')}>
        <SettingRow
          label={t('privacy.social.activity')}
          description={t('privacy.social.activityHint')}
          htmlFor="p-activity"
        >
          <Switch
            id="p-activity"
            checked={p.share_activity_with_friends}
            onCheckedChange={(v) => patch({ share_activity_with_friends: v })}
          />
        </SettingRow>
        <DiscoverableRow />
        <SettingRow label={t('privacy.social.requests')} htmlFor="p-requests">
          <Select
            id="p-requests"
            className="w-60"
            value={p.friend_requests_from}
            onValueChange={(v) =>
              patch({ friend_requests_from: v as PrivacySettings['friend_requests_from'] })
            }
            options={(['everyone', 'friends_of_friends', 'nobody'] as const).map((v) => ({
              value: v,
              label: t(`privacy.social.requestsOptions.${v}`),
            }))}
          />
        </SettingRow>
      </SettingsCard>

      <DataCard />
    </div>
  );
}

/** Account setting (Rustic Cloud, ADR 0145): forced off for minors. */
function DiscoverableRow() {
  const { t } = useTranslation('settings');
  const { data: me } = useMe();
  const set = useSetDiscoverable();
  const allowed = me?.discoverable_allowed ?? false;
  return (
    <SettingRow
      label={t('privacy.social.discoverable')}
      description={
        me && !allowed
          ? t('privacy.social.discoverableMinor')
          : t('privacy.social.discoverableHint')
      }
      htmlFor="p-discoverable"
    >
      <Switch
        id="p-discoverable"
        data-testid="settings-discoverable"
        checked={set.isPending ? !!set.variables : !!me?.discoverable}
        disabled={!me || !allowed || set.isPending}
        onCheckedChange={(v) =>
          set.mutate(v, {
            onError: () => toast({ tone: 'error', title: t('privacy.social.discoverableFailed') }),
          })
        }
      />
    </SettingRow>
  );
}

function DataCard() {
  const { t, i18n } = useTranslation('settings');
  const { data: me } = useMe();
  const [exportLink, setExportLink] = useState<{ url: string; expires_at: string } | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [typed, setTyped] = useState('');
  const [scheduled, setScheduled] = useState<string | null>(null);

  const exp = useMutation({
    mutationFn: async () => (await cloud()).exportAccount(),
    onSuccess: (r) => {
      setExportLink(r);
      toast({ tone: 'success', title: t('privacy.data.exportReady') });
    },
    onError: () => toast({ tone: 'error', title: t('privacy.data.exportFailed') }),
  });
  const del = useMutation({
    mutationFn: async () => (await cloud()).deleteAccount(typed.trim()),
    onSuccess: (r) => {
      setScheduled(r.scheduled_for);
      setConfirmOpen(false);
      setTyped('');
      toast({
        tone: 'info',
        title: t('privacy.data.deleteScheduled', {
          date: formatDate(r.scheduled_for, i18n.language),
        }),
      });
    },
    onError: () => toast({ tone: 'error', title: t('privacy.data.deleteFailed') }),
  });
  const username = me?.username ?? '';

  return (
    <SettingsCard title={t('privacy.data.title')} description={t('privacy.data.description')}>
      <SettingRow label={t('privacy.data.export')} description={t('privacy.data.exportHint')}>
        {exportLink ? (
          <Button
            size="sm"
            variant="soft"
            rightIcon={<ExternalLink />}
            onClick={() => void invoke('app_open_url', { url: exportLink.url })}
          >
            {t('privacy.data.download', { date: formatDate(exportLink.expires_at, i18n.language) })}
          </Button>
        ) : (
          <Button
            size="sm"
            leftIcon={<Download />}
            loading={exp.isPending}
            onClick={() => exp.mutate()}
          >
            {t('privacy.data.exportButton')}
          </Button>
        )}
      </SettingRow>
      <SettingRow label={t('privacy.data.delete')} description={t('privacy.data.deleteHint')}>
        {scheduled ? (
          <span className="text-warning-text text-sm" role="status">
            {t('privacy.data.deleteScheduled', { date: formatDate(scheduled, i18n.language) })}
          </span>
        ) : (
          <Button
            size="sm"
            variant="danger"
            leftIcon={<Trash2 />}
            onClick={() => setConfirmOpen(true)}
            data-testid="delete-account"
          >
            {t('privacy.data.deleteButton')}
          </Button>
        )}
      </SettingRow>
      <p className="text-fg-subtle mt-2 flex items-start gap-2 text-xs">
        <ShieldCheck className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        {t('privacy.data.rights')}
      </p>
      <Dialog
        open={confirmOpen}
        onOpenChange={(o) => {
          setConfirmOpen(o);
          if (!o) setTyped('');
        }}
        tone="destructive"
        icon={<Trash2 />}
        title={t('privacy.data.confirmTitle')}
        description={t('privacy.data.confirmBody')}
        closeLabel={t('close')}
        footer={
          <>
            <Button onClick={() => setConfirmOpen(false)}>{t('cancel')}</Button>
            <Button
              variant="danger"
              disabled={!username || typed.trim() !== username}
              loading={del.isPending}
              onClick={() => del.mutate()}
            >
              {t('privacy.data.confirmButton')}
            </Button>
          </>
        }
      >
        <label htmlFor="delete-confirm" className="text-fg-muted mb-1.5 block text-sm">
          {t('privacy.data.typeUsername', { name: username })}
        </label>
        <Input
          id="delete-confirm"
          autoComplete="off"
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
          placeholder={username}
        />
      </Dialog>
    </SettingsCard>
  );
}
