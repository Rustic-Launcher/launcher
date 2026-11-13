import { cn, useAnimationsPaused, usePrefersReducedMotion } from '@rustic/ui';
import { ChevronLeft, ChevronRight, Pause, Play } from 'lucide-react';
import { AnimatePresence, motion } from 'motion/react';
import { useCallback, useEffect, useId, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

export interface HeroCarouselProps<T> {
  items: T[];
  getKey: (item: T) => string;
  renderSlide: (item: T, index: number) => ReactNode;
  /** Accessible name of the carousel. */
  label: string;
  /** Auto-advance delay; 0 turns rotation off. */
  intervalMs?: number;
  className?: string;
  testId?: string;
}

/**
 * The large featured carousel of a catalogue page (APG carousel pattern): visible previous/next
 * arrows, dots, a pause/play button; rotation stops while the pointer or focus is inside, while
 * a game runs or the window is hidden, and never starts with reduced motion (slides then cross-fade
 * without moving). Left/Right/Home/End move between slides when focus is inside.
 */
export function HeroCarousel<T>({
  items,
  getKey,
  renderSlide,
  label,
  intervalMs = 7000,
  className,
  testId,
}: HeroCarouselProps<T>) {
  const { t } = useTranslation();
  const id = useId();
  const n = items.length;
  const [index, setIndex] = useState(0);
  const [dir, setDir] = useState(1);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [stopped, setStopped] = useState(false);
  const reduced = usePrefersReducedMotion();
  const appPaused = useAnimationsPaused();
  const rotating = !stopped && !reduced && intervalMs > 0 && n > 1;
  const resting = hovered || focused || appPaused;

  const go = useCallback(
    (to: number, d: number) => {
      if (n === 0) return;
      setDir(d);
      setIndex(((to % n) + n) % n);
    },
    [n],
  );

  // Fewer items after a refetch: stay in range.
  useEffect(() => {
    if (index >= n && n > 0) setIndex(0);
  }, [index, n]);

  useEffect(() => {
    if (!rotating || resting) return;
    const timer = setTimeout(() => go(index + 1, 1), intervalMs);
    return () => clearTimeout(timer);
  }, [index, rotating, resting, intervalMs, go]);

  const item = items[Math.min(index, n - 1)];
  if (!item) return null;

  return (
    <section
      aria-roledescription="carousel"
      aria-label={label}
      data-testid={testId}
      className={cn('group/hero relative isolate overflow-hidden rounded-2xl', className)}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setFocused(false);
      }}
      onKeyDown={(e) => {
        const tag = (e.target as HTMLElement).tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA') return;
        if (e.key === 'ArrowRight') go(index + 1, 1);
        else if (e.key === 'ArrowLeft') go(index - 1, -1);
        else if (e.key === 'Home') go(0, -1);
        else if (e.key === 'End') go(n - 1, 1);
        else return;
        e.preventDefault();
      }}
    >
      <div
        id={`${id}-slides`}
        className="relative h-full"
        aria-live={rotating && !resting ? 'off' : 'polite'}
      >
        <AnimatePresence initial={false} custom={dir} mode="popLayout">
          <motion.div
            key={getKey(item)}
            role="group"
            aria-roledescription="slide"
            aria-label={t('catalog.carousel.slide', { index: index + 1, total: n })}
            custom={dir}
            initial={reduced ? { opacity: 0 } : { opacity: 0, x: dir * 48 }}
            animate={{ opacity: 1, x: 0 }}
            exit={reduced ? { opacity: 0 } : { opacity: 0, x: dir * -48 }}
            transition={{ duration: reduced ? 0.15 : 0.35, ease: [0.22, 1, 0.36, 1] }}
            className="h-full"
          >
            {renderSlide(item, index)}
          </motion.div>
        </AnimatePresence>
      </div>
      {n > 1 && (
        <>
          <button
            type="button"
            aria-label={t('catalog.carousel.prev')}
            aria-controls={`${id}-slides`}
            onClick={() => go(index - 1, -1)}
            className="rx-focus-ring absolute top-1/2 left-3 z-10 grid size-10 -translate-y-1/2 place-items-center rounded-full bg-black/45 text-white backdrop-blur-sm transition-colors hover:bg-black/65"
            data-testid={testId ? `${testId}-prev` : undefined}
          >
            <ChevronLeft className="size-5" />
          </button>
          <button
            type="button"
            aria-label={t('catalog.carousel.next')}
            aria-controls={`${id}-slides`}
            onClick={() => go(index + 1, 1)}
            className="rx-focus-ring absolute top-1/2 right-3 z-10 grid size-10 -translate-y-1/2 place-items-center rounded-full bg-black/45 text-white backdrop-blur-sm transition-colors hover:bg-black/65"
            data-testid={testId ? `${testId}-next` : undefined}
          >
            <ChevronRight className="size-5" />
          </button>
          <div className="absolute right-4 bottom-4 z-10 flex items-center gap-2 rounded-full bg-black/40 px-2 py-1.5 backdrop-blur-sm">
            {!reduced && intervalMs > 0 && (
              <button
                type="button"
                onClick={() => setStopped((s) => !s)}
                aria-label={stopped ? t('catalog.carousel.play') : t('catalog.carousel.pause')}
                className="rx-focus-ring grid size-5 place-items-center rounded-full text-white/85 hover:text-white"
              >
                {stopped ? (
                  <Play className="size-3 fill-current" />
                ) : (
                  <Pause className="size-3 fill-current" />
                )}
              </button>
            )}
            {items.map((it, i) => (
              <button
                key={getKey(it)}
                type="button"
                aria-label={t('catalog.carousel.slide', { index: i + 1, total: n })}
                aria-current={i === index || undefined}
                onClick={() => go(i, i > index ? 1 : -1)}
                className={cn(
                  'rx-focus-ring h-1.5 rounded-full transition-all duration-200',
                  i === index ? 'w-6 bg-white' : 'w-1.5 bg-white/45 hover:bg-white/70',
                )}
              />
            ))}
          </div>
        </>
      )}
    </section>
  );
}
