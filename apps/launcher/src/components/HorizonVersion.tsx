import { Badge, cn } from '@rustic/ui';
import { useTranslation } from 'react-i18next';
import type { HorizonVersion as Version } from '@/ipc/types/instances';
import { shortVersion } from '@/lib/format';
import { useHorizonVersions } from '@/queries/core';

/**
 * Whether a build is a Rustic edition: an explicit answer wins, otherwise the backend's version
 * list says (unknown builds are plain Horizon: never claim "Rustic edition" without proof).
 */
export function isRusticEdition(
  id: string,
  versions: Version[] | undefined,
  explicit?: boolean,
): boolean {
  if (explicit !== undefined) return explicit;
  return versions?.find((v) => v.id === id)?.rustic_edition ?? false;
}

/** Localised Horizon version text: "Horizon 0.2.14 (Rustic edition)" or "Horizon 0.2.14". */
export function useHorizonVersionLabel() {
  const { t } = useTranslation();
  const { data: versions } = useHorizonVersions();
  return (id: string, rusticEdition?: boolean) =>
    isRusticEdition(id, versions, rusticEdition)
      ? t('horizon.versionRustic', { version: shortVersion(id) })
      : t('horizon.version', { version: shortVersion(id) });
}

export function HorizonVersion({
  id,
  rusticEdition,
  className,
  badge,
}: {
  id: string;
  rusticEdition?: boolean;
  className?: string;
  /** Show "Rustic edition" as a badge instead of inline text. */
  badge?: boolean;
}) {
  const { t } = useTranslation();
  const { data: versions } = useHorizonVersions();
  const label = useHorizonVersionLabel();
  const rustic = isRusticEdition(id, versions, rusticEdition);
  if (badge) {
    return (
      <span className={cn('inline-flex items-center gap-1.5', className)}>
        <span>{t('horizon.version', { version: shortVersion(id) })}</span>
        {rustic && <Badge variant="accent">{t('horizon.rusticEdition')}</Badge>}
      </span>
    );
  }
  return <span className={className}>{label(id, rusticEdition)}</span>;
}
