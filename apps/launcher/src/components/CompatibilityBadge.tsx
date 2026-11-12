import { Badge, Tooltip } from '@rustic/ui';
import { CheckCircle2, CircleHelp, Hammer, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { Compatibility } from '@/ipc/types/content';

/** "Builds on your Horizon" / "Compatible" / "Incompatible: reason" badges. */
export function CompatibilityBadge({ compat }: { compat: Compatibility }) {
  const { t } = useTranslation();
  if (compat.status === 'compatible') {
    return compat.builds_on_horizon ? (
      <Badge variant="success" icon={<Hammer />}>
        {t('compat.buildsOnHorizon')}
      </Badge>
    ) : (
      <Badge variant="info" icon={<CheckCircle2 />}>
        {t('compat.compatible')}
      </Badge>
    );
  }
  if (compat.status === 'incompatible') {
    const reason = t(`compat.reasons.${compat.reason}`);
    return (
      <Tooltip content={compat.detail}>
        <Badge variant="danger" icon={<XCircle />} tabIndex={0}>
          {t('compat.incompatible', { reason })}
        </Badge>
      </Tooltip>
    );
  }
  return (
    <Badge variant="neutral" icon={<CircleHelp />}>
      {t('compat.unknown')}
    </Badge>
  );
}
