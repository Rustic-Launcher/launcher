import { Button, EmptyState } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { Compass } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function NotFound() {
  const { t } = useTranslation();
  return (
    <div className="grid h-full place-items-center p-8">
      <EmptyState
        icon={<Compass />}
        title={t('errors.notFound.title')}
        description={t('errors.notFound.description')}
        action={
          <Button asChild variant="primary">
            <Link to="/">{t('nav.home')}</Link>
          </Button>
        }
      />
    </div>
  );
}
