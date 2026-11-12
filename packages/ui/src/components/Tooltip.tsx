import { Tooltip as T } from 'radix-ui';
import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

export const TooltipProvider = T.Provider;

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  side?: 'top' | 'right' | 'bottom' | 'left';
  /** Keyboard shortcut shown after the content. */
  shortcut?: string;
  delay?: number;
  className?: string;
}

export function Tooltip({
  content,
  children,
  side = 'top',
  shortcut,
  delay,
  className,
}: TooltipProps) {
  return (
    <T.Root delayDuration={delay ?? 250}>
      <T.Trigger asChild>{children}</T.Trigger>
      <T.Portal>
        <T.Content
          side={side}
          sideOffset={8}
          className={cn(
            'border-glass-border bg-surface-3 text-fg z-[90] flex items-center gap-2 rounded-sm border px-2.5 py-1.5 text-xs font-medium shadow-md',
            'data-[state=delayed-open]:animate-in data-[state=closed]:opacity-0',
            className,
          )}
        >
          {content}
          {shortcut && (
            <kbd className="border-border bg-surface-2 text-2xs text-fg-muted rounded border px-1 font-sans">
              {shortcut}
            </kbd>
          )}
        </T.Content>
      </T.Portal>
    </T.Root>
  );
}
