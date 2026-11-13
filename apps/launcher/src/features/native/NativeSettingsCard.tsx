/** Settings › Game & instances: native mods' Rust and compile cache (ADR 0157). */
import { Badge, Button, Dialog, Input, Skeleton, formatBytes, toast } from '@rustic/ui';
import { Cpu, Eraser } from 'lucide-react';
import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SettingRow, SettingsCard } from '@/features/settings/components';
import { useNativeActions, useNativeCache, useToolchainTask, waitForTask } from './useNative';
import { NativeToolchainDialog } from './NativeDialogs';

export function NativeSettingsCard() {
  const { t, i18n } = useTranslation('settings');
  const { t: tc } = useTranslation();
  const { data, isLoading } = useNativeCache();
  const actions = useNativeActions();
  const installing = useToolchainTask();
  const [limit, setLimit] = useState('');
  const [confirm, setConfirm] = useState(false);
  const [rust, setRust] = useState(false);
  const limitId = useId();
  useEffect(() => {
    if (data) setLimit(String(data.limit_gib));
  }, [data]);
  const bytes = (n: number) => formatBytes(n, i18n.language);

  const saveLimit = () => {
    const n = Number(limit);
    if (!Number.isInteger(n) || n < 0 || n > 4096) {
      toast({ tone: 'error', title: t('game.native.limitInvalid') });
      if (data) setLimit(String(data.limit_gib));
      return;
    }
    if (data && n === data.limit_gib) return;
    actions.setLimit.mutate(n, {
      onSuccess: () => toast({ tone: 'success', title: t('game.native.limitSaved') }),
      onError: (e) => toast({ tone: 'error', title: e.message }),
    });
  };

  const tc0 = data?.toolchain;
  const rustRunning = installing && installing.state !== 'failed';
  return (
    <SettingsCard
      title={t('game.native.title')}
      description={t('game.native.description')}
      testId="settings-native"
    >
      {isLoading || !data ? (
        <Skeleton className="h-24" />
      ) : (
        <>
          <SettingRow
            label={t('game.native.rust')}
            description={
              tc0?.state === 'ready'
                ? t(`game.native.rustReady.${tc0.kind ?? 'system'}`, { version: tc0.version })
                : t('game.native.rustMissing', { version: tc0?.version ?? '' })
            }
          >
            {tc0?.state === 'ready' ? (
              <Badge variant="success" data-testid="native-rust-state">
                {t('game.native.ready')}
              </Badge>
            ) : rustRunning ? (
              <Badge variant="accent">{t('game.native.installing')}</Badge>
            ) : (
              <>
                <Badge variant="warning" data-testid="native-rust-state">
                  {t('game.native.missing')}
                </Badge>
                <Button size="sm" leftIcon={<Cpu />} onClick={() => setRust(true)}>
                  {t('game.native.installRust')}
                </Button>
              </>
            )}
          </SettingRow>
          <SettingRow
            label={t('game.native.cache')}
            description={t('game.native.cacheDetail', {
              cache: bytes(data.usage.cache_bytes),
              crates: bytes(data.usage.crates_bytes),
              toolchain: bytes(data.usage.toolchain_bytes),
              source: bytes(data.usage.source_bytes),
            })}
          >
            <span
              className="text-fg text-sm font-semibold tabular-nums"
              data-testid="native-cache-size"
            >
              {bytes(data.usage.cache_bytes)}
            </span>
            <Button
              size="sm"
              leftIcon={<Eraser />}
              disabled={data.building || data.usage.cache_bytes === 0}
              onClick={() => setConfirm(true)}
              data-testid="native-cache-clear"
            >
              {t('game.native.clear')}
            </Button>
          </SettingRow>
          <SettingRow
            label={t('game.native.limit')}
            description={t('game.native.limitHint')}
            htmlFor={limitId}
          >
            <Input
              id={limitId}
              type="number"
              min={0}
              max={4096}
              step={1}
              inputMode="numeric"
              className="w-24 text-right tabular-nums"
              value={limit}
              onChange={(e) => setLimit(e.target.value)}
              onBlur={saveLimit}
              onKeyDown={(e) => e.key === 'Enter' && saveLimit()}
              data-testid="native-cache-limit"
            />
            <span className="text-fg-muted text-sm">{t('game.native.gib')}</span>
          </SettingRow>
          <p className="text-fg-subtle mt-1 text-xs break-all">
            {t('game.native.folder', { path: data.usage.path })}
          </p>
        </>
      )}
      <Dialog
        open={confirm}
        onOpenChange={setConfirm}
        tone="destructive"
        icon={<Eraser />}
        title={t('game.native.clearTitle')}
        description={t('game.native.clearBody')}
        closeLabel={tc('actions.close')}
        footer={
          <>
            <Button onClick={() => setConfirm(false)}>{tc('actions.cancel')}</Button>
            <Button
              variant="danger"
              loading={actions.clearCache.isPending}
              data-testid="native-cache-clear-confirm"
              onClick={() =>
                actions.clearCache.mutate(undefined, {
                  onSuccess: (freed) => {
                    setConfirm(false);
                    toast({
                      tone: 'success',
                      title: t('game.native.cleared', { size: bytes(freed) }),
                    });
                  },
                  onError: (e) => toast({ tone: 'error', title: e.message }),
                })
              }
            >
              {t('game.native.clear')}
            </Button>
          </>
        }
      />
      <NativeToolchainDialog
        open={rust}
        onOpenChange={setRust}
        toolchain={tc0 ?? null}
        pending={actions.installToolchain.isPending}
        onInstall={() =>
          actions.installToolchain.mutate(undefined, {
            onSuccess: ({ task_id }) => {
              setRust(false);
              void waitForTask(task_id).then((ok) =>
                toast(
                  ok
                    ? { tone: 'success', title: t('game.native.rustInstalled') }
                    : { tone: 'error', title: t('game.native.rustFailed') },
                ),
              );
            },
            onError: (e) => toast({ tone: 'error', title: e.message }),
          })
        }
      />
    </SettingsCard>
  );
}
