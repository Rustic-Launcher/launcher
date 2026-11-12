import {
  Badge,
  Button,
  cn,
  Dialog,
  Field,
  IconButton,
  Input,
  SegmentedControl,
  Skeleton,
  toast,
} from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  FolderOpen,
  KeyRound,
  Plug,
  ShieldCheck,
  ShieldQuestion,
  XCircle,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { GeneratedSshKey, VpsResources, VpsTestResult } from '@/ipc/types/hosting';
import { formatMemory } from '@/lib/format';
import { copyText, serverKeys } from '../list/queries';
import { buildVpsRequest, validateVpsForm, type VpsForm, type WizardState } from '../wizard/model';

interface Props {
  state: WizardState;
  update: (patch: Partial<WizardState>) => void;
}

function Check({ ok, warn, children }: { ok: boolean; warn?: boolean; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-2 text-sm">
      {ok ? (
        <CheckCircle2
          className={cn('mt-0.5 size-4 shrink-0', warn ? 'text-warning-text' : 'text-success-text')}
          aria-hidden
        />
      ) : (
        <XCircle className="text-danger-text mt-0.5 size-4 shrink-0" aria-hidden />
      )}
      <span>{children}</span>
    </li>
  );
}

/** Preflight results for a trusted VPS (UX_SPEC 4.4.2 step 3). */
export function Preflight({ res }: { res: VpsResources }) {
  const { t, i18n } = useTranslation('servers');
  return (
    <div
      className="border-border bg-surface/60 flex flex-col gap-2 rounded-lg border p-3"
      data-testid="vps-preflight"
    >
      <p className="flex items-center gap-2 text-sm font-semibold">
        {res.requirements_met ? (
          <CheckCircle2 className="text-success-text size-4" aria-hidden />
        ) : (
          <XCircle className="text-danger-text size-4" aria-hidden />
        )}
        {res.requirements_met ? t('hosting.preflight.ok') : t('hosting.preflight.notOk')}
      </p>
      <ul className="grid gap-1.5 sm:grid-cols-2">
        <Check ok>{t('hosting.preflight.os', { os: res.os, arch: res.arch })}</Check>
        <Check ok={res.cpu_cores >= 1}>
          {t('hosting.preflight.cpu', { count: res.cpu_cores })}
        </Check>
        <Check ok={res.memory_mb >= 2048}>
          {t('hosting.preflight.memory', { memory: formatMemory(res.memory_mb, i18n.language) })}
        </Check>
        <Check ok={res.disk_free_mb >= 4096}>
          {t('hosting.preflight.disk', { disk: formatMemory(res.disk_free_mb, i18n.language) })}
        </Check>
        <Check ok={res.has_systemd}>{t('hosting.preflight.systemd')}</Check>
        {res.public_ip && (
          <Check ok>
            {t('hosting.preflight.ip')} <code className="font-mono">{res.public_ip}</code>
          </Check>
        )}
      </ul>
      {res.problems.length > 0 && (
        <ul className="border-border flex flex-col gap-1 border-t pt-2">
          {res.problems.map((p) => (
            <li key={p.code} className="text-warning-text flex items-start gap-2 text-xs">
              <AlertTriangle className="mt-0.5 size-3.5 shrink-0" aria-hidden />
              <span>
                <strong>{t(`hosting.problems.${p.code}`)}</strong> · {p.detail}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function VpsSetup({ state, update }: Props) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const qc = useQueryClient();
  const conns = useQuery({ queryKey: serverKeys.vps, queryFn: () => invoke('hosting_vps_list') });
  const [mode, setMode] = useState<'existing' | 'new'>('existing');
  const [touched, setTouched] = useState(false);
  const [testResult, setTestResult] = useState<VpsTestResult | null>(null);
  const [confirmFp, setConfirmFp] = useState(false);
  const [generated, setGenerated] = useState<GeneratedSshKey | null>(null);
  const [resources, setResources] = useState<VpsResources | null>(null);
  const f = state.vps;
  const errors = validateVpsForm(f);
  const err = (k: string) => (touched ? errors[k] : undefined);
  const setForm = (patch: Partial<VpsForm>) => {
    update({ vps: { ...f, ...patch }, vpsConnectionId: null, vpsReady: false });
    setTestResult(null);
    setResources(null);
  };

  const effectiveMode = conns.data && conns.data.length === 0 ? 'new' : mode;

  const check = useMutation({
    mutationFn: (connection_id: string) => invoke('hosting_vps_check', { connection_id }),
    onSuccess: (res, connection_id) => {
      setResources(res);
      update({
        vpsConnectionId: connection_id,
        vpsReady: res.requirements_met,
        vpsMemoryMb: res.memory_mb,
        memoryMb: Math.min(state.memoryMb, Math.max(1024, res.memory_mb - 1024)),
      });
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('hosting.checkFailed'), description: e.message }),
  });

  const trust = useMutation({
    mutationFn: (fp: string) =>
      invoke('hosting_vps_trust', {
        ...buildVpsRequest(f),
        fingerprint_sha256: fp,
        label: f.label || undefined,
      }),
    onSuccess: (conn) => {
      setConfirmFp(false);
      void qc.invalidateQueries({ queryKey: serverKeys.vps });
      check.mutate(conn.id);
    },
  });

  const test = useMutation({
    mutationFn: () => invoke('hosting_vps_test', buildVpsRequest(f)),
    onSuccess: (r) => {
      setTestResult(r);
      if (r.reachable && r.auth_ok && r.fingerprint) {
        if (r.known) trust.mutate(r.fingerprint.sha256);
        else setConfirmFp(true);
      }
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('hosting.testFailed'), description: e.message }),
  });

  const generate = useMutation({
    mutationFn: () => invoke('hosting_vps_generate_key', { label: f.host || 'vps' }),
    onSuccess: (k) => {
      setGenerated(k);
      setForm({ keyPath: k.key_path });
    },
  });

  const copy = (text: string) =>
    void copyText(text).then((ok) =>
      toast({
        tone: ok ? 'success' : 'error',
        title: ok ? tc('actions.copied') : t('list.copyFailed'),
      }),
    );

  // Coming back to this step: re-run the preflight of the chosen connection.
  useEffect(() => {
    if (state.vpsConnectionId && !resources && !check.isPending)
      check.mutate(state.vpsConnectionId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex flex-col gap-4" data-testid="vps-setup">
      {conns.isLoading ? (
        <Skeleton className="h-9 w-72" />
      ) : (
        conns.data &&
        conns.data.length > 0 && (
          <SegmentedControl
            label={t('hosting.vps.which')}
            value={effectiveMode}
            onValueChange={(v) => {
              setMode(v as 'existing' | 'new');
              update({ vpsConnectionId: null, vpsReady: false });
              setResources(null);
            }}
            options={[
              { value: 'existing', label: t('hosting.vps.existing', { count: conns.data.length }) },
              { value: 'new', label: t('hosting.vps.new') },
            ]}
          />
        )
      )}

      {effectiveMode === 'existing' && conns.data && conns.data.length > 0 && (
        <ul className="flex flex-col gap-2">
          {conns.data.map((c) => {
            const on =
              state.vpsConnectionId === c.id || (check.isPending && check.variables === c.id);
            return (
              <li key={c.id}>
                <button
                  type="button"
                  onClick={() => check.mutate(c.id)}
                  aria-pressed={on}
                  className={cn(
                    'rx-focus-ring flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors',
                    on
                      ? 'border-accent bg-accent-soft'
                      : 'border-border hover:border-border-strong/60',
                  )}
                >
                  <ShieldCheck className="text-success-text size-5" aria-hidden />
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="text-sm font-medium">{c.label}</span>
                    <code className="text-fg-subtle truncate font-mono text-xs">
                      {c.username}@{c.host}:{c.port} · {c.fingerprint_sha256.slice(0, 20)}…
                    </code>
                  </span>
                  <Badge variant="success">{t('hosting.vps.pinned')}</Badge>
                </button>
              </li>
            );
          })}
        </ul>
      )}

      {effectiveMode === 'new' && (
        <div className="flex flex-col gap-4">
          <div className="grid gap-3 sm:grid-cols-[1fr_6rem_10rem]">
            <Field
              label={t('hosting.vps.host')}
              error={err('host') ? t('wizard.errors.host') : undefined}
            >
              {(p) => (
                <Input
                  {...p}
                  className="[&_input]:font-mono"
                  value={f.host}
                  placeholder="203.0.113.10"
                  onChange={(e) => setForm({ host: e.target.value })}
                  invalid={!!err('host')}
                  data-testid="vps-host"
                />
              )}
            </Field>
            <Field
              label={t('hosting.vps.port')}
              error={err('port') ? t('wizard.errors.port') : undefined}
            >
              {(p) => (
                <Input
                  {...p}
                  className="[&_input]:font-mono"
                  inputMode="numeric"
                  value={f.port}
                  onChange={(e) => setForm({ port: e.target.value })}
                  invalid={!!err('port')}
                />
              )}
            </Field>
            <Field
              label={t('hosting.vps.user')}
              error={err('username') ? t('wizard.errors.username') : undefined}
            >
              {(p) => (
                <Input
                  {...p}
                  className="[&_input]:font-mono"
                  value={f.username}
                  placeholder={t('hosting.vps.userPlaceholder')}
                  onChange={(e) => setForm({ username: e.target.value })}
                  invalid={!!err('username')}
                />
              )}
            </Field>
          </div>

          <fieldset className="flex flex-col gap-2">
            <legend className="mb-2 text-sm font-medium">{t('hosting.vps.auth')}</legend>
            <SegmentedControl
              label={t('hosting.vps.auth')}
              value={f.authKind}
              onValueChange={(v) => {
                setGenerated(null);
                setForm({ authKind: v as VpsForm['authKind'], keyPath: '' });
              }}
              options={[
                { value: 'generate', label: t('hosting.vps.authGenerate') },
                { value: 'key', label: t('hosting.vps.authKey') },
                { value: 'password', label: t('hosting.vps.authPassword') },
              ]}
            />
            {f.authKind === 'generate' && (
              <div className="border-border bg-surface/60 flex flex-col gap-2 rounded-lg border p-3">
                <p className="text-fg-muted text-xs">{t('hosting.vps.generateHint')}</p>
                {!generated ? (
                  <Button
                    className="w-fit"
                    leftIcon={<KeyRound />}
                    loading={generate.isPending}
                    onClick={() => generate.mutate()}
                    data-testid="vps-generate-key"
                  >
                    {t('hosting.vps.generate')}
                  </Button>
                ) : (
                  <>
                    <p className="text-xs font-medium">{t('hosting.vps.addKeyCommand')}</p>
                    <div className="flex items-start gap-2">
                      <code className="bg-bg text-2xs text-fg-muted flex-1 rounded-md px-2.5 py-2 font-mono break-all">
                        {generated.authorize_command}
                      </code>
                      <IconButton
                        size="sm"
                        label={tc('actions.copy')}
                        icon={<Copy />}
                        onClick={() => copy(generated.authorize_command)}
                      />
                    </div>
                    <p className="text-fg-subtle text-xs">
                      {t('hosting.vps.keyStored', { path: generated.key_path })}
                    </p>
                  </>
                )}
                {err('keyPath') && (
                  <p className="text-danger-text text-xs">{t('wizard.errors.generateKey')}</p>
                )}
              </div>
            )}
            {f.authKind === 'key' && (
              <div className="grid gap-3 sm:grid-cols-[1fr_12rem]">
                <Field
                  label={t('hosting.vps.keyFile')}
                  error={err('keyPath') ? t('wizard.errors.keyPath') : undefined}
                >
                  {(p) => (
                    <div className="flex gap-2">
                      <Input
                        {...p}
                        className="flex-1 [&_input]:font-mono"
                        value={f.keyPath}
                        placeholder="~/.ssh/id_ed25519"
                        onChange={(e) => setForm({ keyPath: e.target.value })}
                        invalid={!!err('keyPath')}
                      />
                      <IconButton
                        variant="secondary"
                        label={tc('actions.browse')}
                        icon={<FolderOpen />}
                        onClick={() =>
                          void invoke('app_pick_path', {
                            kind: 'file',
                            title: t('hosting.vps.keyFile'),
                          }).then((path) => path && setForm({ keyPath: path }))
                        }
                      />
                    </div>
                  )}
                </Field>
                <Field label={t('hosting.vps.passphrase')} hint={t('hosting.vps.optional')}>
                  {(p) => (
                    <Input
                      {...p}
                      type="password"
                      autoComplete="off"
                      value={f.passphrase}
                      onChange={(e) => setForm({ passphrase: e.target.value })}
                    />
                  )}
                </Field>
              </div>
            )}
            {f.authKind === 'password' && (
              <Field
                label={t('hosting.vps.password')}
                hint={t('hosting.vps.passwordHint')}
                error={err('password') ? t('wizard.errors.password') : undefined}
              >
                {(p) => (
                  <Input
                    {...p}
                    type="password"
                    autoComplete="off"
                    value={f.password}
                    onChange={(e) => setForm({ password: e.target.value })}
                    invalid={!!err('password')}
                    data-testid="vps-password"
                  />
                )}
              </Field>
            )}
          </fieldset>

          <div className="flex flex-wrap items-center gap-3">
            <Button
              variant="primary"
              leftIcon={<Plug />}
              loading={test.isPending || trust.isPending}
              onClick={() => {
                setTouched(true);
                if (Object.keys(errors).length === 0) test.mutate();
              }}
              data-testid="vps-test"
            >
              {t('hosting.vps.test')}
            </Button>
            {testResult && testResult.reachable && testResult.auth_ok && (
              <span className="text-success-text flex items-center gap-1.5 text-sm">
                <CheckCircle2 className="size-4" aria-hidden />
                {t('hosting.vps.reachable', { ms: testResult.latency_ms ?? 0 })}
              </span>
            )}
          </div>
          {testResult && (!testResult.reachable || !testResult.auth_ok) && (
            <div
              role="alert"
              className="border-danger/40 bg-danger/10 flex items-start gap-2 rounded-lg border p-3 text-sm"
            >
              <XCircle className="text-danger-text mt-0.5 size-4 shrink-0" aria-hidden />
              <div className="flex flex-col gap-0.5">
                <strong>
                  {!testResult.reachable
                    ? t('hosting.vps.unreachable')
                    : t('hosting.vps.authFailed')}
                </strong>
                <span className="text-fg-muted text-xs">{testResult.error}</span>
              </div>
            </div>
          )}
        </div>
      )}

      {check.isPending && (
        <div className="flex flex-col gap-2" aria-busy="true">
          <p className="text-fg-muted text-sm">{t('hosting.preflight.running')}</p>
          <Skeleton className="h-28" />
        </div>
      )}
      {!check.isPending && resources && state.vpsConnectionId && <Preflight res={resources} />}

      <Dialog
        open={confirmFp}
        onOpenChange={setConfirmFp}
        icon={<ShieldQuestion />}
        title={t('hosting.fingerprint.title', { host: f.host })}
        description={t('hosting.fingerprint.description')}
        closeLabel={tc('actions.close')}
        footer={
          <>
            <Button onClick={() => setConfirmFp(false)}>{tc('actions.cancel')}</Button>
            <Button
              variant="primary"
              leftIcon={<ShieldCheck />}
              loading={trust.isPending}
              onClick={() => testResult?.fingerprint && trust.mutate(testResult.fingerprint.sha256)}
              data-testid="vps-trust"
            >
              {t('hosting.fingerprint.trust')}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          <div className="border-border bg-bg rounded-lg border p-3">
            <p className="text-2xs text-fg-subtle font-semibold tracking-wider uppercase">
              {testResult?.fingerprint?.algorithm}
            </p>
            <code className="text-fg font-mono text-sm break-all" data-testid="vps-fingerprint">
              {testResult?.fingerprint?.sha256}
            </code>
          </div>
          <p className="text-warning-text flex items-start gap-2 text-xs">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0" aria-hidden />
            {t('hosting.fingerprint.warning')}
          </p>
        </div>
      </Dialog>
    </div>
  );
}
