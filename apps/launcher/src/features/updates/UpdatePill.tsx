/**
 * The title bar's update pill (ADR 0162): discreet progress while an update downloads, "Restarting…"
 * before the launcher replaces itself, "Update after your game" while a game holds it back, and
 * "Update and restart" when automatic updates are off. Hidden otherwise.
 */
import { cn, Progress, toast } from '@rustic/ui';
import { ArrowDownToLine, Clock, Loader2, RotateCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import { formatPercent } from '@/lib/format';
import { updatePill, useUpdates } from '@/stores/updates';

export function UpdatePill() {
  const { t, i18n } = useTranslation();
  const status = useUpdates((s) => s.status);
  const kind = updatePill(status);
  if (!status || !kind) return null;

  const base =
    'relative flex h-8 items-center gap-1.5 overflow-hidden rounded-lg border px-2.5 text-xs font-medium';
  if (kind === 'apply') {
    return (
      <button
        type="button"
        data-testid="update-pill"
        className={cn(
          base,
          'rx-focus-ring border-accent/50 bg-accent-soft text-accent-text hover:bg-accent/20',
        )}
        onClick={() =>
          void invoke('app_update_apply').catch((e: Error) =>
            toast({ tone: 'error', title: t('updates.toast.failed'), description: e.message }),
          )
        }
      >
        <ArrowDownToLine className="size-3.5" />
        {t('updates.pill.apply')}
      </button>
    );
  }
  const label =
    kind === 'deferred'
      ? t('updates.pill.deferred')
      : kind === 'restarting'
        ? t('updates.pill.restarting')
        : status.phase === 'installing'
          ? t('updates.pill.installing')
          : t('updates.pill.downloading', {
              pct: status.progress != null ? formatPercent(status.progress, i18n.language) : '…',
            });
  const Icon = kind === 'deferred' ? Clock : kind === 'restarting' ? RotateCw : Loader2;
  return (
    <span
      role="status"
      data-testid="update-pill"
      data-kind={kind}
      className={cn(base, 'bg-surface-2 border-border text-fg-muted')}
    >
      <Icon className={cn('size-3.5', kind === 'working' && 'animate-spin')} aria-hidden />
      {label}
      {kind === 'working' && (
        <span className="absolute inset-x-0 bottom-0">
          <Progress
            value={status.progress}
            size="xs"
            className="rounded-none bg-transparent"
            label={label}
          />
        </span>
      )}
    </span>
  );
}
