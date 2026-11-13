/**
 * An instance's native build (ADR 0157): its mods, whether its client is up to date, the running
 * build's progress with Cancel, the last failure, the mods a build left out. Hidden while the
 * instance has no native mod.
 */
import { Badge, Button, Progress, formatBytes } from '@rustic/ui';
import { AlertTriangle, Cpu, Hammer, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { Instance } from '@/ipc/types/instances';
import { formatDate, formatDuration, formatPercent } from '@/lib/format';

/** "42 s" under a minute, else hours and minutes. */
export function buildTime(seconds: number, lang: string): string {
  if (seconds < 60)
    return new Intl.NumberFormat(lang, {
      style: 'unit',
      unit: 'second',
      unitDisplay: 'narrow',
    }).format(Math.max(1, Math.round(seconds)));
  return formatDuration(seconds, lang);
}
import { taskFraction, useNativeActions, useNativeStatus, useNativeTask } from './useNative';
import { useNativeInstall } from './useNativeInstall';

export function NativeBuildPanel({ instance }: { instance: Instance }) {
  const { t, i18n } = useTranslation('instances');
  const { data: s } = useNativeStatus(instance.id);
  const task = useNativeTask(instance.id);
  const actions = useNativeActions();
  const flow = useNativeInstall();
  if (!s || s.mods.length === 0) return null;

  const running = task && (task.state === 'running' || task.state === 'queued');
  // A failure is news only until a later build brought the client up to date.
  const failed = task?.state === 'failed' && s.state !== 'up_to_date' ? task : undefined;
  const fraction = taskFraction(task);
  const badge = running
    ? { variant: 'accent' as const, label: t('native.state.building') }
    : s.state === 'up_to_date'
      ? { variant: 'success' as const, label: t('native.state.up_to_date') }
      : s.state === 'not_needed'
        ? { variant: 'neutral' as const, label: t('native.state.not_needed') }
        : s.state === 'needs_rebuild'
          ? { variant: 'warning' as const, label: t('native.state.needs_rebuild') }
          : { variant: 'warning' as const, label: t('native.state.not_built') };
  const canBuild = s.supported && !running && s.state !== 'not_needed';

  return (
    <section
      className="rx-surface flex flex-col gap-3 rounded-2xl p-4"
      aria-label={t('native.title')}
      data-testid="native-build-panel"
    >
      <div className="flex flex-wrap items-center gap-3">
        <span className="bg-info/15 text-info-text grid size-9 place-items-center rounded-lg">
          <Cpu className="size-4.5" />
        </span>
        <div className="flex min-w-0 flex-1 flex-col">
          <h3 className="text-fg text-sm font-semibold">{t('native.title')}</h3>
          <p className="text-fg-muted text-xs">
            {s.mods
              .filter((m) => m.enabled)
              .map((m) => `${m.id} ${m.version}`)
              .join(' · ') || t('native.allOff')}
          </p>
        </div>
        <Badge variant={badge.variant} data-testid="native-build-state">
          {badge.label}
        </Badge>
        {running ? (
          <Button
            size="sm"
            leftIcon={<X />}
            loading={actions.cancel.isPending}
            onClick={() => actions.cancel.mutate(task.id)}
          >
            {t('native.cancel')}
          </Button>
        ) : (
          canBuild && (
            <Button
              size="sm"
              variant={s.state === 'up_to_date' ? 'ghost' : 'primary'}
              leftIcon={<Hammer />}
              loading={flow.busy}
              data-testid="native-build-button"
              onClick={() =>
                void flow.build({
                  instanceId: instance.id,
                  instanceName: instance.name,
                  modName: instance.name,
                })
              }
            >
              {s.state === 'up_to_date' ? t('native.rebuild') : t('native.build')}
            </Button>
          )
        )}
      </div>

      {running && (
        <div className="flex flex-col gap-1.5" aria-live="polite">
          <Progress
            value={fraction}
            label={t('native.progress')}
            valueText={fraction != null ? formatPercent(fraction, i18n.language) : undefined}
          />
          <p className="text-fg-subtle text-xs tabular-nums" data-testid="native-build-detail">
            {fraction != null && `${formatPercent(fraction, i18n.language)} · `}
            {task.detail ?? t('native.starting')}
          </p>
          <p className="text-fg-subtle text-xs">{t('native.firstBuildHint')}</p>
        </div>
      )}

      {!s.supported && <p className="text-warning-text text-xs">{t('native.notEdition')}</p>}

      {failed && !running && (
        <div
          className="border-danger/30 bg-danger/10 flex gap-2 rounded-xl border p-3 text-xs"
          role="alert"
        >
          <AlertTriangle className="text-danger-text size-4 shrink-0" />
          <div className="flex min-w-0 flex-col gap-1">
            <span className="text-fg font-medium">{t('native.failed')}</span>
            <pre className="text-fg-muted max-h-32 overflow-auto font-mono text-[11px] whitespace-pre-wrap">
              {failed.error}
            </pre>
          </div>
        </div>
      )}

      {s.build && !running && (
        <p className="text-fg-subtle text-xs">
          {t('native.built', {
            date: formatDate(new Date(s.build.built_at * 1000).toISOString(), i18n.language),
            duration: buildTime(s.build.seconds, i18n.language),
            size: formatBytes(s.build.client_bytes, i18n.language),
          })}
        </p>
      )}
      {s.build?.left_out.length ? (
        <ul className="flex flex-col gap-1 text-xs" data-testid="native-left-out">
          {s.build.left_out.map((l) => (
            <li key={l.id} className="text-warning-text">
              {t(l.why === 'unsupported' ? 'native.leftOut.unsupported' : 'native.leftOut.other', {
                name: `${l.id} ${l.version}`,
              })}{' '}
              <span className="text-fg-subtle">({l.detail})</span>
            </li>
          ))}
        </ul>
      ) : null}
      {flow.dialogs}
    </section>
  );
}
