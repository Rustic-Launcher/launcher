/** The Java Launchpad's right column: the selected profile's log and its mods (ADR 0172). */
import {
  Button,
  cn,
  EmptyState,
  formatBytes,
  IconButton,
  SearchInput,
  SegmentedControl,
  Skeleton,
  Switch,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  toast,
} from '@rustic/ui';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Copy, Download, FileText, FolderOpen, Puzzle, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, IpcFailure } from '@/ipc';
import type { JavaProfile } from '@/ipc/types/java';
import { formatCompact } from '@/lib/format';
import { filterLog, LOADER_NAMES } from './logic';
import { jk, useJavaLive, useJavaMods } from './queries';

export function JavaAside({
  profile,
  onEdit,
}: {
  profile: JavaProfile | undefined;
  onEdit: (id: string) => void;
}) {
  const { t } = useTranslation('java');
  const [tab, setTab] = useState('log');
  return (
    <aside className="flex min-w-0 flex-col gap-3" aria-label={t('aside.label')}>
      <Tabs value={tab} onValueChange={setTab} className="min-h-0 flex-1 gap-3">
        <TabsList>
          <TabsTrigger value="log" data-testid="java-tab-log">
            {t('aside.log')}
          </TabsTrigger>
          <TabsTrigger value="mods" data-testid="java-tab-mods">
            {t('aside.mods')}
          </TabsTrigger>
        </TabsList>
        <TabsContent value="log" className="min-h-0 flex-1">
          {profile ? <LogPanel profile={profile} /> : <NoProfile />}
        </TabsContent>
        <TabsContent value="mods" className="min-h-0 flex-1">
          {profile ? <ModsPanel profile={profile} onEdit={onEdit} /> : <NoProfile />}
        </TabsContent>
      </Tabs>
    </aside>
  );
}

function NoProfile() {
  const { t } = useTranslation('java');
  return <p className="text-fg-muted px-1 py-6 text-center text-sm">{t('aside.noProfile')}</p>;
}

const EMPTY: never[] = [];

function LogPanel({ profile }: { profile: JavaProfile }) {
  const { t } = useTranslation('java');
  const lines = useJavaLive((s) => s.logs[profile.id] ?? EMPTY);
  const state = useJavaLive((s) => s.states[profile.id]);
  const [level, setLevel] = useState<'all' | 'warn' | 'error'>('all');
  const [query, setQuery] = useState('');
  const shown = useMemo(() => filterLog(lines, level, query), [lines, level, query]);
  const box = useRef<HTMLDivElement>(null);
  const stick = useRef(true);
  useEffect(() => {
    const el = box.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [shown.length]);

  const copy = () => {
    const text = shown
      .map((l) => `[${l.thread ?? '-'}/${l.level.toUpperCase()}] ${l.text}`)
      .join('\n');
    void navigator.clipboard?.writeText(text).then(
      () => toast({ tone: 'success', title: t('log.copied') }),
      () => undefined,
    );
  };

  return (
    <div className="flex h-full min-h-[420px] flex-col gap-2" data-testid="java-log">
      {state?.state === 'crashed' && (
        <div
          role="alert"
          className="border-danger/40 bg-danger/10 flex flex-col gap-2 rounded-lg border p-3 text-sm"
        >
          <span className="text-danger-text flex items-center gap-2 font-semibold">
            <AlertTriangle className="size-4" />
            {t('hero.crashed', { code: state.exit_code ?? '?' })}
          </span>
          <div className="flex flex-wrap gap-2">
            {state.crash_report && (
              <Button
                size="sm"
                leftIcon={<FileText />}
                onClick={() => void invoke('app_open_path', { path: state.crash_report! })}
                data-testid="java-open-crash"
              >
                {t('log.openCrash')}
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              leftIcon={<FolderOpen />}
              onClick={() =>
                void invoke('java_profile_open_folder', { id: profile.id, folder: 'crash_reports' })
              }
            >
              {t('log.crashFolder')}
            </Button>
          </div>
        </div>
      )}
      <div className="flex items-center gap-2">
        <SegmentedControl
          size="sm"
          label={t('log.level')}
          value={level}
          onValueChange={(v) => setLevel(v as typeof level)}
          options={[
            { value: 'all', label: t('log.all') },
            { value: 'warn', label: t('log.warnings') },
            { value: 'error', label: t('log.errors') },
          ]}
        />
        <div className="flex-1" />
        <IconButton size="sm" label={t('log.copy')} icon={<Copy />} onClick={copy} />
        <IconButton
          size="sm"
          label={t('log.folder')}
          icon={<FolderOpen />}
          onClick={() =>
            void invoke('java_profile_open_folder', { id: profile.id, folder: 'logs' })
          }
        />
      </div>
      <SearchInput
        value={query}
        onValueChange={setQuery}
        clearLabel={t('log.clear')}
        placeholder={t('log.search')}
        className="h-8"
      />
      <div
        ref={box}
        onScroll={(e) => {
          const el = e.currentTarget;
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
        className="border-border bg-surface-2/60 min-h-0 flex-1 overflow-auto rounded-lg border p-2 font-mono text-[11px] leading-relaxed"
        role="log"
        aria-live="off"
        aria-label={t('aside.log')}
      >
        {!shown.length ? (
          <p className="text-fg-subtle p-4 text-center font-sans text-xs">{t('log.empty')}</p>
        ) : (
          shown.map((l, i) => (
            <div
              key={i}
              className={cn(
                'break-words whitespace-pre-wrap',
                l.level === 'error' && 'text-danger-text',
                l.level === 'warn' && 'text-warning-text',
                l.level === 'debug' && 'text-fg-subtle',
              )}
            >
              {l.thread && <span className="text-fg-subtle">[{l.thread}] </span>}
              {l.text}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function ModsPanel({ profile, onEdit }: { profile: JavaProfile; onEdit: (id: string) => void }) {
  const { t, i18n } = useTranslation('java');
  const qc = useQueryClient();
  const vanilla = profile.loader === 'vanilla';
  const [query, setQuery] = useState('');
  const [debounced, setDebounced] = useState('');
  const [installing, setInstalling] = useState<string | null>(null);
  useEffect(() => {
    const h = window.setTimeout(() => setDebounced(query), 300);
    return () => window.clearTimeout(h);
  }, [query]);
  const installed = useJavaMods(profile.id, !vanilla);
  const search = useQuery({
    queryKey: jk.modSearch(profile.id, debounced),
    queryFn: () => invoke('java_mods_search', { profile_id: profile.id, query: debounced }),
    enabled: !vanilla,
    staleTime: 60_000,
  });

  if (vanilla) {
    return (
      <EmptyState
        icon={<Puzzle />}
        title={t('mods.vanillaTitle')}
        description={t('mods.vanilla')}
        action={<Button onClick={() => onEdit(profile.id)}>{t('mods.pickLoader')}</Button>}
      />
    );
  }

  const install = async (projectId: string, name: string) => {
    setInstalling(projectId);
    try {
      await invoke('java_mod_install', { profile_id: profile.id, project_id: projectId });
      toast({ tone: 'success', title: t('mods.installed', { name }) });
      await Promise.all([
        qc.invalidateQueries({ queryKey: jk.mods(profile.id) }),
        qc.invalidateQueries({ queryKey: ['java', 'mod-search', profile.id] }),
      ]);
    } catch (e) {
      toast({
        tone: 'error',
        title: t('mods.failed', { name }),
        description: e instanceof IpcFailure ? e.message : String(e),
      });
    } finally {
      setInstalling(null);
    }
  };

  return (
    <div className="flex flex-col gap-4" data-testid="java-mods">
      <section className="flex flex-col gap-2" aria-labelledby="java-mods-installed">
        <div className="flex items-center justify-between px-1">
          <h3 id="java-mods-installed" className="text-sm font-semibold">
            {t('mods.installedTitle')}
          </h3>
          <Button
            size="sm"
            variant="ghost"
            leftIcon={<FolderOpen />}
            onClick={() =>
              void invoke('java_profile_open_folder', { id: profile.id, folder: 'mods' })
            }
          >
            {t('mods.folder')}
          </Button>
        </div>
        {installed.isLoading && <Skeleton className="h-16 rounded-lg" />}
        {installed.data && !installed.data.length && (
          <p className="text-fg-subtle px-1 text-xs">{t('mods.none')}</p>
        )}
        <ul className="flex flex-col gap-1">
          {installed.data?.map((m) => (
            <li
              key={m.file_name}
              className="hover:bg-surface-2 flex items-center gap-2 rounded-md px-2 py-1.5"
              data-testid={`java-mod-${m.file_name}`}
            >
              <span
                className={cn(
                  'min-w-0 flex-1 truncate text-sm',
                  !m.enabled && 'text-fg-subtle line-through',
                )}
              >
                {m.file_name}
              </span>
              <span className="text-fg-subtle text-xs tabular-nums">{formatBytes(m.size)}</span>
              <Switch
                checked={m.enabled}
                aria-label={t('mods.enabled', { name: m.file_name })}
                onCheckedChange={(v) =>
                  void invoke('java_mod_set_enabled', {
                    profile_id: profile.id,
                    file_name: m.file_name,
                    enabled: v,
                  }).then(() => qc.invalidateQueries({ queryKey: jk.mods(profile.id) }))
                }
              />
              <IconButton
                size="sm"
                variant="danger"
                label={t('mods.remove', { name: m.file_name })}
                icon={<Trash2 />}
                onClick={() =>
                  void invoke('java_mod_remove', {
                    profile_id: profile.id,
                    file_name: m.file_name,
                  }).then(() => qc.invalidateQueries({ queryKey: jk.mods(profile.id) }))
                }
              />
            </li>
          ))}
        </ul>
      </section>
      <section className="flex flex-col gap-2" aria-labelledby="java-mods-find">
        <h3 id="java-mods-find" className="px-1 text-sm font-semibold">
          {t('mods.find')}
        </h3>
        <SearchInput
          value={query}
          onValueChange={setQuery}
          clearLabel={t('log.clear')}
          placeholder={t('mods.search', {
            loader: LOADER_NAMES[profile.loader],
            version: profile.game_version,
          })}
          data-testid="java-mods-search"
        />
        {search.isLoading && <Skeleton className="h-24 rounded-lg" />}
        {search.isError && (
          <p className="text-danger-text px-1 text-xs">{t('mods.searchFailed')}</p>
        )}
        <ul className="flex flex-col gap-1.5">
          {search.data?.hits.map((h) => (
            <li
              key={h.project_id}
              className="border-border bg-surface flex items-center gap-3 rounded-lg border p-2"
              data-testid={`java-hit-${h.slug}`}
            >
              {h.icon_url ? (
                <img src={h.icon_url} alt="" className="size-10 shrink-0 rounded-md" />
              ) : (
                <span className="bg-surface-3 grid size-10 shrink-0 place-items-center rounded-md">
                  <Puzzle className="text-fg-subtle size-5" />
                </span>
              )}
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-semibold">{h.name}</span>
                <span className="text-fg-muted line-clamp-1 text-xs">{h.summary}</span>
                <span className="text-fg-subtle text-[11px]">
                  {h.author} · {formatCompact(h.downloads, i18n.language)}
                  {h.flagged && ` · ${t('mods.flagged')}`}
                </span>
              </span>
              <Button
                size="sm"
                variant={h.installed ? 'ghost' : 'primary'}
                disabled={h.installed}
                loading={installing === h.project_id}
                leftIcon={h.installed ? undefined : <Download />}
                onClick={() => void install(h.project_id, h.name)}
              >
                {h.installed ? t('mods.installedShort') : t('mods.install')}
              </Button>
            </li>
          ))}
        </ul>
        <p className="text-fg-subtle px-1 text-[11px]">{t('mods.source')}</p>
      </section>
    </div>
  );
}
