import { Badge, Button, Select, Slider, Switch, toast } from '@rustic/ui';
import { useMutation } from '@tanstack/react-query';
import { ArrowDownToLine, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { LauncherSettings } from '@/ipc/types/app';
import { formatPercent, formatRelative } from '@/lib/format';
import { useSettings } from '@/queries/core';
import { useUpdates } from '@/stores/updates';
import { SettingRow, SettingsCard, SectionSkeleton, useSaveSettings } from '../components';
import { LanguageControl } from '../ThemeControl';

export function LauncherSection() {
  const { t } = useTranslation('settings');
  const { data: settings, isLoading, isError, refetch } = useSettings();
  const save = useSaveSettings();
  if (isLoading) return <SectionSkeleton />;
  if (isError || !settings) return <QueryError onRetry={() => void refetch()} />;
  const l = settings.launcher;
  const patch = (p: Partial<LauncherSettings>) => save.mutate({ launcher: p });

  return (
    <div className="flex flex-col gap-4">
      <SettingsCard
        title={t('launcher.language.title')}
        description={t('launcher.language.description')}
      >
        <LanguageControl />
      </SettingsCard>

      <SettingsCard title={t('launcher.startup.title')}>
        <SettingRow
          label={t('launcher.startup.onBoot')}
          description={t('launcher.startup.onBootHint')}
          htmlFor="start-boot"
        >
          <Switch
            id="start-boot"
            checked={l.start_on_boot}
            onCheckedChange={(v) => patch({ start_on_boot: v })}
          />
        </SettingRow>
        <SettingRow label={t('launcher.startup.minimized')} htmlFor="start-min">
          <Switch
            id="start-min"
            checked={l.start_minimized}
            disabled={!l.start_on_boot}
            onCheckedChange={(v) => patch({ start_minimized: v })}
          />
        </SettingRow>
      </SettingsCard>

      <SettingsCard title={t('launcher.behaviour.title')}>
        <SettingRow label={t('launcher.behaviour.onLaunch')} htmlFor="on-launch">
          <Select
            id="on-launch"
            className="w-60"
            value={l.on_game_launch}
            onValueChange={(v) =>
              patch({ on_game_launch: v as LauncherSettings['on_game_launch'] })
            }
            options={(['keep_open', 'minimize', 'hide', 'close'] as const).map((v) => ({
              value: v,
              label: t(`launcher.behaviour.launchOptions.${v}`),
            }))}
          />
        </SettingRow>
        <SettingRow
          label={t('launcher.behaviour.onClose')}
          description={t('launcher.behaviour.onCloseHint')}
          htmlFor="on-close"
        >
          <Select
            id="on-close"
            className="w-60"
            value={l.on_close}
            onValueChange={(v) => patch({ on_close: v as LauncherSettings['on_close'] })}
            options={(['tray', 'quit'] as const).map((v) => ({
              value: v,
              label: t(`launcher.behaviour.closeOptions.${v}`),
            }))}
          />
        </SettingRow>
      </SettingsCard>

      <UpdatesCard autoUpdate={l.auto_update} onAutoUpdate={(v) => patch({ auto_update: v })} />

      <SettingsCard title={t('launcher.performance.title')}>
        <SettingRow
          label={t('launcher.performance.downloads')}
          description={t('launcher.performance.downloadsHint')}
        >
          <div className="flex w-64 items-center gap-3">
            <Slider
              label={t('launcher.performance.downloads')}
              value={l.concurrent_downloads}
              min={1}
              max={8}
              onValueChange={(v) => patch({ concurrent_downloads: v })}
            />
            <span className="w-4 text-sm font-semibold tabular-nums">{l.concurrent_downloads}</span>
          </div>
        </SettingRow>
        <SettingRow
          label={t('launcher.performance.hardware')}
          description={t('launcher.performance.hardwareHint')}
          htmlFor="hw-accel"
        >
          <Switch
            id="hw-accel"
            checked={l.hardware_acceleration}
            onCheckedChange={(v) => patch({ hardware_acceleration: v })}
          />
        </SettingRow>
      </SettingsCard>
    </div>
  );
}

/**
 * Updates (ADR 0162): the launcher's version and channel, the installed Rustic edition build, the
 * last check, "Check now", and "Update and restart" when something waits (automatic updates
 * off, or held back). The toggle lives in Settings > Launcher.
 */
export function UpdatesCard({
  autoUpdate,
  onAutoUpdate,
}: {
  autoUpdate?: boolean;
  onAutoUpdate?: (v: boolean) => void;
}) {
  const { t, i18n } = useTranslation('settings');
  const status = useUpdates((s) => s.status);
  const check = useMutation({
    mutationFn: () => invoke('app_update_check'),
    onSuccess: (r) => {
      if (r.error) toast({ tone: 'error', title: t('launcher.updates.failed') });
      else if (!r.launcher && !r.edition)
        toast({ tone: 'success', title: t('launcher.updates.upToDate') });
    },
    onError: () => toast({ tone: 'error', title: t('launcher.updates.failed') }),
  });
  const apply = useMutation({ mutationFn: () => invoke('app_update_apply') });
  const busy =
    status?.phase === 'checking' ||
    status?.phase === 'downloading' ||
    status?.phase === 'installing' ||
    status?.phase === 'restarting';
  const found = !!(status?.launcher || status?.edition);
  const stateText = !status
    ? null
    : status.phase === 'checking'
      ? t('launcher.updates.checking')
      : status.phase === 'downloading' || status.phase === 'installing'
        ? t('launcher.updates.working', {
            pct: status.progress != null ? formatPercent(status.progress, i18n.language) : '…',
          })
        : status.phase === 'restarting'
          ? t('launcher.updates.restarting')
          : status.phase === 'deferred'
            ? t('launcher.updates.deferred')
            : null;
  return (
    <SettingsCard title={t('launcher.updates.title')} testId="updates-card">
      {onAutoUpdate && (
        <SettingRow
          label={t('launcher.updates.auto')}
          description={t('launcher.updates.autoHint')}
          htmlFor="auto-update"
        >
          <Switch id="auto-update" checked={!!autoUpdate} onCheckedChange={onAutoUpdate} />
        </SettingRow>
      )}
      <SettingRow
        label={t('launcher.updates.check')}
        description={
          status?.last_check
            ? t('launcher.updates.lastCheck', {
                when: formatRelative(status.last_check, i18n.language),
              })
            : t('launcher.updates.neverChecked')
        }
      >
        <span className="flex flex-wrap items-center justify-end gap-2">
          {status && (
            <span className="text-fg-muted text-sm tabular-nums" data-testid="launcher-version">
              {status.current_version}
            </span>
          )}
          {status && (
            <Badge variant={status.channel === 'beta' ? 'warning' : 'neutral'}>
              {t(`about.channel.${status.channel}`)}
            </Badge>
          )}
          {status?.launcher ? (
            <Badge variant="accent">
              {t('launcher.updates.available', { version: status.launcher.version })}
            </Badge>
          ) : (
            status?.last_check &&
            !status.error && <Badge variant="success">{t('launcher.updates.upToDate')}</Badge>
          )}
        </span>
      </SettingRow>
      <SettingRow label={t('launcher.updates.edition')}>
        <span className="flex flex-wrap items-center justify-end gap-2">
          <span className="text-fg-muted max-w-72 truncate text-sm" data-testid="edition-version">
            {status?.edition_current ?? t('launcher.updates.editionNone')}
          </span>
          {status?.edition && (
            <Badge variant="accent">
              {t('launcher.updates.available', { version: status.edition.version })}
            </Badge>
          )}
        </span>
      </SettingRow>
      <div className="mt-3 flex flex-wrap items-center justify-end gap-3">
        {stateText && (
          <span className="text-fg-muted mr-auto text-sm" role="status">
            {stateText}
          </span>
        )}
        {!stateText && status?.error && (
          <span className="text-warning-text mr-auto text-sm" title={status.error ?? undefined}>
            {t('launcher.updates.lastError')}
          </span>
        )}
        {found && (status?.phase === 'available' || status?.phase === 'error') && (
          <Button
            size="sm"
            variant="primary"
            leftIcon={<ArrowDownToLine />}
            loading={apply.isPending}
            onClick={() => apply.mutate()}
            data-testid="update-apply"
          >
            {t('launcher.updates.apply')}
          </Button>
        )}
        <Button
          size="sm"
          leftIcon={<RefreshCw />}
          loading={check.isPending}
          disabled={busy}
          onClick={() => check.mutate()}
        >
          {t('launcher.updates.checkNow')}
        </Button>
      </div>
    </SettingsCard>
  );
}
