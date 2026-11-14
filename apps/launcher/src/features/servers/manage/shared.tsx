import { Button, cn, IconButton, toast } from '@rustic/ui';
import { AnimatePresence, motion } from 'motion/react';
import { Check, ChevronDown, Copy } from 'lucide-react';
import { useId, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { copyText } from '../list/queries';

/** Copy-to-clipboard button with a short "Copied" confirmation. */
export function CopyButton({
  value,
  label,
  size = 'sm',
  withText,
  testId,
}: {
  value: string;
  label: string;
  size?: 'sm' | 'md';
  withText?: boolean;
  testId?: string;
}) {
  const { t } = useTranslation();
  const [done, setDone] = useState(false);
  const onCopy = async () => {
    const ok = await copyText(value);
    if (ok) {
      setDone(true);
      setTimeout(() => setDone(false), 1500);
    } else {
      toast({ tone: 'error', title: t('errors.generic') });
    }
  };
  if (withText) {
    return (
      <Button
        size={size}
        variant="secondary"
        leftIcon={done ? <Check /> : <Copy />}
        onClick={() => void onCopy()}
        aria-label={label}
        data-testid={testId}
      >
        {done ? t('actions.copied') : t('actions.copy')}
      </Button>
    );
  }
  return (
    <IconButton
      size={size}
      label={done ? t('actions.copied') : label}
      icon={done ? <Check className="text-success-text" /> : <Copy />}
      onClick={() => void onCopy()}
      data-testid={testId}
    />
  );
}

/** Monospace value with a copy button (addresses, records, codes). */
export function CopyField({
  value,
  label,
  testId,
  className,
}: {
  value: string;
  label: string;
  testId?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        'border-border bg-surface-2 flex min-w-0 items-center gap-1 rounded-md border py-1 pr-1 pl-3',
        className,
      )}
    >
      <code
        className="text-fg min-w-0 flex-1 truncate font-mono text-sm"
        data-testid={testId}
        title={value}
      >
        {value}
      </code>
      <CopyButton value={value} label={label} />
    </div>
  );
}

/** Disclosure section (button + region), keyboard accessible. */
export function Collapsible({
  title,
  children,
  defaultOpen = false,
  icon,
  className,
  tone = 'default',
}: {
  title: ReactNode;
  children: ReactNode;
  defaultOpen?: boolean;
  icon?: ReactNode;
  className?: string;
  tone?: 'default' | 'warning';
}) {
  const [open, setOpen] = useState(defaultOpen);
  const id = useId();
  return (
    <div
      className={cn(
        'rounded-xl border',
        tone === 'warning' ? 'border-warning/30' : 'border-border',
        className,
      )}
    >
      <button
        type="button"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen((o) => !o)}
        className="rx-focus-ring text-fg hover:bg-surface-2/60 flex w-full items-center gap-2 rounded-xl px-4 py-3 text-left text-sm font-medium [&_svg]:size-4"
      >
        {icon}
        <span className="flex-1">{title}</span>
        <ChevronDown
          className={cn('text-fg-subtle transition-transform duration-200', open && 'rotate-180')}
        />
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            id={id}
            role="region"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
            className="overflow-hidden"
          >
            <div className="px-4 pb-4">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

/** Card-like panel used throughout the management tabs. */
export function Panel({
  title,
  description,
  action,
  children,
  className,
  testId,
}: {
  title?: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  children?: ReactNode;
  className?: string;
  testId?: string;
}) {
  return (
    <section className={cn('rx-surface rounded-2xl p-5', className)} data-testid={testId}>
      {(title || action) && (
        <header className="mb-4 flex flex-wrap items-start justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            {title && <h3 className="text-fg text-sm font-semibold">{title}</h3>}
            {description && <p className="text-fg-muted text-xs">{description}</p>}
          </div>
          {action}
        </header>
      )}
      {children}
    </section>
  );
}
