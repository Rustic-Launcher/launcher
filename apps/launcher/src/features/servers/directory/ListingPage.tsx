import { Badge, Button, Dialog, Field, Select, Skeleton, Textarea, toast } from '@rustic/ui';
import { useMutation } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import {
  ArrowLeft,
  BadgeCheck,
  BookmarkCheck,
  BookmarkPlus,
  Copy,
  ExternalLink,
  Flag,
  Gamepad2,
  Globe2,
  MessageCircle,
  Puzzle,
  ServerOff,
  Signal,
  Star,
  Users,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CatalogCover, CatalogEmpty, CatalogThumb, Chip, OverlayBadge } from '@/components/catalog';
import { cloud } from '@/cloud/client';
import { errorCode } from '@/cloud/errors';
import type { ContentReportReason, ServerListing } from '@/cloud/types';
import { Markdown } from '@/features/content/Markdown';
import { invoke } from '@/ipc';
import { formatCompact, formatNumber, formatRelative } from '@/lib/format';
import { copyText, useMultiplayerActions, usePing } from '../list/queries';
import { listingRoute } from '../routes';
import { PlayButton } from './cards';
import { listingWebUrl, useJoinListing, useListing } from './queries';
import { languageName } from './vocab';

const REASONS: ContentReportReason[] = [
  'impersonation',
  'spam',
  'inappropriate_content',
  'cheating',
  'harassment',
  'copyright',
  'malware',
  'other',
];

function ReportDialog({
  listing,
  open,
  onOpenChange,
}: {
  listing: ServerListing;
  open: boolean;
  onOpenChange: (o: boolean) => void;
}) {
  const { t } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const [reason, setReason] = useState<ContentReportReason>('impersonation');
  const [details, setDetails] = useState('');
  const send = useMutation({
    mutationFn: async () =>
      (await cloud()).reportServerListing(listing.id, reason, details.trim() || undefined),
    onSuccess: () => {
      onOpenChange(false);
      setDetails('');
      toast({ tone: 'success', title: t('directory.report.sent') });
    },
    onError: (e) =>
      toast({
        tone: 'error',
        title:
          errorCode(e) === 'unauthorized'
            ? t('directory.report.signIn')
            : t('directory.report.failed'),
      }),
  });
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      tone="destructive"
      icon={<Flag />}
      title={t('directory.report.title', { name: listing.name })}
      description={t('directory.report.lead')}
      closeLabel={tc('actions.close')}
      footer={
        <>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {tc('actions.cancel')}
          </Button>
          <Button
            variant="danger"
            loading={send.isPending}
            onClick={() => send.mutate()}
            data-testid="listing-report-send"
          >
            {t('directory.report.send')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <Field label={t('directory.report.reason')}>
          {(p) => (
            <Select
              id={p.id}
              aria-label={t('directory.report.reason')}
              value={reason}
              onValueChange={(v) => setReason(v as ContentReportReason)}
              options={REASONS.map((r) => ({
                value: r,
                label: t(`directory.report.reasons.${r}`),
              }))}
            />
          )}
        </Field>
        <Field label={t('directory.report.details')} hint={t('directory.report.detailsHint')}>
          {(p) => (
            <Textarea
              {...p}
              rows={4}
              maxLength={2000}
              value={details}
              onChange={(e) => setDetails(e.target.value)}
            />
          )}
        </Field>
      </div>
    </Dialog>
  );
}

/** The live status read from this computer (the game's server list ping). */
function LiveStatus({ listing }: { listing: ServerListing }) {
  const { t, i18n } = useTranslation('servers');
  const lang = i18n.language;
  const { data, isLoading } = usePing(listing.address);
  const cloudStatus = listing.status;
  return (
    <section
      aria-labelledby="listing-status"
      className="rx-surface flex flex-col gap-3 rounded-xl p-4"
      data-testid="listing-status"
    >
      <h2
        id="listing-status"
        className="text-fg-subtle text-xs font-semibold tracking-wider uppercase"
      >
        {t('directory.page.status')}
      </h2>
      {isLoading ? (
        <Skeleton className="h-10 w-full" />
      ) : data?.online ? (
        <>
          <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-sm tabular-nums">
            <span className="text-success-text inline-flex items-center gap-1.5 font-semibold">
              <Users className="size-4" aria-hidden />
              {t('multiplayer.players', {
                online: formatNumber(data.players_online, lang),
                max: formatNumber(data.players_max, lang),
              })}
            </span>
            {data.latency_ms != null && (
              <span className="text-fg-muted inline-flex items-center gap-1.5">
                <Signal className="size-4" aria-hidden />
                {t('multiplayer.ping', { ms: formatNumber(data.latency_ms, lang) })}
              </span>
            )}
            {data.version && <span className="text-fg-subtle">{data.version}</span>}
          </div>
          {data.motd && (
            <p className="bg-surface-2 rounded-lg px-3 py-2 font-mono text-xs">{data.motd}</p>
          )}
        </>
      ) : (
        <p className="text-fg-muted text-sm">
          {t('multiplayer.unreachable')}
          {cloudStatus?.online
            ? ` · ${t('directory.page.cloudSaw', {
                count: cloudStatus.players_online ?? 0,
                value: formatCompact(cloudStatus.players_online ?? 0, lang),
                when: formatRelative(cloudStatus.checked_at, lang),
              })}`
            : ''}
        </p>
      )}
    </section>
  );
}

/** One server of the directory (`/servers/listing/<slug>`, `rustic://server/<slug>`). */
export function ListingPage() {
  const { t, i18n } = useTranslation('servers');
  const { t: tc } = useTranslation();
  const { slug } = listingRoute.useParams();
  const q = useListing(slug);
  const { inList } = useJoinListing();
  const { add } = useMultiplayerActions();
  const [reporting, setReporting] = useState(false);
  const l = q.data;

  const back = (
    <Button asChild variant="ghost" size="sm" className="self-start">
      <Link to="/servers" search={{ tab: 'discover' }}>
        <ArrowLeft />
        {t('directory.page.back')}
      </Link>
    </Button>
  );

  if (q.isLoading) {
    return (
      <div className="flex flex-col gap-4 p-4" aria-busy="true">
        {back}
        <Skeleton className="h-[clamp(200px,28vw,320px)] w-full rounded-2xl" />
        <Skeleton className="h-8 w-72" />
        <Skeleton className="h-24 w-full" />
      </div>
    );
  }
  if (q.isError || !l) {
    return (
      <div className="flex flex-col gap-4 p-4">
        {back}
        <CatalogEmpty
          icon={<ServerOff />}
          testId="listing-missing"
          title={t('directory.page.missing')}
          description={t('directory.page.missingHint')}
        />
      </div>
    );
  }

  const saved = inList(l);
  return (
    <div className="rx-page-enter flex min-h-full flex-col gap-5 p-4" data-testid="listing-page">
      {back}
      <CatalogCover
        src={l.banner_url}
        seed={l.slug}
        className="h-[clamp(200px,28vw,320px)] w-full rounded-2xl"
      >
        <span
          aria-hidden
          className="absolute inset-0 bg-gradient-to-t from-black/70 via-black/20 to-transparent"
        />
        <div className="absolute inset-x-5 top-4 flex flex-wrap gap-1.5">
          {l.featured && (
            <OverlayBadge tone="accent" icon={<Star />}>
              {t('directory.featured')}
            </OverlayBadge>
          )}
          {l.verified && (
            <OverlayBadge tone="success" icon={<BadgeCheck />}>
              {t('directory.verified')}
            </OverlayBadge>
          )}
          {l.engine === 'rustic' && (
            <OverlayBadge icon={<Puzzle />}>{t('list.engine.rustic')}</OverlayBadge>
          )}
        </div>
        <div className="absolute inset-x-5 bottom-5 flex items-end gap-4 text-white">
          <CatalogThumb
            src={l.icon_url}
            seed={l.slug}
            name={l.name}
            size="xl"
            className="ring-2 ring-white/25"
          />
          <div className="flex min-w-0 flex-col gap-1">
            <h1
              className="truncate text-3xl font-bold tracking-tight drop-shadow"
              data-testid="listing-name"
            >
              {l.name}
            </h1>
            <p className="line-clamp-2 max-w-2xl text-sm text-white/85">{l.summary}</p>
          </div>
        </div>
      </CatalogCover>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="flex min-w-0 flex-col gap-5">
          <div className="flex flex-wrap items-center gap-1.5">
            {l.tags.map((tag) => (
              <Chip key={tag}>
                {t(`directory.tags.${tag as 'survival'}`, { defaultValue: tag })}
              </Chip>
            ))}
            {l.requires_rustic_edition && (
              <Badge variant="accent">{t('directory.page.rusticEdition')}</Badge>
            )}
          </div>
          <LiveStatus listing={l} />
          <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-fg-subtle">{t('directory.page.engine')}</dt>
              <dd className="font-semibold">{t(`list.engine.${l.engine}`)}</dd>
            </div>
            <div>
              <dt className="text-fg-subtle">{t('directory.page.version')}</dt>
              <dd className="font-semibold">{l.game_version ?? l.status?.version ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-fg-subtle">{t('directory.page.languages')}</dt>
              <dd className="font-semibold">
                {l.languages.map((c) => languageName(c, i18n.language)).join(', ')}
              </dd>
            </div>
            <div>
              <dt className="text-fg-subtle">{t('directory.page.plays')}</dt>
              <dd className="font-semibold tabular-nums">
                {formatCompact(l.plays_7d, i18n.language)}
              </dd>
            </div>
          </dl>
          <section aria-labelledby="listing-about" className="flex flex-col gap-2">
            <h2 id="listing-about" className="text-base font-semibold">
              {t('directory.page.about')}
            </h2>
            {l.description?.trim() ? (
              <div
                className="text-fg-muted flex flex-col gap-2 text-sm leading-relaxed"
                data-testid="listing-description"
              >
                <Markdown source={l.description} />
              </div>
            ) : (
              <p className="text-fg-subtle text-sm">{t('directory.page.noDescription')}</p>
            )}
          </section>
        </div>

        <aside className="flex flex-col gap-3">
          <div className="rx-surface flex flex-col gap-3 rounded-xl p-4">
            <span className="text-fg-subtle text-xs font-semibold tracking-wider uppercase">
              {t('directory.page.address')}
            </span>
            <div className="bg-surface-2 flex items-center gap-2 rounded-lg py-1.5 pr-1.5 pl-3">
              <code
                className="min-w-0 flex-1 truncate font-mono text-sm"
                data-testid="listing-address"
              >
                {l.address}
              </code>
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<Copy />}
                onClick={() =>
                  void copyText(l.address).then(
                    (ok) => ok && toast({ tone: 'success', title: tc('actions.copied') }),
                  )
                }
                data-testid="listing-copy"
              >
                {t('directory.page.copy')}
              </Button>
            </div>
            <PlayButton listing={l} size="lg" testId="listing-play" block />
            <Button
              variant="secondary"
              leftIcon={saved ? <BookmarkCheck /> : <BookmarkPlus />}
              disabled={saved || add.isPending}
              onClick={() =>
                add.mutate(
                  { name: l.name, address: l.address },
                  {
                    onSuccess: () =>
                      toast({ tone: 'success', title: t('multiplayer.added', { name: l.name }) }),
                  },
                )
              }
              data-testid="listing-save"
            >
              {saved ? t('list.featured.inList') : t('list.featured.save')}
            </Button>
            <Button
              variant="ghost"
              leftIcon={<ExternalLink />}
              onClick={() => void invoke('app_open_url', { url: listingWebUrl(l.slug) })}
            >
              {t('directory.page.openSite')}
            </Button>
          </div>
          {(l.website || l.discord) && (
            <div className="rx-surface flex flex-col gap-1 rounded-xl p-3">
              {l.website && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="justify-start"
                  leftIcon={<Globe2 />}
                  onClick={() => void invoke('app_open_url', { url: l.website! })}
                >
                  {t('directory.page.website')}
                </Button>
              )}
              {l.discord && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="justify-start"
                  leftIcon={<MessageCircle />}
                  onClick={() => void invoke('app_open_url', { url: l.discord! })}
                >
                  {t('directory.page.discord')}
                </Button>
              )}
            </div>
          )}
          <div className="text-fg-subtle flex items-center justify-between gap-2 px-1 text-xs">
            <span className="inline-flex items-center gap-1">
              <Gamepad2 className="size-3.5" aria-hidden />
              {l.owner
                ? t('directory.page.by', { name: l.owner.display_name })
                : t('directory.page.byStaff')}
            </span>
            <Button
              variant="ghost"
              size="sm"
              leftIcon={<Flag />}
              onClick={() => setReporting(true)}
              data-testid="listing-report"
            >
              {t('directory.page.report')}
            </Button>
          </div>
        </aside>
      </div>
      <ReportDialog listing={l} open={reporting} onOpenChange={setReporting} />
    </div>
  );
}
