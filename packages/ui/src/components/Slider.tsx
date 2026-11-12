import { Slider as S } from 'radix-ui';
import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

export interface SliderMark {
  value: number;
  label?: ReactNode;
}

export interface SliderProps {
  value: number;
  onValueChange: (v: number) => void;
  onValueCommit?: (v: number) => void;
  min: number;
  max: number;
  step?: number;
  disabled?: boolean;
  /** Accessible name. */
  label: string;
  /** Human text for the current value (aria-valuetext + bubble). */
  formatValue?: (v: number) => string;
  marks?: SliderMark[];
  /** Values above this are highlighted as risky (e.g. too much RAM). */
  warnAbove?: number;
  className?: string;
}

export function Slider({
  value,
  onValueChange,
  onValueCommit,
  min,
  max,
  step = 1,
  disabled,
  label,
  formatValue = String,
  marks,
  warnAbove,
  className,
}: SliderProps) {
  const pct = (v: number) => ((v - min) / (max - min)) * 100;
  const warn = warnAbove != null && value > warnAbove;
  return (
    <div className={cn('flex w-full flex-col gap-2', className)}>
      <S.Root
        value={[value]}
        onValueChange={(v) => onValueChange(v[0] ?? min)}
        onValueCommit={(v) => onValueCommit?.(v[0] ?? min)}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        className="relative flex h-5 w-full touch-none items-center select-none data-[disabled]:opacity-45"
      >
        <S.Track className="bg-surface-3 relative h-1.5 grow overflow-hidden rounded-full">
          {warnAbove != null && (
            <span
              aria-hidden
              className="bg-warning/25 absolute inset-y-0 right-0"
              style={{ left: `${pct(warnAbove)}%` }}
            />
          )}
          <S.Range
            className={cn('absolute h-full rounded-full', warn ? 'bg-warning' : 'bg-accent')}
          />
        </S.Track>
        <S.Thumb
          aria-label={label}
          aria-valuetext={formatValue(value)}
          className="border-accent focus-visible:ring-accent/35 block size-4.5 rounded-full border-2 bg-white shadow-md transition-transform duration-150 outline-none hover:scale-110 focus-visible:ring-4"
        />
      </S.Root>
      {marks && (
        <div className="text-2xs text-fg-subtle relative h-4" aria-hidden>
          {marks.map((m) => (
            <span
              key={m.value}
              className="absolute -translate-x-1/2 tabular-nums"
              style={{ left: `${pct(m.value)}%` }}
            >
              {m.label ?? formatValue(m.value)}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
