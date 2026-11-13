/** The directory's presentations over the catalog components: card, dense row, hero slide. */
import { Badge, Button, Tooltip } from '@rustic/ui';
import { Link } from '@tanstack/react-router';
import { BadgeCheck, Eye, Flame, Globe2, Play, Puzzle, Star, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  CatalogCover,
  CatalogListRow,
  CatalogThumb,
  Chip,
  MetaItem,
  OverlayBadge,
  VisualCard,
} from '@/components/catalog';
import type { ServerListing } from '@/cloud/types';
import { formatCompact } from '@/lib/format';
import { useJoinListing } from './queries';

/** "Play": add to the list if needed, then launch and join with the active instance. */
export function PlayButton({
  listing,
  size = 'sm',
  testId,
  block = false,
}: {
  listing: ServerListing;
  size?: 'sm' | 'md' | 'lg';
  testId?: string;
  /** Full width (the server page's side panel). */
  block?: boolean;
}) {
  const { t } = useTranslation('servers');
  const { join, instance } = useJoinListing();
  const button = (
    <Button
      size={size}
      variant="primary"
      leftIcon={<Play className="fill-current" />}
      disabled={!instance}
      className={block ? 'w-full' : undefined}
      onClick={() => void join(listing)}
      aria-label={t('directory.playAria', { name: listing.name })}
      data-testid={testId ?? `listing-play-${listing.slug}`}
    >
      {t('directory.play')}
    </Button>
  );
  return (
    <Tooltip
      content={
        instance
          ? t('multiplayer.joinWith', { instance: instance.name })
          : t('multiplayer.noInstance')
      }
    >
      <span className={block ? 'block w-full' : undefined}>{button}</span>
    </Tooltip>
  );
}

/** Players online from the cloud's last ping (or "Offline"), compact. */
export function OnlineMeta({ listing }: { listing: ServerListing }) {
  const { t, i18n } = useTranslation('servers');
  const st = listing.status;
  if (!st) return null;
  if (!st.online) return <span className="text-fg-subtle">{t('multiplayer.offline')}</span>;
  return (
    <MetaItem icon={<Users />} label={t('directory.playersLabel')}>
      <span className="text-success-text">
        {t('directory.online', {
          count: st.players_online ?? 0,
          value: formatCompact(st.players_online ?? 0, i18n.language),
        })}
      </span>
    </MetaItem>
  );
}

function TagChips({ listing, max = 3 }: { listing: ServerListing; max?: number }) {
  const { t } = useTranslation('servers');
  return (
    <>
      {listing.tags.slice(0, max).map((tag) => (
        <Chip key={tag}>{t(`directory.tags.${tag as 'survival'}`, { defaultValue: tag })}</Chip>
      ))}
    </>
  );
}

function TitleLink({ listing, className }: { listing: ServerListing; className: string }) {
  return (
    <Link to="/servers/listing/$slug" params={{ slug: listing.slug }} className={className}>
      {listing.name}
    </Link>
  );
}

/** Discover rows and the Browse grid. */
export function ListingCard({
  listing,
  withAction = true,
}: {
  listing: ServerListing;
  withAction?: boolean;
}) {
  const { t } = useTranslation('servers');
  return (
    <VisualCard
      testId={`listing-card-${listing.slug}`}
      seed={listing.slug}
      title={listing.name}
      cover={listing.banner_url}
      icon={listing.icon_url}
      renderTitle={(className) => <TitleLink listing={listing} className={className} />}
      byline={
        <span className="flex items-center gap-1">
          <span className="truncate">{listing.summary}</span>
        </span>
      }
      overlay={
        <>
          {listing.verified && (
            <OverlayBadge tone="success" icon={<BadgeCheck />}>
              {t('directory.verified')}
            </OverlayBadge>
          )}
          {listing.featured && (
            <OverlayBadge tone="accent" icon={<Star />}>
              {t('directory.featured')}
            </OverlayBadge>
          )}
          {listing.engine === 'rustic' && (
            <OverlayBadge icon={<Puzzle />}>{t('list.engine.rustic')}</OverlayBadge>
          )}
        </>
      }
      meta={
        <>
          <OnlineMeta listing={listing} />
          <MetaItem icon={<Globe2 />} label={t('directory.languagesLabel')}>
            <span className="uppercase">{listing.languages.slice(0, 3).join(' ')}</span>
          </MetaItem>
        </>
      }
      action={withAction ? <PlayButton listing={listing} /> : undefined}
    />
  );
}

/** Browse's dense row. */
export function ListingRow({ listing }: { listing: ServerListing }) {
  const { t, i18n } = useTranslation('servers');
  return (
    <CatalogListRow
      testId={`listing-row-${listing.slug}`}
      thumb={
        <CatalogThumb src={listing.icon_url} seed={listing.slug} name={listing.name} size="lg" />
      }
      title={
        <TitleLink
          listing={listing}
          className="rx-focus-ring text-fg hover:text-accent-text truncate rounded text-base font-semibold"
        />
      }
      byline={<code className="font-mono text-xs">{listing.address}</code>}
      badges={
        <>
          {listing.verified && (
            <Badge variant="accent" icon={<BadgeCheck />}>
              {t('directory.verified')}
            </Badge>
          )}
          {listing.featured && (
            <Badge variant="info" icon={<Star />}>
              {t('directory.featured')}
            </Badge>
          )}
        </>
      }
      summary={listing.summary}
      chips={
        <>
          <Chip>{t(`list.engine.${listing.engine}`)}</Chip>
          <TagChips listing={listing} />
        </>
      }
      meta={
        <>
          <OnlineMeta listing={listing} />
          <MetaItem icon={<Globe2 />} label={t('directory.languagesLabel')}>
            <span className="uppercase">{listing.languages.join(' ')}</span>
          </MetaItem>
          {listing.plays_7d > 0 && (
            <MetaItem icon={<Flame />} label={t('directory.playsLabel')}>
              {formatCompact(listing.plays_7d, i18n.language)}
            </MetaItem>
          )}
        </>
      }
      actions={
        <>
          <Button asChild size="sm" variant="ghost">
            <Link to="/servers/listing/$slug" params={{ slug: listing.slug }}>
              <Eye />
              {t('directory.view')}
            </Link>
          </Button>
          <PlayButton listing={listing} />
        </>
      }
    />
  );
}

/** One slide of Discover's hero carousel. */
export function ListingHeroSlide({ listing }: { listing: ServerListing }) {
  const { t, i18n } = useTranslation('servers');
  const st = listing.status;
  return (
    <CatalogCover
      src={listing.banner_url}
      seed={listing.slug}
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
          <OverlayBadge tone="accent">
            {listing.featured ? t('directory.featured') : t('directory.hero.eyebrow')}
          </OverlayBadge>
          {listing.verified && (
            <OverlayBadge tone="success" icon={<BadgeCheck />}>
              {t('directory.verified')}
            </OverlayBadge>
          )}
          {listing.tags.slice(0, 2).map((tag) => (
            <OverlayBadge key={tag}>
              {t(`directory.tags.${tag as 'survival'}`, { defaultValue: tag })}
            </OverlayBadge>
          ))}
        </div>
        <div className="flex items-center gap-4">
          <CatalogThumb
            src={listing.icon_url}
            seed={listing.slug}
            name={listing.name}
            size="lg"
            className="ring-2 ring-white/20"
          />
          <div className="flex min-w-0 flex-col gap-0.5">
            <h3 className="truncate text-3xl font-bold tracking-tight drop-shadow">
              {listing.name}
            </h3>
            <p className="truncate text-sm text-white/80">
              {st?.online
                ? t('directory.online', {
                    count: st.players_online ?? 0,
                    value: formatCompact(st.players_online ?? 0, i18n.language),
                  })
                : st
                  ? t('multiplayer.offline')
                  : listing.address}
              <span aria-hidden> · </span>
              <span className="uppercase">{listing.languages.join(' ')}</span>
            </p>
          </div>
        </div>
        <p className="line-clamp-2 max-w-xl text-sm text-white/85">{listing.summary}</p>
        <div className="flex flex-wrap items-center gap-2 pt-1">
          <PlayButton listing={listing} size="md" testId={`hero-play-${listing.slug}`} />
          <Button
            asChild
            size="md"
            variant="secondary"
            className="border-white/15 bg-white/10 text-white hover:bg-white/20"
          >
            <Link to="/servers/listing/$slug" params={{ slug: listing.slug }}>
              <Eye />
              {t('directory.view')}
            </Link>
          </Button>
        </div>
      </div>
    </CatalogCover>
  );
}
