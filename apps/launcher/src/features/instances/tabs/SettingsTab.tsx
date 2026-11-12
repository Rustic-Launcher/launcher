import {
  Button,
  Card,
  CardDescription,
  CardTitle,
  Field,
  Input,
  SwitchRow,
  toast,
} from '@rustic/ui';
import { Camera, FileText, FolderOpen, Globe2, Layers, Package, Sun } from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Instance, InstanceFolder, InstancePatch } from '@/ipc/types/instances';
import { useSettings } from '@/queries/core';
import { useInstanceMutations } from '../queries';

interface Draft {
  name: string;
  useDefaultResolution: boolean;
  width: string;
  height: string;
  fullscreen: boolean;
  game_args: string;
}

function toDraft(i: Instance): Draft {
  return {
    name: i.name,
    useDefaultResolution: i.resolution == null,
    width: String(i.resolution?.width ?? 1600),
    height: String(i.resolution?.height ?? 900),
    fullscreen: i.resolution?.fullscreen ?? false,
    game_args: i.game_args,
  };
}

const FOLDERS: { id: InstanceFolder; icon: ReactNode }[] = [
  { id: 'root', icon: <FolderOpen /> },
  { id: 'mods', icon: <Package /> },
  { id: 'resourcepacks', icon: <Layers /> },
  { id: 'shaderpacks', icon: <Sun /> },
  { id: 'saves', icon: <Globe2 /> },
  { id: 'screenshots', icon: <Camera /> },
  { id: 'logs', icon: <FileText /> },
];

/**
 * Per-instance settings. Horizon instances have no RAM / Java / loader fields (they only pin a
 * Horizon build: docs/HORIZON_FACTS.md A10), so this page offers name, resolution, launch options
 * and folders; the build is changed from Overview.
 */
export function SettingsTab({ instance }: { instance: Instance }) {
  const { t } = useTranslation('instances');
  const { t: tc } = useTranslation();
  const { data: settings } = useSettings();
  const { update, openFolder } = useInstanceMutations();
  const [draft, setDraft] = useState<Draft>(() => toDraft(instance));

  useEffect(() => setDraft(toDraft(instance)), [instance]);

  const width = Number(draft.width);
  const height = Number(draft.height);
  const resError =
    !draft.useDefaultResolution &&
    (!Number.isInteger(width) || !Number.isInteger(height) || width < 640 || height < 360)
      ? t('settings.resolutionError')
      : undefined;
  const nameError = !draft.name.trim() ? t('new.nameRequired') : undefined;

  const patch = useMemo<InstancePatch>(() => {
    const p: InstancePatch = {};
    if (draft.name.trim() !== instance.name) p.name = draft.name.trim();
    const res = draft.useDefaultResolution ? null : { width, height, fullscreen: draft.fullscreen };
    if (JSON.stringify(res) !== JSON.stringify(instance.resolution)) p.resolution = res;
    if (draft.game_args !== instance.game_args) p.game_args = draft.game_args;
    return p;
  }, [draft, instance, width, height]);
  const dirty = Object.keys(patch).length > 0;
  const set = <K extends keyof Draft>(k: K, v: Draft[K]) => setDraft((d) => ({ ...d, [k]: v }));
  const def = settings?.game.default_resolution;

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        if (!dirty || resError || nameError) return;
        update.mutate(
          { id: instance.id, patch },
          { onSuccess: () => toast({ tone: 'success', title: t('settings.saved') }) },
        );
      }}
    >
      <Card padding="lg" className="flex flex-col gap-4">
        <CardTitle>{t('settings.general')}</CardTitle>
        <Field label={t('new.name')} error={nameError}>
          {(p) => (
            <Input
              {...p}
              value={draft.name}
              onChange={(e) => set('name', e.target.value)}
              maxLength={48}
              invalid={!!nameError}
            />
          )}
        </Field>
      </Card>

      <Card padding="lg" className="flex flex-col gap-2">
        <CardTitle>{t('settings.display')}</CardTitle>
        <SwitchRow
          label={t('settings.useDefaultResolution')}
          description={
            def ? t('settings.defaultIs', { width: def.width, height: def.height }) : undefined
          }
          checked={draft.useDefaultResolution}
          onCheckedChange={(v) => set('useDefaultResolution', v)}
        />
        {!draft.useDefaultResolution && (
          <div className="flex flex-col gap-3">
            <div className="grid grid-cols-2 gap-3">
              <Field label={t('settings.width')} error={resError}>
                {(p) => (
                  <Input
                    {...p}
                    inputMode="numeric"
                    value={draft.width}
                    onChange={(e) => set('width', e.target.value)}
                    invalid={!!resError}
                  />
                )}
              </Field>
              <Field label={t('settings.height')}>
                {(p) => (
                  <Input
                    {...p}
                    inputMode="numeric"
                    value={draft.height}
                    onChange={(e) => set('height', e.target.value)}
                    invalid={!!resError}
                  />
                )}
              </Field>
            </div>
            <SwitchRow
              label={t('settings.fullscreen')}
              checked={draft.fullscreen}
              onCheckedChange={(v) => set('fullscreen', v)}
            />
          </div>
        )}
      </Card>

      <Card padding="lg" className="flex flex-col gap-3">
        <div className="flex flex-col gap-1">
          <CardTitle>{t('settings.launch')}</CardTitle>
          <CardDescription>{t('settings.launchHint')}</CardDescription>
        </div>
        <Field label={t('settings.gameArgs')} hint={t('settings.gameArgsHint')}>
          {(p) => (
            <Input
              {...p}
              value={draft.game_args}
              onChange={(e) => set('game_args', e.target.value)}
              className="font-mono"
              placeholder="--quickPlaySingleplayer …"
            />
          )}
        </Field>
      </Card>

      <Card padding="lg" className="flex flex-col gap-3">
        <div className="flex flex-col gap-1">
          <CardTitle>{t('settings.folders')}</CardTitle>
          <CardDescription className="truncate font-mono text-xs">
            {instance.folder}
          </CardDescription>
        </div>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-2">
          {FOLDERS.map((f) => (
            <Button
              key={f.id}
              leftIcon={f.icon}
              className="justify-start"
              onClick={() => openFolder.mutate({ id: instance.id, folder: f.id })}
            >
              {t(`settings.folder.${f.id}`)}
            </Button>
          ))}
        </div>
      </Card>

      <div className="border-border bg-bg-elevated/90 sticky bottom-0 z-10 flex items-center justify-end gap-2 rounded-xl border px-4 py-3 backdrop-blur-md">
        <span className="text-fg-muted mr-auto text-sm" aria-live="polite">
          {dirty ? t('settings.unsaved') : t('settings.allSaved')}
        </span>
        <Button disabled={!dirty} onClick={() => setDraft(toDraft(instance))}>
          {tc('actions.reset')}
        </Button>
        <Button
          type="submit"
          variant="primary"
          disabled={!dirty || !!resError || !!nameError}
          loading={update.isPending}
        >
          {tc('actions.save')}
        </Button>
      </div>
    </form>
  );
}
