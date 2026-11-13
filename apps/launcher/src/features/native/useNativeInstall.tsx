/**
 * Installing a native mod (ADR 0157): the trust notice the first time for the instance, the mod's
 * file, Rust when it is missing (with the player's agreement), then the build, which shows in the
 * downloads panel and on the instance. Every step stops at the player's "no".
 */
import { toast } from '@rustic/ui';
import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import type { NativeToolchain } from '@/ipc/types/native';
import { NativeToolchainDialog, NativeTrustDialog } from './NativeDialogs';
import { useNativeActions, waitForTask } from './useNative';

export interface NativeInstallVars {
  instanceId: string;
  instanceName: string;
  modName: string;
  sourceUrl?: string | null;
  /** Installs the mod's file (content_install); resolves once it is in the instance. */
  install: () => Promise<unknown>;
}

const reasonOf = (e: unknown): string | undefined => {
  const r = e instanceof IpcFailure ? e.details?.reason : undefined;
  return typeof r === 'string' ? r : undefined;
};

export function useNativeInstall(): {
  run: (v: NativeInstallVars) => Promise<void>;
  /** Starts (or restarts) a build of an instance, asking for what it lacks. */
  build: (v: Omit<NativeInstallVars, 'install'>) => Promise<void>;
  busy: boolean;
  dialogs: ReactNode;
} {
  const { t } = useTranslation('content');
  const actions = useNativeActions();
  const [pending, setPending] = useState<NativeInstallVars | null>(null);
  const [step, setStep] = useState<'trust' | 'toolchain' | null>(null);
  const [toolchain, setToolchain] = useState<NativeToolchain | null>(null);
  const [busy, setBusy] = useState(false);

  const fail = (title: string, e: unknown) => {
    const reason = reasonOf(e);
    toast({
      tone: 'error',
      title,
      description:
        reason === 'native.not_edition'
          ? t('native.errors.notEdition')
          : reason === 'native.running'
            ? t('native.errors.running')
            : reason === 'native.disk_space'
              ? t('native.errors.disk', { detail: (e as Error).message })
              : (e as Error).message,
    });
  };

  const startBuild = async (v: Omit<NativeInstallVars, 'install'>) => {
    try {
      await actions.build.mutateAsync(v.instanceId);
      toast({
        tone: 'info',
        title: t('native.build.started', { instance: v.instanceName }),
        description: t('native.build.startedHint'),
      });
    } catch (e) {
      const reason = reasonOf(e);
      if (reason === 'native.toolchain_missing') {
        const s = await invoke('native_status', { instance_id: v.instanceId });
        setToolchain(s.toolchain);
        setPending({ ...v, install: async () => undefined });
        setStep('toolchain');
        return;
      }
      if (reason === 'native.consent_required') {
        setPending({ ...v, install: async () => undefined });
        setStep('trust');
        return;
      }
      fail(t('native.build.failed', { instance: v.instanceName }), e);
    }
  };

  /** After consent: the file, then Rust if needed, then the build. */
  const proceed = async (v: NativeInstallVars) => {
    setBusy(true);
    try {
      await v.install();
    } catch (e) {
      setBusy(false);
      if (reasonOf(e) === 'native.consent_required') {
        setPending(v);
        setStep('trust');
        return;
      }
      fail(t('install.failed', { name: v.modName }), e);
      return;
    }
    try {
      const s = await invoke('native_status', { instance_id: v.instanceId });
      if (s.toolchain.state === 'missing') {
        setToolchain(s.toolchain);
        setPending(v);
        setStep('toolchain');
        return;
      }
      await startBuild(v);
    } finally {
      setBusy(false);
    }
  };

  const run = async (v: NativeInstallVars) => {
    setBusy(true);
    try {
      const s = await invoke('native_status', { instance_id: v.instanceId });
      if (!s.supported) {
        toast({
          tone: 'error',
          title: t('install.failed', { name: v.modName }),
          description: t('native.errors.notEdition'),
        });
        return;
      }
      if (!s.consent) {
        setPending(v);
        setStep('trust');
        return;
      }
    } catch (e) {
      fail(t('install.failed', { name: v.modName }), e);
      return;
    } finally {
      setBusy(false);
    }
    await proceed(v);
  };

  const build = async (v: Omit<NativeInstallVars, 'install'>) => {
    setBusy(true);
    try {
      await startBuild(v);
    } finally {
      setBusy(false);
    }
  };

  const close = () => {
    setStep(null);
    setPending(null);
  };

  const dialogs = (
    <>
      <NativeTrustDialog
        open={step === 'trust'}
        onOpenChange={(o) => !o && close()}
        modName={pending?.modName ?? ''}
        instanceName={pending?.instanceName ?? ''}
        sourceUrl={pending?.sourceUrl}
        pending={actions.consent.isPending}
        onAccept={() => {
          const v = pending;
          if (!v) return;
          actions.consent.mutate(
            { instanceId: v.instanceId, accept: true },
            {
              onSuccess: () => {
                close();
                void proceed(v);
              },
              onError: (e) => fail(t('install.failed', { name: v.modName }), e),
            },
          );
        }}
      />
      <NativeToolchainDialog
        open={step === 'toolchain'}
        onOpenChange={(o) => !o && close()}
        toolchain={toolchain}
        pending={actions.installToolchain.isPending}
        onInstall={() => {
          const v = pending;
          if (!v) return;
          actions.installToolchain.mutate(undefined, {
            onSuccess: ({ task_id }) => {
              close();
              toast({
                tone: 'info',
                title: t('native.toolchain.started'),
                description: t('native.toolchain.startedHint'),
              });
              void waitForTask(task_id).then((ok) => {
                if (ok) void startBuild(v);
                else
                  toast({
                    tone: 'error',
                    title: t('native.toolchain.failed'),
                  });
              });
            },
            onError: (e) => fail(t('native.toolchain.failed'), e),
          });
        }}
      />
    </>
  );

  return { run, build, busy, dialogs };
}
