/** A project's pictures: the strip on its description, the Gallery tab's grid, the lightbox. */
import { Button, cn, Dialog, IconButton } from '@rustic/ui';
import { ChevronLeft, ChevronRight, Images, Maximize2 } from 'lucide-react';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { CatalogCover } from '@/components/catalog';
import type { ContentDetail } from '@/ipc/types/content';

type Picture = ContentDetail['gallery'][number];

const seedOf = (d: ContentDetail, g: Picture) => `${d.id}-${g.hue}`;

/** One picture as a button: opens the lightbox on it. */
function Thumb({
  detail: d,
  picture: g,
  index,
  onOpen,
  more,
  className,
  caption,
}: {
  detail: ContentDetail;
  picture: Picture;
  index: number;
  onOpen: (i: number) => void;
  /** Pictures left after this one ("+3" over the last thumbnail of the strip). */
  more?: number;
  className?: string;
  caption?: boolean;
}) {
  const { t } = useTranslation('content');
  const label = `${t('detail.slide', { index: index + 1, total: d.gallery.length })}: ${g.title}`;
  return (
    <button
      type="button"
      onClick={() => onOpen(index)}
      aria-label={label}
      title={g.title}
      data-testid={`gallery-thumb-${index}`}
      className={cn(
        'group/thumb rx-focus-ring relative block overflow-hidden rounded-xl text-left',
        className,
      )}
    >
      <CatalogCover
        src={g.url}
        seed={seedOf(d, g)}
        className="aspect-video w-full"
        imgClassName="transition-transform duration-300 group-hover/thumb:scale-[1.04] motion-reduce:transition-none motion-reduce:group-hover/thumb:scale-100"
      >
        <span
          aria-hidden
          className="absolute inset-0 bg-gradient-to-t from-black/55 via-transparent to-transparent opacity-80 transition-opacity group-hover/thumb:opacity-100"
        />
        {caption && (
          <span className="absolute inset-x-3 bottom-2.5 truncate text-sm font-semibold text-white drop-shadow">
            {g.title}
          </span>
        )}
        {more ? (
          <span
            aria-hidden
            className="absolute inset-0 grid place-items-center bg-black/55 text-xl font-semibold text-white backdrop-blur-[2px]"
          >
            +{more}
          </span>
        ) : (
          <span
            aria-hidden
            className="absolute top-2 right-2 grid size-7 place-items-center rounded-lg bg-black/50 text-white opacity-0 backdrop-blur-sm transition-opacity group-hover/thumb:opacity-100 group-focus-visible/thumb:opacity-100"
          >
            <Maximize2 className="size-3.5" />
          </span>
        )}
      </CatalogCover>
    </button>
  );
}

/** The description's strip: the first pictures large enough to read, the rest behind "+N". */
export function GalleryStrip({
  detail: d,
  onOpen,
  onSeeAll,
}: {
  detail: ContentDetail;
  onOpen: (i: number) => void;
  onSeeAll: () => void;
}) {
  const { t } = useTranslation('content');
  const { t: tc } = useTranslation();
  if (d.gallery.length === 0) return null;
  const shown = d.gallery.slice(0, 3);
  const rest = d.gallery.length - shown.length;
  return (
    <section
      aria-labelledby="gallery-strip"
      className="flex flex-col gap-3"
      data-testid="gallery-strip"
    >
      <div className="flex items-center justify-between gap-3">
        <h2 id="gallery-strip" className="text-fg flex items-center gap-2 text-sm font-semibold">
          <Images className="text-fg-muted size-4" aria-hidden />
          {t('detail.tabs.gallery')}
          <span className="text-fg-subtle font-normal tabular-nums">{d.gallery.length}</span>
        </h2>
        {d.gallery.length > 1 && (
          <Button variant="ghost" size="sm" rightIcon={<ChevronRight />} onClick={onSeeAll}>
            {tc('actions.seeAll')}
          </Button>
        )}
      </div>
      <div
        className={cn(
          'grid gap-3',
          shown.length === 1 ? 'grid-cols-1' : shown.length === 2 ? 'grid-cols-2' : 'grid-cols-3',
        )}
      >
        {shown.map((g, i) => (
          <Thumb
            key={`${g.title}-${i}`}
            detail={d}
            picture={g}
            index={i}
            onOpen={onOpen}
            more={i === shown.length - 1 && rest > 0 ? rest : undefined}
          />
        ))}
      </div>
    </section>
  );
}

/** The Gallery tab: every picture with its title. */
export function GalleryGrid({
  detail: d,
  onOpen,
}: {
  detail: ContentDetail;
  onOpen: (i: number) => void;
}) {
  const { t } = useTranslation('content');
  if (d.gallery.length === 0) {
    return <p className="text-fg-muted text-sm">{t('detail.noGallery')}</p>;
  }
  return (
    <ul
      className="grid gap-4 sm:grid-cols-2 2xl:grid-cols-3"
      aria-label={t('detail.galleryLabel', { name: d.name })}
    >
      {d.gallery.map((g, i) => (
        <li key={`${g.title}-${i}`}>
          <Thumb detail={d} picture={g} index={i} onOpen={onOpen} caption />
        </li>
      ))}
    </ul>
  );
}

/** A picture full size, with previous/next (buttons, arrow keys) and the strip to jump. */
export function Lightbox({
  detail: d,
  index,
  onIndex,
  onClose,
}: {
  detail: ContentDetail;
  index: number | null;
  onIndex: (i: number) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation('content');
  const { t: tc } = useTranslation();
  const n = d.gallery.length;
  const i = index ?? 0;
  const g = d.gallery[i];
  const go = (delta: number) => onIndex((i + delta + n) % n);
  const open = index !== null && !!g;
  // Left/Right move between pictures while the lightbox is open (not inside a text field).
  useEffect(() => {
    if (!open || n < 2) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
      e.preventDefault();
      const delta = e.key === 'ArrowRight' ? 1 : -1;
      onIndex(((index ?? 0) + delta + n) % n);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, n, index, onIndex]);
  return (
    <Dialog
      open={open}
      onOpenChange={(o) => !o && onClose()}
      title={g?.title ?? ''}
      description={t('detail.slide', { index: i + 1, total: n })}
      closeLabel={tc('actions.close')}
      size="lg"
      className="max-w-5xl"
    >
      {g && (
        <div className="flex flex-col gap-3" data-testid="gallery-lightbox">
          <div className="relative overflow-hidden rounded-xl bg-black">
            <CatalogCover
              src={g.url}
              seed={seedOf(d, g)}
              className="aspect-video w-full"
              imgClassName="object-contain"
            />
            {n > 1 && (
              <>
                <IconButton
                  label={t('detail.prev')}
                  icon={<ChevronLeft />}
                  onClick={() => go(-1)}
                  data-autofocus
                  className="absolute top-1/2 left-3 -translate-y-1/2 bg-black/55 text-white backdrop-blur-sm hover:bg-black/75 hover:text-white"
                />
                <IconButton
                  label={t('detail.next')}
                  icon={<ChevronRight />}
                  onClick={() => go(1)}
                  className="absolute top-1/2 right-3 -translate-y-1/2 bg-black/55 text-white backdrop-blur-sm hover:bg-black/75 hover:text-white"
                />
              </>
            )}
          </div>
          {n > 1 && (
            <ol
              className="flex gap-2 overflow-x-auto pb-1"
              aria-label={t('detail.galleryLabel', { name: d.name })}
            >
              {d.gallery.map((p, j) => (
                <li key={`${p.title}-${j}`} className="w-28 shrink-0">
                  <button
                    type="button"
                    onClick={() => onIndex(j)}
                    aria-label={`${t('detail.slide', { index: j + 1, total: n })}: ${p.title}`}
                    aria-current={j === i ? 'true' : undefined}
                    className={cn(
                      'rx-focus-ring block w-full overflow-hidden rounded-lg ring-2 transition-opacity',
                      j === i ? 'ring-accent' : 'opacity-60 ring-transparent hover:opacity-100',
                    )}
                  >
                    <CatalogCover src={p.url} seed={seedOf(d, p)} className="aspect-video w-full" />
                  </button>
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
    </Dialog>
  );
}
