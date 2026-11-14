import {
  Badge,
  Button,
  Checkbox,
  cn,
  Field,
  IconButton,
  Input,
  Select,
  Skeleton,
  Switch,
} from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import {
  Blocks,
  Dices,
  ExternalLink,
  FolderOpen,
  FolderInput,
  Gamepad2,
  Globe2,
  IdCard,
  Mountain,
  Network,
  Puzzle,
  Scale,
  Server,
} from 'lucide-react';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { IconPicker } from '@/components/IconPicker';
import { invoke } from '@/ipc';
import { shortVersion } from '@/lib/format';
import { useActiveInstance, useHorizonVersions, useInstances } from '@/queries/core';
import { randomSeed, type FieldErrors, type WizardState } from './model';
import { RadioCard } from './RadioCard';
import { WizardSection } from './Section';

export const EULA_URL = 'https://www.minecraft.net/eula';

interface StepProps {
  state: WizardState;
  update: (patch: Partial<WizardState>) => void;
  errors: FieldErrors;
  showErrors: boolean;
}

function FieldError({ id, msg }: { id?: string; msg?: string }) {
  const { t } = useTranslation('servers');
  if (!msg) return null;
  return (
    <p id={id} className="text-danger-text text-xs">
      {t(msg as 'wizard.errors.nameRequired')}
    </p>
  );
}

/** Generated-art seeds of the visual choice cards (their hues: orange, blue, green, teal, violet). */
const ART = {
  rustic: 'lake',
  horizon: 'iron',
  new: 'world-new',
  import: 'world-import',
  from_instance: 'world-instance',
} as const;

export function BasicsStep({ state, update, errors, showErrors }: StepProps) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const err = (k: string) => (showErrors ? errors[k] : undefined);
  const versions = useHorizonVersions();
  const { instance: active } = useActiveInstance();
  const { data: instances } = useInstances();
  const starters = useQuery({
    queryKey: ['servers', 'starter-plugins'],
    queryFn: () => invoke('servers_starter_plugins'),
  });
  const worlds = useQuery({
    queryKey: ['instances', state.world.instanceId, 'worlds'],
    queryFn: () => invoke('instance_worlds_list', { instance_id: state.world.instanceId }),
    enabled: state.world.kind === 'from_instance' && !!state.world.instanceId,
  });

  // Defaults: the client's version, recommended starter plugins.
  useEffect(() => {
    if (!state.horizonVersion && versions.data) {
      const same = active && versions.data.find((v) => v.id === active.horizon_version);
      const rec = versions.data.find((v) => v.recommended) ?? versions.data[0];
      update({ horizonVersion: (same ?? rec)?.id ?? '' });
    }
  }, [versions.data, active, state.horizonVersion, update]);
  useEffect(() => {
    if (starters.data && state.starterPlugins.length === 0 && !starters.isRefetching) {
      update({ starterPlugins: starters.data.filter((p) => p.recommended).map((p) => p.id) });
    }
    // Only once, when the list first arrives.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [starters.data]);

  const w = state.world;
  const setWorld = (patch: Partial<WizardState['world']>) => update({ world: { ...w, ...patch } });
  const sameAsClient = active?.horizon_version === state.horizonVersion;

  return (
    <div className="flex flex-col gap-4">
      <WizardSection icon={<IdCard />} title={t('wizard.sections.identity')}>
        <Field
          label={t('wizard.basics.name')}
          error={err('name') ? t(err('name') as 'wizard.errors.nameRequired') : undefined}
        >
          {(p) => (
            <Input
              {...p}
              autoFocus
              value={state.name}
              maxLength={40}
              placeholder={t('wizard.basics.namePlaceholder')}
              onChange={(e) => update({ name: e.target.value })}
              invalid={!!err('name')}
              data-testid="wizard-name"
            />
          )}
        </Field>
        <div className="flex flex-col gap-2">
          <span className="text-sm font-medium">{t('wizard.basics.icon')}</span>
          <IconPicker
            preset={state.preset}
            color={state.color}
            onPreset={(preset) => update({ preset })}
            onColor={(color) => update({ color })}
            iconsLabel={t('wizard.basics.icon')}
            colorsLabel={t('wizard.basics.color')}
            iconLabel={(p) => t(`wizard.basics.icons.${p}`)}
            colorLabel={(n) => t('wizard.basics.colorN', { n })}
          />
        </div>
      </WizardSection>

      <WizardSection icon={<Server />} title={t('wizard.basics.type')}>
        <div
          className="grid gap-3 @xl:grid-cols-2"
          role="radiogroup"
          aria-label={t('wizard.basics.type')}
        >
          <RadioCard
            name="engine"
            value="rustic"
            art={ART.rustic}
            checked={state.engine === 'rustic'}
            onChange={() => update({ engine: 'rustic' })}
            icon={<Puzzle />}
            title={t('list.engine.rustic')}
            badge={<Badge variant="accent">{t('wizard.basics.recommended')}</Badge>}
            description={t('wizard.basics.rusticDesc')}
            testId="engine-rustic"
          />
          <RadioCard
            name="engine"
            value="horizon"
            art={ART.horizon}
            checked={state.engine === 'horizon'}
            onChange={() => update({ engine: 'horizon' })}
            icon={<Server />}
            title={t('list.engine.horizon')}
            description={t('wizard.basics.horizonDesc')}
            testId="engine-horizon"
          />
        </div>
        <div className="grid gap-4 @lg:grid-cols-[1fr_9rem]">
          <Field
            label={t('wizard.basics.version')}
            error={err('version') ? t('wizard.errors.version') : undefined}
          >
            {(p) =>
              versions.isLoading ? (
                <Skeleton className="h-9" />
              ) : (
                <Select
                  id={p.id}
                  value={state.horizonVersion}
                  onValueChange={(v) => update({ horizonVersion: v })}
                  options={(versions.data ?? []).map((v) => ({
                    value: v.id,
                    label: tc('horizon.version', { version: shortVersion(v.id) }),
                    description: v.recommended
                      ? t('wizard.basics.recommended')
                      : v.channel === 'nightly'
                        ? t('wizard.basics.nightly')
                        : undefined,
                  }))}
                />
              )
            }
          </Field>
          <Field
            label={t('wizard.basics.maxPlayers')}
            error={err('maxPlayers') ? t('wizard.errors.maxPlayers') : undefined}
          >
            {(p) => (
              <Input
                {...p}
                inputMode="numeric"
                value={state.maxPlayers}
                onChange={(e) => update({ maxPlayers: e.target.value.replace(/[^\d]/g, '') })}
                invalid={!!err('maxPlayers')}
              />
            )}
          </Field>
        </div>
        {state.horizonVersion && (
          <p
            className="bg-surface-2/70 text-fg-muted -mt-1 rounded-lg px-3 py-2 text-xs"
            data-testid="version-line"
          >
            {state.engine === 'rustic'
              ? t('wizard.basics.versionLineRustic', {
                  version: shortVersion(state.horizonVersion),
                })
              : t('wizard.basics.versionLineHorizon', {
                  version: shortVersion(state.horizonVersion),
                })}{' '}
            {sameAsClient && (
              <span className="text-success-text">{t('wizard.basics.sameAsClient')}</span>
            )}
          </p>
        )}
      </WizardSection>

      <WizardSection icon={<Globe2 />} title={t('wizard.world.title')}>
        <div
          className="grid gap-3 @xl:grid-cols-3"
          role="radiogroup"
          aria-label={t('wizard.world.title')}
        >
          <RadioCard
            name="world-kind"
            value="new"
            art={ART.new}
            checked={w.kind === 'new'}
            onChange={() => setWorld({ kind: 'new' })}
            icon={<Mountain />}
            title={t('wizard.world.new')}
            description={t('wizard.world.newDesc')}
            testId="world-new"
          />
          <RadioCard
            name="world-kind"
            value="import"
            art={ART.import}
            checked={w.kind === 'import'}
            onChange={() => setWorld({ kind: 'import' })}
            icon={<FolderInput />}
            title={t('wizard.world.import')}
            description={t('wizard.world.importDesc')}
            testId="world-import"
          />
          <RadioCard
            name="world-kind"
            value="from_instance"
            art={ART.from_instance}
            checked={w.kind === 'from_instance'}
            onChange={() => setWorld({ kind: 'from_instance' })}
            icon={<Gamepad2 />}
            title={t('wizard.world.fromInstance')}
            description={t('wizard.world.fromInstanceDesc')}
            testId="world-from-instance"
          />
        </div>

        {w.kind === 'new' && (
          <div className="grid gap-4 @xl:grid-cols-2 @4xl:grid-cols-4">
            <Field
              label={t('wizard.world.seed')}
              hint={t('wizard.world.seedHint')}
              className="@xl:col-span-2 @4xl:col-span-4"
              error={err('seed') ? t('wizard.errors.seed') : undefined}
            >
              {(p) => (
                <Input
                  {...p}
                  value={w.seed}
                  onChange={(e) => setWorld({ seed: e.target.value })}
                  className="[&_input]:font-mono"
                  placeholder={t('wizard.world.seedPlaceholder')}
                  rightSlot={
                    <IconButton
                      size="sm"
                      label={t('wizard.world.randomSeed')}
                      icon={<Dices />}
                      onClick={() => setWorld({ seed: randomSeed() })}
                    />
                  }
                />
              )}
            </Field>
            <Field label={t('wizard.world.levelType')}>
              {(p) => (
                <Select
                  id={p.id}
                  value={w.levelType}
                  onValueChange={(v) =>
                    setWorld({ levelType: v as WizardState['world']['levelType'] })
                  }
                  options={(['default', 'flat', 'large_biomes', 'amplified'] as const).map((v) => ({
                    value: v,
                    label: t(`wizard.world.levelTypes.${v}`),
                  }))}
                />
              )}
            </Field>
            <Field label={t('wizard.world.gamemode')}>
              {(p) => (
                <Select
                  id={p.id}
                  value={w.gamemode}
                  onValueChange={(v) =>
                    setWorld({ gamemode: v as WizardState['world']['gamemode'] })
                  }
                  options={(['survival', 'creative', 'adventure'] as const).map((v) => ({
                    value: v,
                    label: t(`wizard.world.gamemodes.${v}`),
                  }))}
                />
              )}
            </Field>
            <Field label={t('wizard.world.difficulty')}>
              {(p) => (
                <Select
                  id={p.id}
                  value={w.hardcore ? 'hard' : w.difficulty}
                  disabled={w.hardcore}
                  onValueChange={(v) =>
                    setWorld({ difficulty: v as WizardState['world']['difficulty'] })
                  }
                  options={(['peaceful', 'easy', 'normal', 'hard'] as const).map((v) => ({
                    value: v,
                    label: t(`wizard.world.difficulties.${v}`),
                  }))}
                />
              )}
            </Field>
            <div className="border-border flex flex-col justify-center gap-2.5 rounded-lg border px-3 py-2">
              <label className="flex items-center justify-between gap-3 text-sm">
                {t('wizard.world.structures')}
                <Switch
                  checked={w.structures}
                  onCheckedChange={(v) => setWorld({ structures: v })}
                />
              </label>
              <label className="flex items-center justify-between gap-3 text-sm">
                {t('wizard.world.hardcore')}
                <Switch checked={w.hardcore} onCheckedChange={(v) => setWorld({ hardcore: v })} />
              </label>
            </div>
          </div>
        )}
        {w.kind === 'import' && (
          <Field
            label={t('wizard.world.folder')}
            error={err('importPath') ? t('wizard.errors.importPath') : undefined}
          >
            {(p) => (
              <div className="flex gap-2">
                <Input
                  {...p}
                  className="flex-1 [&_input]:font-mono"
                  value={w.importPath}
                  onChange={(e) => setWorld({ importPath: e.target.value })}
                  placeholder="C:/Users/…/saves/My World"
                />
                <Button
                  leftIcon={<FolderOpen />}
                  onClick={() =>
                    void invoke('app_pick_path', {
                      kind: 'folder',
                      title: t('wizard.world.folder'),
                    }).then((path) => path && setWorld({ importPath: path }))
                  }
                >
                  {tc('actions.browse')}
                </Button>
              </div>
            )}
          </Field>
        )}
        {w.kind === 'from_instance' && (
          <div className="grid gap-4 @xl:grid-cols-2">
            <Field label={t('wizard.world.instance')}>
              {(p) => (
                <Select
                  id={p.id}
                  value={w.instanceId || undefined}
                  placeholder={t('wizard.world.pickInstance')}
                  onValueChange={(v) => setWorld({ instanceId: v, worldId: '' })}
                  options={(instances ?? []).map((i) => ({ value: i.id, label: i.name }))}
                />
              )}
            </Field>
            <Field
              label={t('wizard.world.world')}
              error={err('world') ? t('wizard.errors.world') : undefined}
            >
              {(p) =>
                worlds.isLoading ? (
                  <Skeleton className="h-9" />
                ) : (
                  <Select
                    id={p.id}
                    value={w.worldId || undefined}
                    disabled={!w.instanceId || !worlds.data?.length}
                    placeholder={
                      w.instanceId && worlds.data?.length === 0
                        ? t('wizard.world.noWorlds')
                        : t('wizard.world.pickWorld')
                    }
                    onValueChange={(v) => setWorld({ worldId: v })}
                    options={(worlds.data ?? []).map((x) => ({
                      value: x.id,
                      label: x.name,
                      description: t(
                        `wizard.world.gamemodes.${x.mode === 'hardcore' || x.mode === 'spectator' ? 'survival' : x.mode}`,
                      ),
                    }))}
                  />
                )
              }
            </Field>
          </div>
        )}
      </WizardSection>

      {state.engine === 'rustic' && (
        <WizardSection
          icon={<Blocks />}
          title={t('wizard.basics.starterPlugins')}
          hint={t('wizard.basics.starterHint')}
        >
          {starters.isLoading && <Skeleton className="h-24" />}
          <ul className="grid gap-2 @2xl:grid-cols-2">
            {starters.data?.map((pl) => {
              const on = state.starterPlugins.includes(pl.id);
              return (
                <li key={pl.id}>
                  <label
                    className={cn(
                      'flex h-full cursor-pointer items-start gap-3 rounded-xl border p-3 transition-colors',
                      'has-[:focus-visible]:ring-accent/50 has-[:focus-visible]:ring-2',
                      on
                        ? 'border-accent/50 bg-accent-soft/50'
                        : 'border-border hover:bg-surface-2/70',
                    )}
                  >
                    <Checkbox
                      className="mt-0.5"
                      checked={on}
                      onCheckedChange={(v) =>
                        update({
                          starterPlugins: v
                            ? [...state.starterPlugins, pl.id]
                            : state.starterPlugins.filter((x) => x !== pl.id),
                        })
                      }
                    />
                    <span className="flex min-w-0 flex-1 flex-col gap-1">
                      <span className="text-sm font-medium">
                        {pl.name}{' '}
                        <span className="text-fg-subtle font-mono text-xs">{pl.version}</span>
                      </span>
                      <span className="text-fg-muted text-xs">{pl.description}</span>
                      <span className="flex flex-wrap gap-1 pt-0.5">
                        <Badge
                          variant={pl.capabilities.network ? 'warning' : 'neutral'}
                          icon={<Network />}
                        >
                          {pl.capabilities.network
                            ? t('wizard.basics.capNetwork')
                            : t('wizard.basics.capNoNetwork')}
                        </Badge>
                        <Badge>
                          {pl.capabilities.filesystem === 'none'
                            ? t('wizard.basics.capNoFiles')
                            : t('wizard.basics.capDataFolder')}
                        </Badge>
                      </span>
                    </span>
                  </label>
                </li>
              );
            })}
          </ul>
        </WizardSection>
      )}

      <section
        aria-labelledby="eula-title"
        className={cn(
          'flex flex-col gap-3 rounded-2xl border p-5 transition-colors',
          err('eula')
            ? 'border-danger/60 bg-danger/5'
            : state.eulaAccepted
              ? 'border-success/40 bg-success/5'
              : 'border-border bg-surface/70',
        )}
        data-testid="wizard-eula-block"
      >
        <div className="flex items-start gap-3">
          <span
            aria-hidden
            className="bg-surface-3 text-fg grid size-8 shrink-0 place-items-center rounded-lg [&_svg]:size-4"
          >
            <Scale />
          </span>
          <div className="flex min-w-0 flex-1 flex-col gap-1 pt-0.5">
            <h3 id="eula-title" className="text-sm font-semibold">
              {t('wizard.basics.eulaTitle')}
            </h3>
            <p id="eula-hint" className="text-fg-muted max-w-[70ch] text-sm leading-relaxed">
              {t('wizard.basics.eulaHint')}
            </p>
          </div>
          <Button
            variant="secondary"
            size="sm"
            rightIcon={<ExternalLink />}
            onClick={() => void invoke('app_open_url', { url: EULA_URL })}
          >
            {t('wizard.basics.eulaRead')}
          </Button>
        </div>
        <label className="bg-surface-2/70 hover:bg-surface-2 flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors">
          <Checkbox
            checked={state.eulaAccepted}
            onCheckedChange={(v) => update({ eulaAccepted: v === true })}
            aria-describedby="eula-hint"
            data-testid="wizard-eula"
          />
          <span>{t('wizard.basics.eula')}</span>
        </label>
        <FieldError msg={err('eula')} />
      </section>
    </div>
  );
}
