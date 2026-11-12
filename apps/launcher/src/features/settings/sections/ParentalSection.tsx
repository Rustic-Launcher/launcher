import { Button, Input, SegmentedControl, Select, Skeleton, Switch, toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { KeyRound, Lock, ShieldCheck, Unlock } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import type { ParentalControls } from '@/cloud/types';
import { QueryError } from '@/components/RouteError';
import { formatMoney } from '@/lib/format';
import { useMe } from '@/queries/social';
import { SettingRow, SettingsCard } from '../components';
import { MIN_ACCOUNT_AGE } from '../legal';
import { parseAmountMinor, validatePin } from '../validation';

export const parentalKey = ['parental'] as const;

export function ParentalSection() {
  const { t } = useTranslation('settings');
  const qc = useQueryClient();
  const { data: me } = useMe();
  const q = useQuery({ queryKey: parentalKey, queryFn: async () => (await cloud()).parental() });
  const [unlocked, setUnlocked] = useState(false);
  const update = useMutation({
    mutationFn: async (patch: Partial<ParentalControls> & { pin?: string }) =>
      (await cloud()).setParental(patch),
    onSuccess: (p) => qc.setQueryData(parentalKey, p),
    onError: () => toast({ tone: 'error', title: t('parental.saveFailed') }),
  });

  if (q.isLoading) return <Skeleton className="h-72 rounded-2xl" />;
  if (q.isError || !q.data) return <QueryError onRetry={() => void q.refetch()} />;
  const p = q.data;
  const locked = p.enabled && p.pin_set && !unlocked;
  const minor = me && me.age_group !== 'adult';

  return (
    <div className="flex flex-col gap-4">
      <SettingsCard
        title={t('parental.title')}
        description={t('parental.description', { age: MIN_ACCOUNT_AGE })}
        testId="settings-parental"
      >
        {minor && (
          <p
            className="border-info/30 bg-info/10 text-fg-muted mb-3 rounded-xl border p-3 text-sm"
            role="note"
          >
            {t('parental.minorNote')}
          </p>
        )}
        <SettingRow
          label={t('parental.enable')}
          description={t('parental.enableHint')}
          htmlFor="parental-enable"
        >
          <Switch
            id="parental-enable"
            checked={p.enabled}
            disabled={locked || (!p.enabled && !p.pin_set)}
            onCheckedChange={(v) => update.mutate({ enabled: v })}
          />
        </SettingRow>
        {!p.pin_set && <p className="text-fg-subtle text-xs">{t('parental.pinFirst')}</p>}
      </SettingsCard>

      <PinCard pinSet={p.pin_set} locked={locked} onSave={(pin) => update.mutateAsync({ pin })} />

      {locked ? (
        <UnlockCard onUnlocked={() => setUnlocked(true)} />
      ) : (
        <SettingsCard title={t('parental.rules.title')}>
          <SettingRow
            label={t('parental.rules.purchases')}
            description={t('parental.rules.purchasesHint')}
          >
            <SegmentedControl
              className="isolate"
              size="sm"
              label={t('parental.rules.purchases')}
              value={p.purchases}
              onValueChange={(v) =>
                update.mutate({ purchases: v as ParentalControls['purchases'] })
              }
              options={(['blocked', 'pin', 'allowed'] as const).map((v) => ({
                value: v,
                label: t(`parental.rules.purchaseOptions.${v}`),
              }))}
            />
          </SettingRow>
          <LimitRow p={p} onSave={(m) => update.mutate({ monthly_limit: m })} />
          <SettingRow label={t('parental.rules.chat')} htmlFor="parental-chat">
            <Select
              id="parental-chat"
              className="w-52"
              value={p.chat}
              onValueChange={(v) => update.mutate({ chat: v as ParentalControls['chat'] })}
              options={(['off', 'friends', 'everyone'] as const).map((v) => ({
                value: v,
                label: t(`parental.rules.chatOptions.${v}`),
              }))}
            />
          </SettingRow>
          <SettingRow
            label={t('parental.rules.friendRequests')}
            description={t('parental.rules.friendRequestsHint')}
            htmlFor="parental-friends"
          >
            <Switch
              id="parental-friends"
              checked={p.friend_requests}
              onCheckedChange={(v) => update.mutate({ friend_requests: v })}
            />
          </SettingRow>
          {unlocked && (
            <Button
              size="sm"
              variant="ghost"
              className="mt-2"
              leftIcon={<Lock />}
              onClick={() => setUnlocked(false)}
            >
              {t('parental.lockAgain')}
            </Button>
          )}
        </SettingsCard>
      )}
    </div>
  );
}

function LimitRow({
  p,
  onSave,
}: {
  p: ParentalControls;
  onSave: (m: ParentalControls['monthly_limit']) => void;
}) {
  const { t, i18n } = useTranslation('settings');
  const currency = p.spent_this_month.currency;
  const [value, setValue] = useState(
    p.monthly_limit ? String(p.monthly_limit.amount_minor / 100) : '',
  );
  const parsed = parseAmountMinor(value);
  const invalid = parsed != null && Number.isNaN(parsed);
  return (
    <SettingRow
      label={t('parental.rules.limit')}
      description={t('parental.rules.limitHint', {
        spent: formatMoney(p.spent_this_month, i18n.language),
      })}
      htmlFor="parental-limit"
    >
      <div className="flex flex-col items-end gap-1">
        <Input
          id="parental-limit"
          inputMode="decimal"
          className="w-36"
          value={value}
          invalid={invalid}
          placeholder={t('parental.rules.noLimit')}
          disabled={p.purchases === 'blocked'}
          rightSlot={<span className="text-fg-subtle text-xs">{currency}</span>}
          onChange={(e) => setValue(e.target.value)}
          onBlur={() => {
            if (invalid) return;
            onSave(parsed == null ? null : { amount_minor: parsed, currency });
          }}
        />
        {invalid && (
          <span className="text-danger-text text-xs">{t('parental.rules.limitInvalid')}</span>
        )}
      </div>
    </SettingRow>
  );
}

function PinCard({
  pinSet,
  locked,
  onSave,
}: {
  pinSet: boolean;
  locked: boolean;
  onSave: (pin: string) => Promise<unknown>;
}) {
  const { t } = useTranslation('settings');
  const [open, setOpen] = useState(false);
  const [pin, setPin] = useState('');
  const [confirm, setConfirm] = useState('');
  const [touched, setTouched] = useState(false);
  const [saving, setSaving] = useState(false);
  const err = validatePin(pin, confirm);
  return (
    <SettingsCard
      title={t('parental.pin.title')}
      description={pinSet ? t('parental.pin.set') : t('parental.pin.notSet')}
      action={
        !open && (
          <Button
            size="sm"
            leftIcon={<KeyRound />}
            disabled={locked}
            onClick={() => setOpen(true)}
            data-testid="pin-open"
          >
            {pinSet ? t('parental.pin.change') : t('parental.pin.create')}
          </Button>
        )
      }
    >
      {open && (
        <form
          className="flex flex-wrap items-start gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            setTouched(true);
            if (err) return;
            setSaving(true);
            try {
              await onSave(pin);
              toast({ tone: 'success', title: t('parental.pin.saved') });
              setOpen(false);
              setPin('');
              setConfirm('');
              setTouched(false);
            } finally {
              setSaving(false);
            }
          }}
        >
          <div className="flex flex-col gap-1">
            <label htmlFor="pin-new" className="text-fg-muted text-xs font-medium">
              {t('parental.pin.new')}
            </label>
            <Input
              id="pin-new"
              type="password"
              inputMode="numeric"
              autoComplete="new-password"
              maxLength={6}
              className="w-36"
              value={pin}
              onChange={(e) => setPin(e.target.value.replace(/\D/g, ''))}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label htmlFor="pin-confirm" className="text-fg-muted text-xs font-medium">
              {t('parental.pin.confirm')}
            </label>
            <Input
              id="pin-confirm"
              type="password"
              inputMode="numeric"
              autoComplete="new-password"
              maxLength={6}
              className="w-36"
              value={confirm}
              invalid={touched && !!err}
              onChange={(e) => setConfirm(e.target.value.replace(/\D/g, ''))}
            />
          </div>
          <div className="flex items-end gap-2 self-end">
            <Button type="submit" variant="primary" loading={saving}>
              {t('parental.pin.save')}
            </Button>
            <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
              {t('cancel')}
            </Button>
          </div>
          <p
            className={`w-full text-xs ${touched && err ? 'text-danger-text' : 'text-fg-subtle'}`}
            role={touched && err ? 'alert' : undefined}
          >
            {touched && err ? t(`parental.pin.errors.${err}`) : t('parental.pin.hint')}
          </p>
        </form>
      )}
    </SettingsCard>
  );
}

function UnlockCard({ onUnlocked }: { onUnlocked: () => void }) {
  const { t } = useTranslation('settings');
  const [pin, setPin] = useState('');
  const [wrong, setWrong] = useState(false);
  const verify = useMutation({
    mutationFn: async (v: string) => (await cloud()).verifyParentalPin(v),
    onSuccess: (ok) => {
      if (ok) onUnlocked();
      else setWrong(true);
    },
  });
  return (
    <SettingsCard title={t('parental.unlock.title')} description={t('parental.unlock.description')}>
      <form
        className="flex flex-wrap items-start gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          setWrong(false);
          verify.mutate(pin);
        }}
      >
        <div className="flex flex-col gap-1">
          <label htmlFor="pin-unlock" className="sr-only">
            {t('parental.unlock.label')}
          </label>
          <Input
            id="pin-unlock"
            type="password"
            inputMode="numeric"
            maxLength={6}
            className="w-40"
            leftIcon={<ShieldCheck />}
            value={pin}
            invalid={wrong}
            onChange={(e) => setPin(e.target.value.replace(/\D/g, ''))}
          />
          {wrong && (
            <span className="text-danger-text text-xs" role="alert">
              {t('parental.unlock.wrong')}
            </span>
          )}
        </div>
        <Button
          type="submit"
          variant="primary"
          leftIcon={<Unlock />}
          loading={verify.isPending}
          disabled={pin.length < 4}
        >
          {t('parental.unlock.button')}
        </Button>
      </form>
    </SettingsCard>
  );
}
