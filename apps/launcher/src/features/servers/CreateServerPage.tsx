import { Button, Card, Dialog, Stepper, toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useBlocker, useNavigate } from '@tanstack/react-router';
import { AnimatePresence, motion } from 'motion/react';
import { ArrowLeft, ArrowRight, Rocket, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Page, PageHeader } from '@/components/Page';
import { invoke } from '@/ipc';
import { RUSTIC_SERVER_ANCHOR } from '@/features/settings/anchors';
import type { CreateServerResult } from '@/ipc/types/servers';
import { LocalProgress, VpsInstallProgress } from './hosting/InstallProgress';
import { serverKeys } from './list/queries';
import { AccessStep } from './wizard/AccessStep';
import { BasicsStep } from './wizard/BasicsStep';
import { HostingStep } from './wizard/HostingStep';
import {
  buildCreateRequest,
  canProceed,
  initialWizardState,
  isNoRusticServer,
  validateStep,
  WIZARD_STEPS,
  type WizardState,
} from './wizard/model';

export function CreateServerPage() {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const [state, setState] = useState<WizardState>(initialWizardState);
  const [step, setStep] = useState(0);
  const [dir, setDir] = useState(1);
  const [showErrors, setShowErrors] = useState(false);
  const [result, setResult] = useState<CreateServerResult | null>(null);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const current = WIZARD_STEPS[step]!;
  const errors = validateStep(current, state);
  const dirty = state.name.trim() !== '' && !result;

  const update = useCallback(
    (patch: Partial<WizardState>) => setState((s) => ({ ...s, ...patch })),
    [],
  );

  const vpsConns = useQuery({
    queryKey: serverKeys.vps,
    queryFn: () => invoke('hosting_vps_list'),
    enabled: state.hosting === 'vps',
  });
  const vpsHost = vpsConns.data?.find((c) => c.id === state.vpsConnectionId)?.host ?? null;

  // Leaving with unsaved input asks first (Esc / Cancel / navigation).
  const leaving = useRef(false);
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;
  const blocker = useBlocker({
    shouldBlockFn: () => dirtyRef.current && !leaving.current,
    withResolver: true,
    enableBeforeUnload: false,
  });

  const create = useMutation({
    mutationFn: () => {
      const req = buildCreateRequest(state);
      return req.hosting.mode === 'vps'
        ? invoke('hosting_vps_install', {
            connection_id: req.hosting.connection_id,
            server: req,
            open_firewall: state.vpsOpenFirewall,
          })
        : invoke('server_create', req);
    },
    onSuccess: (r) => {
      setResult(r);
      void qc.invalidateQueries({ queryKey: serverKeys.all });
    },
    onError: (e) =>
      isNoRusticServer(e)
        ? toast({
            tone: 'error',
            title: t('wizard.createFailed'),
            description: t('wizard.noRusticServer'),
            duration: 12_000,
            action: {
              label: t('wizard.openRusticServerSettings'),
              onClick: () =>
                void navigate({
                  to: '/settings',
                  search: { section: 'game', focus: RUSTIC_SERVER_ANCHOR },
                }),
            },
          })
        : toast({ tone: 'error', title: t('wizard.createFailed'), description: e.message }),
  });

  const go = (to: number) => {
    setDir(to > step ? 1 : -1);
    setShowErrors(false);
    setStep(to);
    document.getElementById('main')?.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const next = () => {
    if (!canProceed(current, state)) {
      setShowErrors(true);
      return;
    }
    if (step < WIZARD_STEPS.length - 1) go(step + 1);
    else create.mutate();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || result || e.defaultPrevented) return;
      if (document.querySelector('[role="dialog"], [data-radix-popper-content-wrapper]')) return;
      if (dirty) setConfirmDiscard(true);
      else void navigate({ to: '/servers' });
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dirty, result, navigate]);

  const stepTitles = WIZARD_STEPS.map((s) => t(`wizard.steps.${s}`));
  const doneTab = state.domainAfter ? 'access' : undefined;

  return (
    <Page>
      <PageHeader
        title={
          result ? t('wizard.creatingTitle', { name: result.server.name }) : t('wizard.heading')
        }
        description={result ? undefined : t('wizard.subtitle')}
      />

      {!result && (
        <Stepper
          steps={stepTitles}
          current={step}
          stepLabel={(i, n) => t('wizard.stepOf', { index: i, total: n })}
          className="max-w-3xl"
        />
      )}

      <Card variant="glass" padding="lg" className="overflow-hidden">
        {result ? (
          result.server.hosting.mode === 'vps' ? (
            <VpsInstallProgress
              server={result.server}
              taskId={result.task_id}
              host={result.server.hosting.host}
              tab={doneTab}
            />
          ) : (
            <LocalProgress server={result.server} taskId={result.task_id} tab={doneTab} />
          )
        ) : (
          <>
            <h2 className="mb-5 text-lg font-semibold">{stepTitles[step]}</h2>
            <AnimatePresence mode="wait" initial={false} custom={dir}>
              <motion.div
                key={current}
                initial={{ opacity: 0, x: dir * 24 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: dir * -24 }}
                transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
              >
                {current === 'basics' && (
                  <BasicsStep
                    state={state}
                    update={update}
                    errors={errors}
                    showErrors={showErrors}
                  />
                )}
                {current === 'hosting' && (
                  <HostingStep
                    state={state}
                    update={update}
                    errors={errors}
                    showErrors={showErrors}
                  />
                )}
                {current === 'access' && (
                  <AccessStep state={state} update={update} vpsHost={vpsHost} />
                )}
              </motion.div>
            </AnimatePresence>
            {showErrors && Object.keys(errors).length > 0 && (
              <p role="alert" className="text-danger-text mt-4 text-sm">
                {t('wizard.fixErrors')}
              </p>
            )}
            <div className="border-border mt-6 flex items-center justify-between gap-3 border-t pt-5">
              <Button
                variant="ghost"
                onClick={() =>
                  step === 0
                    ? dirty
                      ? setConfirmDiscard(true)
                      : void navigate({ to: '/servers' })
                    : go(step - 1)
                }
                leftIcon={<ArrowLeft />}
              >
                {step === 0 ? tc('actions.cancel') : tc('actions.back')}
              </Button>
              <Button
                variant="primary"
                size="lg"
                onClick={next}
                loading={create.isPending}
                rightIcon={step === WIZARD_STEPS.length - 1 ? <Rocket /> : <ArrowRight />}
                data-testid="wizard-next"
              >
                {step === WIZARD_STEPS.length - 1
                  ? state.hosting === 'vps'
                    ? t('wizard.installOn', { host: vpsHost ?? '' })
                    : t('wizard.create')
                  : tc('actions.next')}
              </Button>
            </div>
          </>
        )}
      </Card>

      <Dialog
        open={confirmDiscard || blocker.status === 'blocked'}
        onOpenChange={(o) => {
          if (!o) {
            setConfirmDiscard(false);
            if (blocker.status === 'blocked') blocker.reset();
          }
        }}
        tone="destructive"
        icon={<Trash2 />}
        title={t('wizard.discard.title')}
        description={t('wizard.discard.body')}
        closeLabel={tc('actions.close')}
        footer={
          <>
            <Button
              onClick={() => {
                setConfirmDiscard(false);
                if (blocker.status === 'blocked') blocker.reset();
              }}
            >
              {t('wizard.discard.keep')}
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                if (blocker.status === 'blocked') blocker.proceed();
                else {
                  leaving.current = true;
                  setConfirmDiscard(false);
                  void navigate({ to: '/servers' });
                }
              }}
            >
              {t('wizard.discard.confirm')}
            </Button>
          </>
        }
      />
    </Page>
  );
}
