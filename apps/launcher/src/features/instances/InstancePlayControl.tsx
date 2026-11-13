import { Button, cn, Progress } from '@rustic/ui';
import { Download, Play, Square, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { usePlay } from '@/features/play/usePlay';
import type { Instance } from '@/ipc/types/instances';
import { usePrefs } from '@/stores/prefs';

/** Compact play control for cards and lists (the big PLAY lives in the bottom bar). */
export function InstancePlayControl({
  instance,
  className,
  testId,
}: {
  instance: Instance;
  className?: string;
  /** Defaults to `play-<id>`; another list of the same page passes its own. */
  testId?: string;
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
      data-testid={testId ?? `play-${instance.id}`}
      data-state={play.state}
      aria-label={`${m.label} — ${instance.name}`}
    >
      {m.label}
    </Button>
  );
}
