import { cn } from '@rustic/ui';
import { Globe2, KeyRound, Laptop, MemoryStick, Puzzle, Server, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { MetaItem, OverlayBadge, VisualCard } from '@/components/catalog';
import { InstanceIcon } from '@/components/InstanceIcon';
import { InstanceCover } from '@/features/instances/InstanceCover';
import type { InstanceIcon as Icon } from '@/ipc/types/instances';
import { formatMemory, shortVersion } from '@/lib/format';
import type { WizardState } from './model';

/** The draft server's cover seed: stable while typing, so the art doesn't jump. */
const DRAFT_ID = 'server-draft';

/**
 * The wizard's live preview (beside every step): the server's card as the Servers page will show
 * it (cover in its colour, icon, name, type and version), then the summary of every choice.
 */
export function ServerPreview({
  state,
  vpsHost,
  className,
}: {
  state: WizardState;
  vpsHost: string | null;
  className?: string;
}) {
  const { t, i18n } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const icon: Icon = { kind: 'preset', preset: state.preset, color: state.color };
  const name = state.name.trim();
  const local = state.hosting !== 'vps';
  const version = state.horizonVersion ? shortVersion(state.horizonVersion) : '–';
  const world =
    state.world.kind === 'new'
      ? `${t('wizard.world.new')} · ${t(`wizard.world.gamemodes.${state.world.gamemode}`)}${state.world.hardcore ? ` · ${t('wizard.world.hardcore')}` : ''}`
      : state.world.kind === 'import'
        ? t('wizard.world.import')
        : t('wizard.world.fromInstance');
  const rows: [string, string][] = [
    [t('wizard.summary.version'), tc('horizon.version', { version })],
    [
      t('wizard.summary.host'),
      local ? t('hosting.local.title') : t('list.hosting.remote', { host: vpsHost ?? '' }),
    ],
    [t('wizard.summary.world'), world],
    [t('wizard.summary.memory'), formatMemory(state.memoryMb, i18n.language)],
    [t('wizard.summary.players'), state.maxPlayers || '–'],
  ];
  if (state.engine === 'rustic') {
    rows.push([
      t('wizard.summary.plugins'),
      state.starterPlugins.length ? state.starterPlugins.join(', ') : t('wizard.summary.none'),
    ]);
  }

  return (
    <aside
      className={cn('flex flex-col gap-3', className)}
      aria-label={t('wizard.preview.title')}
      data-testid="server-preview"
    >
      <p className="rx-eyebrow text-fg-subtle px-1">{t('wizard.preview.title')}</p>
      <VisualCard
        seed={DRAFT_ID}
        title={name || t('wizard.preview.unnamed')}
        className="hover:translate-y-0"
        coverArt={<InstanceCover instance={{ id: DRAFT_ID, icon }} />}
        thumb={
          <InstanceIcon
            icon={icon}
            size="md"
            className="ring-surface relative -mt-7 mb-0.5 ring-4"
          />
        }
        renderTitle={(cls) => (
          <span
            className={cn(cls, 'hover:text-fg', !name && 'text-fg-subtle')}
            data-testid="summary-name"
          >
            {name || t('wizard.preview.unnamed')}
          </span>
        )}
        overlay={
          <>
            {state.engine === 'rustic' ? (
              <OverlayBadge tone="accent" icon={<Puzzle />}>
                {t('list.engine.rustic')}
              </OverlayBadge>
            ) : (
              <OverlayBadge icon={<Server />}>{t('list.engine.horizon')}</OverlayBadge>
            )}
          </>
        }
        coverFooter={
          <div className="flex items-center justify-end gap-1.5 px-2.5 pb-2.5">
            <OverlayBadge>{tc('horizon.version', { version })}</OverlayBadge>
          </div>
        }
        byline={
          <span className="flex items-center gap-1">
            {local ? (
              <Laptop className="size-3 shrink-0" aria-hidden />
            ) : (
              <Globe2 className="size-3 shrink-0" aria-hidden />
            )}
            <span className="truncate">
              {local ? t('list.hosting.local') : t('list.hosting.remote', { host: vpsHost ?? '' })}
              {' · '}
              {t('wizard.preview.draft')}
            </span>
          </span>
        }
        meta={
          <>
            <MetaItem icon={<Users />} label={t('wizard.summary.players')}>
              {state.maxPlayers || '–'}
            </MetaItem>
            <MetaItem icon={<MemoryStick />} label={t('wizard.summary.memory')}>
              {formatMemory(state.memoryMb, i18n.language)}
            </MetaItem>
          </>
        }
      />
      <section className="rx-surface flex flex-col gap-2 rounded-xl p-4">
        <h3 className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
          {t('wizard.summary.title')}
        </h3>
        <dl className="flex flex-col gap-1.5 text-xs">
          {rows.map(([k, v]) => (
            <div key={k} className="flex justify-between gap-3">
              <dt className="text-fg-subtle shrink-0">{k}</dt>
              <dd className="text-fg truncate text-right font-medium" title={v}>
                {v}
              </dd>
            </div>
          ))}
        </dl>
        {state.eulaAccepted && (
          <p className="text-2xs text-fg-subtle border-border mt-1 flex items-start gap-2 border-t pt-2">
            <KeyRound className="size-3.5 shrink-0" aria-hidden />
            {t('wizard.summary.eulaAccepted')}
          </p>
        )}
      </section>
    </aside>
  );
}
