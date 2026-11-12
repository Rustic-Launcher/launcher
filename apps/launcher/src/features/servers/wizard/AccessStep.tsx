import { Badge, Checkbox, cn, Switch } from '@rustic/ui';
import { Globe, Home, KeyRound, Link2, ShieldCheck, Ticket } from 'lucide-react';
import { useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { InstanceIcon } from '@/components/InstanceIcon';
import { formatMemory, shortVersion } from '@/lib/format';
import { relayHost } from '@/lib/relay';
import type { WizardState } from './model';

interface StepProps {
  state: WizardState;
  update: (patch: Partial<WizardState>) => void;
  vpsHost: string | null;
}

function Option({
  icon,
  title,
  detail,
  control,
  muted,
  testId,
}: {
  icon: ReactNode;
  title: ReactNode;
  detail: ReactNode;
  control: ReactNode;
  muted?: boolean;
  testId?: string;
}) {
  return (
    <li
      data-testid={testId}
      className={cn(
        'rx-surface flex items-center gap-4 rounded-xl px-4 py-3',
        muted && 'opacity-70',
      )}
    >
      <span className="bg-surface-3 text-fg-muted grid size-10 shrink-0 place-items-center rounded-lg [&_svg]:size-5">
        {icon}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="text-sm font-semibold">{title}</span>
        <span className="text-fg-muted text-xs">{detail}</span>
      </div>
      {control}
    </li>
  );
}

export function AccessStep({ state, update, vpsHost }: StepProps) {
  const { t, i18n } = useTranslation('servers');
  const { t: tc } = useTranslation();
  // A preview label; the real one is assigned by the relay when the server is created.
  const previewRelay = useMemo(() => relayHost('brave-otter-4821'), []);
  const local = state.hosting === 'local';

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_20rem]">
      <div className="flex flex-col gap-3">
        <p className="text-fg-muted text-sm">{t('wizard.access.intro')}</p>
        <ul className="flex flex-col gap-2">
          {local ? (
            <Option
              icon={<Home />}
              title={t('wizard.access.lan')}
              detail={t('wizard.access.lanDetail')}
              control={<Badge variant="success">{t('wizard.access.alwaysOn')}</Badge>}
            />
          ) : (
            <Option
              icon={<Globe />}
              title={t('wizard.access.publicIp')}
              detail={
                <>
                  <code className="font-mono">{vpsHost}:25565</code> ·{' '}
                  {t('wizard.access.publicIpDetail')}
                </>
              }
              control={<Badge variant="success">{t('wizard.access.alwaysOn')}</Badge>}
            />
          )}
          {local && (
            <Option
              icon={<Link2 />}
              testId="access-relay"
              title={t('wizard.access.relay')}
              detail={
                <>
                  <code className="font-mono">{previewRelay}</code> ·{' '}
                  {t('wizard.access.relayDetail')}
                </>
              }
              control={
                <Switch
                  checked={state.access.relay}
                  onCheckedChange={(v) => update({ access: { ...state.access, relay: v } })}
                  aria-label={t('wizard.access.relay')}
                />
              }
            />
          )}
          <Option
            icon={<Ticket />}
            title={t('wizard.access.invite')}
            detail={t('wizard.access.inviteDetail')}
            control={
              <Switch
                checked={state.access.invite}
                onCheckedChange={(v) => update({ access: { ...state.access, invite: v } })}
                aria-label={t('wizard.access.invite')}
              />
            }
          />
          <Option
            icon={<Globe />}
            title={t('wizard.access.domain')}
            detail={t('wizard.access.domainDetail')}
            control={
              <label className="flex items-center gap-2 text-xs font-medium">
                <Checkbox
                  checked={state.domainAfter}
                  onCheckedChange={(v) => update({ domainAfter: v === true })}
                />
                {t('wizard.access.domainAfter')}
              </label>
            }
          />
        </ul>
        <p className="bg-surface-2 text-fg-muted flex items-center gap-2 rounded-lg px-3 py-2 text-xs">
          <ShieldCheck className="text-success-text size-4 shrink-0" aria-hidden />
          {t('wizard.access.safety')}
        </p>
      </div>

      <aside
        className="rx-surface flex flex-col gap-3 rounded-xl p-5"
        aria-label={t('wizard.summary.title')}
      >
        <h3 className="text-sm font-semibold">{t('wizard.summary.title')}</h3>
        <div className="flex items-center gap-3">
          <InstanceIcon
            icon={{ kind: 'preset', preset: state.preset, color: state.color }}
            size="md"
          />
          <div className="flex min-w-0 flex-col">
            <span className="truncate font-semibold" data-testid="summary-name">
              {state.name}
            </span>
            <span className="text-fg-muted text-xs">
              {state.engine === 'rustic' ? t('list.engine.rustic') : t('list.engine.horizon')}
            </span>
          </div>
        </div>
        <dl className="flex flex-col gap-1.5 text-xs">
          {[
            [
              t('wizard.summary.version'),
              tc('horizon.version', { version: shortVersion(state.horizonVersion || '-') }),
            ],
            [
              t('wizard.summary.host'),
              local ? t('hosting.local.title') : t('list.hosting.remote', { host: vpsHost ?? '' }),
            ],
            [
              t('wizard.summary.world'),
              state.world.kind === 'new'
                ? `${t('wizard.world.new')} · ${t(`wizard.world.gamemodes.${state.world.gamemode}`)}${state.world.hardcore ? ` · ${t('wizard.world.hardcore')}` : ''}`
                : state.world.kind === 'import'
                  ? t('wizard.world.import')
                  : t('wizard.world.fromInstance'),
            ],
            [t('wizard.summary.memory'), formatMemory(state.memoryMb, i18n.language)],
            [t('wizard.summary.players'), state.maxPlayers],
            ...(state.engine === 'rustic'
              ? [
                  [
                    t('wizard.summary.plugins'),
                    state.starterPlugins.length
                      ? state.starterPlugins.join(', ')
                      : t('wizard.summary.none'),
                  ],
                ]
              : []),
          ].map(([k, v]) => (
            <div key={k} className="flex justify-between gap-3">
              <dt className="text-fg-subtle">{k}</dt>
              <dd className="text-fg truncate text-right font-medium">{v}</dd>
            </div>
          ))}
        </dl>
        <p className="text-2xs text-fg-subtle mt-1 flex items-start gap-2">
          <KeyRound className="size-3.5 shrink-0" aria-hidden />
          {t('wizard.summary.eulaAccepted')}
        </p>
      </aside>
    </div>
  );
}
