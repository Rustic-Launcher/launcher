import { Button, EmptyState } from '@rustic/ui';
import { useRouter, type ErrorComponentProps } from '@tanstack/react-router';
import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function RouteError({ error, reset }: ErrorComponentProps) {
  const { t } = useTranslation();
  const router = useRouter();
  return (
    <div className="grid h-full place-items-center p-8">
      <EmptyState
        tone="error"
        icon={<AlertTriangle />}
        title={t('errors.page.title')}
        description={error instanceof Error ? error.message : t('errors.generic')}
        action={
          <Button
            variant="primary"
            onClick={() => {
              reset();
              void router.invalidate();
            }}
          >
            {t('actions.retry')}
          </Button>
        }
      />
    </div>
  );
}

/** Inline error block for a failed query. */
export function QueryError({ onRetry, message }: { onRetry?: () => void; message?: string }) {
  const { t } = useTranslation();
  return (
    <EmptyState
      tone="error"
      icon={<AlertTriangle />}
      title={t('errors.load.title')}
      description={message ?? t('errors.generic')}
      action={
        onRetry ? (
          <Button size="sm" onClick={onRetry}>
            {t('actions.retry')}
          </Button>
        ) : undefined
      }
    />
  );
}
