import { motion } from 'motion/react';
import { Tabs as T } from 'radix-ui';
import {
  createContext,
  useContext,
  useId,
  useState,
  type ComponentProps,
  type ReactNode,
} from 'react';
import { cn } from '../lib/cn';

const TabsCtx = createContext<{ value?: string; id: string; variant: 'underline' | 'pill' }>({
  id: '',
  variant: 'underline',
});

export interface TabsProps extends ComponentProps<typeof T.Root> {
  variant?: 'underline' | 'pill';
}

export function Tabs({
  variant = 'underline',
  className,
  value,
  defaultValue,
  onValueChange,
  ...props
}: TabsProps) {
  const id = useId();
  const [inner, setInner] = useState(defaultValue);
  const current = value ?? inner;
  return (
    <TabsCtx.Provider value={{ value: current, id, variant }}>
      <T.Root
        value={current}
        onValueChange={(v) => {
          setInner(v);
          onValueChange?.(v);
        }}
        className={cn('flex flex-col', className)}
        {...props}
      />
    </TabsCtx.Provider>
  );
}

export function TabsList({ className, ...props }: ComponentProps<typeof T.List>) {
  const { variant } = useContext(TabsCtx);
  return (
    <T.List
      className={cn(
        'relative flex items-center',
        variant === 'underline' && 'border-border gap-1 border-b',
        variant === 'pill' && 'border-border bg-surface w-fit gap-1 rounded-lg border p-1',
        className,
      )}
      {...props}
    />
  );
}

export interface TabsTriggerProps extends ComponentProps<typeof T.Trigger> {
  icon?: ReactNode;
  count?: number;
}

export function TabsTrigger({
  className,
  children,
  icon,
  count,
  value,
  ...props
}: TabsTriggerProps) {
  const ctx = useContext(TabsCtx);
  const active = ctx.value === value;
  return (
    <T.Trigger
      value={value}
      className={cn(
        'rx-focus-ring text-fg-muted hover:text-fg data-[state=active]:text-fg relative inline-flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium transition-colors duration-150 disabled:opacity-45 [&_svg]:size-4',
        ctx.variant === 'underline' && 'mb-[-1px] rounded-b-none',
        className,
      )}
      {...props}
    >
      {icon}
      {children}
      {count != null && (
        <span className="bg-surface-3 text-2xs text-fg-muted rounded-full px-1.5 tabular-nums">
          {count}
        </span>
      )}
      {active && ctx.variant === 'underline' && (
        <motion.span
          layoutId={`${ctx.id}-ind`}
          className="bg-accent absolute inset-x-2 -bottom-px h-0.5 rounded-full"
          transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
        />
      )}
      {active && ctx.variant === 'pill' && (
        <motion.span
          layoutId={`${ctx.id}-ind`}
          className="bg-surface-3 absolute inset-0 -z-10 rounded-md"
          transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
        />
      )}
    </T.Trigger>
  );
}

export function TabsContent({ className, ...props }: ComponentProps<typeof T.Content>) {
  return <T.Content className={cn('mt-4 outline-none', className)} {...props} />;
}
