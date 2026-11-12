import { ScrollArea as SA, ToggleGroup } from 'radix-ui';
import { Check } from 'lucide-react';
import { motion } from 'motion/react';
import { useId, type ComponentProps, type ReactNode } from 'react';
import { cn } from '../lib/cn';

export function ScrollArea({ className, children, ...props }: ComponentProps<typeof SA.Root>) {
  return (
    <SA.Root className={cn('relative overflow-hidden', className)} type="hover" {...props}>
      <SA.Viewport className="size-full [&>div]:!block">{children}</SA.Viewport>
      <SA.Scrollbar orientation="vertical" className="flex w-2 touch-none p-0.5 select-none">
        <SA.Thumb className="bg-surface-3 hover:bg-border-strong relative flex-1 rounded-full" />
      </SA.Scrollbar>
    </SA.Root>
  );
}

export interface SegmentedOption {
  value: string;
  label: ReactNode;
  icon?: ReactNode;
  disabled?: boolean;
}

export interface SegmentedControlProps {
  value: string;
  onValueChange: (v: string) => void;
  options: SegmentedOption[];
  label: string;
  className?: string;
  size?: 'sm' | 'md';
}

export function SegmentedControl({
  value,
  onValueChange,
  options,
  label,
  className,
  size = 'md',
}: SegmentedControlProps) {
  const id = useId();
  return (
    <ToggleGroup.Root
      type="single"
      value={value}
      onValueChange={(v) => v && onValueChange(v)}
      aria-label={label}
      className={cn(
        'border-border bg-surface inline-flex w-fit gap-1 rounded-lg border p-1',
        className,
      )}
    >
      {options.map((o) => (
        <ToggleGroup.Item
          key={o.value}
          value={o.value}
          disabled={o.disabled}
          className={cn(
            'rx-focus-ring text-fg-muted hover:text-fg data-[state=on]:text-fg relative inline-flex items-center gap-1.5 rounded-md px-3 font-medium transition-colors disabled:opacity-45 [&_svg]:size-4',
            size === 'md' ? 'h-8 text-sm' : 'h-7 text-xs',
          )}
        >
          {value === o.value && (
            <motion.span
              layoutId={`${id}-seg`}
              className="bg-surface-3 absolute inset-0 -z-10 rounded-md"
              transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
            />
          )}
          {o.icon}
          {o.label}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}

export interface StepperProps {
  steps: string[];
  current: number;
  className?: string;
  /** e.g. (i, n) => `Step ${i} of ${n}` */
  stepLabel: (index: number, total: number) => string;
}

export function Stepper({ steps, current, className, stepLabel }: StepperProps) {
  return (
    <ol
      className={cn('flex items-center gap-2', className)}
      aria-label={stepLabel(current + 1, steps.length)}
    >
      {steps.map((s, i) => {
        const done = i < current;
        const active = i === current;
        return (
          <li
            key={s}
            className="flex flex-1 items-center gap-2"
            aria-current={active ? 'step' : undefined}
          >
            <span
              className={cn(
                'grid size-7 shrink-0 place-items-center rounded-full border text-xs font-semibold transition-colors',
                done && 'border-accent bg-accent text-accent-fg',
                active && 'border-accent bg-accent-soft text-accent-text',
                !done && !active && 'border-border text-fg-subtle',
              )}
            >
              {done ? <Check className="size-3.5" strokeWidth={3} /> : i + 1}
            </span>
            <span
              className={cn('truncate text-sm', active ? 'text-fg font-semibold' : 'text-fg-muted')}
            >
              {s}
            </span>
            {i < steps.length - 1 && (
              <span className={cn('h-px flex-1', done ? 'bg-accent' : 'bg-border')} />
            )}
          </li>
        );
      })}
    </ol>
  );
}

export interface LineChartSeries {
  id: string;
  values: number[];
  /** CSS colour, defaults to the accent. */
  color?: string;
}

export interface LineChartProps {
  series: LineChartSeries[];
  max?: number;
  min?: number;
  height?: number;
  label: string;
  className?: string;
  /** Threshold line (e.g. 50 ms for MSPT). */
  threshold?: number;
  area?: boolean;
}

/** Tiny dependency-free SVG line chart for live metrics. */
export function LineChart({
  series,
  max,
  min = 0,
  height = 80,
  label,
  className,
  threshold,
  area = true,
}: LineChartProps) {
  const id = useId();
  const all = series.flatMap((s) => s.values);
  const hi = max ?? Math.max(1, ...all) * 1.15;
  const w = 100;
  // Out-of-range samples (a server's first TPS reading can be in the thousands) are clamped:
  // the SVG is overflow-visible, an unclamped point would draw across the whole window.
  const clamp = (v: number) => (Number.isFinite(v) ? Math.min(hi, Math.max(min, v)) : min);
  const y = (v: number) => height - ((clamp(v) - min) / (hi - min || 1)) * height;
  return (
    <svg
      role="img"
      aria-label={label}
      viewBox={`0 0 ${w} ${height}`}
      preserveAspectRatio="none"
      className={cn('w-full overflow-visible', className)}
      style={{ height }}
    >
      {threshold != null && (
        <line
          x1={0}
          x2={w}
          y1={y(threshold)}
          y2={y(threshold)}
          stroke="var(--rl-warning)"
          strokeDasharray="2 2"
          strokeWidth={0.6}
          vectorEffect="non-scaling-stroke"
        />
      )}
      {series.map((s) => {
        const n = s.values.length;
        if (n < 2) return null;
        const pts = s.values.map((v, i) => `${(i / (n - 1)) * w},${y(v)}`);
        const color = s.color ?? 'var(--rl-accent)';
        return (
          <g key={s.id}>
            {area && (
              <>
                <defs>
                  <linearGradient id={`${id}-${s.id}`} x1="0" x2="0" y1="0" y2="1">
                    <stop offset="0%" stopColor={color} stopOpacity={0.3} />
                    <stop offset="100%" stopColor={color} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <polygon
                  points={`0,${height} ${pts.join(' ')} ${w},${height}`}
                  fill={`url(#${id}-${s.id})`}
                />
              </>
            )}
            <polyline
              points={pts.join(' ')}
              fill="none"
              stroke={color}
              strokeWidth={1.75}
              vectorEffect="non-scaling-stroke"
              strokeLinejoin="round"
            />
          </g>
        );
      })}
    </svg>
  );
}
