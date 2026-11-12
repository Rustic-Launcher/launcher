import { prepareOutcome } from '@/lib/prepare';
import { RusticAuthForm } from '@/features/settings/RusticAuthForm';
import { Avatar, Badge, Button, cn, Input, Progress, Skeleton, Switch } from '@rustic/ui';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  CheckCircle2,
  ExternalLink,
  FolderSearch,
  Hammer,
  Lock,
  ShieldCheck,
  Sparkles,
} from 'lucide-react';
import { useEffect, useId, useState, type Dispatch, type SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import { MicrosoftLogin } from '@/features/settings/MicrosoftLogin';
import { PalettePicker } from '@/features/settings/PalettePicker';
import { LanguageControl, ThemeControl } from '@/features/settings/ThemeControl';
import { DIGITAL_CONSENT_AGE, LEGAL_URLS, MIN_ACCOUNT_AGE } from '@/features/settings/legal';
import { invoke, listen } from '@/ipc';
import type { PrepareProgress } from '@/ipc/types/app';
import {
  qk,
  useAccounts,
  useHorizonVersions,
  useInstances,
  useRusticSession,
} from '@/queries/core';
import type { OnboardingState } from './OnboardingPage';
import { AccountAvatar } from '@/components/AccountAvatar';

export interface StepProps {
  state: OnboardingState;
  setState: Dispatch<SetStateAction<OnboardingState>>;
  setCanContinue: (v: boolean) => void;
}

function Block({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2.5">
      <h2 className="text-fg text-sm font-semibold">{title}</h2>
      {children}
    </div>
  );
}

export function WelcomeStep() {
  const { t } = useTranslation('onboarding');
  return (
    <div className="flex flex-col gap-6">
      <Block title={t('welcome.language')}>
        <LanguageControl />
      </Block>
      <Block title={t('welcome.theme')}>
        <ThemeControl />
      </Block>
      <Block title={t('welcome.palette')}>
        <PalettePicker />
      </Block>
    </div>
  );
}

export function MicrosoftStep({ setCanContinue }: StepProps) {
  const { t } = useTranslation('onboarding');
  const { data: accounts, isLoading } = useAccounts();
  const [ownsGame, setOwnsGame] = useState<boolean | null>(null);
  const existing = accounts?.find(
    (a) => a.kind === 'microsoft' && a.ownership?.owns_game && a.ownership.status === 'valid',
  );
  const ok = !!existing || ownsGame === true;

  useEffect(() => setCanContinue(ok), [ok, setCanContinue]);

  return (
    <div className="flex flex-col gap-5">
      {isLoading ? (
        <Skeleton className="h-16 rounded-xl" />
      ) : existing ? (
        <div
          className="border-success/30 bg-success/10 flex items-center gap-3 rounded-xl border p-4"
          role="status"
          data-testid="ms-already"
        >
          <AccountAvatar account={existing} size="md" />
          <div className="flex flex-1 flex-col">
            <span className="text-sm font-semibold">
              {t('microsoft.signedInAs', { name: existing.username })}
            </span>
            <span className="text-fg-muted text-xs">{t('microsoft.ownershipOk')}</span>
          </div>
          <CheckCircle2 className="text-success-text size-5" aria-hidden />
        </div>
      ) : (
        <MicrosoftLogin onSuccess={(a) => setOwnsGame(!!a.ownership?.owns_game)} />
      )}
      {ownsGame === false && (
        <div
          role="alert"
          className="border-danger/30 bg-danger/10 flex flex-col gap-2 rounded-xl border p-4"
        >
          <span className="text-sm font-semibold">{t('microsoft.notOwnedTitle')}</span>
          <span className="text-fg-muted text-xs">{t('microsoft.notOwnedBody')}</span>
          <Button
            size="sm"
            className="w-fit"
            rightIcon={<ExternalLink />}
            onClick={() => void invoke('app_open_url', { url: LEGAL_URLS.javaEdition })}
          >
            {t('microsoft.buyGame')}
          </Button>
        </div>
      )}
      <div className="border-border bg-surface-2/50 flex items-start gap-3 rounded-xl border p-4">
        <ShieldCheck className="text-accent-text mt-0.5 size-5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-1">
          <span className="text-sm font-semibold">{t('microsoft.whyTitle')}</span>
          <span className="text-fg-muted text-xs">{t('microsoft.whyBody')}</span>
        </div>
      </div>
    </div>
  );
}

export function RusticStep() {
  const { t } = useTranslation('onboarding');
  const { data: session, isLoading } = useRusticSession();
  const perks = ['friends', 'locker', 'servers'] as const;
  return (
    <div className="flex flex-col gap-5">
      <ul className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {perks.map((p) => (
          <li key={p} className="border-border bg-surface-2/50 rounded-xl border p-4">
            <span className="text-sm font-semibold">{t(`rustic.perks.${p}.title`)}</span>
            <p className="text-fg-muted mt-1 text-xs">{t(`rustic.perks.${p}.body`)}</p>
          </li>
        ))}
      </ul>
      {isLoading ? (
        <Skeleton className="h-16 rounded-xl" />
      ) : session?.signed_in ? (
        <div
          className="border-success/30 bg-success/10 flex items-center gap-3 rounded-xl border p-4"
          role="status"
        >
          <Avatar name={session.username ?? '?'} size="md" shape="circle" />
          <span className="flex-1 text-sm font-semibold">
            {t('rustic.signedInAs', { name: session.username })}
          </span>
          <CheckCircle2 className="text-success-text size-5" aria-hidden />
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <RusticAuthForm />
          <span className="text-fg-subtle text-xs">{t('rustic.skipHint')}</span>
        </div>
      )}
      <p className="text-fg-subtle text-xs">
        {t('rustic.age', { min: MIN_ACCOUNT_AGE, consent: DIGITAL_CONSENT_AGE })}
      </p>
    </div>
  );
}

function ToggleCard({
  title,
  body,
  checked,
  onChange,
  testId,
}: {
  title: string;
  body: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  testId: string;
}) {
  const id = useId();
  return (
    <div className="border-border bg-surface-2/50 flex items-start justify-between gap-6 rounded-xl border p-4">
      <label htmlFor={id} className="flex cursor-pointer flex-col gap-1">
        <span className="text-sm font-semibold">{title}</span>
        <span className="text-fg-muted text-xs">{body}</span>
      </label>
      <Switch id={id} checked={checked} onCheckedChange={onChange} data-testid={testId} />
    </div>
  );
}

export function PrivacyStep({ state, setState }: StepProps) {
  const { t } = useTranslation('onboarding');
  return (
    <div className="flex flex-col gap-3">
      <ToggleCard
        testId="consent-usage"
        title={t('privacy.usage.title')}
        body={t('privacy.usage.body')}
        checked={state.telemetry}
        onChange={(v) => setState((s) => ({ ...s, telemetry: v }))}
      />
      <ToggleCard
        testId="consent-performance"
        title={t('privacy.performance.title')}
        body={t('privacy.performance.body')}
        checked={state.performance}
        onChange={(v) => setState((s) => ({ ...s, performance: v }))}
      />
      <ToggleCard
        testId="consent-crash"
        title={t('privacy.crash.title')}
        body={t('privacy.crash.body')}
        checked={state.crash}
        onChange={(v) => setState((s) => ({ ...s, crash: v }))}
      />
      <p className="text-fg-subtle flex items-start gap-2 text-xs">
        <Lock className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        {t('privacy.note')}
      </p>
      <Button
        variant="link"
        size="sm"
        className="w-fit"
        rightIcon={<ExternalLink />}
        onClick={() => void invoke('app_open_url', { url: LEGAL_URLS.privacy })}
      >
        {t('privacy.policy')}
      </Button>
    </div>
  );
}

export function GameStep({ setCanContinue }: StepProps) {
  const { t } = useTranslation('onboarding');
  const qc = useQueryClient();
  const detect = useQuery({
    queryKey: ['game-detect'],
    queryFn: () => invoke('game_detect_install'),
  });
  const prep = useQuery({ queryKey: qk.prepare, queryFn: () => invoke('game_prepare_status') });
  const [progress, setProgress] = useState<PrepareProgress | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const started = progress != null;
  const done = progress?.step === 'done';
  const ready = prep.data?.state === 'ready';

  useEffect(
    () =>
      listen('prepare-progress', (p) => {
        setProgress(p);
        if (prepareOutcome(p) === 'failed') {
          setFailure(p.message ?? null);
          setProgress(null);
        } else if (p.step === 'done') void qc.invalidateQueries({ queryKey: qk.prepare });
      }),
    [qc],
  );
  useEffect(() => setCanContinue(started || ready), [started, ready, setCanContinue]);

  const prepare = () => {
    setFailure(null);
    setProgress({ step: 'detect', progress: 0 });
    invoke('game_prepare', { source_path: detect.data?.path ?? null }).catch((e: Error) => {
      setProgress(null);
      setFailure(e.message);
    });
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="border-border bg-surface-2/50 flex items-start gap-3 rounded-xl border p-4">
        <FolderSearch className="text-accent-text mt-0.5 size-5 shrink-0" aria-hidden />
        <div className="flex min-w-0 flex-1 flex-col gap-1" aria-live="polite">
          {detect.isLoading ? (
            <>
              <span className="text-sm font-semibold">{t('game.detecting')}</span>
              <Progress value={null} label={t('game.detecting')} size="xs" className="mt-1" />
            </>
          ) : detect.data?.found ? (
            <>
              <span className="text-sm font-semibold" data-testid="game-found">
                {t('game.found', { versions: detect.data.versions.join(', ') })}
              </span>
              <code className="text-fg-subtle truncate text-xs">{detect.data.path}</code>
            </>
          ) : (
            <>
              <span className="text-sm font-semibold">{t('game.notFound')}</span>
              <span className="text-fg-muted text-xs">{t('game.notFoundBody')}</span>
            </>
          )}
        </div>
      </div>

      <div className="border-border bg-surface-2/50 flex flex-col gap-3 rounded-xl border p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <Hammer className="text-accent-text size-5" aria-hidden />
            <span className="text-sm font-semibold">{t('game.prepareTitle')}</span>
            {ready && !started && <Badge variant="success">{t('game.alreadyReady')}</Badge>}
            {done && <Badge variant="success">{t('game.done')}</Badge>}
          </div>
          <Button
            variant={ready ? 'secondary' : 'primary'}
            onClick={prepare}
            disabled={detect.isLoading || (started && !done)}
            data-testid="prepare-start"
          >
            {ready || done ? t('game.prepareAgain') : t('game.prepare')}
          </Button>
        </div>
        {failure && (
          <p role="alert" className="text-danger-text text-sm" data-testid="prepare-failed">
            {t('game.prepareFailed', { reason: failure })}
          </p>
        )}
        {started && (
          <div className="flex flex-col gap-1.5" aria-live="polite" data-testid="prepare-progress">
            <Progress value={progress.progress} label={t('game.prepareTitle')} />
            <ol className="mt-1 grid grid-cols-3 gap-1 sm:grid-cols-6">
              {(
                ['detect', 'download', 'verify', 'compile_exporters', 'export', 'index'] as const
              ).map((s, i, all) => {
                const cur = all.indexOf(progress.step as (typeof all)[number]);
                const state = done || i < cur ? 'done' : i === cur ? 'active' : 'todo';
                return (
                  <li
                    key={s}
                    className={cn(
                      'text-2xs',
                      state === 'done' && 'text-success-text',
                      state === 'active' && 'text-accent-text font-semibold',
                      state === 'todo' && 'text-fg-subtle',
                    )}
                  >
                    {t(`game.steps.${s}`)}
                  </li>
                );
              })}
            </ol>
          </div>
        )}
        <p className="text-fg-muted text-xs">{t('game.explain')}</p>
        {started && !done && <p className="text-fg-subtle text-xs">{t('game.background')}</p>}
      </div>
    </div>
  );
}

export function ReadyStep({ state, setState }: StepProps) {
  const { t } = useTranslation('onboarding');
  const { data: versions } = useHorizonVersions();
  const { data: instances } = useInstances();
  const rec = versions?.find((v) => v.recommended);
  const nameId = useId();
  return (
    <div className="flex flex-col gap-4">
      <div
        className={cn(
          'flex flex-col gap-4 rounded-xl border p-4 transition-colors',
          state.createInstance
            ? 'border-accent/50 bg-accent-soft'
            : 'border-border bg-surface-2/50',
        )}
      >
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-3">
            <InstanceIcon icon={{ kind: 'preset', preset: 'grass', color: '#4E9F5B' }} size="lg" />
            <div className="flex flex-col gap-1">
              <span className="text-sm font-semibold">{t('ready.createTitle')}</span>
              {rec ? (
                <span className="text-fg-muted flex items-center gap-2 text-xs">
                  <HorizonVersion id={rec.id} rusticEdition={rec.rustic_edition} />
                  <Badge variant="accent" icon={<Sparkles />}>
                    {t('ready.recommended')}
                  </Badge>
                </span>
              ) : (
                <Skeleton className="h-4 w-48" />
              )}
            </div>
          </div>
          <Switch
            checked={state.createInstance}
            onCheckedChange={(v) => setState((s) => ({ ...s, createInstance: v }))}
            aria-label={t('ready.createTitle')}
            data-testid="create-first-instance"
          />
        </div>
        {state.createInstance && (
          <div className="flex flex-col gap-1.5">
            <label htmlFor={nameId} className="text-fg-muted text-xs font-medium">
              {t('ready.nameLabel')}
            </label>
            <Input
              id={nameId}
              value={state.instanceName}
              placeholder={t('ready.defaultName')}
              maxLength={40}
              onChange={(e) => setState((s) => ({ ...s, instanceName: e.target.value }))}
            />
          </div>
        )}
      </div>
      {instances && instances.length > 0 && (
        <p className="text-fg-subtle text-xs">{t('ready.existing', { count: instances.length })}</p>
      )}
      <ul className="text-fg-muted flex flex-col gap-2 text-sm">
        {(['play', 'friends', 'servers'] as const).map((k) => (
          <li key={k} className="flex items-center gap-2">
            <CheckCircle2 className="text-success-text size-4" aria-hidden />
            {t(`ready.tips.${k}`)}
          </li>
        ))}
      </ul>
    </div>
  );
}
