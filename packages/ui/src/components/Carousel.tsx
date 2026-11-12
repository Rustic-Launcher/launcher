import { AnimatePresence, motion } from 'motion/react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { cn } from '../lib/cn';
import { usePrefersReducedMotion } from '../lib/useReducedMotion';

export interface CarouselProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => ReactNode;
  getKey: (item: T) => string;
  /** Accessible name of the carousel region. */
  label: string;
  prevLabel: string;
  nextLabel: string;
  /** e.g. (i, n) => `Slide ${i} of ${n}` */
  slideLabel: (index: number, total: number) => string;
  autoPlayMs?: number;
  className?: string;
}

/** Accessible slideshow (APG carousel pattern): pauses on hover/focus and with reduced motion. */
export function Carousel<T>({
  items,
  renderItem,
  getKey,
  label,
  prevLabel,
  nextLabel,
  slideLabel,
  autoPlayMs = 7000,
  className,
}: CarouselProps<T>) {
  const [index, setIndex] = useState(0);
  const [dir, setDir] = useState(1);
  const [paused, setPaused] = useState(false);
  const reduced = usePrefersReducedMotion();
  const n = items.length;

  const go = useCallback(
    (to: number, d: number) => {
      if (n === 0) return;
      setDir(d);
      setIndex(((to % n) + n) % n);
    },
    [n],
  );

  useEffect(() => {
    if (paused || reduced || n < 2 || !autoPlayMs) return;
    const t = setTimeout(() => go(index + 1, 1), autoPlayMs);
    return () => clearTimeout(t);
  }, [index, paused, reduced, n, autoPlayMs, go]);

  const item = items[index];
  if (!item) return null;

  return (
    <section
      aria-roledescription="carousel"
      aria-label={label}
      className={cn('group relative overflow-hidden rounded-2xl', className)}
      onPointerEnter={() => setPaused(true)}
      onPointerLeave={() => setPaused(false)}
      onFocus={() => setPaused(true)}
      onBlur={() => setPaused(false)}
      onKeyDown={(e) => {
        if (e.key === 'ArrowRight') go(index + 1, 1);
        if (e.key === 'ArrowLeft') go(index - 1, -1);
      }}
    >
      <div className="relative h-full" aria-live={paused ? 'polite' : 'off'}>
        <AnimatePresence initial={false} custom={dir} mode="popLayout">
          <motion.div
            key={getKey(item)}
            role="group"
            aria-roledescription="slide"
            aria-label={slideLabel(index + 1, n)}
            custom={dir}
            initial={reduced ? { opacity: 0 } : { opacity: 0, x: dir * 40 }}
            animate={{ opacity: 1, x: 0 }}
            exit={reduced ? { opacity: 0 } : { opacity: 0, x: dir * -40 }}
            transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
            className="h-full"
          >
            {renderItem(item, index)}
          </motion.div>
        </AnimatePresence>
      </div>
      {n > 1 && (
        <>
          <button
            type="button"
            aria-label={prevLabel}
            onClick={() => go(index - 1, -1)}
            className="rx-focus-ring rx-glass text-fg absolute top-1/2 left-3 grid size-9 -translate-y-1/2 place-items-center rounded-full opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
          >
            <ChevronLeft className="size-4.5" />
          </button>
          <button
            type="button"
            aria-label={nextLabel}
            onClick={() => go(index + 1, 1)}
            className="rx-focus-ring rx-glass text-fg absolute top-1/2 right-3 grid size-9 -translate-y-1/2 place-items-center rounded-full opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
          >
            <ChevronRight className="size-4.5" />
          </button>
          <div className="absolute right-5 bottom-4 flex gap-1.5">
            {items.map((it, i) => (
              <button
                key={getKey(it)}
                type="button"
                aria-label={slideLabel(i + 1, n)}
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
