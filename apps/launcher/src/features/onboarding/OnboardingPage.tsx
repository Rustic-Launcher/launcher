/**
 * First launch (UX_SPEC J1): welcome + look, Microsoft sign-in (ownership), Rustic account (optional),
 * privacy choices (all off), game files, ready. Full screen, outside the launcher chrome.
 */
import { Button, Stepper } from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { AnimatePresence, motion } from 'motion/react';
import { ArrowLeft, ArrowRight } from 'lucide-react';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import { isNative } from '@/ipc';
import { RusticMark } from '@/layout/Logo';
import { WindowControls } from '@/layout/WindowControls';
import { qk } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import i18n, { type Language } from '@/i18n';
import {
  GameStep,
  MicrosoftStep,
  PrivacyStep,
  ReadyStep,
  RusticStep,
  WelcomeStep,
  type StepProps,
} from './steps';

export const STEPS = ['welcome', 'microsoft', 'rustic', 'privacy', 'game', 'ready'] as const;
export type StepId = (typeof STEPS)[number];

/**
 * Steps that wait on something outside the launcher (Mojang's approval of the app, the player's
 * game files) and may be left for later. Skipping changes nothing about the owner gate: playing
 * still needs a Microsoft account that owns the game (the Play button and `game_launch` check it).
 */
const SKIPPABLE: readonly StepId[] = ['microsoft', 'game'];

export interface OnboardingState {
  telemetry: boolean;
  performance: boolean;
  crash: boolean;
  createInstance: boolean;
  instanceName: string;
}

const VIEWS: Record<StepId, (p: StepProps) => React.ReactNode> = {
  welcome: WelcomeStep,
  microsoft: MicrosoftStep,
  rustic: RusticStep,
  privacy: PrivacyStep,
  game: GameStep,
  ready: ReadyStep,
};

export function OnboardingPage() {
  const { t } = useTranslation('onboarding');
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [index, setIndex] = useState(0);
  const [dir, setDir] = useState(1);
  const [canContinue, setCanContinue] = useState(true);
  const [finishing, setFinishing] = useState(false);
  const [state, setState] = useState<OnboardingState>({
    telemetry: false,
    performance: false,
    crash: false,
    createInstance: true,
    instanceName: '',
  });
  const step = STEPS[index]!;
  const View = VIEWS[step];

  // Make sure the detected language is stored, so the choice is explicit from the first screen.
  useEffect(() => {
    if (!usePrefs.getState().language) usePrefs.getState().setLanguage(i18n.language as Language);
  }, []);

  const go = (d: number) => {
    setDir(d);
    setCanContinue(true);
    setIndex((i) => Math.min(STEPS.length - 1, Math.max(0, i + d)));
  };

  const next = async () => {
    if (step === 'privacy') {
      await invoke('settings_set', {
        privacy: {
          telemetry: state.telemetry,
          performance_reports: state.performance,
          crash_reports: state.crash,
        },
      });
    }
    if (step === 'ready') {
      setFinishing(true);
      try {
        if (state.createInstance) {
          const versions = await qc.ensureQueryData({
            queryKey: qk.versions,
            queryFn: () => invoke('horizon_versions_list'),
          });
          const rec = versions.find((v) => v.recommended) ?? versions[0];
          if (rec) {
            const inst = await invoke('instance_create', {
              name: state.instanceName.trim() || t('ready.defaultName'),
              icon: { kind: 'preset', preset: 'grass', color: '#4E9F5B' },
              horizon_version: rec.id,
            });
            usePrefs.getState().setActiveInstance(inst.id);
            void qc.invalidateQueries({ queryKey: qk.instances });
          }
        }
        const s = await invoke('settings_set', { onboarding_completed: true });
        qc.setQueryData(qk.settings, s);
        await navigate({ to: '/' });
      } finally {
        setFinishing(false);
      }
      return;
    }
    go(1);
  };

  return (
    <div className="rx-ambient flex h-screen w-screen flex-col overflow-y-auto">
      <header
        className="flex items-center gap-3 pt-6 pr-3 pl-8"
        data-tauri-drag-region={isNative() ? true : undefined}
      >
        <RusticMark size={36} />
        <span className="text-base font-semibold">{t('brand')}</span>
        <span
          className="text-fg-subtle ml-auto self-stretch pr-2 text-xs leading-9"
          data-tauri-drag-region={isNative() ? true : undefined}
        >
          {t('stepOf', { current: index + 1, total: STEPS.length })}
        </span>
        <WindowControls />
      </header>
      <main className="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-6 px-6 py-8">
        <Stepper
          steps={STEPS.map((s) => t(`steps.${s}`))}
          current={index}
          stepLabel={(i, n) => t('stepOf', { current: i, total: n })}
        />
        <section
          aria-labelledby="onboarding-title"
          className="rx-glass relative flex min-h-[460px] flex-col overflow-hidden rounded-2xl shadow-lg"
          data-testid={`onboarding-step-${step}`}
        >
          <AnimatePresence mode="wait" initial={false} custom={dir}>
            <motion.div
              key={step}
              initial={{ opacity: 0, x: dir * 24 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: dir * -24 }}
              transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
              className="flex flex-1 flex-col gap-6 p-8"
            >
              <div className="flex flex-col gap-2">
                <StepHeading>{t(`${step}.title`)}</StepHeading>
                <p className="text-fg-muted max-w-xl text-sm">{t(`${step}.subtitle`)}</p>
              </div>
              <View state={state} setState={setState} setCanContinue={setCanContinue} />
            </motion.div>
          </AnimatePresence>
          <footer className="border-border bg-bg-elevated/40 flex items-center justify-between gap-3 border-t px-8 py-4">
            {index > 0 ? (
              <Button variant="ghost" leftIcon={<ArrowLeft />} onClick={() => go(-1)}>
                {t('back')}
              </Button>
            ) : (
              <span />
            )}
            <div className="flex items-center gap-2">
              {SKIPPABLE.includes(step) && !canContinue && (
                <Button variant="ghost" onClick={() => go(1)} data-testid="onboarding-skip">
                  {t('skip')}
                </Button>
              )}
              <Button
                variant="primary"
                size="lg"
                rightIcon={<ArrowRight />}
                disabled={!canContinue}
                loading={finishing}
                onClick={() => void next()}
                data-testid="onboarding-next"
              >
                {step === 'ready' ? t('ready.finish') : t('continue')}
              </Button>
            </div>
          </footer>
        </section>
      </main>
    </div>
  );
}

/** Step title; takes focus when the step appears so screen readers announce it. */
function StepHeading({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);
  return (
    <h1
      id="onboarding-title"
      ref={ref}
      tabIndex={-1}
      className="text-fg text-3xl font-bold tracking-tight outline-none"
    >
      {children}
    </h1>
  );
}
