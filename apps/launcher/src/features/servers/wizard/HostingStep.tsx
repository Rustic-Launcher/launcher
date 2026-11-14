import { Badge, Button, cn, Input, Skeleton, Slider, Switch, toast } from '@rustic/ui';
import { useMutation, useQuery } from '@tanstack/react-query';
import {
  CheckCircle2,
  Cloud,
  Laptop,
  ListChecks,
  MemoryStick,
  Server,
  XCircle,
} from 'lucide-react';
import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import { formatMemory } from '@/lib/format';
import { qk, useSystemInfo } from '@/queries/core';
import { VpsSetup } from '../hosting/VpsSetup';
import { memoryBounds, type FieldErrors, type WizardState } from './model';
import { RadioCard } from './RadioCard';
import { WizardSection } from './Section';

interface StepProps {
  state: WizardState;
  update: (patch: Partial<WizardState>) => void;
  errors: FieldErrors;
  showErrors: boolean;
}

function Waitlist() {
  const { t } = useTranslation('servers');
  const [email, setEmail] = useState('');
  const join = useMutation({
    mutationFn: () => invoke('hosting_waitlist_join', { email: email.trim() }),
    onError: () => toast({ tone: 'error', title: t('hosting.cloud.invalidEmail') }),
  });
  if (join.data) {
    return (
      <p className="text-success-text flex items-center gap-2 text-sm" role="status">
        <CheckCircle2 className="size-4" aria-hidden />
        {t('hosting.cloud.joined', { position: join.data.position })}
      </p>
    );
  }
  return (
    <form
      className="flex gap-2"
      onSubmit={(e: FormEvent) => {
        e.preventDefault();
        if (email.trim()) join.mutate();
      }}
    >
      <Input
        type="email"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        placeholder={t('hosting.cloud.email')}
        aria-label={t('hosting.cloud.email')}
        className="flex-1"
      />
      <Button type="submit" size="md" loading={join.isPending}>
        {t('hosting.cloud.join')}
      </Button>
    </form>
  );
}

function Requirement({ ok, label }: { ok: boolean | null; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      {ok == null ? (
        <Skeleton className="size-4 rounded-full" />
      ) : ok ? (
        <CheckCircle2 className="text-success-text size-4" aria-hidden />
      ) : (
        <XCircle className="text-danger-text size-4" aria-hidden />
      )}
      {label}
    </span>
  );
}

export function HostingStep({ state, update, errors, showErrors }: StepProps) {
  const { t, i18n } = useTranslation('servers');
  const system = useSystemInfo();
  const prepare = useQuery({ queryKey: qk.prepare, queryFn: () => invoke('game_prepare_status') });
  const bounds = memoryBounds(state.hosting, system.data?.total_memory_mb, state.vpsMemoryMb);
  const fmt = (mb: number) => formatMemory(mb, i18n.language);
  const ramOk = system.data ? system.data.available_memory_mb >= state.memoryMb : null;

  return (
    <div className="flex flex-col gap-4">
      <div
        className="grid gap-3 @xl:grid-cols-3"
        role="radiogroup"
        aria-label={t('wizard.steps.hosting')}
      >
        <RadioCard
          name="hosting"
          value="local"
          art="hosting-local"
          checked={state.hosting === 'local'}
          onChange={() => update({ hosting: 'local' })}
          icon={<Laptop />}
          title={t('hosting.local.title')}
          badge={<Badge variant="success">{t('hosting.local.free')}</Badge>}
          description={t('hosting.local.description')}
          testId="hosting-local"
        />
        <RadioCard
          name="hosting"
          value="vps"
          art="ember"
          checked={state.hosting === 'vps'}
          onChange={() => update({ hosting: 'vps' })}
          icon={<Server />}
          title={t('hosting.vps.title')}
          description={t('hosting.vps.description')}
          testId="hosting-vps"
        />
        <RadioCard
          name="hosting"
          value="cloud"
          art="sky"
          checked={false}
          disabled
          onChange={() => undefined}
          icon={<Cloud />}
          title={t('hosting.cloud.title')}
          badge={<Badge variant="info">{t('hosting.cloud.soon')}</Badge>}
          description={t('hosting.cloud.description')}
          testId="hosting-cloud"
        />
      </div>

      {showErrors && errors.hosting && (
        <p role="alert" className="text-danger-text text-sm">
          {t(errors.hosting as 'wizard.errors.vpsNotReady')}
        </p>
      )}

      <WizardSection
        icon={state.hosting === 'vps' ? <Server /> : <Laptop />}
        title={state.hosting === 'vps' ? t('hosting.vps.title') : t('hosting.local.settings')}
        hint={state.hosting === 'local' ? t('hosting.local.reach') : undefined}
      >
        {state.hosting === 'local' && (
          <label className="bg-surface-2/60 flex items-center justify-between gap-4 rounded-lg px-3 py-2.5 text-sm">
            <span className="flex flex-col">
              <span className="font-medium">{t('hosting.local.startWithLauncher')}</span>
              <span className="text-fg-muted text-xs">
                {t('hosting.local.startWithLauncherHint')}
              </span>
            </span>
            <Switch
              checked={state.startWithLauncher}
              onCheckedChange={(v) => update({ startWithLauncher: v })}
            />
          </label>
        )}
        {state.hosting === 'vps' && (
          <>
            <VpsSetup state={state} update={update} />
            <label className="bg-surface-2/60 flex items-center justify-between gap-4 rounded-lg px-3 py-2.5 text-sm">
              <span className="flex flex-col">
                <span className="font-medium">{t('hosting.vps.openFirewall')}</span>
                <span className="text-fg-muted text-xs">{t('hosting.vps.openFirewallHint')}</span>
              </span>
              <Switch
                checked={state.vpsOpenFirewall}
                onCheckedChange={(v) => update({ vpsOpenFirewall: v })}
                data-testid="vps-open-firewall"
              />
            </label>
          </>
        )}
      </WizardSection>

      <WizardSection
        icon={<MemoryStick />}
        title={t('wizard.type.memory')}
        hint={t('wizard.type.memoryHint')}
        action={
          <span className="text-accent-text bg-accent-soft rounded-lg px-2.5 py-1 font-mono text-sm font-semibold tabular-nums">
            {fmt(state.memoryMb)}
          </span>
        }
      >
        <Slider
          label={t('wizard.type.memory')}
          value={Math.min(state.memoryMb, bounds.max)}
          onValueChange={(v) => update({ memoryMb: v })}
          min={bounds.min}
          max={bounds.max}
          step={512}
          formatValue={fmt}
          warnAbove={Math.round(bounds.max * 0.75)}
          marks={[bounds.min, bounds.max].map((v) => ({
            value: v,
            label: <span className="whitespace-nowrap">{fmt(v)}</span>,
          }))}
        />
      </WizardSection>

      <div className={cn('grid gap-4', state.hosting === 'local' && '@2xl:grid-cols-2')}>
        {state.hosting === 'local' && (
          <WizardSection
            icon={<ListChecks />}
            title={t('hosting.requirements.title')}
            hint={t('hosting.requirements.hint')}
          >
            <div className="flex flex-col gap-2 text-sm" data-testid="requirements">
              <Requirement
                ok={ramOk}
                label={t('hosting.requirements.ram', {
                  free: system.data ? fmt(system.data.available_memory_mb) : '…',
                })}
              />
              <Requirement
                ok={prepare.data ? prepare.data.state === 'ready' : null}
                label={t('hosting.requirements.gameFiles')}
              />
              <Requirement ok label={t('hosting.requirements.port')} />
            </div>
          </WizardSection>
        )}
        <WizardSection
          icon={<Cloud />}
          title={t('hosting.cloud.title')}
          hint={t('hosting.cloud.waitlist')}
          action={<Badge variant="info">{t('hosting.cloud.soon')}</Badge>}
        >
          <Waitlist />
        </WizardSection>
      </div>
    </div>
  );
}
