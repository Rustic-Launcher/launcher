import { Badge, Button, Select, Slider, Switch, toast } from '@rustic/ui';
import { useMutation } from '@tanstack/react-query';
import { RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { LauncherSettings, LauncherUpdate } from '@/ipc/types/app';
import { useSettings } from '@/queries/core';
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

export function UpdatesCard({
  autoUpdate,
  onAutoUpdate,
}: {
  autoUpdate?: boolean;
  onAutoUpdate?: (v: boolean) => void;
}) {
  const { t } = useTranslation('settings');
  const [result, setResult] = useState<LauncherUpdate | null>(null);
  const check = useMutation({
    mutationFn: () => invoke('app_check_update'),
    onSuccess: (r) => {
      setResult(r);
      toast({
        tone: r.available ? 'info' : 'success',
        title: r.available
          ? t('launcher.updates.available', { version: r.version })
          : t('launcher.updates.upToDate'),
      });
    },
    onError: () => toast({ tone: 'error', title: t('launcher.updates.failed') }),
  });
  return (
    <SettingsCard title={t('launcher.updates.title')}>
      {onAutoUpdate && (
        <SettingRow
          label={t('launcher.updates.auto')}
          description={t('launcher.updates.autoHint')}
          htmlFor="auto-update"
        >
          <Switch id="auto-update" checked={!!autoUpdate} onCheckedChange={onAutoUpdate} />
        </SettingRow>
      )}
      <SettingRow label={t('launcher.updates.check')}>
        {result && !check.isPending && (
          <Badge variant={result.available ? 'accent' : 'success'}>
            {result.available
              ? t('launcher.updates.available', { version: result.version })
              : t('launcher.updates.upToDate')}
          </Badge>
        )}
        <Button
          size="sm"
          leftIcon={<RefreshCw />}
          loading={check.isPending}
          onClick={() => check.mutate()}
        >
          {t('launcher.updates.checkNow')}
        </Button>
      </SettingRow>
    </SettingsCard>
  );
}
