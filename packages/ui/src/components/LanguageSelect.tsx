/**
 * The language menu of the launcher and the website (ADR 0161): a button with the current flag
 * that opens a searchable list of the game's 143 locales, translated ones first. Flags are the
 * game's own (drawn from the same shapes, `@rustic/locales` flags.json, loaded on demand); a
 * locale without a flag gets a neutral tile with its code, as in game.
 */
import {
  LOCALES,
  loadedFlags,
  loadFlags,
  locale as localeOf,
  matches,
  sortedLocales,
  tileText,
  type FlagData,
  type FlagShape,
  type LocaleInfo,
  type TranslationStatus,
} from '@rustic/locales';
import { Command } from 'cmdk';
import { Check, ChevronDown, Search } from 'lucide-react';
import { Popover as P } from 'radix-ui';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { cn } from '../lib/cn';

function useFlagData(): FlagData | null {
  const [data, setData] = useState<FlagData | null>(loadedFlags);
  useEffect(() => {
    if (data) return;
    let live = true;
    void loadFlags().then((d) => live && setData(d));
    return () => {
      live = false;
    };
  }, [data]);
  return data;
}

function shapes(list: FlagShape[], key = ''): ReactNode[] {
  return list.map((s, i) => {
    const k = `${key}${i}`;
    switch (s[0]) {
      case 'r':
        return <rect key={k} x={s[1]} y={s[2]} width={s[3]} height={s[4]} fill={s[5]} />;
      case 'c':
        return <circle key={k} cx={s[1]} cy={s[2]} r={s[3]} fill={s[4]} />;
      case 'p':
        return <polygon key={k} points={s[1]} fill={s[2]} />;
      case 'l':
        return (
          <line key={k} x1={s[1]} y1={s[2]} x2={s[3]} y2={s[4]} strokeWidth={s[5]} stroke={s[6]} />
        );
      case 's':
        return (
          <svg key={k} x={s[1]} y={s[2]} width={s[3]} height={s[4]} viewBox={`0 0 ${s[3]} ${s[4]}`}>
            {shapes(s[5], `${k}-`)}
          </svg>
        );
      default:
        return null;
    }
  });
}

export interface FlagProps {
  /** A game locale code (`fr_fr`, `pt_br`, `tok`). */
  code: string;
  className?: string;
}

/** The flag of a game locale, 3:2, rounded; decorative (its name is written next to it). */
export function Flag({ code, className }: FlagProps) {
  const data = useFlagData();
  const design = data?.flags[code.toLowerCase()];
  return (
    <span
      aria-hidden
      data-flag={code}
      className={cn(
        'relative inline-grid aspect-[3/2] h-4 shrink-0 place-items-center overflow-hidden rounded-[3px] bg-[#3a3b42] shadow-[0_0_0_1px_rgb(255_255_255/0.16)]',
        className,
      )}
    >
      {design ? (
        <svg
          viewBox="0 0 30 20"
          preserveAspectRatio="none"
          className="absolute inset-0 block size-full overflow-hidden"
        >
          <g transform={design.f ? 'matrix(1 0 0 -1 0 20)' : undefined}>{shapes(design.s)}</g>
        </svg>
      ) : (
        data && (
          <span className="text-[0.5em] leading-none font-bold tracking-tight text-white/85">
            {tileText(code)}
          </span>
        )
      )}
    </span>
  );
}

export interface LanguageSelectLabels {
  /** The button's accessible name ("Language: Deutsch" is built from it). */
  label: string;
  search: string;
  empty: string;
  /** Heading of the translated locales. */
  translated: string;
  /** Heading of the others (shown in English for now). */
  others: string;
  /** Badge of a partly translated locale. */
  partial: string;
  /** Hint of a locale shown in English. */
  english: string;
  /** Hint of a locale shown in a close language (`in Deutsch for now`). */
  via?: (code: string) => string;
  /** Footer under the list (e.g. a link to help translate). */
  footer?: ReactNode;
}

export interface LanguageSelectProps {
  /** The current game locale code. */
  value: string;
  onChange: (code: string) => void;
  status: (code: string) => TranslationStatus;
  labels: LanguageSelectLabels;
  /** `field`: flag, name and caret (settings); `compact`: the flag alone (a navigation bar). */
  variant?: 'field' | 'compact';
  align?: 'start' | 'center' | 'end';
  side?: 'top' | 'bottom';
  className?: string;
  contentClassName?: string;
  /** `data-testid` of the button; the list is `${testId}-list`, each item `${testId}-<code>`. */
  testId?: string;
  /** Locales offered (all of the game's by default). */
  locales?: readonly LocaleInfo[];
}

/** A searchable menu of the game's locales with their flags. */
export function LanguageSelect({
  value,
  onChange,
  status,
  labels,
  variant = 'field',
  align = 'start',
  side = 'bottom',
  className,
  contentClassName,
  testId = 'language-select',
  locales = LOCALES,
}: LanguageSelectProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(value);
  const listRef = useRef<HTMLDivElement>(null);
  const current = localeOf(value);

  const groups = useMemo(() => {
    const all = sortedLocales(locales).filter((l) => !query || matches(l, query));
    const done = all.filter((l) => status(l.code) === 'complete');
    // Partly translated first, then the ones shown in English.
    const rank = { complete: 0, partial: 0, fallback: 1, none: 2 } as const;
    const rest = all
      .filter((l) => status(l.code) !== 'complete')
      .sort((a, b) => rank[status(a.code)] - rank[status(b.code)]);
    return { done, rest };
  }, [locales, query, status]);

  useEffect(() => {
    if (!open) return;
    setActive(value);
    // The current locale in view once the list is drawn.
    const id = requestAnimationFrame(() => {
      listRef.current
        ?.querySelector<HTMLElement>(`[data-code="${value}"]`)
        ?.scrollIntoView?.({ block: 'center' });
    });
    return () => cancelAnimationFrame(id);
  }, [open, value]);

  const item = (l: LocaleInfo) => {
    const st = status(l.code);
    return (
      <Command.Item
        key={l.code}
        value={l.code}
        data-code={l.code}
        data-testid={`${testId}-${l.code}`}
        onSelect={() => {
          onChange(l.code);
          setOpen(false);
          setQuery('');
        }}
        className="text-fg data-[selected=true]:bg-surface-3 flex cursor-pointer items-center gap-3 rounded-lg px-2.5 py-2 text-sm outline-none"
      >
        <Flag code={l.code} className="h-[18px]" />
        <span className="flex min-w-0 flex-1 flex-col leading-tight">
          <span className="truncate font-medium" lang={l.tag} dir={l.dir}>
            {l.name}
            {l.region && (
              <span className="text-fg-muted font-normal" dir={l.dir}>
                {' '}
                · {l.region}
              </span>
            )}
          </span>
          <span className="text-fg-subtle truncate text-xs">
            {l.english}
            {st === 'none' && ` · ${labels.english}`}
            {st === 'fallback' && labels.via && ` · ${labels.via(l.code)}`}
          </span>
        </span>
        {st === 'partial' && (
          <span className="border-border text-fg-muted rounded-full border px-1.5 py-0.5 text-[10px] font-medium">
            {labels.partial}
          </span>
        )}
        {l.code === value && <Check className="text-accent size-4 shrink-0" aria-hidden />}
      </Command.Item>
    );
  };

  const heading =
    '[&_[cmdk-group-heading]]:text-2xs [&_[cmdk-group-heading]]:text-fg-subtle [&_[cmdk-group-heading]]:px-2.5 [&_[cmdk-group-heading]]:pt-2.5 [&_[cmdk-group-heading]]:pb-1 [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:uppercase';

  const name = current ? `${current.name}${current.region ? ` (${current.region})` : ''}` : value;
  return (
    <P.Root
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (!o) setQuery('');
      }}
    >
      <P.Trigger
        data-testid={testId}
        aria-label={`${labels.label}: ${name}`}
        title={variant === 'compact' ? name : undefined}
        className={cn(
          'rx-focus-ring inline-flex items-center gap-2.5 transition-colors',
          variant === 'field'
            ? 'border-border-strong/50 bg-surface-2 text-fg hover:border-border-strong h-9 w-full min-w-56 justify-between rounded-md border px-3 text-sm'
            : 'size-9 justify-center rounded-full hover:bg-white/10',
          className,
        )}
      >
        <Flag code={value} className={variant === 'field' ? 'h-4' : 'h-[15px]'} />
        {variant === 'field' && (
          <>
            <span
              className="min-w-0 flex-1 truncate text-left"
              lang={current?.tag}
              dir={current?.dir}
            >
              {name}
            </span>
            <ChevronDown className="text-fg-subtle size-4 shrink-0" aria-hidden />
          </>
        )}
      </P.Trigger>
      <P.Portal>
        <P.Content
          align={align}
          side={side}
          sideOffset={6}
          collisionPadding={12}
          className={cn(
            'rx-glass z-[80] flex max-h-[min(460px,var(--radix-popover-content-available-height))] w-[min(360px,calc(100vw-24px))] flex-col overflow-hidden rounded-xl shadow-lg outline-none',
            contentClassName,
          )}
        >
          <Command
            label={labels.label}
            shouldFilter={false}
            loop
            value={active}
            onValueChange={setActive}
            className="flex min-h-0 flex-1 flex-col"
          >
            <div className="border-border flex items-center gap-2.5 border-b px-3">
              <Search className="text-fg-subtle size-4 shrink-0" aria-hidden />
              <Command.Input
                value={query}
                onValueChange={setQuery}
                placeholder={labels.search}
                data-testid={`${testId}-search`}
                className="text-fg placeholder:text-fg-subtle h-11 min-w-0 flex-1 bg-transparent text-sm outline-none"
              />
            </div>
            <Command.List
              ref={listRef}
              data-testid={`${testId}-list`}
              className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-1.5"
            >
              <Command.Empty className="text-fg-muted px-3 py-8 text-center text-sm">
                {labels.empty}
              </Command.Empty>
              {groups.done.length > 0 && (
                <Command.Group heading={labels.translated} className={heading}>
                  {groups.done.map(item)}
                </Command.Group>
              )}
              {groups.rest.length > 0 && (
                <Command.Group heading={labels.others} className={heading}>
                  {groups.rest.map(item)}
                </Command.Group>
              )}
            </Command.List>
            {labels.footer && (
              <div className="border-border text-fg-subtle border-t px-3 py-2 text-xs">
                {labels.footer}
              </div>
            )}
          </Command>
        </P.Content>
      </P.Portal>
    </P.Root>
  );
}
