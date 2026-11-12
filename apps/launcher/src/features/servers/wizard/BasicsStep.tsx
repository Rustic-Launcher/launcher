import {
  Badge,
  Button,
  Checkbox,
  cn,
  Field,
  IconButton,
  Input,
  SegmentedControl,
  Select,
  Skeleton,
  Switch,
} from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { Dices, ExternalLink, FolderOpen, Network, Puzzle, Server } from 'lucide-react';
import { useEffect, useId } from 'react';
import { useTranslation } from 'react-i18next';
import { PRESET_COLORS, PRESET_ICONS } from '@/components/InstanceIcon';
import { invoke } from '@/ipc';
import type { InstanceIconPreset } from '@/ipc/types/instances';
import { shortVersion } from '@/lib/format';
import { useActiveInstance, useHorizonVersions, useInstances } from '@/queries/core';
import { randomSeed, type FieldErrors, type WizardState } from './model';
import { RadioCard } from './RadioCard';

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
  const iconGroup = useId();
  const colorGroup = useId();

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
    <div className="grid gap-6 lg:grid-cols-2">
      <div className="flex flex-col gap-5">
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

        <fieldset className="flex flex-col gap-2">
          <legend className="mb-2 text-sm font-medium">{t('wizard.basics.icon')}</legend>
          <div
            className="flex flex-wrap gap-1.5"
            role="radiogroup"
            aria-label={t('wizard.basics.icon')}
          >
            {(Object.keys(PRESET_ICONS) as InstanceIconPreset[]).map((p) => {
              const Icon = PRESET_ICONS[p];
              const on = state.preset === p;
              return (
                <label
                  key={p}
                  className={cn(
                    'has-[:focus-visible]:ring-accent/60 grid size-10 cursor-pointer place-items-center rounded-lg border text-white transition-transform has-[:focus-visible]:ring-2',
                    on
                      ? 'border-accent scale-105'
                      : 'border-transparent opacity-70 hover:opacity-100',
                  )}
                  style={{ background: state.color }}
                >
                  <input
                    type="radio"
                    name={iconGroup}
                    className="sr-only"
                    checked={on}
                    onChange={() => update({ preset: p })}
                    aria-label={t(`wizard.basics.icons.${p}`)}
                  />
                  <Icon className="size-5" aria-hidden />
                </label>
              );
            })}
          </div>
          <div
            className="flex flex-wrap gap-1.5"
            role="radiogroup"
            aria-label={t('wizard.basics.color')}
          >
            {PRESET_COLORS.map((c, i) => (
              <label
                key={c}
                className={cn(
                  'has-[:focus-visible]:ring-accent/60 size-6 cursor-pointer rounded-full border-2 has-[:focus-visible]:ring-2',
                  state.color === c ? 'border-fg' : 'border-transparent',
                )}
                style={{ background: c }}
              >
                <input
                  type="radio"
                  name={colorGroup}
                  className="sr-only"
                  checked={state.color === c}
                  onChange={() => update({ color: c })}
                  aria-label={t('wizard.basics.colorN', { n: i + 1 })}
                />
              </label>
            ))}
          </div>
        </fieldset>

        <fieldset className="flex flex-col gap-2">
          <legend className="mb-2 text-sm font-medium">{t('wizard.basics.type')}</legend>
          <div className="grid gap-3 sm:grid-cols-2">
            <RadioCard
              name="engine"
              value="rustic"
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
              checked={state.engine === 'horizon'}
              onChange={() => update({ engine: 'horizon' })}
              icon={<Server />}
              title={t('list.engine.horizon')}
              description={t('wizard.basics.horizonDesc')}
              testId="engine-horizon"
            />
          </div>
        </fieldset>

        <div className="grid gap-4 sm:grid-cols-[1fr_8rem]">
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
          <p className="text-fg-muted -mt-2 text-xs" data-testid="version-line">
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
      </div>

      <div className="flex flex-col gap-5">
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-2 text-sm font-medium">{t('wizard.world.title')}</legend>
          <SegmentedControl
            label={t('wizard.world.title')}
            value={w.kind}
            onValueChange={(v) => setWorld({ kind: v as WizardState['world']['kind'] })}
            options={[
              { value: 'new', label: t('wizard.world.new') },
              { value: 'import', label: t('wizard.world.import') },
              { value: 'from_instance', label: t('wizard.world.fromInstance') },
            ]}
          />
          {w.kind === 'new' && (
            <div className="grid gap-3 sm:grid-cols-2">
              <Field
                label={t('wizard.world.seed')}
                hint={t('wizard.world.seedHint')}
                className="sm:col-span-2"
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
                    options={(['default', 'flat', 'large_biomes', 'amplified'] as const).map(
                      (v) => ({ value: v, label: t(`wizard.world.levelTypes.${v}`) }),
                    )}
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
              <div className="flex flex-col justify-end gap-2 pb-1">
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
            <div className="grid gap-3 sm:grid-cols-2">
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
        </fieldset>

        {state.engine === 'rustic' && (
          <fieldset className="flex flex-col gap-2">
            <legend className="mb-1 text-sm font-medium">
              {t('wizard.basics.starterPlugins')}
            </legend>
            <p className="text-fg-muted text-xs">{t('wizard.basics.starterHint')}</p>
            {starters.isLoading && <Skeleton className="h-24" />}
            <ul className="flex flex-col gap-1">
              {starters.data?.map((pl) => {
                const on = state.starterPlugins.includes(pl.id);
                return (
                  <li key={pl.id}>
                    <label className="hover:bg-surface-2 flex cursor-pointer items-start gap-3 rounded-lg px-2 py-2">
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
                      <span className="flex min-w-0 flex-1 flex-col">
                        <span className="text-sm font-medium">
                          {pl.name}{' '}
                          <span className="text-fg-subtle font-mono text-xs">{pl.version}</span>
                        </span>
                        <span className="text-fg-muted text-xs">{pl.description}</span>
                      </span>
                      <span className="flex shrink-0 flex-wrap justify-end gap-1">
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
                    </label>
                  </li>
                );
              })}
            </ul>
          </fieldset>
        )}

        <div
          className={cn(
            'flex flex-col gap-1 rounded-lg border p-3',
            err('eula') ? 'border-danger/60' : 'border-border',
          )}
        >
          <label className="flex cursor-pointer items-start gap-3 text-sm">
            <Checkbox
              className="mt-0.5"
              checked={state.eulaAccepted}
              onCheckedChange={(v) => update({ eulaAccepted: v === true })}
              aria-describedby="eula-hint"
              data-testid="wizard-eula"
            />
            <span>{t('wizard.basics.eula')}</span>
          </label>
          <div className="flex items-center gap-2 pl-7">
            <p id="eula-hint" className="text-fg-subtle text-xs">
              {t('wizard.basics.eulaHint')}
            </p>
            <Button
              variant="link"
              size="sm"
              rightIcon={<ExternalLink />}
              onClick={() => void invoke('app_open_url', { url: EULA_URL })}
            >
              {t('wizard.basics.eulaRead')}
            </Button>
          </div>
          <FieldError msg={err('eula')} />
        </div>
      </div>
    </div>
  );
}
