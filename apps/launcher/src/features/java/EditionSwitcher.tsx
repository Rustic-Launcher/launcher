/**
 * The Launchpad's game switcher, top left (ADR 0172): Rustic (default), Minecraft: Java Edition,
 * Minecraft: Bedrock Edition; remembered between sessions. A short "not affiliated" line stays
 * next to it: the official games are third-party products the launcher only starts.
 */
import {
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownTrigger,
} from '@rustic/ui';
import { Check, ChevronDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { usePrefs, type LaunchpadEdition } from '@/stores/prefs';
import { EditionLogo } from './EditionLogo';

export const EDITIONS: LaunchpadEdition[] = ['rustic', 'java', 'bedrock'];

/**
 * `overlay`: drawn in the top-left corner of the Launchpad's hero (over the video or the edition's
 * backdrop), on a dark glass so it reads on any frame.
 */
export function EditionSwitcher({ className, overlay }: { className?: string; overlay?: boolean }) {
  const { t } = useTranslation('java');
  const edition = usePrefs((s) => s.launchpadEdition);
  const setEdition = usePrefs((s) => s.setLaunchpadEdition);
  return (
    <div
      className={cn(
        'flex flex-wrap items-center gap-x-3 gap-y-1',
        overlay ? 'absolute start-3 top-3 z-20 max-w-[calc(100%-1.5rem)]' : 'px-1',
        className,
      )}
    >
      <Dropdown>
        <DropdownTrigger asChild>
          <button
            type="button"
            data-testid="edition-switcher"
            aria-label={t('switcher.label')}
            className={cn(
              'rx-focus-ring flex h-9 items-center gap-2 rounded-lg border px-2.5 text-sm font-semibold',
              overlay
                ? 'border-white/15 bg-black/45 text-white shadow-lg backdrop-blur-md hover:bg-black/60'
                : 'border-border bg-surface hover:bg-surface-2',
            )}
          >
            <EditionLogo edition={edition} size={18} />
            <span data-testid="edition-current">{t(`switcher.${edition}`)}</span>
            <ChevronDown className={cn('size-4', overlay ? 'text-white/70' : 'text-fg-subtle')} />
          </button>
        </DropdownTrigger>
        <DropdownContent align="start" className="w-72">
          <DropdownLabel>{t('switcher.label')}</DropdownLabel>
          {EDITIONS.map((e) => (
            <DropdownItem
              key={e}
              data-testid={`edition-${e}`}
              icon={<EditionLogo edition={e} size={16} />}
              onSelect={() => setEdition(e)}
            >
              <span className="flex items-center justify-between gap-2">
                {t(`switcher.${e}`)}
                {e === edition && <Check className="text-accent-text size-4" />}
              </span>
            </DropdownItem>
          ))}
          <p className="text-fg-subtle px-2 pt-1.5 pb-1 text-[11px] leading-snug">
            {t('switcher.disclaimer')}
          </p>
        </DropdownContent>
      </Dropdown>
      <span
        className={cn(
          'text-[11px]',
          overlay ? 'text-white/70 [text-shadow:0_1px_2px_rgb(0_0_0/0.8)]' : 'text-fg-subtle',
        )}
        data-testid="edition-disclaimer"
      >
        {t('switcher.disclaimer')}
      </span>
    </div>
  );
}
