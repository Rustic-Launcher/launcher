import {
  Badge,
  Button,
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
  Select,
  toast,
} from '@rustic/ui';
import { Clock, Download, Layers, LogIn, Package, Sparkles, Sun } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { HorizonVersion, useHorizonVersionLabel } from '@/components/HorizonVersion';
import { usePlay } from '@/features/play/usePlay';
import type { Instance } from '@/ipc/types/instances';
import { formatDate, formatDuration, formatRelative, shortVersion } from '@/lib/format';
import { useHorizonVersions } from '@/queries/core';
import { useInstanceMutations } from '../queries';

function Stat({ icon, label, value }: { icon: ReactNode; label: string; value: ReactNode }) {
  return (
    <Card padding="md" className="flex items-center gap-3">
      <span className="bg-accent-soft text-accent-text grid size-10 place-items-center rounded-xl [&_svg]:size-5">
        {icon}
      </span>
      <div className="flex min-w-0 flex-col">
        <span className="text-fg-subtle text-xs">{label}</span>
        <span className="truncate text-lg font-semibold tabular-nums">{value}</span>
      </div>
    </Card>
  );
}

export function OverviewTab({ instance }: { instance: Instance }) {
  const { t, i18n } = useTranslation('instances');
  const lang = i18n.language;
  const play = usePlay(instance);
  const versions = useHorizonVersions();
  const label = useHorizonVersionLabel();
  const { setVersion } = useInstanceMutations();
  const [target, setTarget] = useState(instance.horizon_version);

  const changeVersion = (v: string) =>
    setVersion.mutate(
      { id: instance.id, horizon_version: v },
      {
        onSuccess: () =>
          toast({
            tone: 'info',
            title: t('overview.versionChanging', { version: shortVersion(v) }),
            description: t('import.background'),
          }),
      },
    );

  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat
          icon={<Clock />}
          label={t('card.playtime')}
          value={formatDuration(instance.playtime_s, lang)}
        />
        <Stat icon={<Package />} label={t('overview.mods')} value={instance.mod_count} />
        <Stat
          icon={<Layers />}
          label={t('overview.resourcePacks')}
          value={instance.resource_pack_count}
        />
        <Stat icon={<Sun />} label={t('overview.shaders')} value={instance.shader_count} />
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <Card padding="lg">
          <CardHeader>
            <div className="flex flex-col gap-1">
              <CardTitle>{t('overview.version')}</CardTitle>
              <CardDescription>
                <HorizonVersion id={instance.horizon_version} />
              </CardDescription>
            </div>
            {instance.update_available ? (
              <Badge variant="warning" size="md">
                {t('overview.updateAvailable')}
              </Badge>
            ) : (
              <Badge variant="success" size="md">
                {t('overview.upToDate')}
              </Badge>
            )}
          </CardHeader>
          {instance.update_available && (
            <div className="border-warning/30 bg-warning/8 mb-4 flex items-center justify-between gap-3 rounded-xl border p-3">
              <span className="flex items-center gap-2 text-sm">
                <Sparkles className="text-warning-text size-4" />
                {label(instance.update_available)}
              </span>
              <Button
                size="sm"
                variant="primary"
                leftIcon={<Download />}
                loading={setVersion.isPending}
                onClick={() => changeVersion(instance.update_available!)}
              >
                {t('overview.updateNow')}
              </Button>
            </div>
          )}
          <div className="flex items-end gap-2">
            <Select
              aria-label={t('overview.changeVersion')}
              value={target}
              onValueChange={setTarget}
              className="flex-1"
              options={(versions.data ?? []).map((v) => ({
                value: v.id,
                label: label(v.id, v.rustic_edition),
                description: formatDate(v.released_at, lang),
              }))}
            />
            <Button
              disabled={target === instance.horizon_version || setVersion.isPending}
              onClick={() => changeVersion(target)}
            >
              {t('overview.applyVersion')}
            </Button>
          </div>
        </Card>
        <Card padding="lg" className="flex flex-col">
          <CardHeader>
            <div className="flex flex-col gap-1">
              <CardTitle>{t('overview.lastSession')}</CardTitle>
              <CardDescription>
                {instance.last_played_at
                  ? t('overview.lastPlayed', {
                      when: formatRelative(instance.last_played_at, lang),
                    })
                  : t('overview.neverPlayed')}
              </CardDescription>
            </div>
          </CardHeader>
          {instance.last_server ? (
            <div className="bg-surface-2 mt-auto flex items-center justify-between gap-3 rounded-xl p-3">
              <div className="flex min-w-0 flex-col">
                <span className="truncate text-sm font-semibold">{instance.last_server.name}</span>
                <span className="text-fg-subtle truncate font-mono text-xs">
                  {instance.last_server.address}
                </span>
              </div>
              <Button
                size="sm"
                variant="soft"
                leftIcon={<LogIn />}
                disabled={play.state === 'progress' || play.state === 'running'}
                onClick={() => void play.play({ join: instance.last_server! })}
              >
                {t('overview.rejoin')}
              </Button>
            </div>
          ) : (
            <p className="text-fg-muted mt-auto text-sm">{t('overview.noServer')}</p>
          )}
          <dl className="mt-4 grid grid-cols-2 gap-2 text-xs">
            <dt className="text-fg-subtle">{t('overview.created')}</dt>
            <dd className="text-right">{formatDate(instance.created_at, lang)}</dd>
            <dt className="text-fg-subtle">{t('overview.folder')}</dt>
            <dd className="truncate text-right font-mono" title={instance.folder}>
              {instance.folder}
            </dd>
          </dl>
        </Card>
      </div>
    </div>
  );
}
