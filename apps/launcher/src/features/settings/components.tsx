/** Building blocks shared by the Settings sections (and Onboarding). */
import { cn, Skeleton } from '@rustic/ui';
import { useIsMutating, useMutation, useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import { Check } from 'lucide-react';
import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/ipc';
import type { Settings, SettingsPatch } from '@/ipc/types/app';
import { qk } from '@/queries/core';

export const SAVE_KEY = ['settings-save'] as const;

function merge(s: Settings, p: SettingsPatch): Settings {
  return {
    ...s,
    game: { ...s.game, ...p.game },
    launcher: { ...s.launcher, ...p.launcher },
    privacy: { ...s.privacy, ...p.privacy },
    onboarding_completed: p.onboarding_completed ?? s.onboarding_completed,
  };
}

/** Optimistic settings mutation: the UI updates at once, the backend persists in the background. */
export function useSaveSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationKey: SAVE_KEY,
    mutationFn: (patch: SettingsPatch) => invoke('settings_set', patch),
    onMutate: async (patch) => {
      await qc.cancelQueries({ queryKey: qk.settings });
      const prev = qc.getQueryData<Settings>(qk.settings);
      if (prev) qc.setQueryData(qk.settings, merge(prev, patch));
      return { prev };
    },
    onError: (_e, _p, ctx) => {
      if (ctx?.prev) qc.setQueryData(qk.settings, ctx.prev);
    },
    onSuccess: (s) => qc.setQueryData(qk.settings, s),
  });
}

/** "Saving… / Saved" indicator driven by the settings mutation. */
export function SaveIndicator() {
  const { t } = useTranslation('settings');
  const saving = useIsMutating({ mutationKey: SAVE_KEY }) > 0;
  const [shown, setShown] = useState<'saving' | 'saved' | null>(null);
  const wasSaving = useRef(false);
  useEffect(() => {
    if (saving) {
      wasSaving.current = true;
      setShown('saving');
      return;
    }
    if (wasSaving.current) {
      wasSaving.current = false;
      setShown('saved');
      const id = setTimeout(() => setShown(null), 1800);
      return () => clearTimeout(id);
    }
  }, [saving]);
  return (
    <span
      role="status"
      aria-live="polite"
      className="text-fg-subtle inline-flex h-6 items-center text-xs"
    >
      <AnimatePresence mode="wait">
        {shown && (
          <motion.span
            key={shown}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="inline-flex items-center gap-1.5"
          >
            {shown === 'saved' && <Check className="text-success-text size-3.5" aria-hidden />}
            {shown === 'saving' ? t('saving') : t('saved')}
          </motion.span>
        )}
      </AnimatePresence>
    </span>
  );
}

export function SettingsCard({
  title,
  description,
  children,
  action,
  className,
  testId,
}: {
  title: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  action?: ReactNode;
  className?: string;
  testId?: string;
}) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      data-testid={testId}
      className={cn('rx-surface rounded-2xl p-5', className)}
    >
      <div className="mb-3 flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h3 id={id} className="text-fg text-base font-semibold">
            {title}
          </h3>
          {description && <p className="text-fg-muted max-w-2xl text-sm">{description}</p>}
        </div>
        {action}
      </div>
      {children}
    </section>
  );
}

/** A labelled row with a control on the right. `htmlFor` wires the label to the control. */
export function SettingRow({
  label,
  description,
  children,
  htmlFor,
  className,
}: {
  label: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  htmlFor?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        'border-border flex flex-wrap items-center justify-between gap-x-6 gap-y-2 border-t py-3 first:border-t-0',
        className,
      )}
    >
      <div className="flex max-w-xl min-w-0 flex-1 flex-col gap-0.5">
        {htmlFor ? (
          <label htmlFor={htmlFor} className="text-fg text-sm font-medium">
            {label}
          </label>
        ) : (
          <span className="text-fg text-sm font-medium">{label}</span>
        )}
        {description && <span className="text-fg-muted text-xs">{description}</span>}
      </div>
      <div className="flex shrink-0 items-center gap-2">{children}</div>
    </div>
  );
}

export function SectionSkeleton() {
  return (
    <div className="flex flex-col gap-4" aria-busy="true">
      <Skeleton className="h-40 rounded-2xl" />
      <Skeleton className="h-56 rounded-2xl" />
    </div>
  );
}
