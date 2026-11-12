import {
  createElement,
  forwardRef,
  useCallback,
  useEffect,
  useRef,
  type HTMLAttributes,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from 'react';
import { cn } from '../lib/cn';

/**
 * The premium card (one system for the website and the launcher): layered glass with grain and an
 * inner highlight, a 1 px gradient border (accent → amber → transparent), a conic sweep that rotates
 * around the border on hover, a cursor spotlight (`--mx` / `--my`, shared across a
 * {@link PremiumGrid}), a 3D tilt with depth layers (`data-depth="1..3"` children), an accent bloom on
 * hover and a mechanical settle when the pointer leaves. Everything animates transform, opacity or
 * background position only. No tilt on touch, under 900 px or with reduced motion.
 */
export type PremiumCardVariant = 'default' | 'featured' | 'accent' | 'compact';

export interface PremiumCardProps extends HTMLAttributes<HTMLElement> {
  variant?: PremiumCardVariant;
  /** 3D tilt + spotlight following the pointer (default true). */
  interactive?: boolean;
  /** Max tilt in degrees (default 6). */
  tilt?: number;
  as?: 'div' | 'article' | 'li' | 'section' | 'aside' | 'figure';
  /** Classes of the content box (layout, padding: `p-*` overrides the default 1.5 rem). */
  bodyClassName?: string;
  children?: ReactNode;
}

const canTilt = () =>
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(hover: hover) and (pointer: fine) and (min-width: 900px)').matches &&
  !window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Writes the pointer position relative to `el` as `--mx` / `--my` (px) and `--mo` (0/1). */
export function setSpotlight(el: HTMLElement, clientX: number, clientY: number) {
  const r = el.getBoundingClientRect();
  el.style.setProperty('--mx', `${clientX - r.left}px`);
  el.style.setProperty('--my', `${clientY - r.top}px`);
}

export const PremiumCard = forwardRef<HTMLElement, PremiumCardProps>(function PremiumCard(
  {
    variant = 'default',
    interactive = true,
    tilt = 6,
    as = 'div',
    className,
    bodyClassName,
    children,
    onPointerMove,
    onPointerLeave,
    ...props
  },
  ref,
) {
  const local = useRef<HTMLElement | null>(null);
  const frame = useRef(0);
  const setRef = useCallback(
    (el: HTMLElement | null) => {
      local.current = el;
      if (typeof ref === 'function') ref(el);
      else if (ref) ref.current = el;
    },
    [ref],
  );
  useEffect(() => () => cancelAnimationFrame(frame.current), []);

  const move = (e: ReactPointerEvent<HTMLElement>) => {
    onPointerMove?.(e);
    const el = local.current;
    if (!interactive || !el) return;
    const { clientX, clientY } = e;
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      setSpotlight(el, clientX, clientY);
      if (!canTilt()) return;
      const r = el.getBoundingClientRect();
      const px = (clientX - r.left) / r.width - 0.5;
      const py = (clientY - r.top) / r.height - 0.5;
      el.dataset.tilting = '';
      el.style.setProperty('--rx', `${(-py * tilt).toFixed(2)}deg`);
      el.style.setProperty('--ry', `${(px * tilt).toFixed(2)}deg`);
    });
  };
  const leave = (e: ReactPointerEvent<HTMLElement>) => {
    onPointerLeave?.(e);
    const el = local.current;
    if (!el) return;
    cancelAnimationFrame(frame.current);
    delete el.dataset.tilting;
    el.style.setProperty('--rx', '0deg');
    el.style.setProperty('--ry', '0deg');
  };

  return createElement(
    as,
    {
      ref: setRef,
      ...props,
      'data-variant': variant,
      'data-interactive': interactive ? '' : undefined,
      className: cn('rx-pcard', className),
      onPointerMove: move,
      onPointerLeave: leave,
    },
    <span className="rx-pcard-glass" aria-hidden />,
    <span className="rx-pcard-spot" aria-hidden />,
    <span className="rx-pcard-edge" aria-hidden />,
    <div className={cn('rx-pcard-body', bodyClassName)}>{children}</div>,
  );
});

/**
 * A grid whose cards share one spotlight: moving over the grid lights the borders of the
 * neighbouring cards too (each gets `--mx` / `--my` relative to itself).
 */
export interface PremiumGridProps extends HTMLAttributes<HTMLElement> {
  as?: 'div' | 'ul' | 'ol' | 'section';
}

export const PremiumGrid = forwardRef<HTMLElement, PremiumGridProps>(function PremiumGrid(
  { as = 'div', className, onPointerMove, onPointerLeave, ...props },
  ref,
) {
  const frame = useRef(0);
  useEffect(() => () => cancelAnimationFrame(frame.current), []);
  return createElement(as, {
    ref,
    ...props,
    className: cn('rx-pgrid', className),
    onPointerMove: (e: ReactPointerEvent<HTMLElement>) => {
      onPointerMove?.(e);
      const grid = e.currentTarget;
      const { clientX, clientY } = e;
      cancelAnimationFrame(frame.current);
      frame.current = requestAnimationFrame(() => {
        grid.dataset.lit = '';
        grid
          .querySelectorAll<HTMLElement>('.rx-pcard')
          .forEach((card) => setSpotlight(card, clientX, clientY));
      });
    },
    onPointerLeave: (e: ReactPointerEvent<HTMLElement>) => {
      onPointerLeave?.(e);
      cancelAnimationFrame(frame.current);
      delete e.currentTarget.dataset.lit;
    },
  });
});
