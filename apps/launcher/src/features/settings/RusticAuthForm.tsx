import {
  Button,
  Checkbox,
  Input,
  Label,
  MicrosoftButton,
  SegmentedControl,
  Select,
  toast,
} from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { MailCheck } from 'lucide-react';
import { useId, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import { qk } from '@/queries/core';
import { validateSignup, type SignupForm } from './rusticAuth';

const COUNTRIES = ['FR', 'BE', 'CH', 'LU', 'CA', 'GB', 'IE', 'DE', 'ES', 'IT', 'NL', 'US'] as const;

/**
 * Rustic account sign-in / sign-up against Rustic Cloud: "Continue with Microsoft" (done on the
 * website, which hands a single-use code back through `rustic://auth`, ADR 0090), then email +
 * password (TOTP when enabled). Tokens never reach the webview's storage: the desktop side keeps
 * them in the OS keychain. The game sign-in's Xbox token is never used for this.
 */
export function RusticAuthForm({ onSignedIn }: { onSignedIn?: () => void }) {
  const { t, i18n } = useTranslation('settings');
  const qc = useQueryClient();
  const id = useId();
  const [mode, setMode] = useState<'signin' | 'signup'>('signin');
  const [form, setForm] = useState<SignupForm>({
    email: '',
    password: '',
    username: '',
    date_of_birth: '',
    country: 'FR',
    accept_privacy: false,
  });
  const [code, setCode] = useState('');
  const [mfa, setMfa] = useState(false);
  const [sentTo, setSentTo] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const set = <K extends keyof SignupForm>(k: K, v: SignupForm[K]) =>
    setForm((f) => ({ ...f, [k]: v }));

  const done = () => {
    void qc.invalidateQueries({ queryKey: qk.rustic });
    void qc.invalidateQueries();
    toast({ tone: 'success', title: t('rusticAuth.signedIn') });
    onSignedIn?.();
  };

  const login = useMutation({
    mutationFn: () =>
      invoke('auth_rustic_login_password', { email: form.email, password: form.password }),
    onSuccess: (r) => {
      setError(null);
      if (r.mfa_required) setMfa(true);
      else {
        qc.setQueryData(qk.rustic, r.session);
        done();
      }
    },
    onError: (e) => setError(e.message),
  });
  const verifyMfa = useMutation({
    mutationFn: () => invoke('auth_rustic_login_mfa', { code }),
    onSuccess: (s) => {
      qc.setQueryData(qk.rustic, s);
      done();
    },
    onError: (e) => setError(e.message),
  });
  const signup = useMutation({
    mutationFn: () => invoke('auth_rustic_signup', { ...form, locale: i18n.language }),
    onSuccess: () => {
      setError(null);
      setSentTo(form.email);
      setMode('signin');
    },
    onError: (e) => setError(e.message),
  });
  const [waitingBrowser, setWaitingBrowser] = useState(false);
  const microsoft = useMutation({
    mutationFn: () => invoke('auth_rustic_login'),
    onSuccess: (s) => {
      setError(null);
      if (s.signed_in) {
        qc.setQueryData(qk.rustic, s);
        done();
      } else setWaitingBrowser(true);
    },
    onError: (e) => setError(e.message),
  });
  const resend = useMutation({
    mutationFn: () => invoke('auth_rustic_resend_verification', { email: sentTo ?? form.email }),
    onSuccess: () => toast({ tone: 'success', title: t('rusticAuth.resent') }),
  });

  const problems = mode === 'signup' ? validateSignup(form) : [];
  const submit = (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (mfa) return verifyMfa.mutate();
    if (mode === 'signin') return login.mutate();
    if (problems.length === 0) signup.mutate();
  };

  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={submit}
      data-testid="rustic-auth-form"
      noValidate
    >
      {!mfa && (
        <SegmentedControl
          label={t('rusticAuth.mode')}
          value={mode}
          onValueChange={(v) => {
            setMode(v as 'signin' | 'signup');
            setError(null);
          }}
          options={[
            { value: 'signin', label: t('rusticAuth.signIn') },
            { value: 'signup', label: t('rusticAuth.signUp') },
          ]}
        />
      )}
      {!mfa && (
        <div className="flex flex-col gap-2" data-testid="rustic-microsoft">
          <MicrosoftButton
            label={t('rusticAuth.microsoft')}
            theme={
              typeof document !== 'undefined' && document.documentElement.dataset.theme === 'light'
                ? 'light'
                : 'dark'
            }
            disabled={microsoft.isPending}
            onClick={() => microsoft.mutate()}
            data-testid="rustic-microsoft-signin"
          />
          {waitingBrowser && (
            <p
              role="status"
              className="text-fg-muted text-xs"
              data-testid="rustic-microsoft-waiting"
            >
              {t('rusticAuth.microsoftWaiting')}
            </p>
          )}
          <div className="text-fg-subtle flex items-center gap-3 text-xs" aria-hidden>
            <span className="bg-border h-px flex-1" />
            {t('rusticAuth.orEmail')}
            <span className="bg-border h-px flex-1" />
          </div>
        </div>
      )}
      {sentTo && (
        <div
          role="status"
          className="border-info/30 bg-info/10 flex flex-wrap items-center gap-3 rounded-xl border p-3 text-sm"
          data-testid="rustic-verify-sent"
        >
          <MailCheck className="text-info-text size-4 shrink-0" aria-hidden />
          <span className="flex-1">{t('rusticAuth.checkInbox', { email: sentTo })}</span>
          <Button
            size="sm"
            variant="ghost"
            loading={resend.isPending}
            onClick={() => resend.mutate()}
          >
            {t('rusticAuth.resend')}
          </Button>
        </div>
      )}
      {mfa ? (
        <div className="flex flex-col gap-1.5">
          <Label htmlFor={`${id}-code`}>{t('rusticAuth.code')}</Label>
          <Input
            id={`${id}-code`}
            inputMode="numeric"
            autoComplete="one-time-code"
            value={code}
            onChange={(e) => setCode(e.target.value)}
          />
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor={`${id}-email`}>{t('rusticAuth.email')}</Label>
            <Input
              id={`${id}-email`}
              type="email"
              autoComplete="email"
              value={form.email}
              onChange={(e) => set('email', e.target.value)}
              data-testid="rustic-email"
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor={`${id}-password`}>{t('rusticAuth.password')}</Label>
            <Input
              id={`${id}-password`}
              type="password"
              autoComplete={mode === 'signup' ? 'new-password' : 'current-password'}
              value={form.password}
              onChange={(e) => set('password', e.target.value)}
              data-testid="rustic-password"
            />
          </div>
          {mode === 'signup' && (
            <>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor={`${id}-username`}>{t('rusticAuth.username')}</Label>
                <Input
                  id={`${id}-username`}
                  autoComplete="username"
                  value={form.username}
                  onChange={(e) => set('username', e.target.value)}
                  data-testid="rustic-username"
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor={`${id}-dob`}>{t('rusticAuth.dateOfBirth')}</Label>
                <Input
                  id={`${id}-dob`}
                  type="date"
                  value={form.date_of_birth}
                  onChange={(e) => set('date_of_birth', e.target.value)}
                  data-testid="rustic-dob"
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor={`${id}-country`}>{t('rusticAuth.country')}</Label>
                <Select
                  id={`${id}-country`}
                  aria-label={t('rusticAuth.country')}
                  value={form.country}
                  onValueChange={(v) => set('country', v)}
                  options={COUNTRIES.map((c) => ({
                    value: c,
                    label: new Intl.DisplayNames([i18n.language], { type: 'region' }).of(c) ?? c,
                  }))}
                />
              </div>
              <label className="flex cursor-pointer items-start gap-2.5 self-end pb-2 text-sm">
                <Checkbox
                  checked={form.accept_privacy}
                  onCheckedChange={(v) => set('accept_privacy', v === true)}
                  data-testid="rustic-privacy"
                />
                <span>{t('rusticAuth.acceptPrivacy')}</span>
              </label>
            </>
          )}
        </div>
      )}
      {mode === 'signup' && !mfa && problems.length > 0 && (form.email || form.password) && (
        <ul className="text-fg-subtle list-disc pl-5 text-xs">
          {problems.map((p) => (
            <li key={p}>{t(`rusticAuth.problems.${p}`)}</li>
          ))}
        </ul>
      )}
      {error && (
        <p role="alert" className="text-danger-text text-sm" data-testid="rustic-auth-error">
          {error}
        </p>
      )}
      <div>
        <Button
          type="submit"
          variant="primary"
          loading={login.isPending || signup.isPending || verifyMfa.isPending}
          disabled={mode === 'signup' && !mfa && problems.length > 0}
          data-testid="rustic-auth-submit"
        >
          {mfa
            ? t('rusticAuth.verify')
            : mode === 'signin'
              ? t('rusticAuth.signIn')
              : t('rusticAuth.create')}
        </Button>
      </div>
    </form>
  );
}
