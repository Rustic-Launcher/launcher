import { DropdownMenu as M, Popover as P } from 'radix-ui';
import { Check } from 'lucide-react';
import type { ComponentProps, ReactNode } from 'react';
import { cn } from '../lib/cn';

export const Dropdown = M.Root;
export const DropdownTrigger = M.Trigger;
export const DropdownGroup = M.Group;

export function DropdownContent({
  className,
  align = 'end',
  ...props
}: ComponentProps<typeof M.Content>) {
  return (
    <M.Portal>
      <M.Content
        align={align}
        sideOffset={6}
        className={cn(
          'rx-glass data-[state=open]:animate-in z-[70] min-w-48 overflow-hidden rounded-xl p-1 shadow-lg',
          className,
        )}
        {...props}
      />
    </M.Portal>
  );
}

const itemCls =
  'relative flex cursor-pointer select-none items-center gap-2.5 rounded-md px-2.5 py-2 text-sm text-fg outline-none data-[disabled]:pointer-events-none data-[highlighted]:bg-surface-3 data-[disabled]:opacity-45 [&_svg]:size-4 [&_svg]:text-fg-muted';

export function DropdownItem({
  className,
  icon,
  shortcut,
  destructive,
  children,
  ...props
}: ComponentProps<typeof M.Item> & { icon?: ReactNode; shortcut?: string; destructive?: boolean }) {
  return (
    <M.Item
      className={cn(itemCls, destructive && 'text-danger-text [&_svg]:text-danger-text', className)}
      {...props}
    >
      {icon}
      <span className="flex-1">{children}</span>
      {shortcut && <span className="text-fg-subtle text-xs">{shortcut}</span>}
    </M.Item>
  );
}

export function DropdownCheckboxItem({
  className,
  children,
  ...props
}: ComponentProps<typeof M.CheckboxItem>) {
  return (
    <M.CheckboxItem className={cn(itemCls, 'pl-8', className)} {...props}>
      <M.ItemIndicator className="absolute left-2.5">
        <Check />
      </M.ItemIndicator>
      {children}
    </M.CheckboxItem>
  );
}

export function DropdownLabel({ className, ...props }: ComponentProps<typeof M.Label>) {
  return (
    <M.Label
      className={cn(
        'text-2xs text-fg-subtle px-2.5 pt-2 pb-1 font-semibold tracking-wider uppercase',
        className,
      )}
      {...props}
    />
  );
}

export function DropdownSeparator({ className }: { className?: string }) {
  return <M.Separator className={cn('bg-border my-1 h-px', className)} />;
}

export const Popover = P.Root;
export const PopoverTrigger = P.Trigger;
export const PopoverClose = P.Close;
export function PopoverContent({
  className,
  align = 'end',
  ...props
}: ComponentProps<typeof P.Content>) {
  return (
    <P.Portal>
      <P.Content
        align={align}
        sideOffset={8}
        className={cn('rx-glass z-[70] rounded-xl p-0 shadow-lg outline-none', className)}
        {...props}
      />
    </P.Portal>
  );
}
