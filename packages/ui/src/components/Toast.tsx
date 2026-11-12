import { AnimatePresence, motion } from 'motion/react';
import { Toast as T } from 'radix-ui';
import { AlertTriangle, CheckCircle2, Info, X, XCircle } from 'lucide-react';
import { useSyncExternalStore, type ReactNode } from 'react';
import { cn } from '../lib/cn';

export type ToastTone = 'info' | 'success' | 'warning' | 'error';

export interface ToastItem {
  id: string;
  title: ReactNode;
  description?: ReactNode;
  tone: ToastTone;
  action?: { label: string; onClick: () => void };
  duration: number;
}

type Listener = () => void;
let items: ToastItem[] = [];
const listeners = new Set<Listener>();
let seq = 0;

function emit() {
  for (const l of listeners) l();
}

/** Imperative toast API, usable anywhere (stores, mutations). */
export const toast = Object.assign(
  (
    t: Omit<ToastItem, 'id' | 'tone' | 'duration'> & Partial<Pick<ToastItem, 'tone' | 'duration'>>,
  ) => {
    const id = `t${++seq}`;
    const item: ToastItem = { tone: 'info', duration: 5000, ...t, id };
    items = [...items, item].slice(-4);
    emit();
    return id;
  },
  {
    dismiss(id: string) {
      items = items.filter((i) => i.id !== id);
      emit();
    },
    clear() {
      items = [];
      emit();
    },
  },
);

function subscribe(l: Listener) {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function useToasts(): ToastItem[] {
  return useSyncExternalStore(
    subscribe,
    () => items,
    () => items,
  );
}

const toneIcon: Record<ToastTone, ReactNode> = {
  info: <Info className="text-info-text" />,
  success: <CheckCircle2 className="text-success-text" />,
  warning: <AlertTriangle className="text-warning-text" />,
  error: <XCircle className="text-danger-text" />,
};

/** Mount once at the app root. */
export function Toaster({ closeLabel, regionLabel }: { closeLabel: string; regionLabel: string }) {
  const list = useToasts();
  return (
    <T.Provider swipeDirection="right" label={regionLabel}>
      <AnimatePresence initial={false}>
        {list.map((t) => (
          <T.Root
            key={t.id}
            duration={t.duration}
            forceMount
            asChild
            onOpenChange={(o) => {
              if (!o) toast.dismiss(t.id);
            }}
            type={t.tone === 'error' ? 'foreground' : 'background'}
          >
            <motion.li
              layout
              initial={{ opacity: 0, x: 24, scale: 0.98 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: 24 }}
              transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
              className={cn(
                'rx-glass pointer-events-auto flex w-[360px] items-start gap-3 rounded-xl p-3.5 shadow-lg [&_svg]:size-4.5',
              )}
            >
              <span className="mt-0.5 shrink-0">{toneIcon[t.tone]}</span>
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <T.Title className="text-fg text-sm font-semibold">{t.title}</T.Title>
                {t.description && (
                  <T.Description className="text-fg-muted text-xs">{t.description}</T.Description>
                )}
                {t.action && (
                  <T.Action altText={t.action.label} asChild>
                    <button
                      type="button"
                      onClick={t.action.onClick}
                      className="rx-focus-ring text-accent-text mt-1.5 w-fit rounded text-xs font-semibold hover:underline"
                    >
                      {t.action.label}
                    </button>
                  </T.Action>
                )}
              </div>
              <T.Close
                aria-label={closeLabel}
                className="rx-focus-ring text-fg-subtle hover:text-fg shrink-0 rounded p-0.5"
              >
                <X className="size-3.5!" />
              </T.Close>
            </motion.li>
          </T.Root>
        ))}
      </AnimatePresence>
      <T.Viewport className="pointer-events-none fixed right-4 bottom-[calc(var(--rl-bottombar-height)+16px)] z-[80] flex flex-col gap-2 outline-none" />
    </T.Provider>
  );
}
