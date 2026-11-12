import { Button, cn, SegmentedControl, Skeleton, Switch, toast } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { ChevronRight, CornerDownLeft } from 'lucide-react';
import {
  useDeferredValue,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { ConsoleLine, Server } from '@/ipc/types/servers';
import { useConsole } from './queries';

type Filter = 'all' | 'info' | 'warn' | 'error' | 'chat';

const LEVEL_CLASS: Record<ConsoleLine['level'], string> = {
  info: 'text-fg-muted',
  warn: 'text-warning-text',
  error: 'text-danger-text',
  command: 'text-accent-text',
  chat: 'text-info-text',
};

/** Console input history navigation (pure, unit-tested). */
export function historyStep(history: string[], index: number | null, dir: -1 | 1): number | null {
  if (history.length === 0) return null;
  if (index === null) return dir === -1 ? history.length - 1 : null;
  const next = index + dir;
  if (next < 0) return 0;
  if (next >= history.length) return null;
  return next;
}

export function ConsoleTab({ server }: { server: Server }) {
  const { t, i18n } = useTranslation('servers');
  const { data: lines, isLoading } = useConsole(server.id);
  const [filter, setFilter] = useState<Filter>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const [input, setInput] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [histIndex, setHistIndex] = useState<number | null>(null);
  const [selected, setSelected] = useState(0);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [sending, setSending] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const deferred = useDeferredValue(input);
  const running = server.state === 'running';

  const { data: suggestions = [] } = useQuery({
    queryKey: ['servers', server.id, 'complete', deferred],
    queryFn: () => invoke('server_complete', { server_id: server.id, partial: deferred }),
    enabled: deferred.trim().length > 0,
    staleTime: 30_000,
  });
  const open = showSuggestions && suggestions.length > 0 && input.trim().length > 0;

  const visible = useMemo(
    () =>
      (lines ?? []).filter((l) =>
        filter === 'all'
          ? true
          : filter === 'info'
            ? l.level === 'info' || l.level === 'command'
            : l.level === filter,
      ),
    [lines, filter],
  );

  useLayoutEffect(() => {
    const el = logRef.current;
    if (el && autoScroll) el.scrollTop = el.scrollHeight;
  }, [visible, autoScroll]);

  useEffect(() => setSelected(0), [suggestions]);

  const time = (iso: string) =>
    new Intl.DateTimeFormat(i18n.language, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }).format(new Date(iso));

  const send = async () => {
    const line = input.trim();
    if (!line) return;
    setSending(true);
    try {
      await invoke('server_command', { server_id: server.id, line });
      setHistory((h) => [...h.filter((x) => x !== line), line].slice(-50));
      setHistIndex(null);
      setInput('');
      setShowSuggestions(false);
    } catch (e) {
      toast({
        tone: 'error',
        title: t('manage.console.sendFailed'),
        description: (e as Error).message,
      });
    } finally {
      setSending(false);
    }
  };

  const accept = (s: string) => {
    setInput(`${s} `);
    setShowSuggestions(false);
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <SegmentedControl
          size="sm"
          label={t('manage.console.filter')}
          value={filter}
          onValueChange={(v) => setFilter(v as Filter)}
          options={(['all', 'info', 'warn', 'error', 'chat'] as Filter[]).map((f) => ({
            value: f,
            label: t(`manage.console.filters.${f}`),
          }))}
        />
        <label className="text-fg-muted flex items-center gap-2 text-xs">
          <Switch
            checked={autoScroll}
            onCheckedChange={setAutoScroll}
            aria-label={t('manage.console.autoScroll')}
          />
          {t('manage.console.autoScroll')}
        </label>
      </div>
      <div
        ref={logRef}
        role="log"
        aria-label={t('manage.console.logLabel')}
        aria-live="off"
        tabIndex={0}
        data-testid="console-log"
        onWheel={(e) => {
          if (e.deltaY < 0 && autoScroll) setAutoScroll(false);
        }}
        className="rx-focus-ring border-border bg-bg/80 h-[min(56vh,520px)] overflow-y-auto rounded-2xl border p-4 font-mono text-xs leading-relaxed"
      >
        {isLoading &&
          Array.from({ length: 10 }, (_, i) => (
            <Skeleton
              key={i}
              className="mb-2 h-3.5"
              style={{ width: `${40 + ((i * 17) % 55)}%` }}
            />
          ))}
        {!isLoading && visible.length === 0 && (
          <p className="text-fg-subtle">{t('manage.console.empty')}</p>
        )}
        {visible.map((l, i) => (
          <div key={`${l.ts}-${i}`} className="flex gap-3 break-words whitespace-pre-wrap">
            <span className="text-fg-subtle shrink-0 tabular-nums">{time(l.ts)}</span>
            <span className={cn('w-12 shrink-0 font-semibold uppercase', LEVEL_CLASS[l.level])}>
              {t(`manage.console.levels.${l.level}`)}
            </span>
            <span className={cn('min-w-0', l.level === 'info' ? 'text-fg' : LEVEL_CLASS[l.level])}>
              {l.text}
            </span>
          </div>
        ))}
      </div>
      <form
        className="relative"
        onSubmit={(e) => {
          e.preventDefault();
          void send();
        }}
      >
        {open && (
          <ul
            id={listId}
            role="listbox"
            aria-label={t('manage.console.suggestions')}
            className="rx-glass-strong absolute bottom-full left-0 z-20 mb-2 max-h-56 w-full max-w-lg overflow-y-auto rounded-xl p-1 shadow-lg"
          >
            {suggestions.map((s, i) => (
              <li
                key={s}
                id={`${listId}-${i}`}
                role="option"
                aria-selected={i === selected}
                onMouseDown={(e) => {
                  e.preventDefault();
                  accept(s);
                }}
                className={cn(
                  'text-fg cursor-pointer rounded-md px-3 py-1.5 font-mono text-xs',
                  i === selected && 'bg-surface-3',
                )}
              >
                {s}
              </li>
            ))}
          </ul>
        )}
        <div className="border-border-strong/50 bg-surface-2 focus-within:border-accent focus-within:ring-accent/35 flex items-center gap-2 rounded-xl border py-1 pr-1 pl-3 focus-within:ring-2">
          <ChevronRight className="text-accent-text size-4 shrink-0" aria-hidden />
          <input
            value={input}
            disabled={!running}
            onChange={(e) => {
              setInput(e.target.value);
              setShowSuggestions(true);
              setHistIndex(null);
            }}
            onBlur={() => setShowSuggestions(false)}
            onKeyDown={(e) => {
              if (e.key === 'Tab' && open) {
                e.preventDefault();
                accept(suggestions[selected] ?? suggestions[0]!);
              } else if (e.key === 'ArrowDown' && open) {
                e.preventDefault();
                setSelected((s) => (s + 1) % suggestions.length);
              } else if (e.key === 'ArrowUp' && open) {
                e.preventDefault();
                setSelected((s) => (s - 1 + suggestions.length) % suggestions.length);
              } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
                e.preventDefault();
                const next = historyStep(history, histIndex, e.key === 'ArrowUp' ? -1 : 1);
                setHistIndex(next);
                setInput(next === null ? '' : (history[next] ?? ''));
                setShowSuggestions(false);
              } else if (e.key === 'Escape') {
                setShowSuggestions(false);
              }
            }}
            role="combobox"
            aria-expanded={open}
            aria-controls={listId}
            aria-autocomplete="list"
            aria-activedescendant={open ? `${listId}-${selected}` : undefined}
            aria-label={t('manage.console.inputLabel')}
            placeholder={running ? t('manage.console.placeholder') : t('manage.console.offline')}
            spellCheck={false}
            autoComplete="off"
            data-testid="console-input"
            className="text-fg placeholder:text-fg-subtle h-9 min-w-0 flex-1 bg-transparent font-mono text-sm outline-none placeholder:font-sans disabled:cursor-not-allowed"
          />
          <Button
            type="submit"
            size="sm"
            variant="primary"
            disabled={!running || !input.trim()}
            loading={sending}
            rightIcon={<CornerDownLeft />}
          >
            {t('manage.console.send')}
          </Button>
        </div>
        <p className="text-2xs text-fg-subtle mt-1.5">{t('manage.console.hint')}</p>
      </form>
    </div>
  );
}
