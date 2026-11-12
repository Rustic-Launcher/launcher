import { Check, ChevronDown } from 'lucide-react';
import { Select as S } from 'radix-ui';
import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

export interface SelectOption {
  value: string;
  label: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  disabled?: boolean;
}

export interface SelectProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (v: string) => void;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  id?: string;
  'aria-label'?: string;
  size?: 'sm' | 'md';
}

export function Select({
  value,
  defaultValue,
  onValueChange,
  options,
  placeholder,
  disabled,
  className,
  id,
  size = 'md',
  ...aria
}: SelectProps) {
  return (
    <S.Root
      value={value}
      defaultValue={defaultValue}
      onValueChange={onValueChange}
      disabled={disabled}
    >
      <S.Trigger
        id={id}
        aria-label={aria['aria-label']}
        className={cn(
          'rx-focus-ring border-border-strong/50 bg-surface-2 text-fg hover:border-border-strong data-[placeholder]:text-fg-subtle inline-flex w-full items-center justify-between gap-2 rounded-md border px-3 text-sm transition-colors disabled:cursor-not-allowed disabled:opacity-50 [&_svg]:size-4',
          size === 'md' ? 'h-9' : 'h-8 text-xs',
          className,
        )}
      >
        <S.Value placeholder={placeholder} />
        <S.Icon className="text-fg-subtle">
          <ChevronDown />
        </S.Icon>
      </S.Trigger>
      <S.Portal>
        <S.Content
          position="popper"
          sideOffset={6}
          className="rx-glass z-[70] max-h-[min(var(--radix-select-content-available-height),360px)] min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-lg p-1 shadow-lg"
        >
          <S.Viewport>
            {options.map((o) => (
              <S.Item
                key={o.value}
                value={o.value}
                disabled={o.disabled}
                className="text-fg data-[highlighted]:bg-surface-3 relative flex cursor-pointer items-center gap-2 rounded-md py-2 pr-8 pl-2.5 text-sm outline-none select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-45 [&_svg]:size-4"
              >
                {o.icon}
                <div className="flex flex-col">
                  <S.ItemText>{o.label}</S.ItemText>
                  {o.description && <span className="text-fg-subtle text-xs">{o.description}</span>}
                </div>
                <S.ItemIndicator className="text-accent-text absolute right-2">
                  <Check />
                </S.ItemIndicator>
              </S.Item>
            ))}
          </S.Viewport>
        </S.Content>
      </S.Portal>
    </S.Root>
  );
}
