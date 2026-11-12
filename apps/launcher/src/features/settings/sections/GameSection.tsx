import { prepareOutcome } from '@/lib/prepare';
import {
  Badge,
  Button,
  Input,
  Progress,
  Select,
  Skeleton,
  Slider,
  Switch,
  toast,
} from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { FileSearch, FolderOpen, FolderPen, Hammer, Info, RotateCcw } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { invoke, IpcFailure, listen } from '@/ipc';
import type { GameSettings, PrepareProgress, Settings } from '@/ipc/types/app';
import { formatDate, formatMemory } from '@/lib/format';
import { qk, useSettings, useSystemInfo } from '@/queries/core';
import { RUSTIC_SERVER_ANCHOR } from '../anchors';
import { settingsRoute } from '../routes';
import { SettingRow, SettingsCard, SectionSkeleton, useSaveSettings } from '../components';
import { clampMemory, memoryMax, memoryWarnAbove } from '../validation';

export function GameSection() {
  const { data: settings, isLoading, isError, refetch } = useSettings();
  if (isLoading) return <SectionSkeleton />;
  if (isError || !settings) return <QueryError onRetry={() => void refetch()} />;
  return (
    <div className="flex flex-col gap-4">
      <MemoryCard settings={settings} />
      <PreparationCard settings={settings} />
      <RusticServerCard />
      <FoldersCard game={settings.game} />
      <ResolutionCard game={settings.game} />
    </div>
  );
}

function MemoryCard({ settings }: { settings: Settings }) {
  const { t, i18n } = useTranslation('settings');
  const { data: sys } = useSystemInfo();
  const save = useSaveSettings();
  const [value, setValue] = useState(settings.game.default_memory_mb);
  useEffect(() => setValue(settings.game.default_memory_mb), [settings.game.default_memory_mb]);
  const total = sys?.total_memory_mb ?? 8192;
  const max = memoryMax(total);
  const fmt = (v: number) => formatMemory(v, i18n.language);
  return (
    <SettingsCard title={t('game.memory.title')} description={t('game.memory.description')}>
      {!sys ? (
        <Skeleton className="h-16" />
      ) : (
        <div className="flex flex-col gap-3">
          <div className="flex items-baseline justify-between">
            <span className="text-fg text-2xl font-bold tabular-nums" data-testid="memory-value">
              {fmt(value)}
            </span>
            <span className="text-fg-subtle text-xs">
              {t('game.memory.system', { total: fmt(total), free: fmt(sys.available_memory_mb) })}
            </span>
          </div>
          <Slider
            label={t('game.memory.title')}
            value={value}
            min={1024}
            max={max}
            step={512}
            warnAbove={memoryWarnAbove(total)}
            formatValue={fmt}
            onValueChange={setValue}
            onValueCommit={(v) =>
              save.mutate({ game: { default_memory_mb: clampMemory(v, total) } })
            }
            marks={[4096, 8192, 16384].filter((v) => v <= max - 2048).map((v) => ({ value: v }))}
          />
          {value > memoryWarnAbove(total) && (
            <p className="text-warning-text text-xs" role="note">
              {t('game.memory.warn')}
            </p>
          )}
          <p className="text-fg-subtle text-xs">{t('game.memory.perInstance')}</p>
        </div>
      )}
    </SettingsCard>
  );
}

function PreparationCard({ settings }: { settings: Settings }) {
  const { t, i18n } = useTranslation('settings');
  const qc = useQueryClient();
  const save = useSaveSettings();
  const java = useQuery({ queryKey: qk.java, queryFn: () => invoke('java_runtimes_list') });
  const prep = useQuery({ queryKey: qk.prepare, queryFn: () => invoke('game_prepare_status') });
  const [progress, setProgress] = useState<PrepareProgress | null>(null);
  const running = progress != null && progress.step !== 'done';

  useEffect(
    () =>
      listen('prepare-progress', (p) => {
        setProgress(p);
        const outcome = prepareOutcome(p);
        if (outcome === 'ok') {
          void qc.invalidateQueries({ queryKey: qk.prepare });
          toast({ tone: 'success', title: t('game.prepare.doneToast') });
        } else if (outcome === 'failed') {
          toast({ tone: 'error', title: t('game.prepare.failed'), description: p.message });
        }
      }),
    [qc, t],
  );

  const custom = '__custom__';
  const selected = settings.game.prepare_java_custom_path
    ? custom
    : (settings.game.prepare_java_id ?? '');

  return (
    <SettingsCard
      title={t('game.prepare.title')}
      description={t('game.prepare.description')}
      testId="settings-prepare"
    >
      <SettingRow label={t('game.prepare.status')}>
        {prep.isLoading ? (
          <Skeleton className="h-6 w-32" />
        ) : prep.data ? (
          <span className="flex items-center gap-2 text-sm">
            <Badge
              variant={
                prep.data.state === 'ready'
                  ? 'success'
                  : prep.data.state === 'preparing'
                    ? 'accent'
                    : 'warning'
              }
            >
              {t(`game.prepare.states.${prep.data.state}`)}
            </Badge>
            <span className="text-fg-subtle text-xs">
              {t('game.prepare.gameVersion', { version: prep.data.game_version })}
              {prep.data.prepared_at && ` · ${formatDate(prep.data.prepared_at, i18n.language)}`}
            </span>
          </span>
        ) : null}
      </SettingRow>
      {prep.data?.source_path && (
        <SettingRow label={t('game.prepare.source')} description={t('game.prepare.sourceHint')}>
          <code className="bg-surface-2 text-fg-muted max-w-80 truncate rounded px-2 py-1 text-xs">
            {prep.data.source_path}
          </code>
        </SettingRow>
      )}
      <SettingRow
        label={t('game.java.label')}
        description={t('game.java.description')}
        htmlFor="prepare-java"
      >
        {java.isLoading ? (
          <Skeleton className="h-9 w-64" />
        ) : (
          <Select
            id="prepare-java"
            className="w-72"
            value={selected}
            onValueChange={async (v) => {
              if (v === custom) {
                const path = await invoke('app_pick_path', {
                  kind: 'file',
                  title: t('game.java.pickTitle'),
                });
                if (path) save.mutate({ game: { prepare_java_custom_path: path } });
              } else {
                save.mutate({ game: { prepare_java_id: v, prepare_java_custom_path: null } });
              }
            }}
            options={[
              ...(java.data ?? []).map((j) => ({
                value: j.id,
                label: `${j.vendor} ${j.version}`,
                description: t(`game.java.source.${j.source}`),
              })),
              {
                value: custom,
                label: settings.game.prepare_java_custom_path ?? t('game.java.custom'),
                description: t('game.java.customHint'),
              },
            ]}
          />
        )}
      </SettingRow>
      {running && progress && (
        <div className="mt-2 flex flex-col gap-1.5" aria-live="polite">
          <Progress value={progress.progress} label={t('game.prepare.title')} />
          <span className="text-fg-subtle text-xs">{t(`game.prepare.steps.${progress.step}`)}</span>
        </div>
      )}
      <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
        <p className="text-fg-subtle flex max-w-xl items-start gap-2 text-xs">
          <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
          {t('game.prepare.legal')}
        </p>
        <Button
          leftIcon={<Hammer />}
          loading={running}
          onClick={() => {
            setProgress({ step: 'detect', progress: 0 });
            invoke('game_prepare', { source_path: prep.data?.source_path ?? null }).catch(
              (e: Error) => {
                setProgress(null);
                toast({ tone: 'error', title: t('game.prepare.failed'), description: e.message });
              },
            );
          }}
        >
          {t('game.prepare.again')}
        </Button>
      </div>
    </SettingsCard>
  );
}

/**
 * The `rustic-server` program Rustic Servers run (launcher ADR 0152): the one in use and where it
 * comes from, a manual override checked with `--rustic-version`, and back to automatic.
 */
function RusticServerCard() {
  const { t } = useTranslation('settings');
  const { t: tc } = useTranslation();
  const qc = useQueryClient();
  const { focus } = settingsRoute.useSearch();
  const ref = useRef<HTMLDivElement>(null);
  const status = useQuery({
    queryKey: qk.rusticServer,
    queryFn: () => invoke('rustic_server_status'),
  });
  const set = useMutation({
    mutationFn: (path: string | null) => invoke('rustic_server_set', { path }),
    onSuccess: (s, path) => {
      qc.setQueryData(qk.rusticServer, s);
      toast({
        tone: 'success',
        title: path ? t('game.rusticServer.saved') : t('game.rusticServer.automaticSaved'),
      });
    },
    onError: (e) =>
      toast({
        tone: 'error',
        title: t('game.rusticServer.failed'),
        description:
          e instanceof IpcFailure && e.details?.reason === 'server.not_rustic_server'
            ? t('game.rusticServer.invalid')
            : e.message,
      }),
  });

  // Arriving from a "rustic-server not found" error: bring the row into view.
  const loaded = status.isSuccess;
  useEffect(() => {
    if (focus === RUSTIC_SERVER_ANCHOR && loaded)
      ref.current?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }, [focus, loaded]);

  const s = status.data;
  const configuredMissing = !!s?.configured && s.origin !== 'configured';
  return (
    <div ref={ref} id={RUSTIC_SERVER_ANCHOR} className="scroll-mt-6">
      <SettingsCard
        title={t('game.rusticServer.title')}
        description={t('game.rusticServer.description')}
        testId="settings-rustic-server"
      >
        <SettingRow
          label={t('game.rusticServer.program')}
          description={
            status.isLoading ? (
              <Skeleton className="h-4 w-64" />
            ) : s?.path ? (
              <span className="flex flex-col gap-0.5">
                <code className="text-fg-subtle break-all" data-testid="rustic-server-path">
                  {s.path}
                </code>
                {s.version && <span className="text-fg-subtle">{s.version}</span>}
              </span>
            ) : configuredMissing ? (
              <span className="text-warning-text">
                {t('game.rusticServer.configuredMissing', { path: s?.configured ?? '' })}
              </span>
            ) : (
              <span>{t('game.rusticServer.notFoundHint')}</span>
            )
          }
        >
          {status.isError ? (
            <Button size="sm" variant="ghost" onClick={() => void status.refetch()}>
              {tc('actions.retry')}
            </Button>
          ) : s ? (
            <Badge variant={s.path ? 'success' : 'warning'} data-testid="rustic-server-origin">
              {s.origin
                ? t(`game.rusticServer.origin.${s.origin}`, { build: s.build ?? '' })
                : t('game.rusticServer.notFound')}
            </Badge>
          ) : null}
        </SettingRow>
        <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
          <p className="text-fg-subtle flex max-w-xl items-start gap-2 text-xs">
            <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
            {t('game.rusticServer.order')}
          </p>
          <div className="flex items-center gap-2">
            {s?.configured && (
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<RotateCcw />}
                loading={set.isPending && set.variables === null}
                onClick={() => set.mutate(null)}
              >
                {t('game.rusticServer.automatic')}
              </Button>
            )}
            <Button
              size="sm"
              leftIcon={<FileSearch />}
              loading={set.isPending && set.variables !== null}
              onClick={async () => {
                const path = await invoke('app_pick_path', {
                  kind: 'file',
                  title: t('game.rusticServer.pickTitle'),
                });
                if (path) set.mutate(path);
              }}
            >
              {t('game.rusticServer.choose')}
            </Button>
          </div>
        </div>
      </SettingsCard>
    </div>
  );
}

function FoldersCard({ game }: { game: GameSettings }) {
  const { t } = useTranslation('settings');
  const save = useSaveSettings();
  const rows: { key: 'game_dir' | 'instances_dir' | 'servers_dir'; label: string }[] = [
    { key: 'game_dir', label: t('game.folders.data') },
    { key: 'instances_dir', label: t('game.folders.instances') },
    { key: 'servers_dir', label: t('game.folders.servers') },
  ];
  return (
    <SettingsCard title={t('game.folders.title')} description={t('game.folders.description')}>
      {rows.map((r) => (
        <SettingRow
          key={r.key}
          label={r.label}
          description={<code className="text-fg-subtle break-all">{game[r.key]}</code>}
        >
          <Button
            size="sm"
            variant="ghost"
            leftIcon={<FolderOpen />}
            onClick={() => void invoke('app_open_path', { path: game[r.key] })}
          >
            {t('game.folders.open')}
          </Button>
          <Button
            size="sm"
            leftIcon={<FolderPen />}
            onClick={async () => {
              const path = await invoke('app_pick_path', { kind: 'folder', title: r.label });
              if (path) {
                save.mutate({ game: { [r.key]: path } });
                toast({
                  tone: 'info',
                  title: t('game.folders.changed'),
                  description: t('game.folders.moveNote'),
                });
              }
            }}
          >
            {t('game.folders.change')}
          </Button>
        </SettingRow>
      ))}
    </SettingsCard>
  );
}

function ResolutionCard({ game }: { game: GameSettings }) {
  const { t } = useTranslation('settings');
  const save = useSaveSettings();
  const res = game.default_resolution;
  const [w, setW] = useState(String(res.width));
  const [h, setH] = useState(String(res.height));
  const commit = () => {
    const width = Number.parseInt(w, 10);
    const height = Number.parseInt(h, 10);
    if (width >= 640 && height >= 480 && (width !== res.width || height !== res.height)) {
      save.mutate({ game: { default_resolution: { ...res, width, height } } });
    } else {
      setW(String(res.width));
      setH(String(res.height));
    }
  };
  return (
    <SettingsCard title={t('game.resolution.title')} description={t('game.resolution.description')}>
      <SettingRow label={t('game.resolution.size')}>
        <Input
          aria-label={t('game.resolution.width')}
          inputMode="numeric"
          className="w-24"
          value={w}
          onChange={(e) => setW(e.target.value.replace(/\D/g, ''))}
          onBlur={commit}
        />
        <span className="text-fg-subtle" aria-hidden>
          ×
        </span>
        <Input
          aria-label={t('game.resolution.height')}
          inputMode="numeric"
          className="w-24"
          value={h}
          onChange={(e) => setH(e.target.value.replace(/\D/g, ''))}
          onBlur={commit}
        />
      </SettingRow>
      <SettingRow label={t('game.resolution.fullscreen')} htmlFor="res-fullscreen">
        <Switch
          id="res-fullscreen"
          checked={res.fullscreen}
          onCheckedChange={(v) =>
            save.mutate({ game: { default_resolution: { ...res, fullscreen: v } } })
          }
        />
      </SettingRow>
      <p className="text-fg-subtle text-xs">{t('game.resolution.hint')}</p>
    </SettingsCard>
  );
}
