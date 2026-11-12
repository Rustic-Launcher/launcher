import {
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownSeparator,
  DropdownTrigger,
} from '@rustic/ui';
import { useNavigate } from '@tanstack/react-router';
import { motion } from 'motion/react';
import { Check, ChevronDown, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useHorizonVersionLabel } from '@/components/HorizonVersion';
import { InstanceIcon } from '@/components/InstanceIcon';
import type { Instance } from '@/ipc/types/instances';
import { useInstances } from '@/queries/core';
import { usePrefs } from '@/stores/prefs';
import type { PlayModel } from './usePlay';

/**
 * The big centred LAUNCH button of the Launchpad: two lines (action + Horizon build), a progress
 * fill while downloading (hover = Cancel), and a caret to switch instance.
 */
export function LaunchButton({
  instance,
  play,
}: {
  instance: Instance | undefined;
  play: PlayModel;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const versionLabel = useHorizonVersionLabel();
  const { data: instances } = useInstances();
  const setActive = usePrefs((s) => s.setActiveInstance);

  const line1 =
    play.state === 'update'
      ? t('launch.update')
      : play.state === 'running'
        ? t('launch.stop')
        : play.state === 'progress'
          ? play.progressLabel || t('play.progress.preparing')
          : t('launch.launch');
  const line2 = instance ? versionLabel(instance.horizon_version) : t('play.status.noInstance');
  const pct = play.state === 'progress' ? play.progress : null;

  return (
    <div
      className={cn(
        'shadow-glow relative flex h-[72px] w-[min(340px,100%)] overflow-hidden rounded-xl',
        play.state === 'running' ? 'bg-surface-3 shadow-none' : 'bg-accent',
        play.state === 'progress' && 'bg-surface-3 shadow-none',
      )}
    >
      {play.state === 'progress' && (
        <span aria-hidden className="absolute inset-0">
          {pct == null ? (
            <span className="animate-indeterminate bg-accent/60 absolute inset-y-0 w-1/2" />
          ) : (
            <motion.span
              className="bg-accent absolute inset-y-0 left-0"
              initial={false}
              animate={{ width: `${pct * 100}%` }}
              transition={{ duration: 0.25 }}
            />
          )}
        </span>
      )}
      <button
        type="button"
        data-testid="play-button"
        data-state={play.state}
        disabled={play.state === 'disabled'}
        onClick={play.primary}
        aria-label={play.state === 'progress' ? t('play.cancel') : line1}
        className={cn(
          'rx-focus-ring group relative flex flex-1 flex-col items-center justify-center gap-0.5 px-4 transition-colors disabled:opacity-50',
          play.state === 'running' || play.state === 'progress'
            ? 'text-fg hover:bg-danger hover:text-white'
            : 'text-accent-fg hover:bg-accent-hover',
        )}
      >
        <span className="text-xl leading-6 font-extrabold tracking-wide uppercase">
          {play.state === 'progress' ? (
            <>
              <span className="group-hover:hidden">{line1}</span>
              <span className="hidden group-hover:inline">{t('play.cancel')}</span>
            </>
          ) : (
            line1
          )}
        </span>
        <span className="text-xs font-semibold opacity-80">{line2}</span>
      </button>
      <Dropdown>
        <DropdownTrigger asChild>
          <button
            type="button"
            data-testid="instance-selector"
            aria-label={t('play.selectInstance')}
            className={cn(
              'rx-focus-ring relative grid w-10 place-items-center border-l',
              play.state === 'running' || play.state === 'progress'
                ? 'border-border-default text-fg hover:bg-surface-2'
                : 'text-accent-fg hover:bg-accent-hover border-black/15',
            )}
          >
            <ChevronDown className="size-4" />
            <span className="sr-only">{instance?.name}</span>
          </button>
        </DropdownTrigger>
        <DropdownContent align="end" className="w-80">
          <DropdownLabel>{t('play.instances')}</DropdownLabel>
          {instances?.map((i) => (
            <DropdownItem
              key={i.id}
              onSelect={() => setActive(i.id)}
              icon={<InstanceIcon icon={i.icon} size="sm" />}
            >
              <span className="flex items-center justify-between gap-2">
                <span className="flex flex-col">
                  <span className="text-sm">{i.name}</span>
                  <span className="text-fg-subtle text-xs">{versionLabel(i.horizon_version)}</span>
                </span>
                {i.id === instance?.id && <Check className="text-accent-text size-4" />}
              </span>
            </DropdownItem>
          ))}
          <DropdownSeparator />
          <DropdownItem
            icon={<Plus />}
            onSelect={() => void navigate({ to: '/instances', search: { new: '1' } })}
          >
            {t('nav.newInstance')}
          </DropdownItem>
        </DropdownContent>
      </Dropdown>
    </div>
  );
}
