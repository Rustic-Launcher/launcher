/** Building blocks of the Content pages: icon, target picker, install button, badges. */
import {
  Badge,
  Button,
  cn,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Select,
  Skeleton,
  Tooltip,
  type SelectOption,
} from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import {
  Check,
  Download,
  Globe2,
  Layers,
  PackagePlus,
  Server as ServerIcon,
  Sparkles,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { InstanceIcon } from '@/components/InstanceIcon';
import { useHorizonVersionLabel } from '@/components/HorizonVersion';
import type { Compatibility, ContentKind, ContentSource, ShaderFormat } from '@/ipc/types/content';
import type { Instance } from '@/ipc/types/instances';
import type { Server } from '@/ipc/types/servers';
import { formatPercent } from '@/lib/format';
import {
  eligibleServers,
  formatTarget,
  hueOf,
  isInstalledOn,
  toInstallTarget,
  type TargetRef,
} from './target';
import { useCreateFromModpack, useInstall, useInstallProgress, useWorlds } from './useContent';

const iconSizes = {
  sm: 'size-9 rounded-lg text-xs',
  md: 'size-12 rounded-xl text-sm',
  lg: 'size-14 rounded-xl text-base',
  xl: 'size-20 rounded-2xl text-2xl',
};

/** Generated project icon (initials on a hue gradient; no third-party images). */
export function ContentIcon({
  id,
  name,
  size = 'md',
  className,
}: {
  id: string;
  name: string;
  size?: keyof typeof iconSizes;
  className?: string;
}) {
  const hue = hueOf(id);
  const initials = name
    .split(/[\s&-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join('');
  return (
    <span
      aria-hidden
      className={cn(
        'grid shrink-0 place-items-center font-bold text-white shadow-sm',
        iconSizes[size],
        className,
      )}
      style={{
        background: `linear-gradient(140deg, hsl(${hue} 55% 46%), hsl(${(hue + 35) % 360} 60% 26%))`,
      }}
    >
      {initials}
    </span>
  );
}

export function SourceBadge({ source }: { source: ContentSource }) {
  const { t } = useTranslation('content');
  return (
    <Badge variant={source === 'rustic' ? 'accent' : 'neutral'}>{t(`sources.${source}`)}</Badge>
  );
}

/** Shader pack format and its real support status in Horizon (docs/shaders.md, HORIZON_FACTS A2). */
export function ShaderFormatBadge({ format }: { format: ShaderFormat }) {
  const { t } = useTranslation('content');
  return (
    <Tooltip content={t(`shaders.formatHint.${format}`)}>
      <Badge variant={format === 'horizon' ? 'info' : 'neutral'} icon={<Sparkles />} tabIndex={0}>
        {t(`shaders.format.${format}`)}
      </Badge>
    </Tooltip>
  );
}

export function SideBadge({ side }: { side: 'client' | 'server' | 'both' }) {
  const { t } = useTranslation('content');
  return <Badge variant="neutral">{t(`side.${side}`)}</Badge>;
}

/** "Installing into: <instance ▾>" (instances, or Rustic servers for plugins). */
export function TargetPicker({
  kind,
  value,
  onChange,
  instances,
  servers,
  loading,
}: {
  kind: ContentKind;
  value: TargetRef | null;
  onChange: (t: TargetRef) => void;
  instances: Instance[];
  servers: Server[];
  loading?: boolean;
}) {
  const { t } = useTranslation('content');
  const versionLabel = useHorizonVersionLabel();
  if (loading) return <Skeleton className="h-9 w-64" />;
  const options: SelectOption[] = [];
  if (kind !== 'plugin') {
    for (const i of instances) {
      options.push({
        value: formatTarget({ kind: 'instance', id: i.id }),
        label: i.name,
        description: versionLabel(i.horizon_version),
        icon: (
          <InstanceIcon icon={i.icon} size="sm" className="size-6 rounded-md [&_svg]:size-3.5" />
        ),
      });
    }
  }
  for (const s of eligibleServers(kind, servers)) {
    options.push({
      value: formatTarget({ kind: 'server', id: s.id }),
      label: s.name,
      description: t('target.serverDescription', {
        engine: t(`engine.${s.engine}`),
        hosting: t(`hosting.${s.hosting.mode}`),
      }),
      icon: <ServerIcon className="text-fg-muted" />,
    });
  }
  return (
    <div className="flex items-center gap-2">
      <span className="text-fg-muted shrink-0 text-xs font-medium">{t('target.label')}</span>
      <Select
        aria-label={t('target.label')}
        value={value ? formatTarget(value) : undefined}
        onValueChange={(v) => {
          const [k, ...rest] = v.split(':');
          onChange({ kind: k as TargetRef['kind'], id: rest.join(':') });
        }}
        options={options}
        placeholder={t('target.none')}
        className="w-64"
      />
    </div>
  );
}

export interface InstallableItem {
  id: string;
  name: string;
  kind: ContentKind;
  compatibility: Compatibility;
  installed_in: string[];
}

/** Install / Installed / Installing / Create instance, with the right disabled reasons. */
export function InstallButton({
  item,
  target,
  targetName,
  versionId,
  compatibility,
  size = 'sm',
  className,
}: {
  item: InstallableItem;
  target: TargetRef | null;
  targetName: string;
  versionId?: string;
  /** Overrides the item compatibility (version rows). */
  compatibility?: Compatibility;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}) {
  const { t, i18n } = useTranslation('content');
  const install = useInstall();
  const fromPack = useCreateFromModpack();
  const progress = useInstallProgress(item.id, target?.id);
  const compat = compatibility ?? item.compatibility;
  const incompatible = compat.status === 'incompatible';

  const disabledWithReason = (label: string, reason: string, icon = <Download />) => (
    <Tooltip content={reason}>
      {/* aria-disabled (not disabled) keeps it focusable, so keyboard users get the reason too. */}
      <Button
        size={size}
        aria-disabled
        leftIcon={icon}
        onClick={(e) => e.preventDefault()}
        className={cn('cursor-not-allowed opacity-45 active:scale-100', className)}
        aria-description={reason}
      >
        {label}
      </Button>
    </Tooltip>
  );

  if (item.kind === 'modpack') {
    if (incompatible)
      return disabledWithReason(t('install.createInstance'), compat.detail, <PackagePlus />);
    return (
      <Button
        size={size}
        variant="primary"
        leftIcon={<PackagePlus />}
        loading={fromPack.isPending}
        className={className}
        onClick={() => fromPack.mutate({ projectId: item.id, name: item.name })}
      >
        {t('install.createInstance')}
      </Button>
    );
  }
  if (progress !== undefined) {
    return (
      <Button
        size={size}
        variant="soft"
        loading
        className={cn('tabular-nums', className)}
        aria-live="polite"
      >
        {progress == null
          ? t('install.installing')
          : t('install.installingPct', { pct: formatPercent(progress, i18n.language) })}
      </Button>
    );
  }
  if (isInstalledOn(item, target) && !versionId) {
    return (
      <Button
        size={size}
        variant="ghost"
        disabled
        leftIcon={<Check />}
        className={cn('text-success-text opacity-100', className)}
      >
        {t('install.installed')}
      </Button>
    );
  }
  if (!target) return disabledWithReason(t('install.install'), t('install.noTarget'));
  if (incompatible) return disabledWithReason(t('install.install'), compat.detail);

  const run = (worldId?: string) =>
    install.mutate({
      projectId: item.id,
      name: item.name,
      targetId: target.id,
      targetName,
      target: toInstallTarget(target, item.kind, worldId),
      versionId,
    });

  if (item.kind === 'datapack' && target.kind === 'instance') {
    return (
      <WorldPicker
        instanceId={target.id}
        size={size}
        className={className}
        loading={install.isPending}
        onPick={run}
      />
    );
  }
  return (
    <Button
      size={size}
      variant="primary"
      leftIcon={<Download />}
      loading={install.isPending}
      className={className}
      onClick={() => run()}
      aria-label={t('install.installAria', { name: item.name, target: targetName })}
    >
      {t('install.install')}
    </Button>
  );
}

/** Datapacks install into one world of the instance. */
function WorldPicker({
  instanceId,
  onPick,
  size,
  className,
  loading,
}: {
  instanceId: string;
  onPick: (worldId: string) => void;
  size: 'sm' | 'md' | 'lg';
  className?: string;
  loading: boolean;
}) {
  const { t } = useTranslation('content');
  const { data: worlds, isLoading } = useWorlds(instanceId);
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          size={size}
          variant="primary"
          leftIcon={<Globe2 />}
          loading={loading}
          className={className}
        >
          {t('install.chooseWorld')}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-72 p-2">
        <p className="text-fg-muted px-2 pt-1 pb-2 text-xs font-semibold">
          {t('datapacks.pickWorld')}
        </p>
        {isLoading && <Skeleton className="m-1 h-10" />}
        {worlds?.length === 0 && (
          <p className="text-fg-muted px-2 py-3 text-sm">{t('datapacks.noWorlds')}</p>
        )}
        <ul className="flex flex-col">
          {worlds?.map((w) => (
            <li key={w.id}>
              <button
                type="button"
                onClick={() => onPick(w.id)}
                className="rx-focus-ring hover:bg-surface-3 flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left text-sm"
              >
                <Layers className="text-fg-subtle size-4" />
                <span className="flex-1 truncate">{w.name}</span>
                <span className="text-2xs text-fg-subtle">
                  {t('datapacks.count', { count: w.datapack_count })}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}

/** Banner shown when no Rustic Server server can receive plugins. */
export function NoRusticServer() {
  const { t } = useTranslation('content');
  return (
    <div
      className="border-border flex flex-col items-start gap-3 rounded-xl border border-dashed p-6"
      role="note"
    >
      <p className="text-base font-semibold">{t('plugins.noServerTitle')}</p>
      <p className="text-fg-muted max-w-xl text-sm">{t('plugins.noServerBody')}</p>
      <Button asChild variant="primary" size="sm">
        <Link to="/servers/new">{t('plugins.createServer')}</Link>
      </Button>
    </div>
  );
}
