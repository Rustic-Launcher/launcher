import { Info } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { SettingsCard } from '../components';
import { PalettePicker } from '../PalettePicker';
import { ThemeControl } from '../ThemeControl';

export function AppearanceSection() {
  const { t } = useTranslation('settings');
  return (
    <div className="flex flex-col gap-4">
      <SettingsCard
        title={t('appearance.theme.title')}
        description={t('appearance.theme.description')}
      >
        <ThemeControl />
      </SettingsCard>
      <SettingsCard
        title={t('appearance.palette.title')}
        description={t('appearance.palette.description')}
      >
        <PalettePicker />
      </SettingsCard>
      <SettingsCard title={t('appearance.motion.title')}>
        <p className="text-fg-muted flex items-start gap-2 text-sm">
          <Info className="text-fg-subtle mt-0.5 size-4 shrink-0" aria-hidden />
          {t('appearance.motion.body')}
        </p>
      </SettingsCard>
    </div>
  );
}
