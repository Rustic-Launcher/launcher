import { AnimatePresence, motion } from 'motion/react';
import { Dialog as D } from 'radix-ui';
import { X } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

export interface SheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  side?: 'right' | 'left';
  children: ReactNode;
  footer?: ReactNode;
  closeLabel: string;
  width?: number;
  className?: string;
}

/** Side panel for secondary tasks (settings of an item, details) without leaving the page. */
export function Sheet({
  open,
  onOpenChange,
  title,
  description,
  side = 'right',
  children,
  footer,
  closeLabel,
  width = 440,
  className,
}: SheetProps) {
  const x = side === 'right' ? '100%' : '-100%';
  return (
    <D.Root open={open} onOpenChange={onOpenChange}>
      <AnimatePresence>
        {open && (
          <D.Portal forceMount>
            <D.Overlay asChild forceMount>
              <motion.div
                className="bg-overlay fixed inset-0 z-[50]"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.2 }}
              />
            </D.Overlay>
            <D.Content asChild forceMount>
              <motion.aside
                initial={{ x }}
                animate={{ x: 0 }}
                exit={{ x }}
                transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
                style={{ width: `min(${width}px, 100vw)` }}
                className={cn(
                  'rx-glass fixed inset-y-0 z-[50] flex flex-col shadow-lg outline-none',
                  side === 'right' ? 'right-0 rounded-l-2xl' : 'left-0 rounded-r-2xl',
                  className,
                )}
              >
                <header className="border-border flex items-start justify-between gap-4 border-b px-5 py-4">
                  <div className="flex flex-col gap-0.5">
                    <D.Title className="text-fg text-base font-semibold">{title}</D.Title>
                    {description ? (
                      <D.Description className="text-fg-muted text-xs">{description}</D.Description>
                    ) : (
                      <D.Description className="sr-only">{title}</D.Description>
                    )}
                  </div>
                  <D.Close
                    aria-label={closeLabel}
                    className="rx-focus-ring text-fg-subtle hover:bg-surface-2 hover:text-fg rounded-md p-1.5"
                  >
                    <X className="size-4" />
                  </D.Close>
                </header>
                <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>
                {footer && (
                  <footer className="border-border flex justify-end gap-2 border-t px-5 py-3">
                    {footer}
                  </footer>
                )}
              </motion.aside>
            </D.Content>
          </D.Portal>
        )}
      </AnimatePresence>
    </D.Root>
  );
}
