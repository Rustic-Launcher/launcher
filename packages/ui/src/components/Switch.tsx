import { Checkbox as C, Switch as S } from 'radix-ui';
import { Check, Minus } from 'lucide-react';
import { useId, type ComponentProps, type ReactNode } from 'react';
import { cn } from '../lib/cn';

export function Switch({ className, ...props }: ComponentProps<typeof S.Root>) {
  return (
    <S.Root
      className={cn(
        'rx-focus-ring peer bg-surface-3 data-[state=checked]:bg-accent inline-flex h-5.5 w-10 shrink-0 items-center rounded-full border border-transparent p-0.5 transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-45',
        className,
      )}
      {...props}
    >
      <S.Thumb className="block size-4 rounded-full bg-white shadow-sm transition-transform duration-200 ease-out data-[state=checked]:translate-x-[18px]" />
    </S.Root>
  );
}

export interface SwitchRowProps extends ComponentProps<typeof S.Root> {
  label: ReactNode;
  description?: ReactNode;
}

/** A setting row: label + description on the left, switch on the right. */
export function SwitchRow({ label, description, className, ...props }: SwitchRowProps) {
  const id = useId();
  return (
    <div className={cn('flex items-center justify-between gap-6 py-3', className)}>
      <label htmlFor={id} className="flex cursor-pointer flex-col gap-0.5">
        <span className="text-fg text-sm font-medium">{label}</span>
        {description && <span className="text-fg-muted text-xs">{description}</span>}
      </label>
      <Switch id={id} {...props} />
    </div>
  );
}

export function Checkbox({ className, ...props }: ComponentProps<typeof C.Root>) {
  return (
    <C.Root
      className={cn(
        'rx-focus-ring peer border-border-strong bg-surface-2 data-[state=checked]:border-accent data-[state=checked]:bg-accent data-[state=indeterminate]:border-accent data-[state=indeterminate]:bg-accent inline-flex size-4.5 shrink-0 items-center justify-center rounded-[5px] border transition-colors disabled:cursor-not-allowed disabled:opacity-45',
        className,
      )}
      {...props}
    >
      <C.Indicator className="text-accent-fg">
        {props.checked === 'indeterminate' ? (
          <Minus className="size-3.5" strokeWidth={3} />
        ) : (
          <Check className="size-3.5" strokeWidth={3} />
        )}
      </C.Indicator>
    </C.Root>
  );
}
