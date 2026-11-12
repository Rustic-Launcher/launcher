import { Button, Field, Input } from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ExternalLink, KeyRound, LockKeyhole } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import { mk } from '../queries';

export const PORKBUN_API_URL = 'https://porkbun.com/account/api';

/** Connect the user's own Porkbun account with an API key pair (kept in the OS keychain). */
export function PorkbunConnect() {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const [apiKey, setApiKey] = useState('');
  const [secret, setSecret] = useState('');
  const connect = useMutation({
    mutationFn: () =>
      invoke('dns_connect_porkbun', { api_key: apiKey.trim(), secret_api_key: secret.trim() }),
    onSuccess: (c) => {
      qc.setQueryData(mk.porkbun, c);
      setApiKey('');
      setSecret('');
    },
  });
  const err = connect.error instanceof IpcFailure ? connect.error : null;
  const field = err?.details?.field as string | undefined;

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        connect.mutate();
      }}
      data-testid="porkbun-connect"
    >
      <ol className="text-fg-muted flex flex-col gap-2 text-sm">
        <li className="flex gap-2">
          <span className="bg-accent-soft text-2xs text-accent-text grid size-5 shrink-0 place-items-center rounded-full font-bold">
            1
          </span>
          <span>
            {t('domains.porkbun.step1')}{' '}
            <button
              type="button"
              onClick={() => void invoke('app_open_url', { url: PORKBUN_API_URL })}
              className="rx-focus-ring text-accent-text inline-flex items-center gap-1 rounded font-medium hover:underline"
            >
              {t('domains.porkbun.openApi')}
              <ExternalLink className="size-3" aria-hidden />
            </button>
          </span>
        </li>
        <li className="flex gap-2">
          <span className="bg-accent-soft text-2xs text-accent-text grid size-5 shrink-0 place-items-center rounded-full font-bold">
            2
          </span>
          <span>{t('domains.porkbun.step2')}</span>
        </li>
      </ol>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <Field
          label={t('domains.porkbun.apiKey')}
          error={field === 'api_key' ? t('domains.porkbun.badApiKey') : undefined}
        >
          {(p) => (
            <Input
              {...p}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="pk1_…"
              autoComplete="off"
              spellCheck={false}
              leftIcon={<KeyRound />}
              data-testid="porkbun-api-key"
              className="font-mono"
            />
          )}
        </Field>
        <Field
          label={t('domains.porkbun.secret')}
          error={field === 'secret_api_key' ? t('domains.porkbun.badSecret') : undefined}
        >
          {(p) => (
            <Input
              {...p}
              type="password"
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              placeholder="sk1_…"
              autoComplete="off"
              leftIcon={<LockKeyhole />}
              data-testid="porkbun-secret"
              className="font-mono"
            />
          )}
        </Field>
      </div>
      <p className="text-fg-subtle flex items-start gap-2 text-xs">
        <LockKeyhole className="mt-0.5 size-3.5 shrink-0" aria-hidden />
        {t('domains.porkbun.keychain')}
      </p>
      {err && !field && (
        <p role="alert" className="text-danger-text text-sm">
          {err.details?.reason === 'porkbun_rejected' ? t('domains.porkbun.rejected') : err.message}
        </p>
      )}
      <Button
        type="submit"
        variant="primary"
        className="w-fit"
        disabled={!apiKey.trim() || !secret.trim()}
        loading={connect.isPending}
        data-testid="porkbun-connect-submit"
      >
        {t('domains.porkbun.connect')}
      </Button>
    </form>
  );
}
