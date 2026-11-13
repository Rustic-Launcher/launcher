/** The trust notice before a native mod's first build in an instance, and Rust's installation. */
import { Button, Dialog } from '@rustic/ui';
import { Cpu, ExternalLink, ShieldAlert } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { NativeToolchain } from '@/ipc/types/native';

export function NativeTrustDialog({
  open,
  onOpenChange,
  modName,
  instanceName,
  sourceUrl,
  pending,
  onAccept,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  modName: string;
  instanceName: string;
  sourceUrl?: string | null;
  pending: boolean;
  onAccept: () => void;
}) {
  const { t } = useTranslation('content');
  const { t: tc } = useTranslation();
  const [understood, setUnderstood] = useState(false);
  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) setUnderstood(false);
        onOpenChange(o);
      }}
      tone="destructive"
      icon={<ShieldAlert />}
      title={t('native.trust.title')}
      description={t('native.trust.lead', { name: modName, instance: instanceName })}
      closeLabel={tc('actions.close')}
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>{tc('actions.cancel')}</Button>
          <Button
            variant="danger"
            leftIcon={<Cpu />}
            disabled={!understood}
            loading={pending}
            data-testid="native-trust-accept"
            onClick={onAccept}
          >
            {t('native.trust.accept')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3 text-sm" data-testid="native-trust-dialog">
        <ul className="text-fg-muted flex list-disc flex-col gap-1.5 pl-5">
          <li>{t('native.trust.unsandboxed')}</li>
          <li>{t('native.trust.review')}</li>
          <li>{t('native.trust.build')}</li>
          <li>{t('native.trust.scope')}</li>
        </ul>
        {sourceUrl && (
          <button
            type="button"
            className="text-accent-text rx-focus-ring inline-flex items-center gap-1.5 self-start rounded text-sm hover:underline"
            onClick={() => void invoke('app_open_url', { url: sourceUrl })}
          >
            <ExternalLink className="size-3.5" />
            {t('native.trust.source')}
          </button>
        )}
        <label className="bg-surface-2 flex cursor-pointer items-start gap-2.5 rounded-xl p-3">
          <input
            type="checkbox"
            className="accent-accent mt-0.5 size-4"
            checked={understood}
            onChange={(e) => setUnderstood(e.target.checked)}
            data-testid="native-trust-check"
          />
          <span className="text-fg">{t('native.trust.check')}</span>
        </label>
      </div>
    </Dialog>
  );
}

export function NativeToolchainDialog({
  open,
  onOpenChange,
  toolchain,
  pending,
  onInstall,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  toolchain: NativeToolchain | null;
  pending: boolean;
  onInstall: () => void;
}) {
  const { t } = useTranslation('content');
  const { t: tc } = useTranslation();
  const installable = toolchain?.installable ?? false;
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      icon={<Cpu />}
      title={t('native.toolchain.title', { version: toolchain?.version ?? '' })}
      description={installable ? t('native.toolchain.lead') : t('native.toolchain.manual')}
      closeLabel={tc('actions.close')}
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>{tc('actions.cancel')}</Button>
          {installable && (
            <Button
              variant="primary"
              loading={pending}
              data-testid="native-toolchain-install"
              onClick={onInstall}
            >
              {t('native.toolchain.install')}
            </Button>
          )}
        </>
      }
    >
      <div className="flex flex-col gap-2 text-sm" data-testid="native-toolchain-dialog">
        {installable && (
          <ul className="text-fg-muted flex list-disc flex-col gap-1.5 pl-5">
            <li>{t('native.toolchain.where')}</li>
            <li>{t('native.toolchain.size')}</li>
            <li>{t('native.toolchain.once')}</li>
          </ul>
        )}
        {toolchain?.prerequisites.length ? (
          <ul className="text-fg flex list-disc flex-col gap-1 pl-5">
            {toolchain.prerequisites.map((p) => (
              <li key={p}>
                <code className="text-xs">{p}</code>
              </li>
            ))}
          </ul>
        ) : null}
        {toolchain?.reason && (
          <p className="text-fg-subtle text-xs">
            {t('native.toolchain.found', { reason: toolchain.reason })}
          </p>
        )}
      </div>
    </Dialog>
  );
}
