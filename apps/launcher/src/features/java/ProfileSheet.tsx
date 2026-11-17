/**
 * Create or edit a Java profile (ADR 0172): name, game version (snapshots and old versions behind
 * toggles), mod loader and its build, memory, resolution, Java, JVM arguments, game folder; and
 * deleting it.
 */
import {
  Button,
  Checkbox,
  Dialog,
  Field,
  Input,
  Select,
  Sheet,
  Slider,
  Switch,
  SwitchRow,
  Textarea,
  toast,
} from '@rustic/ui';
import { useQueryClient } from '@tanstack/react-query';
import { FolderOpen, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import type { JavaLoader, JavaProfile } from '@/ipc/types/java';
import { useSettings, useSystemInfo } from '@/queries/core';
import {
  formatJvmArgs,
  formatMemory,
  LOADER_NAMES,
  LOADERS,
  memoryMax,
  parseJvmArgs,
} from './logic';
import { jk, useJavaVersions, useLoaderVersions } from './queries';

interface Draft {
  name: string;
  game_version: string;
  loader: JavaLoader;
  loader_version: string;
  memory_max_mb: number;
  custom_resolution: boolean;
  width: string;
  height: string;
  fullscreen: boolean;
  java_path: string;
  jvm_args: string;
  game_dir: string;
}

const LATEST = '';

function draftOf(p: JavaProfile | undefined, memory: number): Draft {
  return {
    name: p?.name ?? '',
    game_version: p?.game_version ?? '',
    loader: p?.loader ?? 'vanilla',
    loader_version: p?.loader_version ?? LATEST,
    memory_max_mb: p?.memory_max_mb ?? memory,
    custom_resolution: !!p?.resolution,
    width: String(p?.resolution?.width ?? 1280),
    height: String(p?.resolution?.height ?? 720),
    fullscreen: p?.fullscreen ?? false,
    java_path: p?.java_path ?? '',
    jvm_args: formatJvmArgs(p?.jvm_args ?? []),
    game_dir: p?.game_dir ?? '',
  };
}

export function ProfileSheet({
  open,
  onOpenChange,
  profile,
  onCreated,
  onDeleted,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** Editing this profile; creating when absent. */
  profile: JavaProfile | undefined;
  onCreated: (p: JavaProfile) => void;
  onDeleted: () => void;
}) {
  const { t } = useTranslation('java');
  const { t: ts } = useTranslation('settings');
  const { t: tc } = useTranslation();
  const qc = useQueryClient();
  const { data: settings } = useSettings();
  const { data: system } = useSystemInfo();
  const defaultMemory = settings?.game.default_memory_mb ?? 4096;
  const [d, setD] = useState<Draft>(() => draftOf(profile, defaultMemory));
  const [snapshots, setSnapshots] = useState(false);
  const [old, setOld] = useState(false);
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleteFiles, setDeleteFiles] = useState(false);
  const versions = useJavaVersions(snapshots, old);
  const loaders = useLoaderVersions(d.loader, d.game_version);
  const maxMemory = memoryMax(system ? system.total_memory_mb * 1024 * 1024 : undefined);
  const memory = Math.min(d.memory_max_mb, maxMemory);
  const memoryWarn = Math.max(2048, maxMemory - 2048);

  useEffect(() => {
    if (!open) return;
    setD(draftOf(profile, defaultMemory));
    setSnapshots(!!profile && /snapshot|-pre|-rc|w\d/.test(profile.game_version));
    setOld(!!profile && /^[ab]\d/.test(profile.game_version));
  }, [open, profile, defaultMemory]);

  // A new profile starts on the newest release.
  useEffect(() => {
    if (open && !profile && !d.game_version && versions.data?.length) {
      const release = versions.data.find((v) => v.kind === 'release') ?? versions.data[0]!;
      setD((o) => ({ ...o, game_version: release.id }));
    }
  }, [open, profile, d.game_version, versions.data]);

  const set = <K extends keyof Draft>(k: K, v: Draft[K]) => setD((o) => ({ ...o, [k]: v }));

  const versionOptions = useMemo(() => {
    const list = versions.data ?? [];
    const opts = list.map((v) => ({
      value: v.id,
      label: v.id,
      description: t(`versions.kind.${v.kind}`),
    }));
    if (d.game_version && !list.some((v) => v.id === d.game_version))
      opts.unshift({ value: d.game_version, label: d.game_version, description: '' });
    return opts;
  }, [versions.data, d.game_version, t]);

  const loaderOptions = useMemo(() => {
    const list = loaders.data ?? [];
    return [
      { value: '__latest', label: t('loader.latest') },
      ...list.map((v) => ({
        value: v.version,
        label: v.version,
        description: v.stable ? t('loader.stable') : t('loader.beta'),
      })),
    ];
  }, [loaders.data, t]);

  const noLoaderBuild =
    d.loader !== 'vanilla' && !!loaders.data && loaders.data.length === 0 && !!d.game_version;
  const width = Number(d.width);
  const height = Number(d.height);
  const resolutionOk =
    !d.custom_resolution ||
    (Number.isInteger(width) && Number.isInteger(height) && width >= 320 && height >= 240);
  const valid = d.name.trim().length > 0 && !!d.game_version && resolutionOk && !noLoaderBuild;

  const pick = async (kind: 'file' | 'folder', key: 'java_path' | 'game_dir') => {
    const path = await invoke('app_pick_path', {
      kind,
      title: key === 'java_path' ? t('form.javaPick') : t('form.gameDirPick'),
    });
    if (path) set(key, path);
  };

  const save = async () => {
    setSaving(true);
    try {
      if (!profile) {
        const p = await invoke('java_profile_create', {
          name: d.name.trim(),
          game_version: d.game_version,
          loader: d.loader,
          loader_version: d.loader === 'vanilla' ? null : d.loader_version || null,
          memory_max_mb: d.memory_max_mb,
        });
        // Settings the create call doesn't take.
        const extra = patchOf(d);
        const saved = Object.keys(extra).length
          ? await invoke('java_profile_update', { id: p.id, patch: extra })
          : p;
        toast({ tone: 'success', title: t('form.created', { name: saved.name }) });
        onCreated(saved);
      } else {
        await invoke('java_profile_update', {
          id: profile.id,
          patch: {
            name: d.name.trim(),
            game_version: d.game_version,
            loader: d.loader,
            loader_version: d.loader === 'vanilla' ? null : d.loader_version || null,
            memory_max_mb: d.memory_max_mb,
            ...patchOf(d),
            resolution: d.custom_resolution ? { width, height } : null,
            java_path: d.java_path.trim() || null,
            game_dir: d.game_dir.trim() || null,
            jvm_args: parseJvmArgs(d.jvm_args),
            fullscreen: d.fullscreen,
          },
        });
        toast({ tone: 'success', title: t('form.saved') });
        onOpenChange(false);
      }
      await qc.invalidateQueries({ queryKey: jk.profiles });
    } catch (e) {
      toast({
        tone: 'error',
        title: t('form.failed'),
        description: e instanceof IpcFailure ? e.message : String(e),
      });
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!profile) return;
    try {
      await invoke('java_profile_delete', { id: profile.id, delete_files: deleteFiles });
      await qc.invalidateQueries({ queryKey: jk.profiles });
      setConfirmDelete(false);
      onDeleted();
    } catch (e) {
      toast({
        tone: 'error',
        title: t('form.failed'),
        description: e instanceof IpcFailure ? e.message : String(e),
      });
    }
  };

  return (
    <>
      <Sheet
        open={open}
        onOpenChange={onOpenChange}
        title={profile ? t('form.editTitle', { name: profile.name }) : t('form.createTitle')}
        description={t('form.description')}
        closeLabel={tc('actions.close')}
        width={480}
        footer={
          <div className="flex w-full items-center justify-end gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              {tc('actions.cancel')}
            </Button>
            <Button
              variant="primary"
              disabled={!valid}
              loading={saving}
              onClick={() => void save()}
              data-testid="java-profile-save"
            >
              {profile ? t('form.save') : t('form.create')}
            </Button>
          </div>
        }
      >
        <div className="flex flex-col gap-4" data-testid="java-profile-form">
          <Field label={t('form.name')}>
            {(f) => (
              <Input
                {...f}
                value={d.name}
                maxLength={64}
                placeholder={t('form.namePlaceholder')}
                onChange={(e) => set('name', e.target.value)}
                data-testid="java-profile-name"
              />
            )}
          </Field>
          <Field label={t('form.version')}>
            {(f) => (
              <Select
                id={f.id}
                value={d.game_version || undefined}
                onValueChange={(v) =>
                  setD((o) => ({ ...o, game_version: v, loader_version: LATEST }))
                }
                options={versionOptions}
                placeholder={versions.isLoading ? t('form.loading') : t('form.pickVersion')}
                aria-label={t('form.version')}
              />
            )}
          </Field>
          <div className="-mt-1 flex flex-wrap gap-x-5 gap-y-2 text-sm">
            <label className="flex items-center gap-2">
              <Switch
                checked={snapshots}
                onCheckedChange={setSnapshots}
                data-testid="java-snapshots"
              />
              {t('form.snapshots')}
            </label>
            <label className="flex items-center gap-2">
              <Switch checked={old} onCheckedChange={setOld} data-testid="java-old" />
              {t('form.old')}
            </label>
          </div>
          <Field label={t('form.loader')}>
            {(f) => (
              <Select
                id={f.id}
                value={d.loader}
                onValueChange={(v) =>
                  setD((o) => ({ ...o, loader: v as JavaLoader, loader_version: LATEST }))
                }
                options={LOADERS.map((l) => ({ value: l, label: LOADER_NAMES[l] }))}
                aria-label={t('form.loader')}
              />
            )}
          </Field>
          {d.loader !== 'vanilla' && (
            <Field
              label={t('form.loaderVersion', { loader: LOADER_NAMES[d.loader] })}
              error={
                noLoaderBuild
                  ? t('loader.none', { loader: LOADER_NAMES[d.loader], version: d.game_version })
                  : undefined
              }
            >
              {(f) => (
                <Select
                  id={f.id}
                  value={d.loader_version || '__latest'}
                  onValueChange={(v) => set('loader_version', v === '__latest' ? LATEST : v)}
                  options={loaderOptions}
                  disabled={loaders.isLoading || noLoaderBuild}
                  aria-label={t('form.loaderVersion', { loader: LOADER_NAMES[d.loader] })}
                />
              )}
            </Field>
          )}
          <Field label={t('form.memory')} hint={t('form.memoryHint')}>
            {() => (
              // The amount itself, what the PC has, and a few landmarks: a bare slider says nothing.
              <div className="flex flex-col gap-2" data-testid="profile-memory">
                <div className="flex items-baseline justify-between gap-3">
                  <span
                    className="text-fg text-lg font-semibold tabular-nums"
                    data-testid="profile-memory-value"
                  >
                    {formatMemory(memory)}
                  </span>
                  {system && (
                    <span className="text-fg-subtle text-xs">
                      {ts('game.memory.system', {
                        total: formatMemory(system.total_memory_mb),
                        free: formatMemory(system.available_memory_mb),
                      })}
                    </span>
                  )}
                </div>
                <Slider
                  value={memory}
                  onValueChange={(v) => set('memory_max_mb', v)}
                  min={1024}
                  max={maxMemory}
                  step={512}
                  label={t('form.memory')}
                  formatValue={formatMemory}
                  warnAbove={memoryWarn}
                  marks={[4096, 8192, 16384]
                    .filter((v) => v < maxMemory)
                    .map((value) => ({ value }))}
                />
                <p
                  className={
                    memory > memoryWarn ? 'text-warning-text text-xs' : 'text-fg-subtle text-xs'
                  }
                  role={memory > memoryWarn ? 'note' : undefined}
                >
                  {memory > memoryWarn ? ts('game.memory.warn') : ts('game.memory.perInstance')}
                </p>
              </div>
            )}
          </Field>
          <SwitchRow
            label={t('form.resolution')}
            description={t('form.resolutionHint')}
            checked={d.custom_resolution}
            onCheckedChange={(v) => set('custom_resolution', v)}
            className="py-0"
          />
          {d.custom_resolution && (
            <div className="flex items-center gap-2">
              <Input
                inputMode="numeric"
                value={d.width}
                onChange={(e) => set('width', e.target.value.replace(/\D/g, ''))}
                aria-label={t('form.width')}
                className="w-28"
              />
              <span className="text-fg-subtle">×</span>
              <Input
                inputMode="numeric"
                value={d.height}
                onChange={(e) => set('height', e.target.value.replace(/\D/g, ''))}
                aria-label={t('form.height')}
                className="w-28"
              />
            </div>
          )}
          <SwitchRow
            label={t('form.fullscreen')}
            checked={d.fullscreen}
            onCheckedChange={(v) => set('fullscreen', v)}
            className="py-0"
          />
          <Field label={t('form.java')} hint={t('form.javaHint')}>
            {(f) => (
              <div className="flex gap-2">
                <Input
                  {...f}
                  value={d.java_path}
                  placeholder={t('form.javaAuto')}
                  onChange={(e) => set('java_path', e.target.value)}
                />
                <Button
                  variant="secondary"
                  leftIcon={<FolderOpen />}
                  onClick={() => void pick('file', 'java_path')}
                >
                  {t('form.browse')}
                </Button>
              </div>
            )}
          </Field>
          <Field label={t('form.jvmArgs')} hint={t('form.jvmArgsHint')}>
            {(f) => (
              <Textarea
                {...f}
                rows={2}
                value={d.jvm_args}
                placeholder="-XX:+UseG1GC"
                onChange={(e) => set('jvm_args', e.target.value)}
                className="font-mono text-xs"
              />
            )}
          </Field>
          <Field
            label={t('form.gameDir')}
            hint={profile ? profile.game_dir_path : t('form.gameDirHint')}
          >
            {(f) => (
              <div className="flex gap-2">
                <Input
                  {...f}
                  value={d.game_dir}
                  placeholder={t('form.gameDirAuto')}
                  onChange={(e) => set('game_dir', e.target.value)}
                />
                <Button
                  variant="secondary"
                  leftIcon={<FolderOpen />}
                  onClick={() => void pick('folder', 'game_dir')}
                >
                  {t('form.browse')}
                </Button>
              </div>
            )}
          </Field>
          {profile && (
            <div className="border-danger/30 mt-2 flex items-center justify-between gap-3 rounded-lg border p-3">
              <div className="flex flex-col">
                <span className="text-sm font-semibold">{t('form.delete')}</span>
                <span className="text-fg-muted text-xs">{t('form.deleteHint')}</span>
              </div>
              <Button
                variant="danger"
                size="sm"
                leftIcon={<Trash2 />}
                disabled={profile.running}
                onClick={() => setConfirmDelete(true)}
                data-testid="java-profile-delete"
              >
                {t('form.delete')}
              </Button>
            </div>
          )}
        </div>
      </Sheet>
      <Dialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        tone="destructive"
        title={t('form.deleteTitle', { name: profile?.name ?? '' })}
        description={t('form.deleteBody')}
        closeLabel={tc('actions.close')}
        footer={
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              {tc('actions.cancel')}
            </Button>
            <Button
              variant="danger"
              onClick={() => void remove()}
              data-testid="java-delete-confirm"
            >
              {t('form.delete')}
            </Button>
          </div>
        }
      >
        <label className="flex items-center gap-2 text-sm">
          <Checkbox checked={deleteFiles} onCheckedChange={(v) => setDeleteFiles(v === true)} />
          {t('form.deleteFiles')}
        </label>
      </Dialog>
    </>
  );
}

/** The settings a new profile gets with a second call (the create call takes the basics). */
function patchOf(d: Draft) {
  const patch: Record<string, unknown> = {};
  if (d.custom_resolution) patch.resolution = { width: Number(d.width), height: Number(d.height) };
  if (d.fullscreen) patch.fullscreen = true;
  if (d.java_path.trim()) patch.java_path = d.java_path.trim();
  if (d.game_dir.trim()) patch.game_dir = d.game_dir.trim();
  const args = parseJvmArgs(d.jvm_args);
  if (args.length) patch.jvm_args = args;
  return patch;
}
