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
  BadgeCheck,
  Check,
  Clock,
  Cpu,
  Download,
  Globe2,
  Layers,
  PackagePlus,
  Server as ServerIcon,
  Sparkles,
  Tag,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNativeInstall } from '@/features/native/useNativeInstall';
import { CatalogThumb } from '@/components/catalog';
import { CompatibilityBadge } from '@/components/CompatibilityBadge';
import { InstanceIcon } from '@/components/InstanceIcon';
import { useHorizonVersionLabel } from '@/components/HorizonVersion';
import type { Compatibility, ContentKind, ContentSource, ShaderFormat } from '@/ipc/types/content';
import type { Money } from '@/ipc/types/dns';
import type { Instance } from '@/ipc/types/instances';
import type { Server } from '@/ipc/types/servers';
import { formatMoney, formatPercent } from '@/lib/format';
import {
  eligibleServers,
  formatTarget,
  isInstalledOn,
  toInstallTarget,
  type TargetRef,
} from './target';
import { useCreateFromModpack, useInstall, useInstallProgress, useWorlds } from './useContent';

/** A project's icon: its picture from an allowed CDN, else initials on a hue gradient. */
export function ContentIcon({
  id,
  name,
  src,
  size = 'md',
  className,
}: {
  id: string;
  name: string;
  src?: string | null;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
}) {
  return <CatalogThumb src={src} seed={id} name={name} size={size} className={className} />;
}

/** Verified by Rustic: an admin confirmed the project matches its description. */
export function VerifiedBadge({ compact }: { compact?: boolean }) {
  const { t } = useTranslation('content');
  if (compact) {
    return (
      <Tooltip content={t('verified.hint')}>
        <BadgeCheck
          className="text-accent-text size-4 shrink-0"
          aria-label={t('verified.label')}
          role="img"
          tabIndex={0}
        />
      </Tooltip>
    );
  }
  return (
    <Tooltip content={t('verified.hint')}>
      <Badge variant="accent" icon={<BadgeCheck />} tabIndex={0}>
        {t('verified.label')}
      </Badge>
    </Tooltip>
  );
}

/** A paid project's price (creator sales are not open yet: shown, never sold). */
export function PriceBadge({ price }: { price: Money }) {
  const { t, i18n } = useTranslation('content');
  return (
    <Tooltip content={t('price.soonHint')}>
      <Badge variant="warning" icon={<Tag />} tabIndex={0}>
        {formatMoney(price, i18n.language)}
      </Badge>
    </Tooltip>
  );
}

/** A native mod: Rust compiled into the game on the player's PC. */
export function NativeBadge() {
  const { t } = useTranslation('content');
  return (
    <Tooltip content={t('native.hint')}>
      <Badge variant="info" icon={<Cpu />} tabIndex={0}>
        {t('native.badge')}
      </Badge>
    </Tooltip>
  );
}

/**
 * The compatibility badge, except for a native mod that would build on the target: "Builds on
 * your PC" instead of a format refusal (the verdict itself is the desktop side's, unchanged).
 */
export function ItemCompatBadge({
  item,
}: {
  item: { native?: boolean; compatibility: Compatibility };
}) {
  const c = item.compatibility;
  if (item.native && !(c.status === 'incompatible' && c.reason === 'horizon_version')) {
    return <NativeBadge />;
  }
  return <CompatibilityBadge compat={c} />;
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
  stacked,
}: {
  kind: ContentKind;
  value: TargetRef | null;
  onChange: (t: TargetRef) => void;
  instances: Instance[];
  servers: Server[];
  loading?: boolean;
  /** The label above a full-width select (the project page's install panel). */
  stacked?: boolean;
}) {
  const { t } = useTranslation('content');
  const versionLabel = useHorizonVersionLabel();
  if (loading) return <Skeleton className={cn('h-9', stacked ? 'w-full' : 'w-64')} />;
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
    <div className={cn('flex flex-col gap-1', stacked ? 'items-stretch' : 'items-start')}>
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
        className={stacked ? 'w-full' : 'w-64'}
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
  /** Paid (Rustic marketplace): shown with its price, never installed or sold yet. */
  price?: Money | null;
  native?: boolean;
  /** The project's source code (shown in a native mod's trust notice). */
  links?: { source?: string | null };
}

interface InstallButtonProps {
  item: InstallableItem;
  target: TargetRef | null;
  targetName: string;
  versionId?: string;
  /** Overrides the item compatibility (version rows). */
  compatibility?: Compatibility;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

/**
 * Install / Installed / Installing / Create instance, with the right disabled reasons. A native
 * mod's dialogs (trust notice, Rust) stay mounted whatever the button shows meanwhile.
 */
export function InstallButton(props: InstallButtonProps) {
  const native = useNativeInstall();
  return (
    <>
      <InstallButtonInner {...props} native={native} />
      {props.item.native && native.dialogs}
    </>
  );
}

function InstallButtonInner({
  item,
  target,
  targetName,
  versionId,
  compatibility,
  size = 'sm',
  className,
  native,
}: InstallButtonProps & { native: ReturnType<typeof useNativeInstall> }) {
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

  // Creator sales are off (ADR 0155): a paid project never gets a working button.
  if (item.price && item.price.amount_minor > 0) {
    return disabledWithReason(t('price.soon'), t('price.soonHint'), <Clock />);
  }
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
  if (item.native && incompatible) {
    // A native mod that doesn't fit this Horizon (or another target than an instance).
    return disabledWithReason(t('install.native'), compat.detail, <Cpu />);
  }
  if (incompatible) return disabledWithReason(t('install.install'), compat.detail);
  if (item.native && target.kind !== 'instance') {
    return disabledWithReason(t('install.native'), t('native.instanceOnly'), <Cpu />);
  }

  const vars = (worldId?: string) => ({
    projectId: item.id,
    name: item.name,
    targetId: target.id,
    targetName,
    target: toInstallTarget(target, item.kind, worldId),
    versionId,
  });
  const run = (worldId?: string) => {
    if (!item.native) {
      install.mutate(vars(worldId));
      return;
    }
    // Native: the trust notice, Rust, then the build (ADR 0157).
    void native.run({
      instanceId: target.id,
      instanceName: targetName,
      modName: item.name,
      sourceUrl: item.links?.source,
      install: () => install.mutateAsync(vars()),
    });
  };

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
      leftIcon={item.native ? <Cpu /> : <Download />}
      loading={install.isPending || native.busy}
      className={className}
      onClick={() => run()}
      aria-label={t('install.installAria', { name: item.name, target: targetName })}
      data-testid={item.native ? 'native-install-button' : undefined}
    >
      {item.native ? t('install.native') : t('install.install')}
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
