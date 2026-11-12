import { Badge, Button, Skeleton } from '@rustic/ui';
import { ExternalLink } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion } from '@/components/HorizonVersion';
import { RusticMark } from '@/layout/Logo';
import { invoke } from '@/ipc';
import { useAppInfo, useHorizonVersions } from '@/queries/core';
import { SettingRow, SettingsCard } from '../components';
import { LEGAL_URLS } from '../legal';
import { UpdatesCard } from './LauncherSection';

export function AboutSection() {
  const { t } = useTranslation('settings');
  const { data: app, isLoading } = useAppInfo();
  const { data: versions } = useHorizonVersions();
  const recommended = versions?.find((v) => v.recommended);
  const links: { key: keyof typeof LEGAL_URLS; label: string }[] = [
    { key: 'privacy', label: t('about.links.privacy') },
    { key: 'terms', label: t('about.links.terms') },
    { key: 'legalNotice', label: t('about.links.legalNotice') },
    { key: 'fairPlay', label: t('about.links.fairPlay') },
    { key: 'thirdParty', label: t('about.links.thirdParty') },
    { key: 'horizon', label: t('about.links.horizon') },
  ];
  return (
    <div className="flex flex-col gap-4">
      <SettingsCard title={t('about.title')}>
        <div className="flex items-center gap-4">
          <RusticMark size={56} />
          <div className="flex flex-col gap-1">
            <span className="text-lg font-bold">{t('about.product')}</span>
            {isLoading ? (
              <Skeleton className="h-4 w-40" />
            ) : (
              app && (
                <span className="text-fg-muted flex items-center gap-2 text-sm">
                  {t('about.version', { version: app.version })}
                  <Badge variant={app.channel === 'beta' ? 'warning' : 'neutral'}>
                    {t(`about.channel.${app.channel}`)}
                  </Badge>
                  {!app.native && <Badge variant="info">{t('about.browserMock')}</Badge>}
                </span>
              )
            )}
          </div>
        </div>
        <div className="mt-4">
          <SettingRow label={t('about.horizon')} description={t('about.horizonHint')}>
            {recommended ? (
              <HorizonVersion id={recommended.id} badge className="text-sm" />
            ) : (
              <Skeleton className="h-5 w-40" />
            )}
          </SettingRow>
          <SettingRow label={t('about.licence')}>
            <code className="bg-surface-2 rounded px-2 py-1 text-xs">MIT OR Apache-2.0</code>
          </SettingRow>
          <SettingRow label={t('about.platform')}>
            <span className="text-fg-muted text-sm">{app ? `${app.os} · ${app.arch}` : '—'}</span>
          </SettingRow>
        </div>
      </SettingsCard>

      <UpdatesCard />

      <SettingsCard title={t('about.legalTitle')} description={t('about.legalDescription')}>
        <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {links.map((l) => (
            <li key={l.key}>
              <Button
                variant="ghost"
                className="w-full justify-between"
                rightIcon={<ExternalLink />}
                onClick={() => void invoke('app_open_url', { url: LEGAL_URLS[l.key] })}
              >
                {l.label}
              </Button>
            </li>
          ))}
        </ul>
        <div className="border-border bg-surface-2/50 text-fg-muted mt-4 flex flex-col gap-2 rounded-xl border p-4 text-xs">
          <p>{t('about.notAffiliated')}</p>
          <p>{t('about.horizonCredit')}</p>
          <p>{t('about.company')}</p>
        </div>
      </SettingsCard>
    </div>
  );
}
