import { cn } from '../lib/cn';
import accent from '../assets/rustic-wordmark-accent.png';
import letters from '../assets/rustic-wordmark-letters.png';

/** Width over height of the wordmark (`assets/brand/rustic-wordmark.png`, cropped to the ink). */
export const RUSTIC_WORDMARK_RATIO = 726 / 192;

/**
 * The official Rustic wordmark: the "R" mark and the name, as one picture (source of truth:
 * `launcher/assets/brand/rustic-wordmark.png`). Two layers so it follows the theme like
 * {@link RusticLogo}: the letters are a mask painted with `currentColor` (white on dark surfaces,
 * `#15161A` on light ones) and the orange accents keep their colour.
 */
export function RusticWordmark({
  height = 20,
  title = 'Rustic',
  className,
}: {
  /** Height in CSS pixels; the width follows the wordmark's proportions. */
  height?: number;
  /** Accessible name; `null` hides it from assistive technology (a labelled parent). */
  title?: string | null;
  className?: string;
}) {
  const mask = `url("${letters}") center / 100% 100% no-repeat`;
  return (
    <span
      role={title ? 'img' : undefined}
      aria-label={title ?? undefined}
      aria-hidden={title ? undefined : true}
      className={cn(
        'light:text-[#15161A] relative inline-block shrink-0 text-white select-none',
        className,
      )}
      style={{ height, width: Math.round(height * RUSTIC_WORDMARK_RATIO) }}
      data-rustic-wordmark
    >
      <span
        className="absolute inset-0 bg-current"
        style={{ mask, WebkitMask: mask }}
        data-wordmark-letters
      />
      <img
        src={accent}
        alt=""
        draggable={false}
        className="pointer-events-none absolute inset-0 size-full"
        data-wordmark-accent
      />
    </span>
  );
}
