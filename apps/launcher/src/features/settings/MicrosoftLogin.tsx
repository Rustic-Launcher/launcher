/** Microsoft device-code sign-in with the ownership (entitlement) result. */
import { Button, toast } from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Copy, ExternalLink, ShieldCheck } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { Account, MicrosoftLoginStart } from '@/ipc/types/auth';
import { qk } from '@/queries/core';
import { loginFailureDetail, loginFailureReason, type FailureReason } from './validation';

type Phase =
  | { kind: 'idle' }
  | { kind: 'starting' }
  | { kind: 'waiting'; flow: MicrosoftLoginStart }
  | { kind: 'success'; account: Account }
  | { kind: 'failed'; reason: FailureReason; detail?: string };

export function MicrosoftLogin({
  onSuccess,
  autoStart = false,
  onCancel,
}: {
  onSuccess?: (a: Account) => void;
  autoStart?: boolean;
  onCancel?: () => void;
}) {
  const { t } = useTranslation('settings');
  const qc = useQueryClient();
  const [phase, setPhase] = useState<Phase>({ kind: 'idle' });
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const start = async () => {
    setPhase({ kind: 'starting' });
    try {
      const flow = await invoke('auth_microsoft_login_start');
      if (!alive.current) return;
      setPhase({ kind: 'waiting', flow });
      for (;;) {
        const r = await invoke('auth_microsoft_login_poll', { flow_id: flow.flow_id });
        if (!alive.current) return;
        if (r.status === 'pending') continue;
        if (r.status === 'success' && r.account) {
          setPhase({ kind: 'success', account: r.account });
          void qc.invalidateQueries({ queryKey: qk.accounts });
          onSuccess?.(r.account);
        } else {
          setPhase({
            kind: 'failed',
            reason: r.status === 'success' ? 'error' : r.status,
            detail: r.error ?? undefined,
          });
        }
        return;
      }
    } catch (e) {
      if (alive.current)
        setPhase({ kind: 'failed', reason: loginFailureReason(e), detail: loginFailureDetail(e) });
    }
  };

  useEffect(() => {
    if (autoStart) void start();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (phase.kind === 'idle' || phase.kind === 'starting') {
    return (
      <Button
        variant="primary"
        loading={phase.kind === 'starting'}
        onClick={() => void start()}
        data-testid="ms-login-start"
      >
        {t('accounts.microsoft.signIn')}
      </Button>
    );
  }

  if (phase.kind === 'waiting') {
    const { flow } = phase;
    return (
      <div
        className="border-border bg-surface-2/60 flex flex-col gap-4 rounded-xl border p-4"
        data-testid="ms-device-code"
      >
        <ol className="text-fg-muted flex list-decimal flex-col gap-1 pl-5 text-sm">
          <li>{t('accounts.microsoft.step1')}</li>
          <li>{t('accounts.microsoft.step2')}</li>
          <li>{t('accounts.microsoft.step3')}</li>
        </ol>
        <div className="flex flex-wrap items-center gap-3">
          <code
            aria-label={t('accounts.microsoft.codeLabel')}
            className="border-accent/40 bg-accent-soft text-accent-text rounded-lg border px-4 py-2 font-mono text-2xl font-bold tracking-[0.2em]"
          >
            {flow.user_code}
          </code>
          <Button
            size="sm"
            leftIcon={<Copy />}
            onClick={() => {
              void navigator.clipboard?.writeText(flow.user_code).catch(() => undefined);
              toast({ tone: 'success', title: t('accounts.microsoft.copied') });
            }}
          >
            {t('accounts.microsoft.copyCode')}
          </Button>
          <Button
            size="sm"
            variant="soft"
            rightIcon={<ExternalLink />}
            onClick={() => void invoke('app_open_url', { url: flow.verification_uri })}
          >
            {t('accounts.microsoft.openPage')}
          </Button>
        </div>
        <p className="text-fg-subtle flex items-center gap-2 text-xs" role="status">
          <span className="animate-pulse-soft bg-accent size-2 rounded-full" aria-hidden />
          {t('accounts.microsoft.waiting')}
        </p>
        {onCancel && (
          <Button variant="ghost" size="sm" className="w-fit" onClick={onCancel}>
            {t('accounts.microsoft.cancel')}
          </Button>
        )}
      </div>
    );
  }

  if (phase.kind === 'success') {
    const owns = phase.account.ownership?.owns_game;
    return (
      <div
        role="status"
        data-testid="ms-login-success"
        className="border-success/30 bg-success/10 flex items-start gap-3 rounded-xl border p-4"
      >
        <ShieldCheck className="text-success-text mt-0.5 size-5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-0.5">
          <span className="text-fg text-sm font-semibold">
            {t('accounts.microsoft.signedInAs', { name: phase.account.username })}
          </span>
          <span className="text-fg-muted text-xs">
            {owns ? t('accounts.microsoft.ownershipOk') : t('accounts.microsoft.ownershipMissing')}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div
      role="alert"
      className="border-danger/30 bg-danger/10 flex flex-col gap-3 rounded-xl border p-4"
    >
      <div className="flex items-start gap-3">
        <AlertTriangle className="text-danger-text mt-0.5 size-5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-0.5">
          <span className="text-fg text-sm font-semibold">
            {t(`accounts.microsoft.failed.${phase.reason}.title`)}
          </span>
          <span className="text-fg-muted text-xs">
            {t(`accounts.microsoft.failed.${phase.reason}.body`)}
          </span>
          {phase.detail && (
            <span className="text-fg-subtle mt-1 font-mono text-[11px] break-words select-text">
              {t('accounts.microsoft.failed.detail', { detail: phase.detail })}
            </span>
          )}
        </div>
      </div>
      <Button size="sm" className="w-fit" onClick={() => void start()}>
        {t('accounts.microsoft.tryAgain')}
      </Button>
    </div>
  );
}
