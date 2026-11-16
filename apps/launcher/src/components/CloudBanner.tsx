import { cn } from '@rustic/ui';
import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, CheckCircle2, Info, OctagonAlert, X } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import type { CloudBanner as Banner } from '@/cloud/types';
import { Markdown } from '@/features/content/Markdown';

/**
 * Announcements written in the website's back-office for the launcher (ADR 0171):
 * `GET /v1/banners?surface=launcher&lang=…`, read when the launcher starts and every 30 minutes,
 * shown above the page. A dismissal is remembered per banner and version on this computer
 * (`localStorage`, guarded); an edited banner shows again. Offline or with the mock cloud there
 * is none.
 */
const KEY = 'rustic.launcher.banners.dismissed';

const LOOK: Record<Banner['severity'], { box: string; icon: ReactNode }> = {
  info: { box: 'border-info/30 bg-info/10', icon: <Info className="text-info-text size-4" /> },
  success: {
    box: 'border-success/30 bg-success/10',
    icon: <CheckCircle2 className="text-success-text size-4" />,
  },
  warning: {
    box: 'border-warning/30 bg-warning/10',
    icon: <AlertTriangle className="text-warning-text size-4" />,
  },
  critical: {
    box: 'border-danger/35 bg-danger/10',
    icon: <OctagonAlert className="text-danger-text size-4" />,
  },
};

/** `id@updated_at`: an edited banner is a new one for the dismissals. */
export const dismissKey = (b: Pick<Banner, 'id' | 'updated_at'>) => `${b.id}@${b.updated_at}`;

function readDismissed(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '[]') as unknown;
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

export function CloudBanners() {
  const { t, i18n } = useTranslation();
  const [dismissed, setDismissed] = useState<string[]>(readDismissed);
  const q = useQuery({
    queryKey: ['cloud-banners', i18n.language],
    queryFn: async () => (await cloud()).banners(i18n.language),
    staleTime: 10 * 60_000,
    refetchInterval: 30 * 60_000,
    retry: false,
  });
  const shown = (q.data ?? []).filter((b) => !dismissed.includes(dismissKey(b)));
  if (!shown.length) return null;
  const dismiss = (b: Banner) => {
    const next = [
      ...dismissed.filter((k) => (q.data ?? []).some((x) => dismissKey(x) === k)),
      dismissKey(b),
    ];
    setDismissed(next);
    try {
      localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      /* a convenience only */
    }
  };
  return (
    <div className="flex flex-col gap-2 px-4 pt-4" data-testid="cloud-banners">
      {shown.map((b) => {
        const look = LOOK[b.severity] ?? LOOK.info;
        return (
          <section
            key={b.id}
            role={b.severity === 'critical' || b.severity === 'warning' ? 'alert' : 'status'}
            aria-label={b.title}
            className={cn('flex gap-3 rounded-xl border px-3 py-2.5 text-sm', look.box)}
            data-testid="cloud-banner"
            data-severity={b.severity}
          >
            <span aria-hidden className="mt-0.5 shrink-0">
              {look.icon}
            </span>
            <div className="min-w-0 flex-1">
              <p className="text-fg font-semibold">{b.title}</p>
              {b.body.trim() && <Markdown source={b.body} />}
            </div>
            {b.dismissible && (
              <button
                type="button"
                onClick={() => dismiss(b)}
                aria-label={t('actions.close')}
                title={t('actions.close')}
                className="rx-focus-ring text-fg-muted hover:text-fg -mt-0.5 -mr-1 grid size-7 shrink-0 place-items-center rounded-md"
                data-testid="cloud-banner-dismiss"
              >
                <X className="size-4" aria-hidden />
              </button>
            )}
          </section>
        );
      })}
    </div>
  );
}
