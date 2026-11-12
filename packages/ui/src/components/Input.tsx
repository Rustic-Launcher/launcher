import { Search, X } from 'lucide-react';
import { Label as L } from 'radix-ui';
import {
  forwardRef,
  useId,
  type ComponentProps,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
} from 'react';
import { cn } from '../lib/cn';

const fieldBase =
  'w-full rounded-md border border-border-strong/50 bg-surface-2 text-sm text-fg placeholder:text-fg-subtle transition-[border-color,box-shadow] duration-150 outline-none hover:border-border-strong focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent/35 disabled:cursor-not-allowed disabled:opacity-50 aria-[invalid=true]:border-danger aria-[invalid=true]:focus-visible:ring-danger/35';

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  leftIcon?: ReactNode;
  rightSlot?: ReactNode;
  invalid?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, leftIcon, rightSlot, invalid, ...props },
  ref,
) {
  return (
    <div className={cn('relative flex items-center', className)}>
      {leftIcon && (
        <span className="text-fg-subtle pointer-events-none absolute left-3 [&_svg]:size-4">
          {leftIcon}
        </span>
      )}
      <input
        ref={ref}
        aria-invalid={invalid || undefined}
        className={cn(fieldBase, 'h-9 px-3', leftIcon && 'pl-9', rightSlot && 'pr-20')}
        {...props}
      />
      {rightSlot && <span className="absolute right-2 flex items-center gap-1">{rightSlot}</span>}
    </div>
  );
});

export interface SearchInputProps extends Omit<InputProps, 'onChange' | 'value'> {
  value: string;
  onValueChange: (v: string) => void;
  clearLabel: string;
  /** Shortcut hint, e.g. "Ctrl K". */
  shortcut?: string;
}

export const SearchInput = forwardRef<HTMLInputElement, SearchInputProps>(function SearchInput(
  { value, onValueChange, clearLabel, shortcut, ...props },
  ref,
) {
  return (
    <Input
      ref={ref}
      type="search"
      role="searchbox"
      value={value}
      onChange={(e) => onValueChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && value) {
          e.stopPropagation();
          onValueChange('');
        }
      }}
      leftIcon={<Search />}
      rightSlot={
        value ? (
          <button
            type="button"
            aria-label={clearLabel}
            onClick={() => onValueChange('')}
            className="rx-focus-ring text-fg-subtle hover:text-fg rounded p-1"
          >
            <X className="size-3.5" />
          </button>
        ) : shortcut ? (
          <Kbd>{shortcut}</Kbd>
        ) : null
      }
      {...props}
    />
  );
});

export const Textarea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement> & { invalid?: boolean }
>(function Textarea({ className, invalid, ...props }, ref) {
  return (
    <textarea
      ref={ref}
      aria-invalid={invalid || undefined}
      className={cn(fieldBase, 'min-h-24 px-3 py-2', className)}
      {...props}
    />
  );
});

export function Kbd({ className, ...props }: ComponentProps<'kbd'>) {
  return (
    <kbd
      className={cn(
        'border-border bg-surface text-2xs text-fg-muted inline-flex h-5 items-center rounded border px-1.5 font-sans font-medium',
        className,
      )}
      {...props}
    />
  );
}

export function Label({ className, ...props }: ComponentProps<typeof L.Root>) {
  return <L.Root className={cn('text-fg text-sm font-medium', className)} {...props} />;
}

export interface FieldProps {
  label: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  className?: string;
  /** Render prop receiving the id and aria props to spread on the control. */
  children: (p: { id: string; 'aria-describedby'?: string; 'aria-invalid'?: boolean }) => ReactNode;
}

/** Label + control + hint/error, wired for accessibility. */
export function Field({ label, hint, error, className, children }: FieldProps) {
  const id = useId();
  const descId = `${id}-desc`;
  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <Label htmlFor={id}>{label}</Label>
      {children({
        id,
        'aria-describedby': hint || error ? descId : undefined,
        'aria-invalid': error ? true : undefined,
      })}
      {(error || hint) && (
        <p id={descId} className={cn('text-xs', error ? 'text-danger-text' : 'text-fg-subtle')}>
          {error ?? hint}
        </p>
      )}
    </div>
  );
}
