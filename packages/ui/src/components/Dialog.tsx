import { AnimatePresence, motion } from 'motion/react';
import { Dialog as D } from 'radix-ui';
import { X } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

/**
 * Modal dialog. Product rule (ADR 0022): reserved for destructive confirmations and purchases;
 * everything else uses inline UI or a Sheet.
 */
export interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  /** Visual tone of the header icon. */
  tone?: 'default' | 'destructive' | 'purchase';
  icon?: ReactNode;
  closeLabel: string;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  tone = 'default',
  icon,
  closeLabel,
  size = 'md',
  className,
}: DialogProps) {
  return (
    <D.Root open={open} onOpenChange={onOpenChange}>
      <AnimatePresence>
        {open && (
          <D.Portal forceMount>
            <D.Overlay asChild forceMount>
              <motion.div
                className="bg-overlay fixed inset-0 z-[60] backdrop-blur-sm"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.15 }}
              />
            </D.Overlay>
            <div className="pointer-events-none fixed inset-0 z-[60] grid place-items-center p-4">
              <D.Content
                asChild
                forceMount
                onOpenAutoFocus={(e) => {
                  // A control marked `data-autofocus` gets the initial focus instead of the first
                  // tabbable element (e.g. a search field placed after a segmented control).
                  const target = (
                    e.currentTarget as HTMLElement | null
                  )?.querySelector<HTMLElement>('[data-autofocus]');
                  if (target) {
                    e.preventDefault();
                    target.focus();
                  }
                }}
              >
                <motion.div
                  initial={{ opacity: 0, scale: 0.96, y: 8 }}
                  animate={{ opacity: 1, scale: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.97, y: 4 }}
                  transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
                  className={cn(
                    'border-glass-border bg-surface pointer-events-auto relative flex max-h-[85vh] w-full flex-col overflow-hidden rounded-2xl border shadow-lg outline-none',
                    size === 'sm' && 'max-w-sm',
                    size === 'md' && 'max-w-md',
                    size === 'lg' && 'max-w-xl',
                    className,
                  )}
                >
                  <div className="flex items-start gap-3 px-6 pt-6">
                    {icon && (
                      <span
                        className={cn(
                          'grid size-10 shrink-0 place-items-center rounded-full [&_svg]:size-5',
                          tone === 'destructive' && 'bg-danger/15 text-danger-text',
                          tone === 'purchase' && 'bg-accent-soft text-accent-text',
                          tone === 'default' && 'bg-surface-3 text-fg',
                        )}
                      >
                        {icon}
                      </span>
                    )}
                    <div className="flex min-w-0 flex-1 flex-col gap-1 pr-6">
                      <D.Title className="text-fg text-lg font-semibold">{title}</D.Title>
                      {description ? (
                        <D.Description className="text-fg-muted text-sm">
                          {description}
                        </D.Description>
                      ) : (
                        <D.Description className="sr-only">{title}</D.Description>
                      )}
                    </div>
                  </div>
                  {children && <div className="overflow-y-auto px-6 pt-4">{children}</div>}
                  {footer && (
                    <div className="border-border bg-bg-elevated/50 mt-6 flex justify-end gap-2 border-t px-6 py-4">
                      {footer}
                    </div>
                  )}
                  {!footer && <div className="pb-6" />}
                  <D.Close
                    aria-label={closeLabel}
                    className="rx-focus-ring text-fg-subtle hover:bg-surface-2 hover:text-fg absolute top-4 right-4 rounded-md p-1.5"
                  >
                    <X className="size-4" />
                  </D.Close>
                </motion.div>
              </D.Content>
            </div>
          </D.Portal>
        )}
      </AnimatePresence>
    </D.Root>
  );
}
