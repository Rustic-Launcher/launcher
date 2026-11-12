import { Badge, Button, Card, cn, Progress } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { Clock, Download, Layers, Package, Pin, Play, Square, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import { usePlay } from '@/features/play/usePlay';
import type { Instance } from '@/ipc/types/instances';
import { formatDuration, formatRelative, shortVersion } from '@/lib/format';
import { usePrefs } from '@/stores/prefs';
import { InstanceActions } from './InstanceActions';

/** Compact play control for cards and lists (the big PLAY lives in the bottom bar). */
export function InstancePlayControl({
  instance,
  className,
}: {
  instance: Instance;
  className?: string;
}) {
  const { t } = useTranslation();
  const play = usePlay(instance);
  const setActive = usePrefs((s) => s.setActiveInstance);
  const onClick = () => {
    setActive(instance.id);
    play.primary();
  };
  if (play.state === 'progress') {
    return (
      <div className={cn('flex min-w-0 flex-1 items-center gap-2', className)}>
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="text-fg-muted truncate text-xs tabular-nums">{play.progressLabel}</span>
          <Progress value={play.progress} size="xs" label={play.progressLabel} />
        </div>
        <Button size="sm" variant="ghost" leftIcon={<X />} onClick={onClick}>
          {t('play.cancel')}
        </Button>
      </div>
    );
  }
  const map = {
    play: {
      label: t('play.play'),
      icon: <Play className="fill-current" />,
      variant: 'primary' as const,
    },
    update: { label: t('play.update'), icon: <Download />, variant: 'primary' as const },
    running: { label: t('play.stop'), icon: <Square />, variant: 'secondary' as const },
    disabled: { label: t('play.play'), icon: <Play />, variant: 'secondary' as const },
  };
  const m = map[play.state];
  return (
    <Button
      size="sm"
      variant={m.variant}
      leftIcon={m.icon}
      onClick={onClick}
      disabled={play.state === 'disabled'}
      className={className}
      data-testid={`play-${instance.id}`}
      aria-label={`${m.label} — ${instance.name}`}
    >
      {m.label}
    </Button>
  );
}

export function InstanceCard({ instance }: { instance: Instance }) {
  const { t, i18n } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const lang = i18n.language;
  return (
    <Card
      interactive
      padding="none"
      className="group flex h-full flex-col overflow-hidden"
      data-testid={`instance-card-${instance.id}`}
    >
      <div
        aria-hidden
        className="h-16 w-full opacity-80"
        style={{
          background:
            instance.icon.kind === 'preset'
              ? `radial-gradient(120% 140% at 0% 0%, color-mix(in oklab, ${instance.icon.color} 45%, transparent), transparent 70%)`
              : undefined,
        }}
      />
      <div className="-mt-10 flex flex-1 flex-col gap-3 px-4 pb-4">
        <div className="flex items-end justify-between gap-2">
          <InstanceIcon icon={instance.icon} size="lg" className="ring-surface ring-4" />
          <div className="flex flex-wrap justify-end gap-1">
            {instance.pinned && (
              <Badge icon={<Pin />} aria-label={t('card.pinned')}>
                {t('card.pinned')}
              </Badge>
            )}
            {instance.state === 'running' && <Badge variant="accent">{t('state.running')}</Badge>}
            {instance.state === 'installing' && (
              <Badge variant="info">{t('state.installing')}</Badge>
            )}
            {instance.state === 'broken' && <Badge variant="danger">{t('state.broken')}</Badge>}
            {instance.update_available && instance.state === 'needs_update' && (
              <Badge variant="warning">
                {tc('play.newVersion', { version: shortVersion(instance.update_available) })}
              </Badge>
            )}
          </div>
        </div>
        <div className="flex min-w-0 flex-col gap-0.5">
          <h3 className="truncate text-base font-semibold">
            {/* Stretched link: the whole card opens the detail page. */}
            <Link
              to="/instances/$instanceId"
              params={{ instanceId: instance.id }}
              search={{}}
              className="rx-focus-ring rounded outline-none after:absolute after:inset-0 after:content-['']"
            >
              {instance.name}
            </Link>
          </h3>
          <HorizonVersion
            id={instance.horizon_version}
            className="text-fg-muted truncate text-xs"
          />
        </div>
        <dl className="grid grid-cols-3 gap-2 text-xs">
          <div className="flex flex-col gap-0.5">
            <dt className="text-fg-subtle flex items-center gap-1">
              <Package className="size-3" aria-hidden /> {t('card.mods')}
            </dt>
            <dd className="font-medium tabular-nums">{instance.mod_count}</dd>
          </div>
          <div className="flex flex-col gap-0.5">
            <dt className="text-fg-subtle flex items-center gap-1">
              <Clock className="size-3" aria-hidden /> {t('card.playtime')}
            </dt>
            <dd className="font-medium tabular-nums">
              {formatDuration(instance.playtime_s, lang)}
            </dd>
          </div>
          <div className="flex flex-col gap-0.5">
            <dt className="text-fg-subtle flex items-center gap-1">
              <Layers className="size-3" aria-hidden /> {t('card.lastPlayed')}
            </dt>
            <dd className="truncate font-medium">
              {instance.last_played_at
                ? formatRelative(instance.last_played_at, lang)
                : t('card.never')}
            </dd>
          </div>
        </dl>
        {instance.source && (
          <p className="text-fg-subtle truncate text-xs">
            {t('card.fromModpack', {
              name: instance.source.name,
              version: instance.source.version,
            })}
          </p>
        )}
        <div className="relative z-10 mt-auto flex items-center justify-between gap-2 pt-1">
          <InstancePlayControl instance={instance} />
          <InstanceActions instance={instance} afterDelete={() => undefined} />
        </div>
      </div>
    </Card>
  );
}
