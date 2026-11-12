import {
  Badge,
  Button,
  Input,
  SegmentedControl,
  Select,
  Skeleton,
  Switch,
  Textarea,
  toast,
} from '@rustic/ui';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { RotateCw } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QueryError } from '@/components/RouteError';
import { invoke } from '@/ipc';
import type { ConfigField, Server } from '@/ipc/types/servers';
import { mk, useConfig, useRawConfig } from './queries';
import { Panel } from './shared';

type Value = string | number | boolean;
const GROUPS: ConfigField['group'][] = ['general', 'world', 'players', 'network', 'performance'];

/** Changed keys between the saved fields and the draft (pure). */
export function configDiff(
  fields: ConfigField[],
  draft: Record<string, Value>,
): Record<string, Value> {
  const out: Record<string, Value> = {};
  for (const f of fields) {
    const v = draft[f.key];
    if (v !== undefined && v !== f.value) out[f.key] = v;
  }
  return out;
}

export function ConfigTab({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const [mode, setMode] = useState<'form' | 'raw'>('form');
  return (
    <div className="flex flex-col gap-4">
      <SegmentedControl
        size="sm"
        label={t('manage.config.mode')}
        value={mode}
        onValueChange={(v) => setMode(v as 'form' | 'raw')}
        options={[
          { value: 'form', label: t('manage.config.form') },
          { value: 'raw', label: t('manage.config.raw') },
        ]}
      />
      {mode === 'form' ? <ConfigForm server={server} /> : <RawConfig server={server} />}
    </div>
  );
}

function ConfigForm({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const { data, isLoading, isError, refetch } = useConfig(server.id);
  const [draft, setDraft] = useState<Record<string, Value>>({});
  const changes = useMemo(() => (data ? configDiff(data.fields, draft) : {}), [data, draft]);
  const changedCount = Object.keys(changes).length;
  const needsRestart = data?.fields.some((f) => f.restart_required && f.key in changes) ?? false;

  const save = useMutation({
    mutationFn: () => invoke('server_config_set', { server_id: server.id, values: changes }),
    onSuccess: (cfg) => {
      qc.setQueryData(mk.config(server.id), cfg);
      void qc.invalidateQueries({ queryKey: mk.raw(server.id) });
      void qc.invalidateQueries({ queryKey: mk.server(server.id), exact: true });
      setDraft({});
      toast({
        tone: 'success',
        title: t('manage.config.saved'),
        description:
          needsRestart && server.state === 'running' ? t('manage.config.restartNeeded') : undefined,
      });
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('manage.config.saveFailed'), description: e.message }),
  });

  if (isLoading) return <Skeleton className="h-96 rounded-2xl" />;
  if (isError || !data) return <QueryError onRetry={() => void refetch()} />;

  const value = (f: ConfigField): Value => (draft[f.key] !== undefined ? draft[f.key]! : f.value);
  const set = (k: string, v: Value) => setDraft((d) => ({ ...d, [k]: v }));

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        if (changedCount > 0) save.mutate();
      }}
    >
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        {GROUPS.map((g) => {
          const fields = data.fields.filter((f) => f.group === g);
          if (fields.length === 0) return null;
          return (
            <Panel key={g} title={t(`manage.config.groups.${g}`)}>
              <div className="flex flex-col gap-3">
                {fields.map((f) => {
                  const id = `cfg-${f.key}`;
                  const v = value(f);
                  return (
                    <div
                      key={f.key}
                      className="grid grid-cols-[1fr_minmax(0,220px)] items-center gap-4"
                    >
                      <label htmlFor={id} className="flex min-w-0 flex-col gap-0.5">
                        <span className="text-fg flex items-center gap-1.5 text-sm font-medium">
                          {t(`manage.config.keys.${f.key}` as 'manage.config.keys.motd', {
                            defaultValue: f.key,
                          })}
                          {f.restart_required && (
                            <Badge variant="neutral" icon={<RotateCw />}>
                              {t('manage.config.restart')}
                            </Badge>
                          )}
                        </span>
                        <code className="text-2xs text-fg-subtle font-mono">{f.key}</code>
                      </label>
                      {f.type === 'boolean' ? (
                        <Switch
                          id={id}
                          checked={Boolean(v)}
                          onCheckedChange={(c) => set(f.key, c)}
                          className="justify-self-end"
                        />
                      ) : f.type === 'enum' ? (
                        <Select
                          id={id}
                          value={String(v)}
                          onValueChange={(nv) => set(f.key, nv)}
                          options={(f.options ?? []).map((o) => ({ value: o, label: o }))}
                        />
                      ) : (
                        <Input
                          id={id}
                          type={f.type === 'integer' ? 'number' : 'text'}
                          min={f.min}
                          max={f.max}
                          value={String(v)}
                          onChange={(e) =>
                            set(
                              f.key,
                              f.type === 'integer' ? Number(e.target.value) : e.target.value,
                            )
                          }
                        />
                      )}
                    </div>
                  );
                })}
              </div>
            </Panel>
          );
        })}
      </div>
      <div className="border-border bg-bg-elevated/90 sticky bottom-0 flex items-center justify-end gap-3 rounded-xl border px-4 py-3 backdrop-blur">
        <span className="text-fg-muted mr-auto text-sm" aria-live="polite">
          {changedCount > 0
            ? t('manage.config.unsaved', { count: changedCount }) +
              (needsRestart ? ` · ${t('manage.config.restartNeeded')}` : '')
            : t('manage.config.noChanges')}
        </span>
        <Button
          type="button"
          variant="ghost"
          disabled={changedCount === 0}
          onClick={() => setDraft({})}
        >
          {t('manage.config.discard')}
        </Button>
        <Button
          type="submit"
          variant="primary"
          disabled={changedCount === 0}
          loading={save.isPending}
        >
          {t('manage.save')}
        </Button>
      </div>
    </form>
  );
}

function RawConfig({ server }: { server: Server }) {
  const { t } = useTranslation('servers');
  const qc = useQueryClient();
  const { data, isLoading, isError, refetch } = useRawConfig(server.id, true);
  const [text, setText] = useState('');
  useEffect(() => {
    if (data) setText(data.content);
  }, [data]);
  const save = useMutation({
    mutationFn: () => invoke('server_config_raw_set', { server_id: server.id, content: text }),
    onSuccess: (raw) => {
      qc.setQueryData(mk.raw(server.id), raw);
      void qc.invalidateQueries({ queryKey: mk.config(server.id) });
      toast({ tone: 'success', title: t('manage.config.saved') });
    },
    onError: (e) =>
      toast({ tone: 'error', title: t('manage.config.saveFailed'), description: e.message }),
  });
  if (isLoading) return <Skeleton className="h-96 rounded-2xl" />;
  if (isError || !data) return <QueryError onRetry={() => void refetch()} />;
  return (
    <Panel
      title={t('manage.config.rawTitle')}
      description={<code className="font-mono">{data.path}</code>}
      action={
        <Button
          variant="primary"
          size="sm"
          disabled={text === data.content}
          loading={save.isPending}
          onClick={() => save.mutate()}
        >
          {t('manage.save')}
        </Button>
      }
    >
      <p className="text-warning-text mb-3 text-xs">{t('manage.config.rawWarning')}</p>
      <Textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        spellCheck={false}
        aria-label={t('manage.config.rawTitle')}
        className="min-h-[420px] font-mono text-xs leading-relaxed"
      />
    </Panel>
  );
}
