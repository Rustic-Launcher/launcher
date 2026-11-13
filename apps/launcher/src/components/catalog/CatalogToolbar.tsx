import {
  Button,
  cn,
  Dropdown,
  DropdownContent,
  DropdownItem,
  DropdownLabel,
  DropdownTrigger,
  IconButton,
  Popover,
  PopoverContent,
  PopoverTrigger,
  SegmentedControl,
  Select,
  Switch,
} from '@rustic/ui';
import {
  ArrowDownWideNarrow,
  Check,
  ChevronDown,
  LayoutGrid,
  List,
  Search,
  SlidersHorizontal,
  X,
} from 'lucide-react';
import { useId, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

/** A row of toolbar controls: `start` grows, `end` stays right (wraps on narrow windows). */
export function CatalogToolbar({
  start,
  end,
  className,
}: {
  start?: ReactNode;
  end?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn('flex flex-wrap items-center gap-2', className)}>
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">{start}</div>
      {end && <div className="flex shrink-0 flex-wrap items-center gap-2">{end}</div>}
    </div>
  );
}

export interface ScopeOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
  /** Starts a new group with this heading. */
  group?: string;
}

/**
 * The search field of a catalogue page, with an optional scope menu fused on its left (the kind
 * of content, the kind of server...). Escape clears the text.
 */
export function CatalogSearch({
  value,
  onChange,
  placeholder,
  label,
  clearLabel,
  scope,
  testId,
  className,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  /** Accessible name of the input. */
  label: string;
  clearLabel: string;
  scope?: ReactNode;
  testId?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        'bg-surface-2 border-border focus-within:border-accent/60 flex h-11 min-w-0 flex-1 items-center rounded-xl border transition-colors',
        className,
      )}
    >
      {scope && (
        <>
          {scope}
          <span className="bg-border-default h-5 w-px shrink-0" aria-hidden />
        </>
      )}
      <Search className="text-fg-subtle ml-3 size-4 shrink-0" aria-hidden />
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape' && value) {
            e.stopPropagation();
            onChange('');
          }
        }}
        placeholder={placeholder}
        aria-label={label}
        data-testid={testId}
        type="search"
        className="placeholder:text-fg-subtle h-full min-w-0 flex-1 bg-transparent px-2.5 text-sm outline-none [&::-webkit-search-cancel-button]:hidden"
      />
      {value && (
        <IconButton
          size="sm"
          label={clearLabel}
          icon={<X />}
          onClick={() => onChange('')}
          className="mr-1.5"
        />
      )}
    </div>
  );
}

/** The scope menu fused into {@link CatalogSearch}. */
export function ScopeMenu<T extends string>({
  value,
  options,
  onChange,
  label,
  testId,
  itemTestIdPrefix,
}: {
  value: T;
  options: ScopeOption<T>[];
  onChange: (v: T) => void;
  label: string;
  testId?: string;
  itemTestIdPrefix?: string;
}) {
  const current = options.find((o) => o.value === value);
  return (
    <Dropdown>
      <DropdownTrigger asChild>
        <button
          type="button"
          data-testid={testId}
          aria-label={`${label}: ${current?.label ?? ''}`}
          className="rx-focus-ring hover:bg-surface-3/60 flex h-full shrink-0 items-center gap-2 rounded-l-xl px-3.5 text-sm font-semibold transition-colors [&_svg]:size-4"
        >
          {current?.icon}
          <span className="max-[1080px]:sr-only">{current?.label}</span>
          <ChevronDown className="text-fg-subtle size-3.5!" aria-hidden />
        </button>
      </DropdownTrigger>
      <DropdownContent align="start" className="w-60">
        <DropdownLabel>{label}</DropdownLabel>
        {options.map((o) => (
          <div key={o.value}>
            {o.group && (
              <DropdownLabel className="border-border mt-1 border-t pt-2">{o.group}</DropdownLabel>
            )}
            <DropdownItem
              icon={o.icon}
              onSelect={() => onChange(o.value)}
              data-testid={itemTestIdPrefix ? `${itemTestIdPrefix}-${o.value}` : undefined}
            >
              <span className="flex items-center justify-between">
                {o.label}
                {o.value === value && <Check className="text-accent-text size-4" />}
              </span>
            </DropdownItem>
          </div>
        ))}
      </DropdownContent>
    </Dropdown>
  );
}

/** A sort select in the toolbar style. */
export function SortSelect<T extends string>({
  value,
  options,
  onChange,
  className,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <Select
      aria-label={t('catalog.sort')}
      value={value}
      onValueChange={(v) => onChange(v as T)}
      options={options.map((o) => ({ ...o, icon: <ArrowDownWideNarrow /> }))}
      className={cn('bg-surface-2 h-9 w-48 rounded-lg', className)}
    />
  );
}

/** "Filters · n" opening a panel of filter sections. */
export function FilterPopover({
  active,
  onReset,
  children,
  testId,
}: {
  /** How many filters differ from the defaults. */
  active: number;
  onReset?: () => void;
  children: ReactNode;
  testId?: string;
}) {
  const { t } = useTranslation();
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          size="md"
          variant={active ? 'soft' : 'secondary'}
          leftIcon={<SlidersHorizontal />}
          className="rounded-lg"
          data-testid={testId}
        >
          {active
            ? t('catalog.filters.buttonCount', { count: active })
            : t('catalog.filters.button')}
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className="flex max-h-[min(70vh,560px)] w-80 flex-col gap-4 overflow-y-auto p-4"
      >
        {children}
        {onReset && active > 0 && (
          <Button size="sm" variant="ghost" onClick={onReset} className="self-start">
            {t('catalog.filters.reset')}
          </Button>
        )}
      </PopoverContent>
    </Popover>
  );
}

export function FilterSection({
  title,
  children,
  hint,
}: {
  title: string;
  children: ReactNode;
  hint?: ReactNode;
}) {
  const id = useId();
  return (
    <section aria-labelledby={id} className="flex flex-col gap-2">
      <h3 id={id} className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
        {title}
      </h3>
      {children}
      {hint && <p className="text-fg-subtle text-xs">{hint}</p>}
    </section>
  );
}

/** A toggle chip (`aria-pressed`). */
export function FilterChip({
  on,
  onToggle,
  children,
  icon,
  testId,
}: {
  on: boolean;
  onToggle: () => void;
  children: ReactNode;
  icon?: ReactNode;
  testId?: string;
}) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onToggle}
      data-testid={testId}
      className={cn(
        'rx-focus-ring inline-flex h-8 items-center gap-1.5 truncate rounded-lg border px-2.5 text-xs font-medium transition-colors [&_svg]:size-3.5',
        on
          ? 'border-accent/50 bg-accent-soft text-accent-text'
          : 'bg-surface-2 border-border text-fg-muted hover:text-fg',
      )}
    >
      {icon}
      {children}
    </button>
  );
}

/** A labelled switch row inside a filter panel. */
export function FilterSwitch({
  checked,
  onCheckedChange,
  label,
  icon,
  testId,
}: {
  checked: boolean;
  onCheckedChange: (v: boolean) => void;
  label: ReactNode;
  icon?: ReactNode;
  testId?: string;
}) {
  const id = useId();
  return (
    <label
      htmlFor={id}
      className="bg-surface-2 border-border flex cursor-pointer items-center justify-between gap-3 rounded-lg border px-3 py-2.5 text-sm"
    >
      <span className="flex items-center gap-2 [&_svg]:size-4">
        {icon}
        {label}
      </span>
      <Switch id={id} checked={checked} onCheckedChange={onCheckedChange} data-testid={testId} />
    </label>
  );
}

export type CatalogLayout = 'list' | 'grid';

export function LayoutToggle({
  value,
  onChange,
}: {
  value: CatalogLayout;
  onChange: (v: CatalogLayout) => void;
}) {
  const { t } = useTranslation();
  return (
    <SegmentedControl
      label={t('catalog.layout.label')}
      value={value}
      onValueChange={(v) => onChange(v as CatalogLayout)}
      size="sm"
      className="bg-surface-2 isolate rounded-lg"
      options={[
        {
          value: 'list',
          label: <span className="sr-only">{t('catalog.layout.list')}</span>,
          icon: <List />,
        },
        {
          value: 'grid',
          label: <span className="sr-only">{t('catalog.layout.grid')}</span>,
          icon: <LayoutGrid />,
        },
      ]}
    />
  );
}

/** "n results", announced politely. */
export function ResultCount({ count, className }: { count: number; className?: string }) {
  const { t } = useTranslation();
  return (
    <p className={cn('text-fg-muted text-sm tabular-nums', className)} aria-live="polite">
      {t('catalog.results', { count })}
    </p>
  );
}
