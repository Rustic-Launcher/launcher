/** Explore's project presentations over the catalog components: card, dense row, hero slide. */
import { Badge, Button, formatBytes, Tooltip } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import {
  ArrowUpCircle,
  BadgeCheck,
  CalendarClock,
  Download,
  Eye,
  HardDrive,
  Star,
  Tag,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  CatalogCover,
  CatalogListRow,
  Chip,
  MetaItem,
  OverlayBadge,
  VisualCard,
} from '@/components/catalog';
import type { ContentSummary } from '@/ipc/types/content';
import type { InstalledContent } from '@/ipc/types/instances';
import { formatCompact, formatMoney, formatRelative, shortVersion } from '@/lib/format';
import {
  ContentIcon,
  InstallButton,
  ItemCompatBadge,
  PriceBadge,
  ShaderFormatBadge,
  SourceBadge,
  VerifiedBadge,
} from './parts';
import { formatTarget, type TargetRef } from './target';

/** Where an item installs and what is installed there already (Browse, Discover). */
export interface InstallContext {
  target: TargetRef | null;
  targetName: string;
  /** Installed entries of the target instance, by project id (for "Update"). */
  installed?: Map<string, InstalledContent>;
  onUpdate?: (entry: InstalledContent) => void;
  updating?: boolean;
}

function detailSearch(target: TargetRef | null) {
  return { target: target ? formatTarget(target) : undefined };
}

function CategoryChips({ item, max = 3 }: { item: ContentSummary; max?: number }) {
  const { t } = useTranslation('content');
  return (
    <>
      {item.categories.slice(0, max).map((c) => (
        <Chip key={c}>{t(`categories.${c as 'utility'}`, { defaultValue: c })}</Chip>
      ))}
    </>
  );
}

/** Install / Update / Installed for one item, in a row or a card. */
export function ItemAction({
  item,
  ctx,
  size = 'sm',
}: {
  item: ContentSummary;
  ctx: InstallContext;
  size?: 'sm' | 'md';
}) {
  const { t } = useTranslation('content');
  const entry = ctx.installed?.get(item.id);
  if (entry?.update_available && ctx.onUpdate) {
    return (
      <Button
        size={size}
        variant="soft"
        leftIcon={<ArrowUpCircle />}
        loading={ctx.updating}
        onClick={() => ctx.onUpdate?.(entry)}
        aria-label={t('install.updateAria', { name: item.name, version: entry.update_available })}
      >
        {t('install.update')}
      </Button>
    );
  }
  return <InstallButton item={item} target={ctx.target} targetName={ctx.targetName} size={size} />;
}

/** Browse's dense row. */
export function ContentRow({ item, ctx }: { item: ContentSummary; ctx: InstallContext }) {
  const { t, i18n } = useTranslation('content');
  const lang = i18n.language;
  const search = detailSearch(ctx.target);
  return (
    <CatalogListRow
      testId={`content-row-${item.id}`}
      thumb={<ContentIcon id={item.id} name={item.name} src={item.icon_url} size="lg" />}
      title={
        <Link
          to="/content/$projectId"
          params={{ projectId: item.id }}
          search={search}
          className="rx-focus-ring text-fg hover:text-accent-text truncate rounded text-base font-semibold"
        >
          {item.name}
        </Link>
      }
      byline={item.author ? t('by', { author: item.author }) : undefined}
      badges={
        <>
          {item.verified && <VerifiedBadge compact />}
          {item.featured && !item.verified && (
            <Badge variant="accent" icon={<Star />}>
              {t('featured')}
            </Badge>
          )}
        </>
      }
      summary={item.summary}
      chips={
        <>
          <ItemCompatBadge item={item} />
          <SourceBadge source={item.source} />
          {item.shader_format && <ShaderFormatBadge format={item.shader_format} />}
          {item.price && <PriceBadge price={item.price} />}
          <CategoryChips item={item} />
        </>
      }
      meta={
        <>
          <MetaItem icon={<Download />} label={t('stats.downloads')}>
            {formatCompact(item.downloads, lang)}
          </MetaItem>
          {item.updated_at && (
            <MetaItem icon={<CalendarClock />} label={t('stats.updated')}>
              {formatRelative(item.updated_at, lang)}
            </MetaItem>
          )}
          {item.size_bytes ? (
            <MetaItem icon={<HardDrive />} label={t('stats.size')}>
              {formatBytes(item.size_bytes, lang)}
            </MetaItem>
          ) : null}
          {item.game_versions?.length ? (
            <MetaItem label={t('stats.gameVersion')}>
              {item.game_versions.slice(0, 2).join(', ')}
            </MetaItem>
          ) : null}
          {item.latest_version && (
            <MetaItem label={t('stats.version')}>v{shortVersion(item.latest_version)}</MetaItem>
          )}
        </>
      }
      actions={
        <>
          <Button asChild size="sm" variant="ghost">
            <Link to="/content/$projectId" params={{ projectId: item.id }} search={search}>
              <Eye />
              {t('view')}
            </Link>
          </Button>
          <ItemAction item={item} ctx={ctx} />
        </>
      }
    />
  );
}

/** Discover rows and the Browse grid. */
export function ContentCard({
  item,
  ctx,
  withAction,
}: {
  item: ContentSummary;
  ctx: InstallContext;
  withAction?: boolean;
}) {
  const { t, i18n } = useTranslation('content');
  return (
    <VisualCard
      testId={`content-card-${item.id}`}
      seed={item.id}
      title={item.name}
      cover={item.banner_url}
      icon={item.icon_url}
      renderTitle={(className) => (
        <Link
          to="/content/$projectId"
          params={{ projectId: item.id }}
          search={detailSearch(ctx.target)}
          className={className}
        >
          {item.name}
        </Link>
      )}
      byline={
        <span className="flex items-center gap-1">
          <span className="truncate">
            {item.author ? t('by', { author: item.author }) : t(`sourceLong.${item.source}`)}
          </span>
          {item.verified && (
            <BadgeCheck
              className="text-accent-text size-3.5 shrink-0"
              aria-label={t('verified.label')}
            />
          )}
        </span>
      }
      overlay={
        <>
          <OverlayBadge tone={item.source === 'rustic' ? 'accent' : 'neutral'}>
            {t(`sources.${item.source}`)}
          </OverlayBadge>
          {item.price && (
            <OverlayBadge tone="warning" icon={<Tag />}>
              {formatMoney(item.price, i18n.language)}
            </OverlayBadge>
          )}
          {item.native && <OverlayBadge tone="info">{t('native.short')}</OverlayBadge>}
        </>
      }
      meta={
        <>
          <MetaItem icon={<Download />} label={t('stats.downloads')}>
            {formatCompact(item.downloads, i18n.language)}
          </MetaItem>
          {item.compatibility.status === 'incompatible' && !item.native ? (
            <Tooltip content={item.compatibility.detail}>
              <button
                type="button"
                className="rx-focus-ring text-danger-text relative z-10 cursor-help rounded"
              >
                {t('card.incompatible')}
              </button>
            </Tooltip>
          ) : item.compatibility.status === 'compatible' ? (
            <span className="text-success-text">{t('card.compatible')}</span>
          ) : null}
        </>
      }
      action={withAction ? <ItemAction item={item} ctx={ctx} /> : undefined}
    />
  );
}

/** One slide of Discover's hero carousel. */
export function HeroSlide({ item, ctx }: { item: ContentSummary; ctx: InstallContext }) {
  const { t, i18n } = useTranslation('content');
  const search = detailSearch(ctx.target);
  return (
    <CatalogCover
      src={item.banner_url}
      seed={item.id}
      className="h-full min-h-72"
      imgClassName="scale-[1.02]"
    >
      <span
        aria-hidden
        className="absolute inset-0 bg-gradient-to-r from-black/80 via-black/45 to-black/5"
      />
      <span
        aria-hidden
        className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent"
      />
      <div className="relative flex h-full max-w-[720px] flex-col justify-end gap-3 py-8 pr-8 pb-12 pl-16 text-white">
        <div className="flex flex-wrap items-center gap-1.5">
          <OverlayBadge tone="accent">{t('hero.eyebrow')}</OverlayBadge>
          <OverlayBadge>{t(`sources.${item.source}`)}</OverlayBadge>
          {item.verified && (
            <OverlayBadge tone="success" icon={<BadgeCheck />}>
              {t('verified.label')}
            </OverlayBadge>
          )}
          {item.categories.slice(0, 2).map((c) => (
            <OverlayBadge key={c}>
              {t(`categories.${c as 'utility'}`, { defaultValue: c })}
            </OverlayBadge>
          ))}
        </div>
        <div className="flex items-center gap-4">
          <ContentIcon
            id={item.id}
            name={item.name}
            src={item.icon_url}
            size="lg"
            className="ring-2 ring-white/20"
          />
          <div className="flex min-w-0 flex-col gap-0.5">
            <h3 className="truncate text-3xl font-bold tracking-tight drop-shadow">{item.name}</h3>
            <p className="truncate text-sm text-white/80">
              {item.author ? t('by', { author: item.author }) : t(`sourceLong.${item.source}`)}
              <span aria-hidden> · </span>
              {t('downloads', { value: formatCompact(item.downloads, i18n.language) })}
            </p>
          </div>
        </div>
        <p className="line-clamp-2 max-w-xl text-sm text-white/85">{item.summary}</p>
        <div className="flex flex-wrap items-center gap-2 pt-1">
          <ItemAction item={item} ctx={ctx} size="md" />
          <Button
            asChild
            size="md"
            variant="secondary"
            className="border-white/15 bg-white/10 text-white hover:bg-white/20"
          >
            <Link to="/content/$projectId" params={{ projectId: item.id }} search={search}>
              <Eye />
              {t('view')}
            </Link>
          </Button>
          <span className="ml-1">
            <ItemCompatBadge item={item} />
          </span>
        </div>
      </div>
    </CatalogCover>
  );
}
