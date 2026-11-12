import { Button, cn, formatBytes, Progress, Spinner } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { motion } from 'motion/react';
import {
  ArrowRight,
  CheckCircle2,
  ChevronDown,
  Circle,
  MinusCircle,
  PartyPopper,
  Play,
  XCircle,
} from 'lucide-react';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke, listen } from '@/ipc';
import type { InstallStep, InstallStepId } from '@/ipc/types/hosting';
import type { Server, ServerState } from '@/ipc/types/servers';
import { formatPercent } from '@/lib/format';
import { useActivity } from '@/stores/activity';

type StepState = InstallStep['state'];

export function StepIcon({ state }: { state: StepState }) {
  switch (state) {
    case 'done':
      return <CheckCircle2 className="text-success-text size-5" aria-hidden />;
    case 'running':
      return <Spinner className="text-accent-text size-5" />;
    case 'failed':
      return <XCircle className="text-danger-text size-5" aria-hidden />;
    case 'skipped':
      return <MinusCircle className="text-fg-subtle size-5" aria-hidden />;
    default:
      return <Circle className="text-fg-subtle size-5" aria-hidden />;
  }
}

function StepRow({
  label,
  state,
  detail,
  children,
}: {
  label: string;
  state: StepState;
  detail?: ReactNode;
  children?: ReactNode;
}) {
  const { t } = useTranslation('servers');
  return (
    <li className="flex items-start gap-3" data-state={state}>
      <span className="mt-0.5">
        <StepIcon state={state} />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span
          className={cn('text-sm', state === 'pending' ? 'text-fg-subtle' : 'text-fg font-medium')}
        >
          {label}
          <span className="sr-only"> · {t(`hosting.install.state.${state}`)}</span>
        </span>
        {detail && <span className="text-fg-muted text-xs">{detail}</span>}
        {children}
      </div>
    </li>
  );
}

function Done({
  server,
  title,
  body,
  tab,
}: {
  server: Server;
  title: string;
  body: string;
  tab?: string;
}) {
  const { t } = useTranslation('servers');
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      className="border-success/40 bg-success/10 flex flex-col items-start gap-3 rounded-xl border p-5"
      role="status"
    >
      <p className="flex items-center gap-2 text-base font-semibold">
        <PartyPopper className="text-success-text size-5" aria-hidden />
        {title}
      </p>
      <p className="text-fg-muted text-sm">{body}</p>
      <div className="flex gap-2">
        <Button asChild variant="primary">
          <Link
            to="/servers/$serverId"
            params={{ serverId: server.id }}
            search={{ tab }}
            data-testid="open-server"
          >
            {t('hosting.install.open')}
            <ArrowRight />
          </Link>
        </Button>
        <Button asChild variant="ghost">
          <Link to="/servers">{t('hosting.install.backToList')}</Link>
        </Button>
      </div>
    </motion.div>
  );
}

/** Local server: download bundle → initialise → first start (UX_SPEC 4.4). */
export function LocalProgress({
  server,
  taskId,
  autoStart = true,
  tab,
}: {
  server: Server;
  taskId: string;
  autoStart?: boolean;
  tab?: string;
}) {
  const { t, i18n } = useTranslation('servers');
  const task = useActivity((s) => s.downloads[taskId]);
  const [state, setState] = useState<ServerState>(server.state);
  const [phase, setPhase] = useState<'download' | 'init' | 'start' | 'done' | 'error'>('download');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const off = listen('server-state', (e) => {
      if (e.server_id === server.id) setState(e.state);
    });
    // In case the install finished before we subscribed.
    void invoke('server_get', { id: server.id })
      .then((s) => setState((cur) => (cur === 'installing' ? s.state : cur)))
      .catch(() => undefined);
    return off;
  }, [server.id]);

  useEffect(() => {
    if (phase === 'download' && state === 'stopped') setPhase('init');
    if (phase === 'start' && state === 'running') setPhase('done');
    if (state === 'crashed') {
      setError(t('hosting.install.crashed'));
      setPhase('error');
    }
  }, [state, phase, t]);

  // Initialising takes a moment, then the first start (own effect so state changes don't cancel it).
  useEffect(() => {
    if (phase === 'init') {
      const timer = setTimeout(() => {
        if (!autoStart) {
          setPhase('done');
          return;
        }
        setPhase('start');
        invoke('server_start', { id: server.id }).catch((e: Error) => {
          setError(e.message);
          setPhase('error');
        });
      }, 700);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [phase, server.id, autoStart]);

  const pct = task?.total_bytes ? task.received_bytes / task.total_bytes : null;
  const order = ['download', 'init', 'start', 'done'] as const;
  const idx = phase === 'error' ? -1 : order.indexOf(phase);
  const st = (i: number): StepState =>
    phase === 'error' && i === 0 ? 'failed' : idx > i ? 'done' : idx === i ? 'running' : 'pending';

  return (
    <div className="flex flex-col gap-5" data-testid="local-progress">
      <ol className="flex flex-col gap-4">
        <StepRow
          label={t('hosting.install.localDownload', {
            engine: server.engine === 'rustic' ? t('list.engine.rustic') : t('list.engine.horizon'),
          })}
          state={st(0)}
        >
          {phase === 'download' && (
            <div className="flex flex-col gap-1">
              <Progress
                value={pct}
                label={t('hosting.install.localDownload', { engine: '' })}
                size="sm"
              />
              {task?.total_bytes ? (
                <span className="text-2xs text-fg-subtle tabular-nums">
                  {formatBytes(task.received_bytes, i18n.language)} /{' '}
                  {formatBytes(task.total_bytes, i18n.language)} ·{' '}
                  {formatPercent(pct ?? 0, i18n.language)}
                </span>
              ) : null}
            </div>
          )}
        </StepRow>
        <StepRow label={t('hosting.install.localInit')} state={st(1)} />
        <StepRow
          label={autoStart ? t('hosting.install.localStart') : t('hosting.install.localReady')}
          state={st(2)}
        />
      </ol>
      {phase === 'error' && (
        <div
          role="alert"
          className="border-danger/40 bg-danger/10 flex items-center justify-between gap-3 rounded-lg border p-3 text-sm"
        >
          <span>{error ?? t('hosting.install.failed')}</span>
          <Button
            size="sm"
            leftIcon={<Play />}
            onClick={() => {
              setPhase('start');
              void invoke('server_start', { id: server.id });
            }}
          >
            {t('hosting.install.retry')}
          </Button>
        </div>
      )}
      {phase === 'done' && (
        <Done
          server={server}
          tab={tab}
          title={t('hosting.install.localDone', { name: server.name })}
          body={t('hosting.install.localDoneBody')}
        />
      )}
    </div>
  );
}

const STEP_IDS: InstallStepId[] = [
  'connect',
  'check',
  'user',
  'download',
  'engine',
  'service',
  'firewall',
  'world',
  'start',
];

/** VPS install: live step list + collapsible log (wireframe 14). */
export function VpsInstallProgress({
  server,
  taskId,
  host,
  tab,
}: {
  server: Server;
  taskId: string;
  host: string;
  tab?: string;
}) {
  const { t } = useTranslation('servers');
  const [steps, setSteps] = useState<InstallStep[]>(
    STEP_IDS.map((id) => ({ id, state: 'pending' })),
  );
  const [logs, setLogs] = useState<string[]>([]);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showLog, setShowLog] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);

  useEffect(
    () =>
      listen('hosting-install', (p) => {
        if (p.task_id !== taskId) return;
        setSteps(p.steps);
        if (p.log) setLogs((l) => [...l, p.log!]);
        if (p.error) setError(p.error);
        if (p.done) setDone(true);
      }),
    [taskId],
  );
  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [logs]);

  const finished = steps.filter((s) => s.state === 'done' || s.state === 'skipped').length;

  return (
    <div className="flex flex-col gap-5" data-testid="vps-progress">
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center justify-between text-sm">
          <span className="font-medium">{t('hosting.install.vpsTitle', { host })}</span>
          <span className="text-fg-subtle text-xs tabular-nums">
            {finished}/{steps.length}
          </span>
        </div>
        <Progress
          value={finished / steps.length}
          label={t('hosting.install.vpsTitle', { host })}
          size="sm"
          tone={error ? 'danger' : done ? 'success' : 'accent'}
        />
      </div>
      <ol className="grid gap-3 sm:grid-cols-2" aria-live="polite">
        {steps.map((s) => (
          <StepRow
            key={s.id}
            label={t(`hosting.install.steps.${s.id}`)}
            state={s.state}
            detail={s.detail === 'vanilla Horizon' ? t('hosting.install.skippedVanilla') : s.detail}
          />
        ))}
      </ol>
      <div className="border-border bg-bg/70 rounded-xl border">
        <button
          type="button"
          onClick={() => setShowLog((v) => !v)}
          aria-expanded={showLog}
          aria-controls="vps-log"
          className="rx-focus-ring flex w-full items-center justify-between rounded-xl px-4 py-2.5 text-sm font-medium"
        >
          {t('hosting.install.log', { count: logs.length })}
          <ChevronDown
            className={cn('size-4 transition-transform', showLog && 'rotate-180')}
            aria-hidden
          />
        </button>
        {showLog && (
          <pre
            id="vps-log"
            ref={logRef}
            role="log"
            tabIndex={0}
            className="border-border text-2xs text-fg-muted max-h-56 overflow-auto border-t px-4 py-3 font-mono leading-5"
          >
            {logs.join('\n')}
          </pre>
        )}
      </div>
      {error && (
        <div role="alert" className="border-danger/40 bg-danger/10 rounded-lg border p-3 text-sm">
          {t('hosting.install.failed')} · {error}
        </div>
      )}
      {done && !error && (
        <Done
          server={server}
          tab={tab}
          title={t('hosting.install.vpsDone', { name: server.name, host })}
          body={t('hosting.install.vpsDoneBody')}
        />
      )}
    </div>
  );
}
